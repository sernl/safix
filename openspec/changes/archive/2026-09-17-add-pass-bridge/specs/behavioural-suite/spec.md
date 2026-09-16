## ADDED Requirements

### Requirement: A delegated tool that can be driven hermetically is driven for real by at least one check

Where safix reaches an external store through that store's own command, the suite SHALL stub that command for the behavioural claims and, when the command can run inside the hermetic check sandbox — needing no network, no account, and no unfree licence — SHALL additionally carry one check that drives the real command against a store the check creates itself.
The stub SHALL stand only for the delegation boundary: what it establishes is what safix does at that boundary, and what it cannot establish is that the argument vectors mean to the tool what safix thinks they mean.
Where the real command cannot run in the sandbox, that absence SHALL be recorded beside the target's own stub, naming the reason, rather than left as a check nobody noticed was missing.
A real-tool check SHALL name its own store, always under a directory the check made, and a run whose store is anywhere else SHALL be refused before a process is spawned.

#### Scenario: The model and the tool are both exercised

- **WHEN** a target whose tool runs hermetically is enumerated
- **THEN** the suite carries both the stub-driven behavioural tests and one check driving the real command
- **AND** the real-tool check states which measured behaviours of that command the runtime depends on, one assertion each

#### Scenario: The stub's limit is written down, not implied

- **WHEN** a target's stub is read
- **THEN** it states the contract it stands in for, with citations a reader can repeat
- **AND** it states that it cannot establish that the vectors mean to the tool what safix thinks they mean

#### Scenario: An absent real-tool check is a recorded absence

- **WHEN** a target's tool cannot run in the sandbox
- **THEN** the absence of a real-tool check is recorded beside that target's stub with the reason
- **AND** no check states the claim and passes without making it

#### Scenario: The operator's own store cannot be reached by forgetting something

- **WHEN** a suite run reaches a store-backed target
- **THEN** the run is refused before a process is spawned unless both the program override names the stub and the store location is under the fixture's own scratch directory
- **AND** a real-tool check names only a store it created in a directory it made, and removes it however the test ends

#### Scenario: A platform that cannot run the tool says so

- **WHEN** the real-tool check's platform support is read
- **THEN** it is present on the platforms where the tool's own upstream tests run and absent where they do not
- **AND** the absence is stated rather than expressed as a check that quietly asserts less
