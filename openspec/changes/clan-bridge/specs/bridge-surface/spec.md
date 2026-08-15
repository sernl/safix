## Purpose

The declared relationship between an entry in safix and a var in clan: what each side names, how direction is written so that no reader has to supply a frame of reference, and which mistakes evaluation can refuse given that half of every mapping lives in another flake.

## ADDED Requirements

### Requirement: A bridge relationship is declared rather than passed as arguments

Each relationship between a clan var and a safix entry SHALL be declared in the consumer's nix, naming both endpoints and a direction, and no verb SHALL accept an endpoint as a command-line argument.

#### Scenario: A mapping names both sides

- **WHEN** a mapping is declared
- **THEN** it names a clan machine, generator and file
- **AND** it names a safix user and entry name

#### Scenario: A run takes no endpoint arguments

- **WHEN** either transfer verb's arguments are enumerated
- **THEN** none of them names a machine, a generator, a file, a user or an entry
- **AND** the mappings a run acts on come from the declarations

#### Scenario: The mapping carries its own identifier

- **WHEN** a mapping is reported on, committed, or refused
- **THEN** the mapping's own declared name appears
- **AND** it is not derived from either endpoint

#### Scenario: The clan flake is declared once

- **WHEN** more than one clan flake is declared
- **THEN** evaluation refuses
- **AND** the refusal states that one consumer bridges one clan

### Requirement: Direction is written as its endpoints, not relative to a tool

A mapping's direction SHALL be one of two values naming the source and the destination, and SHALL NOT be spelled with a word whose meaning depends on which tool is speaking.

#### Scenario: The two permitted values name endpoints

- **WHEN** a direction is declared
- **THEN** it is either clan-to-safix or safix-to-clan

#### Scenario: Any other direction is refused

- **WHEN** a direction outside those two is declared
- **THEN** evaluation refuses naming both permitted values

#### Scenario: The reason is recorded where an author meets it

- **WHEN** the direction option's documentation is read
- **THEN** it states that the verb named export moves values out of clan in one tool and into clan in the other
- **AND** it states that a declaration is read without a tool in hand to be relative to

### Requirement: Evaluation refuses every mapping mistake that is local to the consumer

Evaluation SHALL refuse a mapping whose safix side is unresolvable, whose target has a second producer, whose source has nothing to send, or which duplicates or contradicts another mapping.

#### Scenario: An unresolvable safix side is refused

- **WHEN** a mapping names a user who does not exist, or an entry that user does not carry
- **THEN** evaluation refuses naming which half is wrong

#### Scenario: Two producers for one value are refused

- **WHEN** a clan-to-safix mapping's target is also produced by a generator
- **THEN** evaluation refuses
- **AND** the reason given is the one already given for two generators naming one output: the winner is whichever ran last

#### Scenario: An export with nothing to send is refused

- **WHEN** a safix-to-clan mapping's source entry has neither a generator nor a declared value
- **THEN** evaluation refuses

#### Scenario: Two mappings writing one target are refused

- **WHEN** two mappings name the same destination
- **THEN** evaluation refuses naming both mappings

#### Scenario: A two-way relationship is refused however it is spelled

- **WHEN** one pair of endpoints appears in two mappings with opposite directions
- **THEN** evaluation refuses
- **AND** the refusal states that this is a two-way synchronisation with no conflict resolution

### Requirement: The clan half is checked at run time and the asymmetry is stated

Evaluation SHALL NOT claim to have verified the clan side of a mapping, and the runtime SHALL refuse a mapping whose clan side does not resolve, naming the machine, the generator and the file.

#### Scenario: Evaluation does not verify the far side

- **WHEN** the evaluation-time refusals are documented
- **THEN** they state that the clan side lives in another flake and is not among them

#### Scenario: A missing clan side is a run-time refusal that names it

- **WHEN** a transfer reaches a mapping whose clan side does not resolve
- **THEN** the run refuses for that mapping
- **AND** the refusal names the machine, the generator and the file

#### Scenario: The messages are assertable against literals

- **WHEN** the bridge refusals are exposed to a consumer's checks
- **THEN** they are exposed as a message function and a builder over it
- **AND** a fixture can assert a message against a literal without building a derivation
