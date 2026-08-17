## Purpose

The verb list gains editing, and the invariant that values move through pipes only is restated with the exception the generator contract introduces.

## MODIFIED Requirements

### Requirement: One command covers the lifecycle, by name and never by file

One command SHALL cover declaring, minting, authoring, editing, reading, auditing and reconciling, and every verb SHALL address an entry by name rather than by the file holding it.

#### Scenario: Editing addresses an entry by name

- **WHEN** an operator edits a value
- **THEN** they name the entry
- **AND** the file holding it is resolved from the declarations

#### Scenario: The verb list is enumerable

- **WHEN** the command is asked for help with no verb
- **THEN** the editing verb appears in the list

#### Scenario: No verb takes a file

- **WHEN** every verb's arguments are enumerated
- **THEN** none of them names an encrypted file

### Requirement: Values move through pipes wherever a pipe remains possible

Every leg of a value's journey SHALL be a pipe except those inside the private staging root, and no value SHALL reach an argument vector or an environment variable on any leg.

#### Scenario: The stream-writing and reading verbs are unchanged

- **WHEN** a value is written from standard input or read to standard output
- **THEN** it travels a pipe end to end

#### Scenario: The encrypting backend is still driven by pipes

- **WHEN** the encrypting backend is invoked for any operation
- **THEN** the value reaches it on a pipe
- **AND** no invocation names a value in its arguments or environment

#### Scenario: The exception is bounded and named

- **WHEN** the exception to this requirement is read
- **THEN** it names the generator staging root and the editor buffer
- **AND** it names no other location

#### Scenario: The change from the earlier absolute is stated

- **WHEN** this requirement is compared against the one it replaces
- **THEN** the difference is stated rather than presented as a clarification
- **AND** the reason is recorded: the interoperable generator contract is a filesystem contract, and emulating it with pipes would break tools that seek or reopen their inputs, with a truncated secret as the failure mode
