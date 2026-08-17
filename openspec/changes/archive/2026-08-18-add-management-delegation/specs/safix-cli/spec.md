## Purpose

The scaffolding verbs learn whose act they are performing, and group membership gets a verb of its own instead of a hand edit.

## ADDED Requirements

### Requirement: Scaffolding verbs honour the delegation records

When the target person declares `managedBy`, `enroll` and the record-editing half of onboarding SHALL refuse an acting identity that is not among that organization's managers, reading the acting identity from the same git identity the resulting commit carries, and a permitted scaffold's commit SHALL record the organization context.
When the target declares no `managedBy`, the verbs SHALL behave exactly as before.

#### Scenario: A manager scaffolds and the record says so

- **WHEN** alice, a manager of acme, enrolls a card for bob, who declares `managedBy` acme
- **THEN** the scaffold proceeds and its commit records the acme context

#### Scenario: An outsider is refused before anything is edited

- **WHEN** mallory, no manager of acme, attempts the same scaffold
- **THEN** the verb refuses before editing any file, naming the organization and where its managers are declared

#### Scenario: Unmanaged people are untouched by the feature

- **WHEN** the target person declares no `managedBy`
- **THEN** the verb neither reads nor mentions delegation

### Requirement: Group membership is a verb with the narrowing disclosure

`safix group add <group> <subject>` and `safix group remove <group> <subject>` SHALL edit the group's declaration as text, parsed by the real parser before staging and committed, with `remove` printing the not-retroactive disclosure and naming the revocation report that will carry the shrink.
Both SHALL honour the delegation records over groups the organization's silo declarations cover, and both SHALL refuse a subject or group the fleet does not declare.

#### Scenario: An addition is one inserted line

- **WHEN** alice runs `safix group add oncall bob`
- **THEN** the group's declaration gains bob as one inserted line, parsed before staging, and the commit names the act

#### Scenario: A removal says what it does not undo

- **WHEN** alice runs `safix group remove oncall bob`
- **THEN** the edit lands and the verb prints that bob has seen what the group could read and rotation is the remedy
- **AND** the next `check` reports the shrink as the revocation it is
