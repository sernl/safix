## Purpose

The end-to-end suite that holds the shipped binary to what it promises an operator: where it lives, what it drives, the one thing it may stub, the rule that an assertion must be against a literal rather than against another runtime, and the parity obligation that decides when a claim may be deleted from the place it currently lives.

## ADDED Requirements

### Requirement: The shipped binary has an integration suite that depends on no second runtime

The repository SHALL carry an integration suite that drives the built `safix` binary end to end, and no assertion in it SHALL be expressed as agreement with another implementation.

#### Scenario: The suite drives the binary

- **WHEN** any test in the suite exercises a subcommand
- **THEN** it runs the built binary as a process
- **AND** it observes that process's standard output, standard error, exit status and effect on a repository

#### Scenario: No assertion is comparative

- **WHEN** each assertion in the suite is read
- **THEN** its expected value is a literal written in the test
- **AND** no expected value is obtained by running a second implementation

#### Scenario: The expectation is not re-derived through the code under test

- **WHEN** a test asserts what a run wrote
- **THEN** the expected bytes, paths and keys are stated in the test
- **AND** none of them is computed by calling the production path that produced them

### Requirement: The suite drives real backends and stubs only the evaluator

The suite SHALL run the real sops, the real age and the real git against a throwaway repository whose keys are minted inside the test, and SHALL stub nothing other than `nix`.

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

### Requirement: Each retired behavioural mode has a named successor before it is deleted

For every mode of the retired shell suite, this change SHALL record the integration test that carries its claim, and the deletion of that mode SHALL NOT occur before the successor asserts.

#### Scenario: Parity is itemized rather than asserted in aggregate

- **WHEN** the parity record is read
- **THEN** it names each retired mode individually
- **AND** for each it states the literal the successor asserts against

#### Scenario: Unit coverage does not discharge a behavioural claim

- **WHEN** a mode's successor is proposed
- **THEN** a unit test of a function that participates in the claim does not satisfy the obligation
- **AND** the successor asserts the end-to-end effect the mode asserted

#### Scenario: Deletion is ordered after the successor

- **WHEN** a task deletes a shell mode or the script carrying it
- **THEN** the task names the parity rows it depends on
- **AND** those rows are green in the same or an earlier commit

### Requirement: The single-runtime claims of the retired comparative harness are preserved

The four claims of the retired harness that were never comparisons — that an interrupted write leaves no residue, that a plaintext value travels only down a pipe, that this is observable at the syscall boundary, and that every channel fails under the mutation it exists to catch — SHALL survive as single-runtime checks.

#### Scenario: An interrupted write leaves nothing

- **WHEN** a write is interrupted in each window it has
- **THEN** no partial file, no scratch file and no created directory remains

#### Scenario: The value travels only down a pipe

- **WHEN** the process that receives a plaintext value is observed
- **THEN** the value reached it on a pipe
- **AND** it did not reach an argument vector, an environment variable or a file

#### Scenario: The pipe claim is observed rather than asserted

- **WHEN** the syscall proof runs on a platform that permits process tracing
- **THEN** the claim is established from observed syscalls
- **AND** on a platform that does not permit tracing the check states that nothing was observed rather than passing silently

#### Scenario: Every channel is shown to fail

- **WHEN** the drill check runs
- **THEN** it mutates the runtime once per channel
- **AND** it fails unless each mutation is caught by the channel that exists to catch it

#### Scenario: The drills outlive the harness that carried them

- **WHEN** the retirement order is read
- **THEN** the drill check is retained
- **AND** the reason is recorded: without it the suite is a set of assertions nobody has shown can fail
