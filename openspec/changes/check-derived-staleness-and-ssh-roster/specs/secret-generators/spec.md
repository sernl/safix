## MODIFIED Requirements

### Requirement: A dependency chains generation and enrols its generator in rotation

Declaring a dependency SHALL make the depending generator re-run whenever the depended-upon value is regenerated, transitively, in dependency order.
The set to be re-run SHALL be shown and confirmed before anything runs.
A depended-upon value written outside a cascade SHALL leave the depending generator reported by `check` until it re-runs.

#### Scenario: Regenerating cascades downstream

- **WHEN** a value with dependents is regenerated
- **THEN** every generator downstream of it re-runs, transitively, in dependency order
- **AND** the cascade is not optional, because a value derived from a retired input is indistinguishable afterwards from one derived from the current input

#### Scenario: The cascade is confirmed once

- **WHEN** a cascade is about to run
- **THEN** the full set is listed and one confirmation is requested
- **AND** a documented flag answers that confirmation in advance

#### Scenario: A dependency written outside a cascade is reported

- **WHEN** a depended-upon value is written by `set`, `edit` or `sync`, none of which run generators
- **THEN** `check` reports every generator declaring that dependency as derived from a value that has changed since it ran
- **AND** the remedy regenerates the depending entry, whose own cascade re-runs everything derived from it in turn

#### Scenario: A dependency on another person's secret is refused

- **WHEN** a generator declares a dependency on a secret held by another person
- **THEN** evaluation fails naming the generator and the dependency
- **AND** the message states that the refusal is structural: the machine running the generator holds no identity that opens the other person's file, so there is no plaintext to read

#### Scenario: A dependency naming a secret the user does not hold is refused

- **WHEN** a generator names a dependency that is not in the same user's resolved set
- **THEN** evaluation fails naming the generator and the missing name
