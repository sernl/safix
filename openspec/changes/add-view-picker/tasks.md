# Tasks: add-view-picker

Citations are as read while designing this change, on 2026-09-15; re-read the named lines before editing, since line numbers drift.
No real secret name, user name or recipient enters this repository; fixtures use `alice`, `bob` and `carol` and synthetic `age1` strings, matching the existing integration fixtures.
Where a task says "hold", add a check that fails when the claim stops being true, not a sentence asserting it.
The decisions this change rests on are `design.md`'s P1 through P11; each group names the ones it realizes.

## 1. The verb, its grammar, and its usage text (P1, P9)

- [ ] 1.1 Add `mod picker;` and `mod tty;` to the module list in `crates/safix/src/main.rs:42-48`, in alphabetical position
- [ ] 1.2 Add a `Verb` row to `VERBS` (`main.rs:117-188`) between the `get` row and the `list` row: `Verb { name: "view", help: usage::VIEW, run: view_command }`
- [ ] 1.3 Add `fn view_command(arguments: &[String]) -> Result<ExitCode, Refusal>` beside `get` (`main.rs:344-377`), with `const FORM: &str = "view [--no-preview] [<user>] [<name>]"`: parse `--no-preview` into a `picker::Options`, refuse any other `-`-prefixed word with `Refusal::Usage { form: FORM }` (P9 — `generate_command`'s treatment, not `edit_command`'s `UnknownOption`), then resolve positionals through the four arms P9 names, calling `picker::choose(&workspace, &user, picker::Scope::Everything, options)?` when no name was given
- [ ] 1.4 Give `view_command`'s decrypt half exactly `get`'s body (`main.rs:356-377`) with one difference: the value is written to the `/dev/tty` handle `tty::probe()` returns, and to `std::io::stdout()` only when no terminal could be opened — so `view <name>` in a pipeline is not a refusal, and the P7 rule that nothing the picker drew reaches a redirected stream still holds because the picker itself never draws to stdout
- [ ] 1.5 Add `pub const VIEW` to `crates/safix/src/usage.rs` beside `GET` (`usage.rs:289-296`), carrying: the two forms; the sentence `view` writes the value to the terminal and `get` writes it to stdout, so a pipeline wants `get`; what the nameless form offers (the same six columns `list` shows); the preview paragraph (what is decrypted, when, that exactly one value is held, that it is drawn in a region cleared on exit, and that nothing is staged); `--no-preview` and what it is for; and the three refusals
- [ ] 1.6 Add the reciprocal sentence to `usage::GET` (`usage.rs:289-296`): `get` is the pipe, and `view` is the verb that shows a value on a terminal and can offer a choice
- [ ] 1.7 Add the `view` row to `usage::SCAFFOLD` (`usage.rs:734-750`) between the `get` and `list` rows, as `  safix view     [<user>] [<name>]                   browse and read, with a preview`, aligned with the existing column
- [ ] 1.8 Rewrite `generate_command`'s doc comment at `main.rs:505-510`, which claims `generate` is "the only subcommand whose single optional argument could be either": name `generate`, `view` and `edit`, and state the shared rule once — a lone argument is a user when the declarations declare one by that name, and an entry name otherwise, with a collision reachable by naming both
- [ ] 1.9 Update the verb-count sentence in `usage::SCAFFOLD` (`usage.rs:761`), "Thirteen of the fourteen subcommands", to "Fourteen of the fifteen" — `view` reads only nix values and behaves identically under `--entry`
- [ ] 1.10 Add a unit test beside `every_verb_is_in_the_scaffold_in_the_order_the_table_declares_them` (`main.rs`'s `mod tests`) asserting `view` sits between `get` and `list` in both `VERBS` and `usage::SCAFFOLD`; the existing test already fails when the scaffold omits it, so this one holds the position rather than the presence
- [ ] 1.11 Add unit tests for P9's five grammar arms over a fixture `Placements` declaring a user `alice` and an entry also named `alice`: `[]`, `["alice"]` (user), `["grafana-token"]` (name), `["alice", "alice"]` (both), and a three-argument usage refusal
- [ ] 1.12 Severity drill: swapping the order of P9's second and third arms (name-first instead of user-first) turns 1.11's `["alice"]` case red, which is the evidence the `declares` lookup is load-bearing rather than incidental
- [ ] 1.13 Severity drill: deleting the `view` row from `usage::SCAFFOLD` while leaving `VERBS` intact turns both the existing scaffold test and 1.10 red, and deleting it from `VERBS` alone moves the two `unknown_subcommand` snapshots (group 5)
- [ ] 1.14 Verify: `cargo test -p safix --lib main::tests` green, drills in 1.12 and 1.13 observed

## 2. The terminal: one device, raw mode, restore on every exit path (P5)

- [ ] 2.1 Create `crates/safix/src/tty.rs` with a module doc comment stating that this is the only place the command puts a terminal into raw mode, that `prompt.rs` remains the only place it toggles echo, and why a picker refuses where `prompt.rs` degrades
- [ ] 2.2 Add `pub(crate) const DEVICE: &str = "/dev/tty";` to `tty.rs` and replace the literal in `prompt.rs`'s `open_source` (`prompt.rs:192-201`) with it, leaving that function's two-step probe and its stdin fallback otherwise unedited — one constant, two openers, so they cannot come to disagree about what they open
- [ ] 2.3 Add `pub(crate) fn probe() -> Option<File>` to `tty.rs`: write-open `DEVICE` first (mirroring `prompt.rs:192-201`, which mirrors the shell runtime's `{ : >/dev/tty; }`), then re-open it with `read(true).write(true)`, returning `None` on either failure
- [ ] 2.4 Add `pub(crate) struct Raw<'a>` and `Raw::over(terminal: &'a File) -> Option<Self>` to `tty.rs`, in `Silenced`'s exact shape (`prompt.rs:283-305`): `tcgetattr`, clear `LocalModes::{ECHO, ICANON, ISIG, IEXTEN}` and `InputModes::IXON`, `tcsetattr(…, OptionalActions::Now, …)`; `None` when the attributes cannot be read, which the picker turns into `Error::PickerNeedsTerminal`
- [ ] 2.5 Implement `Drop for Raw` restoring the saved `Termios` through `tcsetattr(…, OptionalActions::Now, …)`, covering return, error and panic
- [ ] 2.6 Add a module-level `static SAVED: Mutex<Option<Termios>>` to `tty.rs`, written by `Raw::over` and cleared by `Raw`'s `Drop`, plus `pub(crate) fn restore()` applying whatever is stored to `DEVICE` and doing nothing when nothing is stored
- [ ] 2.7 Call `crate::tty::restore()` from `abort::catch_signals`'s handler thread (`crates/safix/src/abort.rs:44-64`), **before** `safix_core::scratch::interrupt(status)`, and extend that function's doc comment to say why: the handler ends the process with `std::process::exit`, so no `Drop` runs, and a sweep that waits on an in-flight `sops` subprocess must not do so with the operator's terminal still in raw mode
- [ ] 2.8 Add a unit test in `tty.rs` asserting `DEVICE` is the string `prompt.rs` opens, by `include_str!`ing `prompt.rs` and asserting it contains `tty::DEVICE` and no second `/dev/tty` literal
- [ ] 2.9 Add a unit test asserting `Raw::over` on a pseudo-terminal clears all four `LocalModes` bits and `IXON`, and that the attributes read back after the guard drops equal the ones read before it was taken
- [ ] 2.10 Severity drill: removing `IEXTEN` from 2.4's cleared set turns 2.9 red on that one bit and leaves the other four green, which is the evidence the assertion is per-bit rather than aggregate
- [ ] 2.11 Severity drill: removing 2.7's `restore()` call turns group 7's interrupted-picker test red (the pty's attributes are still raw after the process is gone) while every `Drop`-path test stays green, which is the evidence the signal path is held independently of the guard
- [ ] 2.12 Verify: `cargo test -p safix --lib tty::` green, drills in 2.10 and 2.11 observed

## 3. The picker: candidates, the scorer, the draw loop (P3, P8, P11)

- [ ] 3.1 Create `crates/safix/src/picker.rs` with a module doc comment carrying P2's dependency verdict in two sentences (a third-party picker cannot render the preview and its matcher is outside `deny.toml`'s enumerated licences) and P8's ranking rule in full
- [ ] 3.2 Add `pub(crate) enum Scope { Everything, Editable }` and `pub(crate) struct Options { pub preview: bool }` with `Default` giving `preview: true`
- [ ] 3.3 Add `pub(crate) fn choose(workspace: &Workspace, user: &str, scope: Scope, options: Options) -> Result<String, Refusal>` — the only public item in the module (P3)
- [ ] 3.4 Build the candidate list from `workspace.placements()?.held_by(user)` (`crates/safix-core/src/model.rs:211-225`), raising `Error::UnknownUser` when the user is absent exactly as `list` does (`main.rs:312-340`), and filtering `placement.public.is_some()` (`model.rs:168`) out under `Scope::Editable`
- [ ] 3.5 Refuse `Error::NothingToPick { user }` when the filtered candidate list is empty — including the case where the user holds only public placements under `Scope::Editable`, which is a distinct reason for the same state and is covered by the same refusal
- [ ] 3.6 Factor `render::listing` (`crates/safix/src/render.rs:473-506`) into `fn listing_row(name: &str, placement: &Placement) -> Vec<String>` plus a `listing` that is the header row and a map over it, and build the picker's rows through the same function (P11); `list`'s own output is byte-identical, which the existing `list` snapshot holds
- [ ] 3.7 Implement the scorer in `picker.rs`: case-insensitive subsequence match over the name, ranked by fewest matched-character runs, then first match at a word boundary (`-`, `_`, `.`, or index 0), then earliest first-match index, then the name itself as a total tiebreak; an empty query matches everything in the `BTreeMap`'s own order, which is the order `list` prints
- [ ] 3.8 Implement the draw loop over the `tty::probe()` handle under a `tty::Raw` guard: enter the alternate screen buffer on entry and leave it on every exit path (P7), draw the aligned rows through `table::aligned` (`crates/safix/src/table.rs:22+`), and handle exactly these keys — printable bytes append to the query, `Backspace`/`^H`/`DEL` remove one, `Up`/`^P` and `Down`/`^N` and the two arrow escape sequences move the highlight, `Enter` chooses, `Esc`, `^C` and `^D` cancel
- [ ] 3.9 Refuse `Error::PickerNeedsTerminal` when `tty::probe()` returns `None` or `tty::Raw::over` returns `None`, before any candidate is read or decrypted
- [ ] 3.10 Refuse `Error::SelectionCancelled` on any cancel key, after the alternate region has been left and the `Raw` guard dropped, with no value retained
- [ ] 3.11 Add unit tests for the scorer, one per ranking rule, each named for the property it demonstrates: fewer runs beats more, a word-boundary start beats a mid-word one, an earlier first match beats a later one, the name breaks a full tie, and an empty query preserves `BTreeMap` order
- [ ] 3.12 Add a unit test asserting `Scope::Editable` drops a public placement and `Scope::Everything` keeps it, over one fixture `Placements` holding both
- [ ] 3.13 Add a unit test asserting the picker's rows and `render::listing`'s rows are equal for the same placement, so 3.6's factoring cannot be undone by copying (P11)
- [ ] 3.14 Severity drill: dropping the word-boundary rule from 3.7 turns exactly one of 3.11's tests red and leaves the other four green
- [ ] 3.15 Severity drill: re-implementing the picker's row construction inline instead of calling `listing_row` turns 3.13 red as soon as either side's `GENERATOR` fallback changes — perform the drill by changing the `-`/`yes` fallback on one side only
- [ ] 3.16 Verify: `cargo test -p safix --lib picker::` green, drills in 3.14 and 3.15 observed

## 4. The preview, and `Secret`'s one new egress (P6)

- [ ] 4.1 Add `pub fn preview_into(&self, sink: &mut impl Write, lines: usize, columns: usize) -> io::Result<()>` to `crates/safix-core/src/secret.rs`, beside `write_to` (`secret.rs:180`), with a doc comment stating it is the third and last egress and why it is shaped as a sink rather than a return: the plaintext's lifetime is the write's lifetime
- [ ] 4.2 Implement it to write at most `lines` lines of at most `columns` columns, replacing control bytes with a visible placeholder, appending a truncation marker when it stopped early, and emitting a single byte-count line for a value that is not valid UTF-8
- [ ] 4.3 Confirm `secret.rs:62-81`'s five `const _` probes are unedited, and add a sixth asserting `Secret` still implements no `ToString` — the probe that would have caught 4.1 being written as a returning method
- [ ] 4.4 Add unit tests in `secret.rs`: a value longer than `lines × columns` renders bounded and marked; a value containing `\x1b`, `\r` and `\n` renders with placeholders and no raw escape byte reaching the sink; a non-UTF-8 value renders as a byte count; a value shorter than the region renders whole with no marker
- [ ] 4.5 Wire the preview into `picker.rs`'s draw loop: on a highlight change, start a 150 ms quiet timer; on expiry, `workspace.sops().decrypt_key(&workspace.vault_absolute(&placement.file), &placement.key)` and render through `preview_into` into the `/dev/tty` writer, sized to the region the draw loop reserved
- [ ] 4.6 Name the debounce constant once, in `picker.rs`, with a comment stating it was chosen for feel rather than measured
- [ ] 4.7 Hold exactly one decrypted value: store the previewed `Secret` in a single `Option<Secret>` field, `take()`n and dropped on every highlight change and on exit, with no map or cache keyed by name anywhere in the module
- [ ] 4.8 Render a failed decrypt as text in the preview region and continue the loop — a failed preview is never a failed selection — and route an entry whose file does not exist yet (`Error::NoValueYet`'s condition, `main.rs:356-363`) to the same region rather than to a refusal
- [ ] 4.9 Skip the decrypt entirely when `options.preview` is false, and assert in a unit test that no `Sops` call is constructed on that path
- [ ] 4.10 Confirm `picker.rs` never calls `safix_core::staging::Staging::establish`, and hold it with a test that `include_str!`s `picker.rs` and asserts the file mentions neither `Staging` nor `establish` (P6, and the `plaintext-staging` delta's new scenario)
- [ ] 4.11 Severity drill: replacing 4.7's single `Option<Secret>` with a `BTreeMap<String, Secret>` cache turns 4.7's own test red — write that test as an assertion over the struct's fields, not over behaviour, since a cache is a shape rather than an observable
- [ ] 4.12 Severity drill: dropping the control-byte replacement from 4.2 turns the escape-byte case in 4.4 red, which is the evidence a value cannot repaint the operator's screen
- [ ] 4.13 Severity drill: removing 4.10's `include_str!` assertion and adding a `Staging::establish` call to `picker.rs` must turn group 7's staging-roots test red as well, so the claim is held both structurally and behaviourally
- [ ] 4.14 Verify: `cargo test -p safix-core --lib secret::` and `cargo test -p safix --lib picker::` green, drills in 4.11, 4.12 and 4.13 observed

## 5. The three refusals, their prose, their codes, and their snapshots

- [ ] 5.1 Add three variants to `crates/safix-core/src/error/mod.rs`, beside `NoTerminal` (`error/mod.rs:1036-1037`): `PickerNeedsTerminal`, `NothingToPick { user: String }`, `SelectionCancelled`, each with a `#[error(...)]` attribute referencing a prose constant
- [ ] 5.2 Add three `refusal_codes!` lines to `crates/safix-core/src/error/code.rs` (`code.rs:32-63`), in the order `Error` declares the variants: `PickerNeedsTerminal => "safix::picker_needs_terminal"`, `NothingToPick => "safix::nothing_to_pick"`, `SelectionCancelled => "safix::selection_cancelled"`
- [ ] 5.3 Add prose for each to `crates/safix-core/src/error/prose.rs`: `PICKER_NEEDS_TERMINAL` naming both remedies (`safix view <name>`, and `safix list` to see what a user holds) and stating that a picker on a terminal it cannot put in raw mode is unusable rather than degraded; a `nothing_to_pick(user)` function whose wording aligns with `list`'s own `flake.safix.users.{user} holds no secret.` sentence (`main.rs:328`) and states that `edit` additionally offers nothing for a user holding only public placements; `SELECTION_CANCELLED` stating that the terminal was restored, nothing was written, and nothing decrypted was kept
- [ ] 5.4 Leave `NO_TERMINAL` (`prose.rs:516-527`) unedited and do not reuse `Error::NoTerminal` for the picker: its prose is enrollment's, down to the touch-policy paragraph, and printing it at an operator who asked to read a secret is the failure this task exists to prevent
- [ ] 5.5 Add three arms to `reporter.rs`'s `sample` (`crates/safix/src/reporter.rs:231+`), which is wildcard-free over `Code::ALL`, using fixture data: `NothingToPick { user: "dave".into() }`
- [ ] 5.6 Accept six new snapshots under `crates/safix/src/snapshots/`: `safix__reporter__tests__{plain,graphical}-picker_needs_terminal.snap`, `…-nothing_to_pick.snap`, `…-selection_cancelled.snap`
- [ ] 5.7 Regenerate and re-accept `safix__reporter__tests__plain-unknown_subcommand.snap` and `…graphical-unknown_subcommand.snap`: the sentence is computed from `VERBS` by `expected_verbs` (`main.rs:191-209`), so the new row changes it
- [ ] 5.8 Add `Error::SelectionCancelled`'s exit code to the run path: `view_command` and `edit_command` return the refusal, so the process exits 1 through the existing `Refusal` path with no special-casing; hold it with an integration assertion in group 7 rather than a unit test, since the exit code is the binary's
- [ ] 5.9 Severity drill: pointing `PickerNeedsTerminal` at `NO_TERMINAL` instead of its own prose turns the accepted snapshot in 5.6 red, which is the evidence the two refusals are distinguished rather than aliased
- [ ] 5.10 Severity drill: deleting the `view` row from `VERBS` turns 5.7's two snapshots red, which is the evidence the refusal sentence is derived from the table rather than written beside it
- [ ] 5.11 Verify: `cargo test -p safix --lib reporter::` green with all eight snapshots accepted, `cargo test -p safix-core --lib error::` green, drills in 5.9 and 5.10 observed

## 6. The nameless `edit` (P4)

- [ ] 6.1 Widen `edit_command`'s `FORM` (`main.rs:414-446`) to `"edit [--allow-disk-staging] [--no-preview] [<user>] <name>"` — keeping the existing `UnknownOption` treatment of every other `-`-prefixed word, which P9 leaves alone deliberately — and parse `--no-preview` into a `picker::Options`
- [ ] 6.2 Add the `[]` arm to `edit_command`'s positional match: resolve `workspace.default_user()?`, then `picker::choose(&workspace, &user, picker::Scope::Editable, options)?`
- [ ] 6.3 Call `safix_core::edit::Editor::from_environment()?` in `edit_command` before `picker::choose`, on every form including the named ones, and leave `edit::run`'s own identical call (`crates/safix-core/src/edit.rs:173`) in place — the duplication is deliberate and `design.md` P4 records why
- [ ] 6.4 Add nothing to `crates/safix-core/src/edit.rs`: its `there_is_no_fallback_to_a_named_program` test `include_str!`s the file and forbids the literals `"vi"`, `"vim"`, `"nano"` and `"emacs"` anywhere in it, and key-handling code is exactly what trips it by accident — confirm the file is unedited at the end of this group
- [ ] 6.5 Add the nameless-form paragraph to `usage::EDIT` (`usage.rs:647-685`): what is offered, that public outputs are not among them, that the editor is settled first, and the three refusals
- [ ] 6.6 Change `edit`'s row in `usage::SCAFFOLD` (`usage.rs:734-750`) to `[<user>] [<name>]`
- [ ] 6.7 Add a unit test asserting `edit_command`'s `[]` arm reaches `Editor::from_environment` before `picker::choose` — assert it by clearing both editor variables in the test's own environment and observing `Error::NoEditor` over a fixture whose user holds nothing, which would otherwise produce `Error::NothingToPick`; the refusal code is the evidence of the order
- [ ] 6.8 Severity drill: moving 6.3's call after `picker::choose` turns 6.7 red by returning `NothingToPick` where `NoEditor` is required, which is the exact ordering P4 exists to hold
- [ ] 6.9 Severity drill: passing `Scope::Everything` from `edit_command` turns group 7's public-output test red — the picker offers an entry whose selection then raises `PublicNotEditable`
- [ ] 6.10 Verify: `cargo test -p safix --lib` green, `cargo test -p safix-core --lib edit::` green and `edit.rs` unedited, drills in 6.8 and 6.9 observed

## 7. Integration tests: a full-duplex pty and the picker end to end (P10)

- [ ] 7.1 Add `pub fn pick_on_a_terminal(&self, arguments: &[&str], keystrokes: &str) -> Run` to `crates/safix/tests/harness/mod.rs`, beside `set_on_a_terminal` (`harness/mod.rs:1663-1702`): open `Pty` (`harness/mod.rs:2269-2290`), wrap in `setsid -w` under the same rule `set_on_a_terminal` uses (`harness/mod.rs:2739-2764`), attach the slave to **stdin and stdout**, keep stderr a pipe, write the keystrokes to the master, and read the master's output into the returned `Run`
- [ ] 7.2 Add a doc comment on 7.1 stating how it differs from `set_on_a_terminal` and why — a picker draws where it reads — and cross-referencing the `behavioural-suite` requirement this change adds
- [ ] 7.3 Create `crates/safix/tests/picker.rs` with a module doc comment naming the check attributes in group 8 that run each test
- [ ] 7.4 `a_run_with_no_terminal_is_refused_naming_both_remedies`: `fixture.run(&["view"])`, pipes on all three streams, expect `safix::picker_needs_terminal`, `says` both remedies
- [ ] 7.5 `a_user_holding_nothing_is_refused_rather_than_offered_an_empty_list`: `pick_on_a_terminal(&["view", "carol"], "")` over a fixture where `carol` declares no entry, expect `safix::nothing_to_pick` naming `carol`
- [ ] 7.6 `a_typed_query_and_enter_prints_the_chosen_value`: a fixture where `alice` holds three entries with distinct values, drive `view` with a query narrowing to one and `\r`, assert the chosen entry's literal value appears and the other two values do not
- [ ] 7.7 `cancelling_writes_nothing_and_restores_the_terminal`: drive `view` with `\x1b`, assert exit code 1, `safix::selection_cancelled`, `scratch_files()` unchanged, `head()` unmoved, and the pty's `termios` read back after the process exits equals the one read before the run
- [ ] 7.8 `an_interrupt_mid_picker_restores_the_terminal_and_exits_130`: reuse `interrupt_after` (`harness/mod.rs:1723`) with `SIGINT` against a `view` run sitting in the picker, assert exit 130 and the pty's `termios` restored — this is the test drill 2.11 reddens
- [ ] 7.9 `the_preview_never_reaches_stdout_or_stderr`: run `view` with the pty attached only to stdin, stdout and stderr as pipes, highlight an entry, pause past the debounce, cancel, and assert neither pipe contains the value's bytes
- [ ] 7.10 `no_preview_decrypts_nothing_until_a_choice_is_made`: drive `view --no-preview`, pause past the debounce, cancel, and assert no decrypt happened — observed through the `sops` shim's own call log, the same way `abort_residue.rs` observes shim calls
- [ ] 7.11 `moving_through_entries_without_pausing_decrypts_only_where_the_highlight_rests`: send several movement keys with no pause, then rest, then cancel; assert exactly one decrypt in the shim's call log
- [ ] 7.12 `edit_with_no_name_reaches_the_editor_for_the_chosen_entry`: `EDITOR` set to the `/bin/sh` shim `editor.rs`'s `editor(fixture, name, body)` helper writes (`crates/safix/tests/editor.rs:34-40`), drive `edit` with a query and `\r`, assert the editor received the staged path for the chosen entry and the written value landed
- [ ] 7.13 `edit_with_no_editor_refuses_before_offering_anything`: unset both variables, drive `edit` with no name on a pty, assert `safix::no_editor` and that nothing was drawn on the pty
- [ ] 7.14 `edit_never_offers_a_public_output`: a fixture where `alice` holds one ordinary entry and one public placement, drive `edit` with an empty query, assert the public entry's name does not appear in the pty's output and that `view` over the same fixture does show it
- [ ] 7.15 `nothing_is_staged_by_a_preview`: capture `staging_roots()` (`harness/mod.rs:1556`) before and after a `view` run that previews and cancels, assert unchanged, and assert no file was created under any staging root during the run
- [ ] 7.16 Do not extend `crates/safix/tests/editor.rs` with the four-outcome assertions for the nameless form: its three existing tests (`editor.rs:48`, `:139`, `:193`) already hold the outcomes, and 7.12 holds only what the nameless form adds — record that division in `editor.rs`'s module doc comment so a later reader does not add the duplicate
- [ ] 7.17 Severity drill: attaching the pty to stdin only in 7.1 turns 7.6 red — the picker's draw output goes to `/dev/tty`, which without a pty-backed stdout is not the stream the test reads
- [ ] 7.18 Severity drill: removing the debounce from 4.5 turns 7.11 red by producing one decrypt per movement key
- [ ] 7.19 Verify: `cargo test -p safix --test picker` green with every test above run under `--test-threads 1`, drills in 7.17 and 7.18 observed

## 8. Nix checks

- [ ] 8.1 Add `checks.safix-view-no-terminal`, `safix-view-nothing-to-pick`, `safix-view-selection`, `safix-view-cancelled`, `safix-view-preview-streams` and `safix-edit-nameless` to `modules/flake/checks/cli.nix`, each through `mode "<check-name>" "picker" "<test_fn_name>"` (`cli.nix:327-343`, following the three `safix-edit*` entries at `cli.nix:510-526`)
- [ ] 8.2 Give each entry the file's prose-comment convention: what the claim is, and which drill from groups 2, 4, 6 or 7 turns it red
- [ ] 8.3 Register `picker` as a whole-target `claim` in `modules/flake/checks/single-runtime.nix` (`single-runtime.nix:38`, following `safix-vault-*` at `:132-148`) as `safix-picker = claim "safix-picker" "picker";`, with a doc comment naming the interrupt and staging drills it carries
- [ ] 8.4 Confirm `crates/safix/tests/picker.rs` needs no `[[test]]` entry in `crates/safix/Cargo.toml`: test targets there are auto-discovered, and only the six support binaries are declared (`crates/safix/Cargo.toml:18-72`) — state the confirmation in 8.3's comment so the next reader does not go looking
- [ ] 8.5 Confirm `modules/flake/rust.nix` needs no change: crane vendors from `Cargo.lock` and `cargoArtifacts` picks up nothing new, because P2 takes no dependency; and confirm the compiled suite installs the new target to `$out/bin/picker` through the existing `rust.nix:104-155` path
- [ ] 8.6 Confirm `flake.nix`, `flake.lock`, `Cargo.toml`, `crates/safix/Cargo.toml`, `Cargo.lock`, `deny.toml` and `openspec/specs/rust-supply-chain/spec.md` are all unedited at the end of this change — P2's whole point, and the evidence is the empty diff over those seven paths
- [ ] 8.7 Severity drill: renaming any test function in `crates/safix/tests/picker.rs` without updating `cli.nix` turns that check red rather than green, because `integration.nix`'s `runOneWith` fails when zero tests ran — perform the drill on one entry
- [ ] 8.8 Severity drill: dropping `safix-picker` from `single-runtime.nix` leaves the six `cli.nix` entries green while the interrupt and staging tests stop running in CI, which is why the whole-target claim exists alongside the per-mode ones; observe it by listing the tests each check runs
- [ ] 8.9 Verify: `nix build .#checks.x86_64-linux.safix-view-selection .#checks.x86_64-linux.safix-edit-nameless .#checks.x86_64-linux.safix-picker` green, drills in 8.7 and 8.8 observed

## 9. Documentation, the changelog, and the three stale counts

- [ ] 9.1 Add a `## Browsing what is there: `safix view`` section to `README.md` before the `## Editing a value` section (`README.md:554`): the two forms, the `get`-is-a-pipe/`view`-is-a-terminal sentence, what the picker offers, the preview's four properties, `--no-preview`, and the three refusals
- [ ] 9.2 Extend `README.md`'s `## Editing a value: `safix edit`` section (`:554-573`) with the nameless form, the public-output exclusion, and the editor-before-picker ordering
- [ ] 9.3 Add `safix view` to the daily-commands block (`README.md:591-599`) between the `get` and `generate` lines
- [ ] 9.4 Correct `README.md`'s three verb counts — `:900`, `:930`/`:933` (the thirteen attribute spellings are unchanged; only the verb counts move), `:935` (add `view` to the enumerated list and move "Thirteen of safix's fourteen verbs" to "Fourteen of safix's fifteen") and `:1222` ("all thirteen subcommands", with `view` added to the read paths)
- [ ] 9.5 Correct `examples/README.md:21`, "Thirteen of safix's fourteen verbs", to fourteen of fifteen
- [ ] 9.6 Add a `### A read verb with a picker, and a nameless `edit`` entry under `## [Unreleased]` in `CHANGELOG.md` (`CHANGELOG.md:22`): the new verb and its position, the widened `edit` form, the three new refusal codes by name, the preview's bounds and `--no-preview`, `Secret::preview_into` as a `safix-core` interface addition, and the sentence that no dependency was added and `deny.toml` is unedited
- [ ] 9.7 State in the changelog entry that `safix edit` with no arguments previously printed a usage line and now opens a picker, recorded under the widened verb rather than as a break, and that `get` is unchanged
- [ ] 9.8 Do not rewrite any released changelog entry; the `[Unreleased]` section is the only one this change touches
- [ ] 9.9 Severity drill: leaving any of 9.4's or 9.5's counts stale is not caught by a check today — record that gap in 9.6's changelog entry rather than inventing a prose-counting check, and instead assert the one count that is machine-checkable, `usage::SCAFFOLD`'s (task 1.9), through the scaffold snapshot
- [ ] 9.10 Verify: `README.md`, `examples/README.md` and `CHANGELOG.md` re-read end to end for the sections touched; `nix build .#checks.x86_64-linux.safix-usage-scaffold` (or whichever check holds the scaffold snapshot) green

## 10. Closing verification

- [ ] 10.1 Run `cargo test --locked --workspace` and record which failures are pre-existing environment-only ones (`bwrap`/`strace`) rather than this change's
- [ ] 10.2 Run `nix flake check` and record every check that moved, confirming the six new `cli.nix` entries and `safix-picker` are among them and that no existing check attribute was renamed or removed
- [ ] 10.3 Confirm the seven paths named in 8.6 are unedited, by diff
- [ ] 10.4 Run `openspec validate --change add-view-picker` and confirm the four delta specs validate
- [ ] 10.5 Severity drill roll-up: confirm every drill named in groups 1 through 9 was performed and its outcome recorded in the task line, and that any drill which did not reproduce carries the reason in place of the claim
