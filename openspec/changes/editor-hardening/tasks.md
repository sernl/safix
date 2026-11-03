# Tasks

## 1. The argument vector

- [ ] 1.1 Create `crates/safix-core/src/edit/hardening.rs` holding the recognition (`Path::new(program).file_name()` equal to `vim` or `nvim`) and the two option strings copied verbatim from agenix-rekey `apps/edit-view.nix:154` and `:156` at `87e5d5f`, with module documentation stating that these are programs recognised rather than fallen back to, and why bare `vi`, `view`, `gvim` and `rvim` are excluded; declare it from `crates/safix-core/src/edit.rs` as `mod hardening;`.
  Verify `cargo test -p safix-core --lib edit::tests::there_is_no_fallback_to_a_named_program` still passes with the scan unchanged.
- [ ] 1.2 Add `Editor::argv(&self, path: &Path) -> Vec<OsString>` in `crates/safix-core/src/edit.rs` returning the operator's arguments, then `-n`, `--cmd` and the matching string when the program is recognised, then the path; change `Editor::open` (`edit.rs:114-126`) to run `Command::new(&self.program).args(self.argv(path))`.
  Verify by 1.3 and `nix build .#checks.x86_64-linux.safix-rs-clippy`.
- [ ] 1.3 Add unit tests in `crates/safix-core/src/edit/hardening.rs` asserting the full vector built through `Editor::choose` and `Editor::argv` for `nvim` (`-n`, `--cmd`, nvim string, path), `vim` (`-n`, `--cmd`, vim string, path), `/nix/store/<hash>-neovim/bin/nvim`, `nvim -u NONE` (operator's `-u NONE` before `-n`), and `code --wait`, `vi`, `gvim` and `/usr/bin/vimdiff` (each the program's arguments and the path only).
  Verify `cargo test -p safix-core --lib edit::hardening` and `nix build .#checks.x86_64-linux.safix-rs-test`.
- [ ] 1.4 Severity drill: make the recognition match on `program.contains("vim")` instead of the last path component; `cargo test -p safix-core --lib edit::hardening` must go red on the `gvim` and `vimdiff` cases.
  Then swap the two strings between vim and nvim; it must go red on the `nvim` and `vim` cases.
  Then drop `-n` from the vector; it must go red on every vim and nvim case.
  Revert all three.

## 2. The end-to-end check

- [ ] 2.1 Add `a_vim_or_nvim_editor_is_opened_with_its_persistence_switched_off` to `crates/safix/tests/editor.rs`: write an executable shell script named `nvim` under `fixture.scratch(...)` that writes each of `"$@"` on its own line to a spool and replaces the buffer, run `safix edit alice api-token` with `EDITOR` set to that script's absolute path, and assert the spool is non-empty, its first three lines are `-n`, `--cmd` and the nvim string, its last line is the staged path, and the value is absent.
  Verify `cargo test -p safix --test editor a_vim_or_nvim_editor_is_opened_with_its_persistence_switched_off`.
- [ ] 2.2 Register it in `modules/flake/checks/cli.nix` beside `checks.safix-edit-argv` as `checks.safix-edit-hardening = mode "safix-edit-hardening" "editor" "a_vim_or_nvim_editor_is_opened_with_its_persistence_switched_off";` with a comment naming what it asserts and its drill.
  Verify `nix build .#checks.x86_64-linux.safix-edit-hardening`.
- [ ] 2.3 Severity drill: change `Editor::open` back to `.args(&self.arguments).arg(path)`; `nix build .#checks.x86_64-linux.safix-edit-hardening` must go red on the missing `-n` while `cargo test -p safix-core --lib edit::hardening` stays green, showing the check covers the wiring and not only the builder.
  Revert.
- [ ] 2.4 Confirm the existing editor checks are unaffected, since their editor is `/bin/sh <script>`: `nix build .#checks.x86_64-linux.safix-edit .#checks.x86_64-linux.safix-edit-selection .#checks.x86_64-linux.safix-edit-argv .#checks.x86_64-linux.safix-edit-nameless`.

## 3. The stated limit

- [ ] 3.1 Rewrite the limit paragraph at `crates/safix-core/src/edit.rs:24-26` to say that vim and nvim are opened with swap, undo, backup and history writes switched off for the staged buffer, and that what remains is every other editor and vim or nvim configuration or plugins that write or send the buffer elsewhere; update the `Editor` documentation (`edit.rs:58-64`) to say the options are inserted before the path.
  Verify `cargo doc -p safix-core --no-deps` builds without warnings.
- [ ] 3.2 Make the same statement in `crates/safix-core/src/staging.rs:43-48`, keeping the generator-script case as it is.
  Verify by reading the paragraph against 3.1.
- [ ] 3.3 Rewrite the paragraph at `crates/safix/src/usage.rs:1128-1133` in the `edit` help text to the same effect, naming `-n` and the `--cmd` option and both remaining cases.
  Verify `cargo run -p safix -- edit --help` shows the paragraph.
- [ ] 3.4 In `docs/reference/cli.md` `## edit` (lines 89-115), add a paragraph after the one on how the command is split: for `vim` and `nvim`, `-n`, `--cmd` and the option string are inserted after the operator's arguments and before the path, quoting both strings, stating that `-n` prevents the swap file nvim otherwise creates in its own state directory before the autocommand runs, naming `vi` as excluded and why, stating there is no opt-out, and stating the remaining limit.
  Verify each of the two strings occurs byte for byte in both `docs/reference/cli.md` and `crates/safix-core/src/edit/hardening.rs` (`grep -F`), and `nix build .#checks.x86_64-linux.treefmt` passes.
- [ ] 3.5 In `docs/reference/environment.md` `#### VISUAL and EDITOR` (lines 159-163), add one sentence that a program word whose last component is `vim` or `nvim` gains the hardening options, linking to the `edit` section of the CLI reference.
  Verify the link resolves.

## Workflow follow-up

- The orchestrator writes the CHANGELOG entry after implementation.
- Archive the change once the checks above are green; the `editor-input` delta then replaces the limit scenario in `openspec/specs/editor-input/spec.md:97-101`.
