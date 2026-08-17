## Purpose

Authoring a value in the operator's editor: which editor is chosen, where the buffer lives, and what happens when the editor fails, changes nothing, or leaves an empty file.

## ADDED Requirements

### Requirement: Editing is its own verb

The command SHALL provide an `edit` verb addressing an entry by name, and SHALL NOT provide editor input as an option on the verb that writes a value from a stream.

#### Scenario: The verb addresses an entry by name

- **WHEN** an operator edits a value
- **THEN** they name the entry
- **AND** they do not name a file

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

- **WHEN** an editor is configured to write undo history or backups to a location of its own
- **THEN** the runtime does not reach it
- **AND** this limit is stated in the verb's documentation rather than left to be discovered

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
