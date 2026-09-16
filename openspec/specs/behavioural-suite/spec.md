# behavioural-suite Specification

## Purpose

The end-to-end suite that holds the shipped binary to what it promises an operator: where it lives, what it drives, the one thing it may stub, the rule that an assertion must be against a literal rather than against another runtime, and the parity obligation that decides when a claim may be deleted from the place it currently lives.

## Requirements

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

### Requirement: An interactive surface is driven through a real pseudo-terminal, never through a seam in the binary

Where a subcommand reads keystrokes and draws, the suite SHALL drive it through a pseudo-terminal attached to both the input it reads and the output it draws on, and the shipped binary SHALL carry no option, environment variable or branch whose only caller is a test wanting to bypass that surface.

#### Scenario: The pseudo-terminal carries both directions

- **WHEN** a test exercises a subcommand that reads keystrokes and draws
- **THEN** the pseudo-terminal's slave is attached to the stream the run reads and to the stream it draws on
- **AND** standard error stays a pipe, so a refusal is observed separately from anything drawn

#### Scenario: A run with no terminal is asserted rather than avoided

- **WHEN** the same subcommand is run with pipes on every stream, as the hermetic check sandbox provides
- **THEN** the test asserts the refusal that state produces, naming its code
- **AND** it does not substitute a terminal to make the run proceed

#### Scenario: No selection override ships

- **WHEN** the binary's options and the environment variables it reads are enumerated
- **THEN** none of them pre-selects, pre-fills or skips an interactive choice
- **AND** the reason is recorded: the non-interactive path already exists as the form that names the entry, so an override would be a second code path shipping to operators whose only caller is a test, and the test it enables would assert that path rather than the surface

#### Scenario: The environment variables the harness does set name programs, not branches

- **WHEN** the harness's environment variables are read
- **THEN** each names a program to locate or substitute
- **AND** none of them selects a branch inside the binary's own logic

### Requirement: The two worked consumers are compared to each other on every field the runtime reads

The check comparing the two declaration-side consumers SHALL compare every projection field the runtime reads, SHALL compare each side's own declaration of a field rather than one side's against itself, and SHALL elide only the part of a value that is a function of where the consumer's own root happens to be.

#### Scenario: Every field the runtime reads is compared

- **WHEN** the compared fields are enumerated against the fields the runtime reads from a projection
- **THEN** every field the runtime reads and that serializes is compared
- **AND** a field excluded from the comparison is excluded because it is a function, with the exclusion recorded beside the list

#### Scenario: A value is never compared with itself

- **WHEN** the expected and the actual side of each compared value are traced to their sources
- **THEN** the expected value comes from one consumer's own declarations and the actual value from the other's
- **AND** no compared value is taken from the same file for both sides, because such a row is green under every possible divergence

#### Scenario: A hook is a compared value like any other

- **WHEN** the hooks the command invokes are compared
- **THEN** each consumer declares them itself and the two declarations are compared
- **AND** one literal assertion holds that the hook is present and carries its fixture's text, so both consumers losing the hook fails rather than comparing empty to empty

#### Scenario: Root-dependent absolute strings are elided, not dropped

- **WHEN** a compared field carries a value derived from a path declared relative to each consumer's own root
- **THEN** the leading root is replaced by one stable marker on both sides and the remainder of the value is compared
- **AND** the field stays in the comparison, because a field dropped from it is a declaration compared nowhere

#### Scenario: The elision is as narrow as the divergence

- **WHEN** the elision is applied
- **THEN** it replaces only the consumer's own root prefix
- **AND** it does not elide store paths generally, because a store path appearing where one is not expected is itself a finding

#### Scenario: The severity drill for the comparison

- **WHEN** one field of one consumer's fleet is changed and the other's is not
- **THEN** the check fails on exactly that field
- **AND** this is the evidence that the two are compared rather than merely both evaluated

### Requirement: A hand-maintained list of the example's own files is refused

Where a check discovers a worked example's files by reading the directory, the example SHALL NOT also enumerate them by hand, and the check SHALL assert that no such enumeration exists.

#### Scenario: The example's own entry point discovers its files the same way

- **WHEN** the worked example that is a flake is read
- **THEN** it obtains its declaration files by reading its own module directory
- **AND** it names no individual file under that directory

#### Scenario: The absence of a hand list is asserted

- **WHEN** the check runs
- **THEN** it asserts that the example's entry point contains no per-file path under its module directory
- **AND** the reason is recorded: a hand list beside a discovered one drifts on the next added file, and the drift is silent because nothing evaluates the entry point

#### Scenario: Comparing two lists is refused as the remedy

- **WHEN** asserting equality between a hand list and the discovered set is considered
- **THEN** it is refused
- **AND** the reason is recorded: extracting an import list from nix source is either a parser with one caller or a text match that breaks when the formatter moves a path to another line

#### Scenario: The severity drill for the list

- **WHEN** a per-file import path is put back into the example's entry point
- **THEN** the check fails naming it
- **AND** when a new declaration file is added to the module directory with no other edit, the check stays green and the new file is in both the example's own evaluation and the check's

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
