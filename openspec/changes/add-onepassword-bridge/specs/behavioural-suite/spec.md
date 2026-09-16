## MODIFIED Requirements

### Requirement: The suite drives real backends and stubs only the evaluator

The suite SHALL run the real sops, the real age and the real git against a throwaway repository whose keys are minted inside the test, and SHALL stub nothing other than the evaluator and the programs that stand at a delegation boundary a hermetic build may not cross.
Those boundary programs SHALL be enumerated rather than left open — the evaluator, the fleet framework's own command, the password database's own command, and the 1Password command — and for each one the suite SHALL record what a stub can establish and what it cannot: a stub establishes what safix sent and what it did with the answer, and never that the argument vector means to the real tool what safix believes it means.
Where a boundary program's real counterpart can be driven by a check, that check SHALL exist and the stub SHALL NOT be offered as its substitute; where it cannot, the absence SHALL be stated in the module that would have carried the check.

#### Scenario: The cryptographic backends are real

- **WHEN** the suite's harness is enumerated
- **THEN** sops, age and git are the real programs
- **AND** the reason is recorded: a stub is what lets a check stay green over a command calling something the tree no longer contains

#### Scenario: The evaluator stub also pins the attribute name

- **WHEN** the `nix` stub answers a placement query
- **THEN** it asserts the attribute path the command asked for
- **AND** renaming that attribute fails the suite rather than an operator's terminal

#### Scenario: No real identity is used

- **WHEN** the suite's fixtures are read
- **THEN** every user name is a fixture name, every recipient is a synthetic age string, and every key a test decrypts with was minted in that test's own scratch directory

#### Scenario: A boundary stub refuses what the runtime promises never to send

- **WHEN** the 1Password stub receives an invocation carrying an argument that assigns a value, or an item command naming no vault
- **THEN** it exits non-zero, so the promise that no value travels an argument vector is enforced by the program the runtime actually ran
- **AND** a run that was not pointed at the stub is refused by the harness before a process is spawned, both on the program override and on the fixture's own session material

#### Scenario: A boundary program no check may ever run is recorded as such

- **WHEN** the 1Password target's checks are enumerated
- **THEN** no check drives the real command, and the absence is stated in the declaring module, in the transport, and in that target's own capability
- **AND** the grounds are named — an unfree package that would break evaluation for consumers, no self-hostable server, and a network no hermetic build has — because a check stating an absence and passing is how a claim stops being made without anybody deciding to stop making it
