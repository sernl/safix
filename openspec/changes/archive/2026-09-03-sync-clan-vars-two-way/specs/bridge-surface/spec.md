## MODIFIED Requirements

### Requirement: A bridge relationship is declared rather than passed as arguments

Each relationship between a clan var and a safix entry SHALL be declared in the consumer's nix, naming both endpoints and a direction, and no verb SHALL accept an endpoint as a command-line argument.

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

Evaluation SHALL refuse a mapping whose safix side is unresolvable, whose target has a second producer, which duplicates or contradicts another mapping, whose clan-side fields do not match what its declared placement requires, or whose id is one the command line reserves as a target keyword.

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

- **WHEN** a mapping's id is `clan`, `keepassxc`, or `all`
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is that `sync` and `audit` read their first argument as a target keyword or a mapping name, never both, so no declared mapping may hold a word either verb reads as a target

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

## ADDED Requirements

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
