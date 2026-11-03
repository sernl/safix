## ADDED Requirements

### Requirement: A vim or nvim editor is opened with its own persistence switched off

When the chosen editor's program is `vim` or `nvim`, named bare or as a path whose last component is that name, the runtime SHALL pass `-n`, so no swap file is created, and `--cmd` with an autocommand switching off backups, modelines, shell temporary files, swap files, undo files and write backups for every buffer read, plus the history file: `shadafile=NONE` for nvim, `viminfo=""` for vim.
These SHALL come after the operator's arguments and before the staged path.

#### Scenario: nvim receives the nvim options

- **WHEN** the editor variable holds `nvim`
- **THEN** the argument vector is `nvim`, `-n`, `--cmd`, `au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup shadafile=NONE`, and the staged path, in that order

#### Scenario: vim receives the vim options

- **WHEN** the editor variable holds `vim`
- **THEN** the argument vector is `vim`, `-n`, `--cmd`, `au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup viminfo=""`, and the staged path, in that order

#### Scenario: A path naming nvim is recognised

- **WHEN** the editor variable holds an absolute path whose last component is `nvim`
- **THEN** the argument vector carries the nvim options exactly as it does for the bare name

#### Scenario: The operator's arguments come first

- **WHEN** the editor variable holds `nvim -u NONE`
- **THEN** the argument vector is `nvim`, `-u`, `NONE`, `-n`, `--cmd`, the nvim autocommand, and the staged path, in that order

#### Scenario: No swap file is created for the staged buffer

- **WHEN** vim or nvim is opened on the staged path
- **THEN** no swap file for it is created in any directory, including the editor's own state directory
- **AND** the reason is recorded: without `-n`, nvim creates its swap file in its own state directory before the autocommand runs, and that file records the user, the host and the staged path

#### Scenario: Any other editor is opened unchanged

- **WHEN** the editor variable holds a program whose last component is neither `vim` nor `nvim`, including `vi`, `view`, `gvim` and `code --wait`
- **THEN** the argument vector is the program, the operator's arguments and the staged path, and nothing else

#### Scenario: Bare vi is not recognised and the reason is recorded

- **WHEN** the rule's scope is documented
- **THEN** it states that `vi` is excluded because on many hosts it is nvi or busybox vi, neither of which accepts `--cmd`, so passing the options would make the edit fail rather than harden it

#### Scenario: There is no opt-out

- **WHEN** the verb's options and the environment variables it reads are enumerated
- **THEN** none of them disables the options for vim or nvim
- **AND** the reason is recorded: the options apply only to the buffer the verb staged, and an operator who wants other behaviour names a different program

## MODIFIED Requirements

### Requirement: The buffer is a staged plaintext file and is removed on every exit path

The file handed to the editor SHALL be created inside the private staging root that `plaintext-staging` governs, and SHALL be removed with that root however the run ends.

#### Scenario: The buffer is in the staging root

- **WHEN** the path handed to the editor is examined
- **THEN** it is inside the run's staging root

#### Scenario: What the editor leaves behind goes too

- **WHEN** the editor writes swap, backup or undo files beside the buffer
- **THEN** they are inside the staging root and are removed with it

#### Scenario: An editor writing outside the root is outside the guarantee

- **WHEN** an editor writes swap files, undo history, backups or its own history file to a location of its own
- **THEN** the runtime does not reach it
- **AND** the verb's documentation states the limit rather than leaving it to be discovered: for vim and nvim the options passed on the command line prevent these writes for the staged buffer, and what remains is every other editor, and vim or nvim configuration or plugins that write or send the buffer's contents elsewhere, including an autocommand registered after the runtime's that switches an option back on
