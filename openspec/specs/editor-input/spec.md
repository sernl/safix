# editor-input Specification

## Purpose

Authoring a value in the operator's editor: which editor is chosen, where the buffer lives, and what happens when the editor fails, changes nothing, or leaves an empty file.

## Requirements

### Requirement: Editing is its own verb

The command SHALL provide an `edit` verb addressing an entry by name or by choosing one from those the user holds, and SHALL NOT provide editor input as an option on the verb that writes a value from a stream.
When no name is given, the verb SHALL offer the entries that user holds excluding every public placement, and SHALL determine the operator's editor before the choice is offered.

#### Scenario: The verb addresses an entry by name

- **WHEN** an operator edits a value
- **THEN** they name the entry
- **AND** they do not name a file

#### Scenario: The verb addresses an entry by choosing one

- **WHEN** an operator edits with no name given
- **THEN** the entries that user holds are offered and choosing one proceeds exactly as naming it would
- **AND** they do not name a file in this form either

#### Scenario: What is offered is what could be edited

- **WHEN** the entries offered for editing are enumerated
- **THEN** no public placement is among them
- **AND** the reason is recorded: a public output is refused for editing because the generator declaring it is what mints it, and offering a choice that would then be refused is a worse interface than not offering it

#### Scenario: The editor is settled before the choice is offered

- **WHEN** the nameless form is run and neither editor variable is set
- **THEN** the run is refused before any entry is offered and before anything is decrypted
- **AND** the ordering is the same one the named form already holds — the editor is known before anything is decrypted or staged — so no operator browses, previews and chooses only to learn there was never an editor

#### Scenario: The stream-writing verb is unchanged

- **WHEN** the stream-writing verb's options are enumerated
- **THEN** none of them invokes an editor
- **AND** its requirement that the value arrives on a stream continues to hold without exception

#### Scenario: The reason for a verb rather than an option is recorded

- **WHEN** the decision is documented
- **THEN** it states that the two verbs have different custody profiles — one reads the existing value and hands it to a program the runtime does not control, and the other does neither
- **AND** it states that an option would make custody a function of a flag

### Requirement: The editor is the one the operator chose, or the run refuses

The runtime SHALL take the editor from the operator's environment, preferring the visual editor variable and falling back to the editor variable, and SHALL refuse when neither is set rather than choosing one.

#### Scenario: The preferred variable wins

- **WHEN** both environment variables are set
- **THEN** the visual one is used

#### Scenario: Neither set is a refusal

- **WHEN** neither variable is set
- **THEN** the run is refused before anything is decrypted or staged
- **AND** the refusal names both variables

#### Scenario: No editor is chosen on the operator's behalf

- **WHEN** the runtime's editor selection is read
- **THEN** it contains no fallback to a named program
- **AND** the reason is recorded: dropping an operator into an editor they did not choose, with a secret in the buffer, produces an accidental write or an accidental abandonment that the runtime cannot tell apart

#### Scenario: An editor command with arguments works

- **WHEN** the editor variable holds a command with arguments
- **THEN** it is split into a program and its arguments and executed directly
- **AND** it is not executed through a shell

#### Scenario: The path is an argument and the value is not

- **WHEN** the editor process's argument vector is examined
- **THEN** it contains the staged file's path
- **AND** it does not contain the value

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

### Requirement: An editing run writes only a changed, non-empty value

The runtime SHALL write nothing when the editor fails, nothing when the buffer is unchanged, and SHALL refuse when the buffer is empty.

#### Scenario: A failed editor writes nothing

- **WHEN** the editor exits non-zero
- **THEN** no value is written and nothing is committed
- **AND** the refusal names the exit status

#### Scenario: An unchanged buffer commits nothing

- **WHEN** the buffer is byte-identical to what was staged
- **THEN** no value is written and nothing is committed

#### Scenario: An emptied buffer is refused

- **WHEN** the buffer is empty after editing
- **THEN** the run is refused
- **AND** the refusal is the same one an empty value produces elsewhere, because an empty value is the state a truncated write leaves behind

#### Scenario: Editing an entry that has no value yet is authoring

- **WHEN** the named entry holds no value
- **THEN** the editor opens on an empty buffer
- **AND** saving a non-empty buffer writes the value through the same path the stream-writing verb writes through

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
