## ADDED Requirements

### Requirement: The far side of a mapping is where a field is declared

A mapping MAY declare, on its far side and nowhere else, metadata to be carried beside the value: a username, a URL, notes, and tags.
Each of the first three SHALL be a single value or absent, and tags SHALL be a list, defaulting to empty.
Each value SHALL be either a literal string or a reference naming another entry of the mapping's own person, whose plaintext is read when the mapping is converged and never interpolated at evaluation.
The safix side of a mapping SHALL NOT carry a field declaration on any target.

#### Scenario: The declaration is on the far side

- **WHEN** a mapping declares fields
- **THEN** they are declared on the half of the mapping that names the other store
- **AND** no field may be declared on the safix half, because a safix entry is a file, a key inside it and an audience, with no slot for a URL, a note or a tag

#### Scenario: A field may name another entry rather than carry a literal

- **WHEN** a field is declared as a reference to another entry of the same person
- **THEN** evaluation accepts it without reading that entry
- **AND** nothing about the referenced entry's plaintext appears in any evaluated output, because an evaluated declaration is world-readable and the referenced value is not

#### Scenario: A reference to an entry the person does not hold is refused

- **WHEN** a field names an entry the mapping's own person does not hold
- **THEN** evaluation refuses, naming the mapping, the field and the entry
- **AND** the refusal is the one an unresolvable safix side already produces, because it is the same question asked about a second name

### Requirement: A field the target cannot carry is refused rather than dropped

Each target SHALL declare, per field, whether it can carry that field and over which channel — an argument vector or a pipe — and evaluation SHALL refuse a declared field the target cannot carry at all, naming the target and the field.
Evaluation SHALL further refuse a field declared as a reference to another entry when the target's only channel for that field is an argument vector, because the referenced plaintext is a secret value and a secret value may not travel an argument vector.
The runtime SHALL make both refusals again over the declarations it is handed, so that a projection no evaluation of these modules ever saw cannot converge having quietly written less than it was told to.

#### Scenario: An unsupported field is refused at evaluation

- **WHEN** a mapping declares a field the target has no way to write
- **THEN** evaluation refuses, naming the target and the field
- **AND** nothing is dropped silently, because a mirror that writes less than the declaration says lies about what it holds

#### Scenario: A secret field on an argument-vector channel is refused

- **WHEN** a mapping declares a field as a reference to another entry and the target's only channel for that field is an argument vector
- **THEN** evaluation refuses, naming the target and the field
- **AND** the reason given is that the referenced value is a secret and a secret value travels a pipe

#### Scenario: The runtime refuses what evaluation would have refused

- **WHEN** the runtime is handed a projected declaration carrying a field the target cannot carry, or a referenced field whose only channel is an argument vector
- **THEN** it refuses with its own variant of that refusal, naming the target and the field
- **AND** it refuses rather than converging the fields it can carry, so the two halves cannot disagree about what the declaration meant

## MODIFIED Requirements

### Requirement: Evaluation refuses every mapping mistake that is local to the consumer

Evaluation SHALL refuse a mapping whose safix side is unresolvable, whose target has a second producer, which duplicates or contradicts another mapping, whose clan-side fields do not match what its declared placement requires, whose declared metadata the target cannot carry, or whose id is one the command line reserves as a target keyword.
The reserved target keywords SHALL be one list, read by every mapping module rather than restated in each, and the runtime SHALL carry the same list in the same order with a check holding the two equal.

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

- **WHEN** a mapping's id is one of the reserved target keywords — `clan`, `keepassxc`, `pass`, `bitwarden`, `1password`, or `all`
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is that `sync` and `audit` read their first argument as a target keyword or a mapping name, never both, so no declared mapping may hold a word either verb reads as a target
- **AND** a word is reserved from the moment it is on the list, whether or not a target answers to it yet, so that adding a target does not change what a declaration means

#### Scenario: Every mapping module refuses the same list, and so does the runtime

- **WHEN** the reserved target keywords are compared across the mapping modules and the runtime
- **THEN** all of them are the same words in the same order, read from one declaration rather than restated
- **AND** a difference between the declared list and the runtime's own fails a check rather than passing silently
