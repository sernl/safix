## MODIFIED Requirements

### Requirement: Evaluation refuses every mapping mistake that is local to the consumer

Evaluation SHALL refuse a mapping whose safix side is unresolvable, whose target has a second producer, which duplicates or contradicts another mapping, or whose id is one the command line reserves as a target keyword.

#### Scenario: An unresolvable safix side is refused

- **WHEN** a mapping names a user who does not exist, or an entry that user does not carry
- **THEN** evaluation refuses naming which half is wrong

#### Scenario: Two producers for one value are refused

- **WHEN** a clan-to-safix mapping's target is also produced by a generator
- **THEN** evaluation refuses
- **AND** the reason given is the one already given for two generators naming one output: the winner is whichever ran last

#### Scenario: Two mappings writing one target are refused

- **WHEN** two mappings name the same destination
- **THEN** evaluation refuses naming both mappings

#### Scenario: A two-way relationship is refused however it is spelled

- **WHEN** one pair of endpoints appears in two mappings with opposite directions
- **THEN** evaluation refuses
- **AND** the refusal states that this is a two-way synchronisation with no conflict resolution

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is `clan`, `keepassxc`, or `all`
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is that `sync` and `audit` read their first argument as a target keyword or a mapping name, never both, so no declared mapping may hold a word either verb reads as a target

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
- **THEN** it states that clan's own `vars export` moves values out of clan, while a safix-to-clan mapping's convergence moves a value the opposite way, so a word one tool already uses for its own verb would mean the opposite thing if reused for this option
- **AND** it states that a declaration is read without a tool in hand to be relative to
