## MODIFIED Requirements

### Requirement: The suite drives real backends and stubs only the evaluator

The suite SHALL run the real sops, the real age and the real git against a throwaway repository whose keys are minted inside the test, and SHALL stub nothing other than `nix` and the delegation boundaries it cannot cross hermetically.
A stubbed delegation boundary SHALL be one safix delegates *across* rather than one its own claims are about, SHALL record what it was handed so a claim is asserted rather than assumed, and SHALL be accompanied by a written statement of what the stub cannot establish.
Where a target's far side cannot be authenticated inside a hermetic build at all, the absence of a real-binary check for it SHALL be recorded where that target's checks live, and the stub SHALL be guarded so that a run cannot reach the operator's own far side by accident.
A stub standing in for a transport whose only channel for a session credential is an environment variable SHALL assert the narrowed invariant rather than quietly weakening the unconditional one: no password and no mapped value in any argument vector or any environment, and a session credential in the environment and nowhere else.
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

#### Scenario: A far side that cannot be authenticated hermetically is stubbed, and the absence is written down

- **WHEN** a sync target's far side needs a reachable server and an account to answer at all
- **THEN** the suite drives a stub that records the invocations, and the target's own check file states that no check in this repository runs the real client
- **AND** for the 1Password target the absence is stated in the declaring module, in the transport, and in that target's own capability, and the grounds are named — an unfree package that would break evaluation for consumers, no self-hostable server, and a network no hermetic build has
- **AND** the reason is recorded: a check that states an absence and passes is how a claim stops being made without anybody deciding to stop making it

#### Scenario: The stub guard refuses a run that could reach a real far side

- **WHEN** a test drives a target whose real client may be configured and logged in on the machine running the suite
- **THEN** the harness refuses the run unless the client's own state directory is inside the fixture's scratch and the configured server is a loopback address
- **AND** the guard is structural rather than advisory, so a test that forgot it fails instead of reaching somebody's vault

#### Scenario: The narrowed credential-channel invariant is asserted where it applies, not assumed everywhere

- **WHEN** a stub for a transport carrying a session credential enumerates each invocation's arguments, environment and input
- **THEN** it asserts that no password and no mapped value is in the arguments or the environment, and that the session credential is in the environment of the invocations needing it and in no argument, no file and no output
- **AND** the narrowing is stated as a decision: this transport offers no standard-input channel for its session credential, an argument vector is readable by every process on the machine, and an environment is readable by the same user, so the credential is placed in the lesser exposure rather than in none

#### Scenario: A boundary stub refuses what the runtime promises never to send

- **WHEN** the 1Password stub receives an invocation carrying an argument that assigns a value, or an item command naming no vault
- **THEN** it exits non-zero, so the promise that no value travels an argument vector is enforced by the program the runtime actually ran
- **AND** a run that was not pointed at the stub is refused by the harness before a process is spawned, both on the program override and on the fixture's own session material
