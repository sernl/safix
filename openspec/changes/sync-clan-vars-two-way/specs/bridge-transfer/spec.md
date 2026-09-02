## MODIFIED Requirements

### Requirement: Two verbs move declared mappings, one per direction

The command SHALL provide a verb that acts on clan-to-safix mappings, a verb that acts on safix-to-clan mappings, and a verb that acts on two-way mappings, each acting on one named mapping or on all mappings of its direction.

#### Scenario: Each verb acts on its own direction only

- **WHEN** a verb runs
- **THEN** it acts on mappings of its direction
- **AND** it does not act on mappings of either other direction

#### Scenario: A run may be scoped or complete

- **WHEN** a verb is given a mapping's name
- **THEN** it acts on that mapping alone
- **AND** when given no mapping's name it acts on every mapping of its direction

#### Scenario: A mapping named to the verb of the other direction is told which verb acts on it

- **WHEN** a verb is given the name of a mapping declared with a different direction
- **THEN** it refuses naming the direction the mapping is declared with
- **AND** it names the verb that does act on it
- **AND** the refusal is distinct from the one for a name nothing declares

#### Scenario: A two-way mapping named to import or export is refused by name

- **WHEN** import or export is given the name of a mapping declared two-way
- **THEN** it refuses naming two-way as the mapping's direction
- **AND** it names bridge as the verb that acts on it

#### Scenario: A one-way mapping named to bridge is refused by name

- **WHEN** bridge is given the name of a mapping declared clan-to-safix or safix-to-clan
- **THEN** it refuses naming that direction
- **AND** it names import or export, whichever acts on it, as the verb that does

#### Scenario: The verbs appear in the command's help

- **WHEN** the command is asked for help with no verb
- **THEN** all three verbs appear with their directions stated

### Requirement: clan is the authority on its own store and is reached only through its command

Every read of a clan value and every write of one SHALL be performed by invoking clan's own command as a subprocess, and the runtime SHALL NOT read, write, decrypt, encrypt or parse clan's stored files.
When a mapping's placement is shared or per-export, the machine named on clan's command line to address it SHALL itself be obtained by invoking clan's command, never from a second field a consumer declares.

#### Scenario: Reading a clan value delegates

- **WHEN** a clan-side value is needed
- **THEN** it is obtained by invoking clan's command
- **AND** the value arrives on that process's standard output

#### Scenario: Writing a clan value delegates

- **WHEN** a clan-side value is written
- **THEN** it is written by invoking clan's command
- **AND** the value is supplied on that process's standard input

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
- **THEN** every verb refuses before transferring anything
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
