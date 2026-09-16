# bridge-surface Specification

## Purpose

The declared relationship between an entry in safix and a var in clan: what each side names, how direction is written so that no reader has to supply a frame of reference, and which mistakes evaluation can refuse given that half of every mapping lives in another flake.

## Requirements

### Requirement: A bridge relationship is declared rather than passed as arguments

Each relationship between a clan var and a safix entry SHALL be declared in the consumer's nix, naming both endpoints and a direction, and no verb SHALL accept an endpoint as a command-line argument.
Where a target's far side is a network service rather than a path, the service SHALL be declared once alongside that target's mappings, and neither it nor any credential or session token it needs SHALL be a run argument.

#### Scenario: A mapping names both sides

- **WHEN** a mapping is declared
- **THEN** it names a clan generator and file, a placement, and a machine exactly as that placement requires
- **AND** it names a safix user and entry name

#### Scenario: A run takes no endpoint arguments

- **WHEN** `sync`'s arguments are enumerated
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

#### Scenario: A network-backed target's endpoint is declared once, beside its mappings

- **WHEN** a target whose far side is a network service is declared
- **THEN** the service is named once for that target rather than per mapping, and MAY be left undeclared where the target's own client already holds that configuration
- **AND** no verb accepts it as an argument, for the reason this requirement gives for every other endpoint: a standing relationship is reviewable where a run's arguments are not

#### Scenario: A session token is not an endpoint a run may be given

- **WHEN** the arguments of every verb are enumerated
- **THEN** none of them accepts a session token, an access token, or a password for a far side
- **AND** a token a run mints for itself is confined to the environment of the children that need it, which each target's own capability states and bounds

### Requirement: Direction is written as its endpoints, not relative to a tool

A mapping's direction SHALL be one of three values — two naming a source and a destination, and a third naming neither because the value may originate on either side — and SHALL NOT be spelled with a word whose meaning depends on which tool is speaking.

#### Scenario: The two permitted values name endpoints

- **WHEN** a direction is declared
- **THEN** it is clan-to-safix, safix-to-clan, or two-way
- **AND** two-way names no source and no destination, because either side may hold the value that last moved

#### Scenario: Any other direction is refused

- **WHEN** a direction outside those three is declared
- **THEN** evaluation refuses naming all three permitted values

#### Scenario: The reason is recorded where an author meets it

- **WHEN** the direction option's documentation is read
- **THEN** it states that clan's own `vars export` moves values out of clan, while a safix-to-clan mapping's convergence moves a value the opposite way, so a word one tool already uses for its own verb would mean the opposite thing if reused for this option
- **AND** it states that a declaration is read without a tool in hand to be relative to
- **AND** it states that two-way carries no such relativity, because it names neither a source nor a destination

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
- **AND** the refusal reads the shared reserved list rather than a copy of it, so a target added to the command line reserves its word in every mapping module at once

#### Scenario: Every mapping module refuses the same list, and so does the runtime

- **WHEN** the reserved target keywords are compared across the mapping modules and the runtime
- **THEN** all of them are the same words in the same order, read from one declaration rather than restated
- **AND** a difference between the declared list and the runtime's own fails a check rather than passing silently

### Requirement: Whether an entry holds a value is not an evaluation question

Evaluation SHALL NOT refuse a safix-to-clan mapping on the ground that its source entry has no value, and SHALL treat a source entry with no generator as the ordinary hand-set export.

#### Scenario: A hand-set source is not refused

- **WHEN** a safix-to-clan mapping names a source entry that no generator produces
- **THEN** evaluation produces no message about it

#### Scenario: The reason is recorded where the refusals are documented

- **WHEN** the evaluation-time refusals are documented
- **THEN** they state that an entry is a declaration of where a value lives rather than that one is there
- **AND** they state that the question is answered when something reads the entry, and is refused there

### Requirement: The clan half is checked at run time and the asymmetry is stated

Evaluation SHALL NOT claim to have verified the clan side of a mapping, and the runtime SHALL refuse a mapping whose clan side does not resolve, naming its placement's address — a machine — the generator and the file.

#### Scenario: Evaluation does not verify the far side

- **WHEN** the evaluation-time refusals are documented
- **THEN** they state that the clan side lives in another flake and is not among them
- **AND** they state that a declared placement is not verified against clan's own generator either, beyond the share comparison this capability performs

#### Scenario: A missing clan side is a run-time refusal that names it

- **WHEN** a transfer reaches a mapping whose clan side does not resolve
- **THEN** the run refuses for that mapping
- **AND** for a per-machine placement the refusal names the machine, the generator and the file
- **AND** for a shared placement the refusal names the placement, the generator and the file

#### Scenario: The messages are assertable against literals

- **WHEN** the bridge refusals are exposed to a consumer's checks
- **THEN** they are exposed as a message function and a builder over it
- **AND** a fixture can assert a message against a literal without building a derivation

### Requirement: A generator's derived share and its mapping's clan placement agree

When a safix-to-clan mapping's source entry is produced by a generator, evaluation SHALL refuse a mismatch between that generator's derived `share` and the mapping's declared clan placement: `share = true` SHALL require `placement = shared`, and `share = false` SHALL require `placement = per-machine`.
This is the comparison `openspec/specs/secret-generators/spec.md`'s "The derived value is what the bridge compares" scenario already describes; this requirement is what performs it.

#### Scenario: A shared generator paired with a shared placement is accepted

- **WHEN** a safix-to-clan mapping's source is produced by a generator whose derived share is true
- **AND** the mapping's placement is shared
- **THEN** evaluation produces no message about it

#### Scenario: A shared generator paired with a per-machine placement is refused

- **WHEN** a safix-to-clan mapping's source is produced by a generator whose derived share is true
- **AND** the mapping's placement is per-machine
- **THEN** evaluation refuses naming the mapping, the generator, its derived share, and the declared placement

#### Scenario: A per-user generator paired with a shared placement is refused

- **WHEN** a safix-to-clan mapping's source is produced by a generator whose derived share is false
- **AND** the mapping's placement is shared
- **THEN** evaluation refuses naming the mapping, the generator, its derived share, and the declared placement

#### Scenario: A hand-set source has no share to compare

- **WHEN** a safix-to-clan mapping's source entry is not produced by a generator
- **THEN** evaluation produces no message about its placement's agreement with a share, for the same reason a hand-set source is exempt from every other generator-shaped rule: there is no generator to derive one from

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

### Requirement: A consumer holding a mapping named for a new target is told what broke and what to do

Reserving a word that was previously an admissible mapping id SHALL be stated as a breaking change in the change that makes the word mean something, and the refusal a consumer meets SHALL name both the collision and the remedy.

#### Scenario: The break is stated where a consumer looks

- **WHEN** a target whose keyword was previously a legal mapping id is added
- **THEN** the change states the break, naming the word and which declarations stop evaluating
- **AND** the refusal itself names the mapping, the word, and that renaming the mapping is the remedy
