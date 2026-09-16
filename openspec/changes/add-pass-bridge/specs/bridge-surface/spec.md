## MODIFIED Requirements

### Requirement: Evaluation refuses every mapping mistake that is local to the consumer

Evaluation SHALL refuse a mapping whose safix side is unresolvable, whose target has a second producer, which duplicates or contradicts another mapping, whose clan-side fields do not match what its declared placement requires, or whose id is one the command line reserves as a target keyword.
The reserved words SHALL be one list, exported once and read by every mapping module of every target, so the words evaluation refuses and the words the command line reads as targets cannot come to disagree.

#### Scenario: An unresolvable safix side is refused

- **WHEN** a mapping names a user who does not exist, or an entry that user does not carry
- **THEN** evaluation refuses naming which half is wrong

#### Scenario: Two producers for one value are refused

- **WHEN** a clan-to-safix or two-way mapping's target is also produced by a generator
- **THEN** evaluation refuses
- **AND** the reason given is the one already given for two generators naming one output: the winner is whichever ran last

#### Scenario: Two mappings writing one target are refused

- **WHEN** two mappings name the same destination
- **THEN** evaluation refuses naming both mappings
- **AND** for a shared placement the destination is identified by generator and file alone, so two mappings naming the same shared var through different machines still collide

#### Scenario: A two-way relationship is refused however it is spelled

- **WHEN** one pair of endpoints appears in two mappings with opposite one-way directions
- **THEN** evaluation refuses
- **AND** the refusal states that a two-way relationship is declared once, as a single mapping whose direction is two-way, naming the two conflicting mappings

#### Scenario: A two-way declaration of the same relationship is accepted

- **WHEN** one pair of endpoints is declared as a single mapping whose direction is two-way
- **THEN** evaluation produces no message about it

#### Scenario: A placement's required field is refused when absent or present out of place

- **WHEN** a mapping's placement is per-machine and no machine is declared, or a machine is declared for a placement that does not call for it
- **THEN** evaluation refuses naming the mapping, its placement, and which field is missing or out of place

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is one of the reserved target words, `pass` among them
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is that `sync` and `audit` read their first argument as a target keyword or a mapping name, never both, so no declared mapping may hold a word either verb reads as a target
- **AND** the refusal reads the shared reserved list rather than a copy of it, so a target added to the command line reserves its word in every mapping module at once

## ADDED Requirements

### Requirement: A consumer holding a mapping named for a new target is told what broke and what to do

Reserving a word that was previously an admissible mapping id SHALL be stated as a breaking change in the change that makes the word mean something, and the refusal a consumer meets SHALL name both the collision and the remedy.

#### Scenario: The break is stated where a consumer looks

- **WHEN** a target whose keyword was previously a legal mapping id is added
- **THEN** the change states the break, naming the word and which declarations stop evaluating
- **AND** the refusal itself names the mapping, the word, and that renaming the mapping is the remedy
