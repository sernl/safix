## Purpose

The evidence that permits one runtime to replace another: which fleet both are driven over, what is compared on each of the four channels and how exactly, what makes the comparison of a channel that renders differently still falsifiable, how a harness is shown to be capable of failing, and what the passing of that harness does and does not authorize.

## ADDED Requirements

### Requirement: Both runtimes are driven over one fixture fleet minted at test time

The harness SHALL execute the shell runtime and the rust runtime against the same fixture repository, whose keys are generated during the test run and whose identities are fixtures.

#### Scenario: The fleet is synthetic

- **WHEN** the fixture repository is inspected
- **THEN** its users are fixture names and its recipients are synthetic public keys
- **AND** no recipient, host or user name of any real fleet appears in it

#### Scenario: The keys do not outlive the run

- **WHEN** the harness mints the identities it decrypts with
- **THEN** they are created inside the run's own scratch directory
- **AND** nothing in the repository carries a private key

#### Scenario: One fleet, two runtimes

- **WHEN** a subcommand is compared
- **THEN** each runtime is given its own pristine copy of the same fixture repository
- **AND** the argument vector each receives is identical

### Requirement: Standard output is compared without normalization

For every compared invocation, the two runtimes' standard output SHALL be identical byte for byte, with no filtering, sorting or rewriting applied to either side.

#### Scenario: The machine-readable channel

- **WHEN** a subcommand writes a value, a table or a report to standard output
- **THEN** the two byte sequences are equal
- **AND** no normalization step is applied before the comparison

### Requirement: Standard error is compared without normalization, under a reporter that exists in the code

The rust command SHALL select its diagnostic reporter from an environment variable, one setting of which emits refusals in the shell's shape — the program name, a colon, the message, and two-space-indented continuation lines, with no colour, no diagnostic code and no source span — and the harness SHALL compare standard error byte for byte with that reporter selected.

#### Scenario: The comparison is exact

- **WHEN** a refusal is compared between the runtimes
- **THEN** the plain reporter is selected on the rust side
- **AND** the two standard error byte sequences are equal, with no normalization applied to either

#### Scenario: Why the comparison is not a pattern

- **WHEN** the choice of a reporter over a normalizing pattern is recorded
- **THEN** it states that a pattern over a graphical rendering is a comparison whose strictness cannot be stated
- **AND** that the reporter puts the definition in code, where it is itself testable

#### Scenario: The unexercised rendering is still pinned

- **WHEN** the graphical reporter renders a refusal
- **THEN** that rendering is held by a snapshot for that variant
- **AND** the channel not compared against the shell is therefore pinned against itself

### Requirement: Reporter selection changes rendering and nothing else

Selecting a reporter SHALL alter only the text written to standard error, and SHALL NOT alter standard output, the exit code, or any effect on the repository.

#### Scenario: The same invocation with and without the selection

- **WHEN** one invocation is run twice, once with the plain reporter selected and once without
- **THEN** its standard output, exit code and repository effects are identical between the two runs
- **AND** only standard error differs

#### Scenario: The argument vectors stay identical

- **WHEN** the harness selects the reporter
- **THEN** it does so through the environment
- **AND** both runtimes receive the same argument vector, so no comparison is made across differing invocations

### Requirement: Exit codes are compared exactly

The two runtimes SHALL exit with the same status for every compared invocation, including the statuses produced on interruption and termination.

#### Scenario: Success and refusal

- **WHEN** an invocation succeeds or is refused
- **THEN** both runtimes exit with the same code
- **AND** the code is compared as a number rather than as success-or-failure

#### Scenario: Signals

- **WHEN** a run is interrupted or terminated
- **THEN** both runtimes exit with the status the shell runtime defines for that signal

### Requirement: Repository effects are compared over a canonical projection computed by one program

The harness SHALL compare the state each runtime leaves behind through a single projection applied to both sides, consisting of the ordered commit subjects with their per-path status, the full porcelain status, the working tree's paths and modes, the decrypted plaintext of every governed file, and the recipient set of every governed file.

#### Scenario: Why the bytes are not the comparison

- **WHEN** the choice of a projection over a byte comparison of the ciphertext is recorded
- **THEN** it states that a newly written value takes a fresh initialization vector and moves the message authentication code and the modification timestamp with it
- **AND** that comparing the files directly would compare the backend's random number generator rather than the runtimes

#### Scenario: One projection, both sides

- **WHEN** the projection is computed
- **THEN** the same program computes it for both runtimes
- **AND** the recipient set it reports is read by the same reader on both sides

#### Scenario: What a difference means

- **WHEN** any component of the projection differs
- **THEN** the harness fails naming the component and the subcommand
- **AND** the failure distinguishes a differing commit set from differing content from a differing recipient set

### Requirement: Neither run leaves plaintext behind

After each compared invocation, the harness SHALL assert that no plaintext residue remains in the temporary directory either runtime was given.

#### Scenario: The scratch directory after a refusal

- **WHEN** an invocation is refused after a value has been read
- **THEN** the temporary directory contains nothing either runtime created
- **AND** this is asserted for the refusal paths specifically, since they are the aborts a value leaks through

### Requirement: Each channel's comparison is held by a severity drill

For each of the four compared channels, the harness SHALL be shown to fail when the rust runtime is deliberately made wrong in the way that channel exists to detect.

#### Scenario: The four mutations

- **WHEN** the drills are run
- **THEN** one alters a refusal's wording, one alters an exit code, one alters the set of staged paths, and one alters a written value
- **AND** each mutation makes the harness red

#### Scenario: Caught by the right channel

- **WHEN** a mutation is caught
- **THEN** the failing channel is the one that mutation targets
- **AND** a mutation caught only incidentally by another channel is recorded as a gap in the targeted one

### Requirement: Retirement is per subcommand, and nothing ships on a partial pass

The shell runtime SHALL remain what the shipping package builds until every subcommand passes the harness, and the rust binary SHALL ship beside it under its own package name until then.

#### Scenario: While the migration is in progress

- **WHEN** the packages are enumerated at any point before the last subcommand passes
- **THEN** the shipping package builds the shell runtime
- **AND** the rust binary is available under a separate name

#### Scenario: What passing authorizes

- **WHEN** every subcommand passes the harness
- **THEN** the shipping package may be moved to the rust binary
- **AND** the authorization rests on the comparison having been shown capable of failing
