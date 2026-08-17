## Purpose

`check` learns to report the drift the new definition record makes detectable, and `set` learns to read a stream, which is the contract safix's own bridge already relies on when it writes into clan.

## ADDED Requirements

### Requirement: check reports a value minted under a definition that has changed

`safix check` SHALL report a finding for every generated value whose recorded definition digest no longer matches the current declaration, naming the entry and both remedies: regenerate the value, or revert the declaration edit.
A value with no record predates the record and SHALL NOT be a finding.

#### Scenario: An edited definition is reported

- **WHEN** a generator's declaration changes after its value was minted
- **THEN** `check` reports the entry as minted under a definition that no longer exists
- **AND** the finding names regeneration and reverting the edit as the remedies
- **AND** the finding carries no value

#### Scenario: Regeneration clears the finding

- **WHEN** the value is regenerated under the current declaration
- **THEN** the finding is gone on the next `check`

#### Scenario: What is out of scope stays quiet

- **WHEN** an entry has no generator, or a generated value has no record
- **THEN** no drift finding exists for it

### Requirement: set reads a stream when one is offered

`safix set` SHALL read the value from standard input when standard input is not a terminal, storing the bytes exactly as received, with no prompt and no confirmation.
When standard input is a terminal, the hidden double prompt SHALL remain the behaviour.

#### Scenario: A piped value is stored as its own bytes

- **WHEN** a value is piped into `safix set <name>`
- **THEN** the stored bytes are exactly the piped bytes
- **AND** nothing prompts and nothing asks for confirmation
- **AND** the value reaches no argument vector and no environment variable

#### Scenario: Empty input keeps its refusal

- **WHEN** the pipe yields no bytes
- **THEN** the write is refused as an empty value, exactly as an empty prompt is

#### Scenario: The terminal path is unchanged

- **WHEN** standard input is a terminal
- **THEN** the hidden prompt, the confirmation, and the single-line contract hold as they do today
