## ADDED Requirements

### Requirement: A clan mapping has no field surface

A clan mapping SHALL NOT declare metadata beside its value, and evaluation SHALL refuse a field declared on either half of one.
A clan var is a file's bytes and clan's own command offers no field beside it, so there is nothing for a declaration to name and nothing a report could compare.

#### Scenario: A field declared on a clan mapping is refused

- **WHEN** a clan mapping declares a username, a URL, notes or tags on either half
- **THEN** evaluation refuses, naming the mapping
- **AND** the reason given is that clan's store holds a value and no metadata beside it

#### Scenario: The absence is stated rather than left as an omission

- **WHEN** the clan mapping's declaration is read
- **THEN** it states that this target carries no field, and why
- **AND** an author who has read another target's field declaration does not have to infer the difference from silence

## MODIFIED Requirements

### Requirement: sync toward clan refuses when clan already considers the generator stale

`sync`, for the clan target's safix-to-clan direction, SHALL refuse a mapping whose clan-side generator has a recorded validation that no longer matches its definition, before writing anything, and SHALL provide no option to proceed.
This refusal SHALL remain clan's own: it SHALL be raised by the clan far side rather than by any contract shared with another target, and no other target SHALL inherit it.

#### Scenario: A stale generator refuses before the write

- **WHEN** `sync clan` reaches a safix-to-clan mapping whose clan-side generator clan reports as having an outdated validation
- **THEN** the mapping is refused and nothing is written into clan
- **AND** the refusal names the machine and the generator

#### Scenario: The refusal names both remedies

- **WHEN** the refusal is read
- **THEN** it names updating the clan-side definition
- **AND** it names declaring the mapping in the other direction instead

#### Scenario: The staleness is clan's answer rather than safix's computation

- **WHEN** the runtime establishes whether a generator is stale
- **THEN** it obtains the answer by invoking clan's own command
- **AND** it reads no recorded validation and computes no hash

#### Scenario: No option defeats the refusal

- **WHEN** `sync`'s arguments are enumerated
- **THEN** none of them proceeds past this refusal

#### Scenario: The refusal stays outside the shared far-side contract

- **WHEN** the shared contract every far side is reached through is read
- **THEN** it says nothing about a stale generator
- **AND** the refusal is raised by the clan far side alone, with the reason it cannot be shared recorded beside it: it is a question only clan's own command can answer, asked only once a write will happen
