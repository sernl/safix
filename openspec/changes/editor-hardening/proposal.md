# Proposal

## Why

`safix edit` stages the value inside a memory-backed staging root and removes that root however the run ends, but vim and nvim keep state outside the directory they are handed.
With its defaults, nvim writes its swap file under `~/.local/state/nvim/swap/` and writes yanked lines into `~/.local/state/nvim/shada/main.shada` on exit, and vim with `defaults.vim` writes registers to `~/.viminfo`.
A value yanked while editing therefore outlives the run on disk, which the stated limit at `crates/safix-core/src/edit.rs:24-26` names but does nothing to reduce.
agenix-rekey already passes per-buffer hardening options to these editors (`apps/edit-view.nix:151-156` at `87e5d5f`), and the upstream survey (agenix-rekey §3.2 in `docs/notes/research/upstream-survey-2026-11.md`) ranks adopting it as high priority.

## What Changes

- When the chosen editor's program word is `vim` or `nvim`, either bare or as a path whose last component is `vim` or `nvim`, `safix edit` inserts three arguments after the operator's own arguments and before the staged path: `-n`, so no swap file is created, then `--cmd` and an autocommand that sets `nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup` on every buffer read, plus `shadafile=NONE` for nvim or `viminfo=""` for vim.
- Any other program, including bare `vi`, `view` and `gvim`, receives exactly the argument vector it receives today.
- There is no opt-out: the options affect only the buffer safix stages, and an operator who wants different behaviour can name a different program.
- The stated limit in the module documentation, the `edit` help text, the CLI reference and the environment reference is rewritten to say what is now prevented for vim and nvim and what is not: other editors, and vim or nvim configuration or plugins that write or send buffer contents elsewhere.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `editor-input`: the editor's argument vector gains a stated rule for vim and nvim, and the scenario that states the limit of the staging root's containment is rewritten to say what that rule covers and what it leaves.

## Impact

- `crates/safix-core/src/edit.rs`: `Editor::open` builds its argument vector through one function that applies the rule; the module documentation's limit paragraph changes.
- `crates/safix-core/src/edit/hardening.rs` (new): the recognition of vim and nvim and the two option strings, kept out of `edit.rs` so that the source scan in `there_is_no_fallback_to_a_named_program` keeps guarding the selection.
- `crates/safix/src/usage.rs`: the `edit` help paragraph on the limit of the containment.
- `crates/safix-core/src/staging.rs:43-48`: the module documentation names vim and nvim as covered.
- `crates/safix/tests/editor.rs` and `modules/flake/checks/cli.nix`: one new end-to-end check.
- `docs/reference/cli.md` and `docs/reference/environment.md`.
- No nix module change, no new dependency, no change to how `VISUAL` and `EDITOR` are chosen or split.
