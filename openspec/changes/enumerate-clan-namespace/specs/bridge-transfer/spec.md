## MODIFIED Requirements

### Requirement: clan is the authority on its own store and is reached only through its command

Every read of a clan value, every write of one, and every enumeration of a machine's vars SHALL be performed by invoking clan's own command as a subprocess, and the runtime SHALL NOT read, write, decrypt, encrypt or parse clan's stored files.

#### Scenario: Reading a clan value delegates

- **WHEN** a clan-side value is needed
- **THEN** it is obtained by invoking clan's command
- **AND** the value arrives on that process's standard output

#### Scenario: Writing a clan value delegates

- **WHEN** a clan-side value is written
- **THEN** it is written by invoking clan's command
- **AND** the value is supplied on that process's standard input

#### Scenario: Enumerating a machine's vars delegates

- **WHEN** the audit verb determines which vars a machine's namespace holds
- **THEN** it is obtained by invoking clan's command
- **AND** no secret var's value is read to make that determination

#### Scenario: No store implementation exists in the runtime

- **WHEN** the runtime is searched for clan's store layout, its recipient handling, or any of its backends
- **THEN** none is found
- **AND** the reason is recorded: the consumer's backend is a choice clan owns, and reimplementing one would silently support only that one

#### Scenario: The raw value is captured rather than a rendered one

- **WHEN** a clan value is read
- **THEN** the runtime establishes that it received the raw bytes rather than a rendering intended for a terminal
- **AND** the reason is recorded: clan's read command substitutes a printable form when its output is a terminal

#### Scenario: An absent clan command refuses the whole run

- **WHEN** clan's command is not available
- **THEN** both verbs refuse before transferring anything
- **AND** the refusal states that clan is the authority on its own store
- **AND** the run does not proceed with a subset of its mappings

## ADDED Requirements

### Requirement: The audit reports clan vars no declared mapping accounts for

For every machine named by a currently declared mapping, the audit verb SHALL enumerate the vars clan's own command reports for that machine, and SHALL report as information each one whose id no currently declared mapping's clan side names — including a var whose only mapping has since been removed from the declarations.
This enumeration SHALL be scoped to machines named by a currently declared mapping and SHALL NOT extend to a clan machine that no declared mapping names.
No mode SHALL delete, export, or import a var by virtue of this report alone, and this report SHALL NOT change the audit's exit status, which continues to answer only whether every compared mapping agreed.

#### Scenario: A var no mapping names is reported

- **WHEN** a machine named by a declared mapping holds a var that no currently declared mapping's clan side names
- **THEN** the audit reports it naming the machine and the var
- **AND** nothing is written on either side of the boundary

#### Scenario: A removed mapping's var keeps appearing until a person acts

- **WHEN** a mapping is removed from the declarations after its var was created
- **THEN** the next audit reports that var among the ones no mapping names
- **AND** the var is not deleted, exported, or imported by the audit

#### Scenario: Enumeration is scoped to the machines the bridge currently names

- **WHEN** the audit enumerates clan vars
- **THEN** it considers only machines named by a currently declared mapping
- **AND** it does not enumerate a clan machine that no declared mapping names, even when clan manages one

#### Scenario: Lingering never changes the exit status

- **WHEN** a run finds one or more vars no mapping accounts for and every compared mapping agrees
- **THEN** the audit still exits reporting agreement
- **AND** the vars no mapping accounts for are reported alongside it as information
