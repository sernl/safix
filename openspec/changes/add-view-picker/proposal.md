# A read verb with a picker, and a nameless `edit`

## Why

Reading a value today requires already knowing its name: `safix get <name>` is a pipe with no way to browse, and `safix list` prints a table an operator then retypes from.
An operator who holds thirty entries across a shared catalogue and two groups has no way to move from "what is in here" to "show me that one" without two commands and a copy-paste of a name they can mistype.
`safix edit` has the same gap from the other side — it refuses with a usage line when given no name, so the one verb that already reads a value before writing it cannot be reached by choosing.

## What Changes

- Add a verb, `safix view [<user>] [<name>]`, placed in the dispatch table after `get` and before `list`, and in `usage::SCAFFOLD` in that same position.
  With `<name>` it is exactly `get`'s decrypt, written to the terminal rather than offered to a pipe.
  Without `<name>` it opens a picker over every entry the user holds, decrypts the highlighted entry as a preview, and then performs the same decrypt-and-show for whatever was chosen.
  The line that keeps `view` from being a second answer to `get`'s question is stated in the help and held by a test: `get` is the pipe, `view` is the terminal.
- Widen `safix edit` to a nameless form, `edit [--allow-disk-staging] [--no-preview] [<user>] [<name>]`, which opens the same picker over the entries that user holds, minus every public placement, because a public output is refused for editing and a refusal reachable by selection is a refusal the picker should not have offered.
  `edit` reads `$VISUAL`/`$EDITOR` before the picker opens, so an operator never browses, previews, and chooses only to be told no editor was ever available.
- Add one shared selection function, `picker::choose`, in a new `crates/safix/src/picker.rs`, called from both verbs.
  It is written in-crate over `rustix::termios` — already a workspace feature — the `/dev/tty` probe `prompt.rs` established, a raw-mode guard shaped like `Silenced`, and a hand-written deterministic subsequence scorer.
  **No new crate dependency is taken**: the rejection of `nucleo`/`nucleo-picker` and the alternatives weighed against it are recorded in `design.md`, decision P2.
- Add a preview of the highlighted entry's value, on by default, with a `--no-preview` opt-out on both verbs.
  The preview decrypts after a short quiet period rather than on every keystroke, holds exactly one decrypted value at a time, drops it when the highlight moves or the picker exits, draws on `/dev/tty` in an alternate screen region that is cleared on exit rather than left in scrollback, and **never stages anything**: a preview is drawn by the runtime itself and needs no path, so `Staging::establish` is never reached on this path.
- Add a single narrow egress on `Secret` for the preview — `preview_into(&self, sink, lines, columns)` — writing at most a bounded rectangle of sanitized bytes straight into the terminal writer, so no plaintext ever becomes a `String` or a `&[u8]` outside `safix-core` and the type's existing `!Debug`/`!Display`/`!Serialize` probes stay green.
- Add three refusals, each with its own code, prose, reporter sample arm and two accepted snapshots: `Error::PickerNeedsTerminal` (a picker was needed and there is no terminal, naming both remedies — name the entry, or run `safix list`), `Error::NothingToPick { user }` (the user holds nothing, so there is no set to choose from), and `Error::SelectionCancelled` (the operator left without choosing; exit non-zero, terminal restored, nothing retained).
  `Error::NoTerminal` is deliberately not reused: its prose is enrollment's, and reusing it would print card-touch instructions at an operator who asked to read a secret.
- Regenerate the two `unknown_subcommand` snapshots, whose sentence is derived from the verb table rather than written beside it, and correct `usage::SCAFFOLD`'s and the README's "thirteen of the fourteen verbs" counts, which the new row makes false.
- **BREAKING** for the integration-check surface, not for consumers: `modules/flake/checks/cli.nix` and `modules/flake/checks/single-runtime.nix` gain new check attributes for the picker's tests; no nix option a consumer writes against changes, and no existing check attribute is renamed or removed.

Not in scope: a `--query <text>` pre-filling flag, and any test-only selection override.
A selection override would be a second code path shipping in the binary whose only caller is a test, and the non-interactive path already exists — it is `view <name>` — so the picker is exercised only through a real pseudo-terminal.
Also not in scope: `Refusal::UnknownOption`'s hard-coded `"(expected --host or --yes)"` hint, which `edit` already raises for its own flags.
`view` avoids inheriting the wart by refusing an unrecognised flag with its own usage line, the way `generate` does; widening that refusal to carry the verb's own expected options is a separate change, named here so its absence is a decision rather than an oversight.
Also not in scope: any change to `get`, which keeps its stdout contract, its silence about terminals, and its exit code byte for byte.

## Capabilities

### Modified Capabilities

- `safix-cli`: the subcommand list gains `view` (and is corrected — it currently omits `edit`, `audit`, `sync`, `enroll`, `group` and `upload`); a new requirement states the picker verb, the `get`-is-a-pipe/`view`-is-a-terminal split, the preview's bounds, and the three refusals; the `--entry` requirement's verb count and its unaffected-verb list take the new row.
- `editor-input`: the requirement "Editing is its own verb" gains the nameless form and the ordering obligation that the editor refusal fires before the picker opens.
- `plaintext-staging`: the requirement "Plaintext staged during a run lives in a private directory on a memory-backed filesystem" gains a scenario stating that a preview stages nothing, so the enumeration of what the requirement governs is not silently widened by a new plaintext-bearing path.
- `behavioural-suite`: a requirement stating that an interactive surface is driven through a real pseudo-terminal, both directions, with no selection override compiled into the shipped binary for a test's benefit.

## Impact

Affected code:

- `crates/safix/src/main.rs` — the `VERBS` row after `get`; `fn view_command`; `edit_command`'s nameless arm, its `--no-preview` flag and its widened `FORM`; `mod picker;` and `mod tty;`; `generate_command`'s doc comment, which claims to be the only verb whose single optional argument could be a user or a name.
- `crates/safix/src/picker.rs` — new: the candidate model, the scorer, the draw loop, the debounce, the preview, and `choose`.
- `crates/safix/src/tty.rs` — new: the `/dev/tty` device constant both openers share, a read-write probe, the `Raw` guard, and the signal-path restore.
- `crates/safix/src/abort.rs` — the handler thread restores the terminal before it sweeps, because it exits by `process::exit` and no `Drop` runs.
- `crates/safix/src/usage.rs` — `pub const VIEW`; `EDIT`'s nameless paragraph; `SCAFFOLD`'s new row, `edit`'s widened form, and the verb count.
- `crates/safix/src/render.rs` — `listing`'s row builder is factored so the picker's rows and `list`'s table cannot drift.
- `crates/safix/src/reporter.rs` — three `sample` arms.
- `crates/safix/src/snapshots/` — six new files, two regenerated.
- `crates/safix-core/src/secret.rs` — `preview_into`.
- `crates/safix-core/src/error/{mod,prose,code}.rs` — three variants, their prose, their codes.
- `crates/safix/tests/harness/mod.rs` — a full-duplex pty helper beside `set_on_a_terminal`.
- `crates/safix/tests/picker.rs` — new.
- `modules/flake/checks/cli.nix`, `modules/flake/checks/single-runtime.nix` — the new checks.
- `README.md`, `CHANGELOG.md`, `examples/README.md` — the verb, the widened `edit`, and the three corrected counts.

Not affected, verified rather than assumed: `flake.nix`, `flake.lock`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `modules/flake/rust.nix`, and `openspec/specs/rust-supply-chain/spec.md` — the in-crate picker takes no dependency, so the enumerated-licence list stays a closed question in this change.
Every guarantee this change states gets a severity drill in `tasks.md`.
