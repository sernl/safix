# Design

## Context

`Editor::choose` (`crates/safix-core/src/edit.rs:91-104`) takes `VISUAL`, then `EDITOR`, splits the value on whitespace into `program` and `arguments`, and `Editor::open` (`:114-126`) runs `Command::new(program).args(arguments).arg(path)` with no shell.
The staged path is the last argument.

The module documentation (`edit.rs:24-26`), the help text (`crates/safix/src/usage.rs:1128-1133`), the staging module documentation (`crates/safix-core/src/staging.rs:43-48`) and the `editor-input` spec state that an editor writing undo history, swap files or backups to a directory of its own is outside the guarantee.

The unit test `there_is_no_fallback_to_a_named_program` (`edit.rs:299-308`) scans the source of `edit.rs` for the literals `"vi"`, `"vim"`, `"nano"` and `"emacs"`, so any code naming vim inside `edit.rs` turns it red.

Upstream, agenix-rekey at `87e5d5f` (`apps/edit-view.nix:151-159`) builds `EDITOR_OPTS` by matching `$EDITOR` against `*nvim*` and then `*vim*` and runs `$EDITOR "${EDITOR_OPTS[@]}" "$CLEARTEXT_FILE"`.
The two option strings, confirmed verbatim at `:154` and `:156`, are:

- nvim: `au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup shadafile=NONE`
- vim: `au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup viminfo=""`

Both are single-quoted in bash, so `viminfo=""` reaches vim as the six characters `viminfo=` followed by two double quotes.

Observed on this workstation (nvim 0.12.6, vim 9.2.1164 from the nix store), with a scratch `XDG_STATE_HOME`:

- nvim's defaults are `directory=~/.local/state/nvim/swap//`, `undodir=~/.local/state/nvim/undo//`, `backupdir=.,~/.local/state/nvim/backup//` and a `shada` that saves registers; vim's are `directory=.,~/tmp,/var/tmp,/tmp`, `undodir=.`, `backupdir=.,~/tmp,~/` and, under `defaults.vim`, `viminfo='100,<50,s10,h`.
- Without the options, `nvim -u NONE` on a file, `yy`, `:q!` writes the yanked line into `shada/main.shada`; with the nvim options no shada file is written.
- With the vim options, vim writes no viminfo file where it writes one without them, and neither editor reports an error for either string.
- With the vim string, nvim (which is what `vim` resolves to on hosts using `programs.neovim.vimAlias`) accepts `viminfo` as its alias for `shada`, sets it empty, and writes no shada file.
- nvim creates its swap file under `~/.local/state/nvim/swap/` before `BufRead` fires; `noswapfile` deletes it during the autocommand. Copied at that moment, the file held the swap header (user, host, staged path) and not the value.
- With `-n`, nvim created no swap file: the swap directory was empty at `BufRead` and after exit. `-n` sets `updatecount=0`, so vim likewise creates none, although `&swapfile` still reads `1`.

## Goals / Non-Goals

**Goals:**

- vim and nvim, the two editors whose defaults write buffer-derived state outside the staging root, are opened with that state switched off for the staged buffer.
- The argument vector for every other editor is unchanged, byte for byte.
- The stated limit says precisely what this covers and what it leaves.

**Non-Goals:**

- Hardening other editors (emacs, helix, kakoune, nano, VS Code); each has its own mechanism and none is in the survey's scope.
- Detecting what a program resolves to through `PATH` or symlinks.
- Defending against the operator's own vim or nvim configuration, which runs inside the editor and can override anything passed on the command line.

## Decisions

### Recognise the program word by its last path component, `vim` or `nvim` only

The rule is `Path::new(program).file_name()` equal to `vim` or `nvim`.
That covers the bare name and any path ending in `/vim` or `/nvim`, such as a nix store path.

Bare `vi` is excluded, as are `view`, `gvim`, `ex` and `rvim`.
On many hosts `vi` is nvi or busybox vi, neither of which accepts `--cmd`; passing the options there would make the edit fail rather than harden it.
`view` and `rvim` are read-only or restricted invocations, and `gvim` forks into its own window unless given `-f`, so neither is in the shape the rest of `edit` assumes.

Alternative: upstream's substring match (`*nvim*`, `*vim*`) on the whole `$EDITOR` string.
Rejected because it matches `gvim`, `vimdiff`, a path such as `/home/vim-user/bin/emacs`, and an argument containing `vim`; the program word is the only part of the value that says which program runs.

Alternative: resolve the program through `PATH` and inspect the target.
Rejected because the rule would then depend on filesystem state at run time and could not be stated or tested over the variable's value alone.

### Use upstream's option strings verbatim

The strings are carried exactly as `apps/edit-view.nix:154` and `:156` write them, so the two projects can be compared line for line and the observations above apply unchanged.

Upstream passes only `--cmd`; this change adds `-n` (no swap file) before it, for both editors.
The reason is the observation above: without `-n`, nvim creates its swap file in its own state directory before the autocommand can switch it off, and that file records the user, the host and the staged path.
`-n` is applied while the editor starts, before any buffer is read, so no swap file exists at any moment.
`noswapfile` stays in the autocommand so the strings remain upstream's verbatim.

Alternative: also pass `-i NONE` (no viminfo or shada).
Not taken: the history settings in the autocommand already prevent the history file, as observed, and `-i NONE` adds no case they leave open.

### The options go after the operator's arguments and before the staged path

The argument vector is `program`, the operator's `arguments`, `-n`, `--cmd`, the option string, the staged path.
For `EDITOR="nvim -u NONE"` this is `nvim -u NONE -n --cmd <nvim string> <path>`.

`--cmd` commands run before any vimrc or `init.lua` regardless of where `--cmd` sits among the other options, so the position does not change when the autocommand is registered.
Keeping the operator's arguments first preserves the existing contract that the value is split into program and arguments and the path is appended last.

An `EDITOR` value whose arguments end with `--` would make vim read `-n`, `--cmd` and the option string as file names.
This is accepted: an `EDITOR` ending in `--` is not a form vim users write, and the staged path would still be opened.

### No opt-out

There is no flag or variable disabling the options.
They affect only the buffer safix stages, which the operator never wants recoverable from a swap, undo or history file; an operator who wants other behaviour can set `VISUAL` to a program the rule does not recognise.
The stated limit still covers every editor the rule does not recognise.

### The recognition lives in a submodule of `edit`

The rule and the two strings go in `crates/safix-core/src/edit/hardening.rs`, declared from `edit.rs` as `mod hardening;`.
`there_is_no_fallback_to_a_named_program` keeps scanning `edit.rs` alone, where `choose` lives, so it continues to guard the claim it states: the selection names no program.
The submodule names vim and nvim as programs to recognise, not to fall back to; its own documentation says so.

Alternative: widen the scan to exclude a marked region, or delete it.
Rejected because either weakens a guard on a stated decision for the sake of an unrelated one.

### `Editor::open` builds its vector through one function

`Editor` gains `fn argv(&self, path: &Path) -> Vec<OsString>` returning everything after the program, and `open` uses it.
The unit tests assert on that function's output, so the vector tested is the vector executed.

## Risks / Trade-offs

- [The operator's vimrc or `init.lua` registers a `BufRead` autocommand after safix's and switches swap, undo or history back on] → Stated in the limit; safix cannot reach inside the editor's configuration.
- [nvim plugins write or send the buffer elsewhere: session managers, undo-tree plugins with their own directory, language servers and completion services that receive the buffer's text] → Stated in the limit by kind, not by plugin name.
- [The operator's vimrc or `init.lua` sets `updatecount` back to a non-zero value, which re-enables the swap file `-n` prevented] → Stated in the limit as configuration that overrides the options.
- [A future vim or nvim rejects one of the option names] → The edit would fail with the editor's exit status and write nothing, which is the existing failed-editor outcome, not a silent leak.
- [`vim` on a host is a wrapper script that passes its arguments to something other than vim] → Out of scope; the rule reads the program word, as the non-goals state.

## Migration Plan

None.
No declaration, file or option changes; the next `safix edit` run with vim or nvim carries the options.

## Open Questions

None.
