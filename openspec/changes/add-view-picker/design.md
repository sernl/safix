# Design: a read verb with a picker, and a nameless `edit`

## Context

Three facts in the tree fix the shape of this change, and none of them is negotiable here.

`Secret` (`crates/safix-core/src/secret.rs:61`) has exactly two public egresses, `write_to` and `write_json_to`, and five `const _` probes asserting it implements none of `Debug`, `Display`, `Serialize`, `From<String>` or `FromStr` (`secret.rs:62-81`).
There is therefore no way to obtain `&[u8]` or `&str` from a decrypted value outside `safix-core`, which means a preview drawn by the command crate is either a new, deliberately narrow core egress or it is nothing.

The command crate touches a terminal in exactly one place: `crates/safix/src/prompt.rs`, which probes `/dev/tty` by write-opening it first and re-opening it for reading (`prompt.rs:192-201`), and toggles `LocalModes::ECHO` through a restore-on-drop guard, `Silenced` (`prompt.rs:283-305`).
There is no raw-mode helper anywhere in the tree — `ICANON`, `ISIG` and `IEXTEN` are never cleared — so a picker is new terminal code however it is written, and `rustix`'s `termios` feature is already enabled workspace-wide (`Cargo.toml:45-46`) with `#![forbid(unsafe_code)]` over the crate that would use it (`main.rs:1`).

The unknown-subcommand refusal's sentence is computed from the dispatch table (`expected_verbs`, `main.rs:191-209`) rather than written beside it, and `usage::SCAFFOLD` is held against the table in table order by a unit test (`main.rs`'s `every_verb_is_in_the_scaffold_in_the_order_the_table_declares_them`).
A new verb row is therefore not an additive edit: it moves two accepted snapshots and fails a unit test until the scaffold carries it in the same position.

One more fact shapes the signal path.
`abort::catch_signals` (`crates/safix/src/abort.rs:44-64`) sweeps the scratch registry on a self-piped thread and ends the process with `std::process::exit`, so no `Drop` implementation runs on the signal path.
A restore-on-drop terminal guard is therefore sufficient for return, error and panic, and insufficient for `SIGINT` — which is the interruption an operator of a full-screen picker is most likely to perform.

## Goals / Non-Goals

**Goals:**

Give an operator one verb that moves from "what is in here" to "show me that one" without retyping a name.
Reach the same selection from `edit`, through one function called from two places rather than two selection paths that drift.
Show the highlighted entry's value without ever placing it on disk, in scrollback, in an argument vector, or in a `String`.
Leave the terminal exactly as it was found, on every exit path a process has.
Keep `get` byte-for-byte unchanged, and keep the distinction between it and `view` stated rather than implied.

**Non-Goals:**

A dependency for the matcher or the terminal loop — decision P2 records why, and records what would have to change for that to be reopened.
A `--query` pre-filling flag, or any other test-visible selection seam: decision P9 records that the non-interactive path already exists and is `view <name>`.
Multi-selection, or acting on more than one entry per run: every verb in this command addresses one secret, and a picker returning a set would need a verb that takes one.
Search over values rather than names — matching a query against decrypted content means decrypting everything the user holds to answer one keystroke, which is the opposite of the one-value-at-a-time property decision P6 exists to hold.
Widening `Refusal::UnknownOption`'s hard-coded hint (`reporter.rs:75-79`): named in the proposal as a separate change rather than folded in, because it moves an accepted snapshot for a refusal this change does not otherwise touch.

## Decisions

### P1. A verb, not a flag on `get`

`view` is a new row in `VERBS` between `get` and `list` — the read paths in operator order — with `usage::SCAFFOLD` carrying it in the same position.

The distinction that has to survive review is not "interactive versus not": it is the stream contract.
`get` writes the value to standard output, says nothing about terminals, and is what a script calls; `view` writes the value to the terminal, needs a terminal only when it must pick, and is never the thing a pipeline calls.
Both `usage::GET` and `usage::VIEW` state it in one sentence each, and the `safix-cli` delta makes it a requirement rather than leaving it in this document alone.

**Alternative rejected**: `safix get --pick`.
It reads as the smaller change and is the larger one: `edit` needs the identical selection, and an option on `get` cannot serve `edit`, so the option shape ends with the picker living inside `get` and being reached from `edit` through a function that has nothing to do with `get`'s own body — the same shared function this change has anyway, minus the verb that makes the stream contract legible.
It also puts a terminal requirement behind a flag on the one verb whose whole documented purpose is to be pipeable.

**Alternative rejected**: extending `list` with a selection mode.
`list` prints a table and exits zero when a user holds nothing (`main.rs:328`); a picker over an empty set cannot proceed and must refuse.
One verb cannot hold both dispositions without a flag deciding which, which is the shape P1's first rejection already turned down.

### P2. No new dependency: the picker is written in-crate

`nucleo`/`nucleo-picker` are rejected, and so is every other third-party picker.
Three independent reasons, any one sufficient.

It cannot do the thing the change exists for: `nucleo-picker`'s own feature-parity table lists `--preview` as unimplemented, tracked upstream as issue #5, and its `Render` trait customises item rows rather than a side pane.
The preview would still be ours to write, so all of the dependency cost is paid for none of the feature.

Its matcher half is MPL-2.0 — `ncp-engine 0.3.0`, and `nucleo 0.5.0`/`nucleo-matcher 0.3.1` before it — and `deny.toml`'s allow list (`deny.toml:14-27`) enumerates `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `0BSD`, `ISC`, `Unicode-3.0`, `Unlicense` and `Zlib`, with its own comment stating that nothing weak-copyleft or stronger is on the list and that adding one is a decision rather than a lock update.
That list is a spec artifact (`openspec/specs/rust-supply-chain/spec.md`, "The allowed licences are enumerated"), so adopting the crate spends a spec change on a crate that cannot do the job.
`ncp-engine`'s own README additionally describes it as an unstable fork with no plans for stable releases.

It drags `crossterm 0.29 → filedescriptor 0.8 → thiserror 1.0` into a workspace on `thiserror 2.0.20` under `multiple-versions = "deny"` (`deny.toml:29-42`), requiring a third `[bans].skip` entry, plus `parking_lot` and a `rayon` thread pool, into a binary that routes every syscall through `rustix` on purpose ("safe wrappers over what would otherwise be `unsafe` in a crate that forbids it", `Cargo.toml:41-47`).

What is actually needed is small and the codebase already holds most of it: `rustix::termios` for the mode switch, the `/dev/tty` probe `prompt.rs` established, a restore-on-drop guard in `Silenced`'s shape, and a scorer over a `BTreeMap` of per-user secret names — tens of entries, not the 10⁶ nucleo's parallel matcher exists for.

**Alternative rejected**: `fuzzy-matcher 0.3.7` for the scoring half alone.
It is the one third-party candidate that passes `deny.toml` unchanged — MIT, one MIT/Apache dependency — and it was rejected because the half it supplies is the half with no hidden difficulty: a deterministic subsequence scorer with an explicit ranking rule is a page of code and a table of unit tests, whereas an opaque third-party ranking makes "why is that entry first" unanswerable from this repository, and the crate is effectively unmaintained.
Recorded rather than dismissed: if the scorer's ranking ever becomes a maintenance burden in practice, this is the crate to reach for, and no licence or duplicate-version decision blocks it.

### P3. One selection function, two callers, one filter

`crates/safix/src/picker.rs` exposes exactly one entry point:

```rust
pub(crate) enum Scope {
    /// Every entry the user holds — `view`.
    Everything,
    /// Every entry except public placements — `edit`.
    Editable,
}

pub(crate) struct Options {
    /// Whether the highlighted entry's value is decrypted and shown.
    pub preview: bool,
}

pub(crate) fn choose(
    workspace: &Workspace,
    user: &str,
    scope: Scope,
    options: Options,
) -> Result<String, Refusal>;
```

It returns a name and nothing else, so each verb keeps its own `resolve`/refuse path intact and neither verb's existing refusals move.
`Scope::Editable` filters `placement.public.is_some()` (`model.rs:168`) out of the candidate list, so `Error::PublicNotEditable` (`edit.rs:175-181`) stays reachable by name and becomes unreachable by selection — the picker does not offer a choice it would then refuse.

**Alternative rejected**: `choose` returning a `Placement` alongside the name.
It would save each caller one `workspace.resolve` call and cost the property that makes this shape safe: `view` and `edit` currently reach a placement through the same resolver every other verb uses, with `UnknownName`, `NoFileForName` and `NotAYamlPath` raised from one place.
A picker handing back a placement it read a moment earlier is a second source of that value.

### P4. `edit` reads the editor before the picker opens

`edit_command` calls `safix_core::edit::Editor::from_environment()?` itself, before it calls `picker::choose`, and `edit::run` keeps its own identical call at the top of its body (`edit.rs:173`) unedited.

The ordering is load-bearing and `edit.rs`'s own comment already says why for the named form: nothing is decrypted or staged before the editor is known.
The nameless form makes the cost concrete — a refusal after a person has browsed a list and had values decrypted for a preview is a refusal that wasted their time and decrypted values for nothing.
The duplicated call is deliberate and cheap: it reads two environment variables and allocates nothing when they are set, and leaving it only in the caller would make `edit::run`'s own ordering guarantee depend on its caller's discipline.

`Editor` and `Editor::from_environment` are already `pub` (`edit.rs:68`, `:74`), so this needs no new core surface.
Nothing is added to `edit.rs`: its `there_is_no_fallback_to_a_named_program` test `include_str!`s the file and forbids the literals `"vi"`, `"vim"`, `"nano"` and `"emacs"` anywhere in it, and a picker's key-handling code is exactly the kind of code that would trip that test by accident.

**Alternative rejected**: letting `edit::run` take an already-chosen name and moving the editor probe wholly into `edit_command`.
That moves a core-level ordering guarantee into the command crate, where a second embedder of `safix-core` would not get it.

### P5. The terminal: a `Raw` guard, a shared device constant, and a signal-path restore

A new `crates/safix/src/tty.rs` holds three things.

`pub(crate) const DEVICE: &str = "/dev/tty"`, referenced by both `tty`'s own probe and `prompt.rs`'s `open_source`, so the two openers cannot come to disagree about what they open.
`prompt.rs`'s body is otherwise unedited: its two-step probe and its stdin fallback are load-bearing for five callers and this change has no reason to move them.

`pub(crate) fn probe() -> Option<File>`, which write-opens `DEVICE` first — mirroring `prompt.rs:192-201`, which mirrors the shell runtime's `{ : >/dev/tty; }` — and then re-opens it read-write for the picker's own reads and draws.
Unlike `prompt.rs`, a failure here is a refusal rather than a fallback: `Error::PickerNeedsTerminal`.
A picker reading keystrokes from a pipe would have nothing to draw on and nothing to read, so degrading is not a lesser mode, it is an unusable one.

`pub(crate) struct Raw<'a>`, in `Silenced`'s exact shape (`prompt.rs:283-305`): `tcgetattr`, clear `LocalModes::{ECHO, ICANON, ISIG, IEXTEN}` and `InputModes::IXON`, `tcsetattr(…, OptionalActions::Now, …)`, restore in `Drop`.
It differs from `Silenced` in one respect: a terminal whose attributes cannot be read is a refusal here (`PickerNeedsTerminal`) where `prompt.rs` proceeds, for the same reason the probe refuses.
`Raw::over` also stores the saved `Termios` in a module-level `Mutex<Option<Termios>>` and clears it on drop, and `tty::restore()` applies whatever is stored and is a no-op when nothing is.

`abort::catch_signals`'s handler thread calls `crate::tty::restore()` before `scratch::interrupt`/`scratch::cleanup`.
This is the only way the terminal comes back on `SIGINT`, because that handler ends the process with `std::process::exit` and no `Drop` runs (`abort.rs:44-64`).
The ordering is restore-then-sweep so that a sweep that blocks on an in-flight `sops` subprocess cannot leave the operator looking at a raw-mode terminal while it waits.
Exit codes are untouched: 130 and 143 are still what the handler exits with, and `ISIG` being cleared inside the picker means `^C` is read as a keystroke there and mapped to `Error::SelectionCancelled` — the signal path stays the path for a `SIGINT` arriving from anywhere else.

**Alternative rejected**: a second `signal_hook::Signals` iterator installed by the picker.
Two iterators over `SIGINT` in one process is a race over which one exits first, and the one that exits first decides whether the scratch registry was swept.

**Alternative rejected**: `libc::atexit`, or restoring from a `panic` hook only.
`atexit` handlers do not run on `process::exit` from another thread in any order this code controls, and a panic hook covers the one exit path `Drop` already covers.

### P6. The preview: one value at a time, decrypted on a quiet period, never staged

Four properties, each a requirement in the `safix-cli` delta and one of them a scenario in the `plaintext-staging` delta.

*It stages nothing.*
`edit` stages because an editor is a program that takes a path; a preview is drawn by the runtime itself, so there is no path to hand anyone and `Staging::establish` is never called on this path.
The `plaintext-staging` delta says so explicitly rather than leaving it to be inferred, because that capability's first requirement enumerates what materializes plaintext — generation, editing, and the provisioning tarball — and a new plaintext-bearing path that quietly did not appear in the enumeration would read as an omission.

*It decrypts on a quiet period, not on a keystroke.*
Each preview is a `sops` subprocess (`workspace.sops().decrypt_key`), so the picker decrypts the highlighted entry only after 150 ms with no input.
A held arrow key moving through twelve entries forks one subprocess, not twelve.
The constant is named, in one place, with a comment saying it was chosen for feel rather than measured.

*It holds exactly one decrypted value.*
The previewed `Secret` is dropped — and therefore zeroized — when the highlight moves or the picker exits.
There is no cache keyed by name: a cache here is a set of live plaintexts whose size is how long the operator browsed.

*It never becomes a `String`.*
`Secret` gains one method:

```rust
pub fn preview_into(&self, sink: &mut impl Write, lines: usize, columns: usize) -> io::Result<()>
```

It writes at most `lines` lines of at most `columns` columns, replacing control bytes with a visible placeholder, appending a truncation marker when it stopped early, and rendering a value that is not valid UTF-8 as a byte-count line rather than as mojibake.
Nothing is returned, nothing escapes as bytes, and the type's five `const _` probes stay exactly as they are.

**Alternative rejected**: `Secret::to_preview_string() -> String`.
It is the same rectangle of characters and it is a heap allocation of plaintext with no zeroizing owner, reachable by every caller in the workspace forever.
The `sink` form makes the plaintext's lifetime the write's lifetime.

**Alternative rejected**: preview off by default, opt-in with `--preview`.
The preview is the feature; defaulting it off means the default `view` is `list` with arrow keys.
The disclosure risk is real — a shoulder-surfer, a shared screen, a recording — and it is answered by `--no-preview` on both verbs and by P7's alternate-region rule, which keeps the value out of scrollback even when the preview was shown.

### P7. Where it draws: `/dev/tty`, in an alternate region, cleared on exit

The picker's rows and its preview are written to the `/dev/tty` handle from P5, never to standard output or standard error.
This keeps `view <name> | cat` honest — nothing a picker drew can land in a redirected stream — and keeps the drawing from interleaving with `Terminal: Progress`'s commentary on standard error (`main.rs:69-81`).

Drawing enters the terminal's alternate screen buffer and leaves it on every exit path the `Raw` guard covers, plus the signal path P5 wires.
The reason is scrollback: a preview that draws inline leaves the plaintext in the emulator's scroll buffer, where it outlives the process, the zeroize, and the operator's attention.
The chosen value's own output on the `view` path is written after the alternate region has been left, so what remains on screen is the value the operator asked for and nothing the picker drew to find it.

**Alternative rejected**: drawing on standard error.
It is where commentary goes, so a run whose standard error is redirected to a log file would write the preview into it.

### P8. The scorer: a deterministic subsequence match with a stated ranking

Matching is case-insensitive subsequence matching over the candidate's name, scored by: number of matched-character runs (fewer is better), whether the first match is at a word boundary (`-`, `_`, `.` or the start), the index of the first match (earlier is better), and the candidate's name as the final tiebreak, so the order is total and reproducible.
An empty query matches everything in the `BTreeMap`'s own order, which is already the order `list` prints.
The ranking rule is stated in `picker.rs`'s module documentation and held by unit tests naming the property each case demonstrates, so "why is that entry first" is answerable from this repository.

Candidate counts here are per-user secret names in a `BTreeMap` — tens, not millions — so the scorer is recomputed over the whole set on every keystroke with no index and no incremental state.

**Alternative rejected**: fuzzy matching with typo tolerance (edit distance).
Secret names are short, mechanical identifiers an operator half-remembers rather than mistypes, and a distance metric makes a non-matching entry appear in a list of matches, which in a picker whose Enter key decrypts something is a worse failure than a query that matches nothing.

### P9. Argument grammar, and the disambiguation `generate` already established

`view [<user>] [<name>]` resolves its arguments exactly as `generate_command` does (`main.rs:499-524`):

- `[]` — the default user, no name: pick.
- `[only]` where `workspace.placements()?.declares(only)` — that user, no name: pick.
- `[only]` otherwise — the default user, that name: no picker.
- `[user, name]` — both named: no picker.
- anything longer — `Refusal::Usage { form: "view [<user>] [<name>]" }`.

`generate_command`'s doc comment claims that `generate` "is the only subcommand whose single optional argument could be either, because it is the only one that means something with no secret named at all".
`view` and the nameless `edit` make that sentence false; it is rewritten to name all three and to state the shared rule — a lone argument is a user when it names one, and a secret name otherwise, with an entry whose name is also a person's reachable by naming both.

An unrecognised `-`-prefixed word on `view` takes `Refusal::Usage` with `view`'s own form, following `generate_command`'s treatment rather than `edit_command`'s `Refusal::UnknownOption`, whose message hard-codes `"(expected --host or --yes)"` (`reporter.rs:75-79`).
`edit`'s own handling of unknown flags is left exactly as it is: changing it moves an accepted snapshot for a refusal this change does not otherwise touch.

There is no `--query` flag and no selection override.
The non-interactive path is `view <name>` itself, so the picker needs no test seam; P10 records how it is driven instead.

### P10. How the picker is tested: a full-duplex pty, no seam in the binary

The harness already opens a pty and wraps a run in `setsid -w` where the developer's machine has a controlling terminal, so `/dev/tty` resolves to the test's pty rather than the developer's terminal — `set_on_a_terminal` (`harness/mod.rs:1663-1702`) over `struct Pty` (`:2269-2290`).
It attaches the slave as standard input only, which is right for a prompt and wrong for a picker: a picker draws where it reads.

A sibling helper, `pick_on_a_terminal(arguments, keystrokes) -> Run`, attaches the slave as standard input **and** standard output, writes the keystrokes to the master, reads the master's output, and keeps standard error a pipe so refusals stay separable from drawing.
Nothing is added to the shipped binary for it.

The no-terminal refusal needs no pty at all: a plain `fixture.run(&["view"])` has pipes on all three streams, which is also exactly the state the nix check sandbox provides, so that case runs unmodified as a hermetic check.

**Alternative rejected**: a `SAFIX_TEST_PICK` environment variable pre-selecting an entry.
The harness already locates six support binaries through environment variables (`harness/mod.rs:86-175`), so the mechanism would fit — and every one of those names an external program to substitute, not a branch inside the binary's own logic.
A selection override is a second code path shipping to operators whose only caller is a test, and the test it enables asserts the behaviour of that path rather than of the picker.

### P11. `render::listing`'s rows are factored, not copied

The picker shows the same columns `list` shows — `NAME ORIGIN SHARED GENERATOR KEY FILE` — so `render::listing` (`render.rs:473-506`) is split: a row builder taking one `(&str, &Placement)` and returning one `Vec<String>`, and `listing` itself becoming the header plus a map over it.
The picker builds its rows through the same builder.

Two tables claiming to show the same six facts about an entry, built by two pieces of code, drift on the first column that gains a rule — the `GENERATOR` column's description/`yes`/`-` fallback is already such a rule.
`list`'s own output is unchanged by the factoring, which a snapshot already holds.

## Risks / Trade-offs

A full-screen terminal UI is new territory for this codebase, and the failure mode of getting it wrong is a terminal left in raw mode — no echo, no `^C` — after the process is gone.
That is why P5 covers four exit paths rather than three, and why the drill for it is a test that asserts the terminal's attributes are what they were, rather than a comment claiming they are.

The preview discloses plaintext to whoever can see the screen, which is a disclosure this command otherwise makes only when an operator asks for one value by name.
The mitigations are stated rather than implied: `--no-preview` on both verbs, the alternate region so the value never enters scrollback, one value at a time, and a 150 ms quiet period so scrolling past an entry does not decrypt it.

One decrypt per highlighted entry means a session browsing twenty entries runs up to twenty `sops` subprocesses, each with its own key access.
For a card-backed identity with touch-policy cached that is one touch per session rather than per entry; for a passphrase-protected identity it may be a prompt the picker cannot answer, in which case the decrypt fails and the preview pane says so while the picker stays usable — a failed preview is never a failed selection.

## Migration Plan

There is nothing to migrate.
`view` is a new verb, `edit`'s existing forms are unchanged, `get` is untouched, and no nix option a consumer writes against moves.
The only consumer-visible surprise is that `safix edit` with no arguments now opens a picker where it previously printed a usage line, which the changelog records under the widened verb rather than as a break.
