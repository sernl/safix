## MODIFIED Requirements

### Requirement: clan is the authority on its own store and is reached only through its command

Every read of a clan value, every write of one, and every enumeration of a machine's vars SHALL be performed by invoking clan's own command as a subprocess, and the runtime SHALL NOT read, write, decrypt, encrypt or parse clan's stored files.
When a mapping's placement is shared or per-export, the machine named on clan's command line to address it SHALL itself be obtained by invoking clan's command, never from a second field a consumer declares.

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
- **THEN** `sync`, for the clan target, refuses before transferring anything, whatever mix of directions the run would have acted on
- **AND** the refusal states that clan is the authority on its own store
- **AND** the run does not proceed with a subset of its mappings

#### Scenario: A shared or per-export mapping's address is discovered from clan, not declared twice

- **WHEN** a mapping's placement is shared or per-export
- **THEN** the runtime asks clan which machines it has, and tries them in turn against the mapping's generator until one resolves it
- **AND** no option or field on the mapping names that machine

#### Scenario: An unaddressable shared or export placement refuses naming the generator

- **WHEN** no machine clan has resolves a shared or per-export mapping's generator
- **THEN** the mapping is refused
- **AND** the refusal names the mapping, the placement, the generator and the file, and states that no machine in clan's own fleet exposed it

## ADDED Requirements

### Requirement: The audit reports clan vars no declared mapping accounts for

For every machine this capability enumerates for a currently declared mapping — the machine a per-machine-placement mapping declares, or the addressing machine discovered for a shared-placement mapping — the audit verb SHALL enumerate the vars clan's own command reports for that machine, and SHALL report as information each one whose id no currently declared mapping's clan side claims — including a var whose only mapping has since been removed from the declarations.
This enumeration SHALL be scoped to machines enumerated for a currently declared mapping and SHALL NOT extend to a clan machine that no declared mapping names or resolves.
No mode SHALL delete, export, or import a var by virtue of this report alone, and this report SHALL NOT change the audit's exit status, which continues to answer only whether every compared mapping agreed.

#### Scenario: A var no mapping names is reported

- **WHEN** a machine enumerated for a declared mapping holds a var that no currently declared mapping's clan side claims
- **THEN** the audit reports it naming the machine and the var
- **AND** nothing is written on either side of the boundary

#### Scenario: A removed mapping's var keeps appearing until a person acts

- **WHEN** a mapping is removed from the declarations after its var was created
- **THEN** the next audit reports that var among the ones no mapping names
- **AND** the var is not deleted, exported, or imported by the audit

#### Scenario: Enumeration is scoped to the machines the bridge currently names or resolves

- **WHEN** the audit enumerates clan vars
- **THEN** it considers only machines enumerated for a currently declared mapping
- **AND** it does not enumerate a clan machine that no declared mapping names or resolves, even when clan manages one

#### Scenario: A per-export-placement mapping is invisible to this report, structurally

- **WHEN** a currently declared mapping's placement is per-export
- **THEN** its clan side is not enumerated and not compared against any machine's listing
- **AND** the reason recorded is that `clan vars list` cannot surface a per-export-placed var for any machine, not that this report chooses to skip it

#### Scenario: Lingering never changes the exit status

- **WHEN** a run finds one or more vars no mapping accounts for and every compared mapping agrees
- **THEN** the audit still exits reporting agreement
- **AND** the vars no mapping accounts for are reported alongside it as information

#### Scenario: This is the clan target's own lingering report, alongside keepassxc's

- **WHEN** `audit` runs bare or with `clan` as its target
- **THEN** this requirement's lingering report is what appears for clan vars
- **AND** naming `keepassxc` as the target instead surfaces `keepassxc-sync`'s own lingering report, which `rename-transfer-verbs` adds as that capability's parallel gap-fill
