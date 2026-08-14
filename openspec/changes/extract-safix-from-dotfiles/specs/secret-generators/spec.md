## Purpose

How a value can write itself: the generator vocabulary, the pipe-only path a generated value travels, the read-once descriptors that carry prompts and dependencies, the rotation cascade that a dependency enrols its generator in, the refusals that keep the generator graph runnable, and the honest limit of what the tool can promise about a script it does not sandbox.

## ADDED Requirements

### Requirement: A generated value travels a pipe and never argv, the environment, or a file

A generator's output SHALL reach the write path as a stream.
It SHALL NOT be placed in a command argument, an environment variable, or a file at any point the package controls.

#### Scenario: The write path

- **WHEN** a generator produces a value
- **THEN** the value is written through the same stream-fed path a hand-typed value takes
- **AND** it appears in no argument vector, no environment variable, and no file the package creates

#### Scenario: Empty output is refused

- **WHEN** a generator produces empty output
- **THEN** nothing is written
- **AND** the run fails, because an empty value is what a truncated write leaves behind

#### Scenario: Diagnostics are separated from the value

- **WHEN** a generator writes to its error stream
- **THEN** that output reaches the operator
- **AND** none of it enters the value

#### Scenario: Trailing-newline handling is stated and stable

- **WHEN** a single-line value is produced
- **THEN** one trailing newline is removed, so the value stores what it appears to store
- **AND** a multi-line value keeps its final newline, and a value read from the structured multi-output form is stored exactly as stated there

### Requirement: Prompts and dependencies arrive as read-once descriptors

A prompt answer or a dependency's plaintext SHALL reach a generator script as a read-only file descriptor addressed by a documented variable, read once.
Neither SHALL reach argv, the environment, or a file.

#### Scenario: Addressing an input

- **WHEN** a script reads a prompt answer or a dependency value
- **THEN** it does so through a variable naming a read-only descriptor
- **AND** the same shape serves prompts and dependencies alike

#### Scenario: Hidden input is the default for a prompt

- **WHEN** a prompt is declared without stating how it is read
- **THEN** the answer is read without echoing
- **AND** the option's documentation states that every prompt reachable from a generator feeds a secret's value

#### Scenario: Colliding input names are refused

- **WHEN** two of a generator's inputs map to the same variable name under the name-to-variable transformation
- **THEN** evaluation fails naming the generator and both inputs
- **AND** the message states that the mapping is not injective, so one input would otherwise silently shadow the other

#### Scenario: The sandbox limit is disclosed rather than implied

- **WHEN** the guarantee about inputs is documented
- **THEN** it states that this is a promise about how a value arrives and not a sandbox the script runs inside
- **AND** it states that a script redirecting an input to a file or echoing it to the error stream has put plaintext where the tool cannot reach it

### Requirement: A dependency chains generation and enrols its generator in rotation

Declaring a dependency SHALL make the depending generator re-run whenever the depended-upon value is regenerated, transitively, in dependency order.
The set to be re-run SHALL be shown and confirmed before anything runs.

#### Scenario: Regenerating cascades downstream

- **WHEN** a value with dependents is regenerated
- **THEN** every generator downstream of it re-runs, transitively, in dependency order
- **AND** the cascade is not optional, because a value derived from a retired input is indistinguishable afterwards from one derived from the current input

#### Scenario: The cascade is confirmed once

- **WHEN** a cascade is about to run
- **THEN** the full set is listed and one confirmation is requested
- **AND** a documented flag answers that confirmation in advance

#### Scenario: A dependency on another person's secret is refused

- **WHEN** a generator declares a dependency on a secret held by another person
- **THEN** evaluation fails naming the generator and the dependency
- **AND** the message states that the refusal is structural: the machine running the generator holds no identity that opens the other person's file, so there is no plaintext to read

#### Scenario: A dependency naming a secret the user does not hold is refused

- **WHEN** a generator names a dependency that is not in the same user's resolved set
- **THEN** evaluation fails naming the generator and the missing name

### Requirement: The generator graph is runnable or evaluation fails

A cycle, a self-dependency, or two generators producing one output SHALL fail evaluation with the offending declarations named.

#### Scenario: A cycle

- **WHEN** a set of generators forms a dependency cycle
- **THEN** evaluation fails with the cycle printed
- **AND** the message states that the alternative is a run that fails part-way through with values already committed

#### Scenario: A self-dependency

- **WHEN** a generator depends on an output of its own run
- **THEN** evaluation fails naming it directly rather than reporting a cycle of length one

#### Scenario: Two producers for one value

- **WHEN** a name is produced by one generator and also carries a generator of its own, or is named by a second generator
- **THEN** evaluation fails naming both producers
- **AND** the message states that whichever ran last would win, and which ran last is not a declaration

### Requirement: One generator may mint several related values, written together

A generator SHALL be able to declare further outputs, each a registry entry in its own right carrying its own mode, path, and key, and all of a run's outputs SHALL be committed together.

#### Scenario: A keypair

- **WHEN** a generator declares a further output and emits a structured object keyed by output name
- **THEN** each named value is written to its own entry
- **AND** all of them land in one commit, because a keypair split across two commits is an incoherent state

#### Scenario: Each output keeps its own entry fields

- **WHEN** a further output is declared as an entry
- **THEN** it carries its own mode, path, and key
- **AND** the generator's list records only which generator produces it

### Requirement: A validation script refuses the write before anything is written

A generator SHALL be able to declare a validation fragment that receives each candidate value and refuses the whole run on a non-zero exit.

#### Scenario: A candidate is judged before it is stored

- **WHEN** a validation fragment exits non-zero for any candidate
- **THEN** the whole run is refused and nothing is written
- **AND** nothing has to be undone, because the values were still only in memory

#### Scenario: One fragment covers several outputs

- **WHEN** a generator writes several outputs
- **THEN** the fragment is told which output is being judged
- **AND** it runs in the same environment, with the same runtime tools, as the script that produced the value

### Requirement: Runtime tool names are resolved by a check, not at rotation time

A generator's runtime tools SHALL be declared as names resolved against the package set by a check, so that a misspelling fails a build rather than an operator's rotation.

#### Scenario: A misspelled tool name

- **WHEN** a generator names a runtime tool that does not resolve
- **THEN** a check fails naming the generator and the unresolved name
- **AND** the failure occurs at build time rather than when the generator is next run

#### Scenario: Why the names are strings

- **WHEN** the choice of strings over package references is documented
- **THEN** it states that the whole generator travels to the command as structured data read from a single evaluation
- **AND** that a package reference cannot cross that boundary
