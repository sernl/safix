# secret-generators Specification

## Purpose

How a value can write itself: the generator vocabulary, the pipe-only path a generated value travels, the read-once descriptors that carry prompts and dependencies, the rotation cascade that a dependency enrols its generator in, the refusals that keep the generator graph runnable, and the honest limit of what the tool can promise about a script it does not sandbox.

## Requirements

### Requirement: A generated value travels a pipe where a pipe is possible, and a private staging file where it is not

A generated value SHALL NOT reach an argument vector or an environment variable at any point. It SHALL reach a file only inside the private staging root that `plaintext-staging` governs, and SHALL travel a pipe on every other leg of its journey.

#### Scenario: The write leg is still a pipe

- **WHEN** a produced value is handed to the encrypting backend
- **THEN** it travels a pipe
- **AND** it does not appear in that process's argument vector or environment

#### Scenario: The staging root is the only filesystem the value reaches

- **WHEN** the filesystem locations a value occupies during a run are enumerated
- **THEN** each is inside the private staging root
- **AND** none is outside it

#### Scenario: The exception is named rather than implied

- **WHEN** this requirement is read
- **THEN** it states that the generator contract is a filesystem contract and that the earlier absolute no longer holds
- **AND** it names what stands in its place

#### Scenario: What the runtime does not control

- **WHEN** a generator whose declaration grants the network moves a value over the connection the declaration opens
- **THEN** that movement is outside what the runtime shreds or observes
- **AND** the interface documentation states this beside the declaration, rather than disclaiming containment on the default path the envelope now enforces

### Requirement: Prompts and dependencies arrive as files in named directories

A generator's prompted answers SHALL arrive one per file under a `prompts` directory keyed by prompt name, and a dependency's outputs SHALL arrive under an `in` directory keyed first by dependency name and then by output name.

#### Scenario: A prompt is addressed by key

- **WHEN** a generator declares a prompt
- **THEN** the answer is readable at the path formed from the prompts directory and the prompt's key
- **AND** the file holds exactly what the operator supplied, with nothing added and nothing removed

#### Scenario: A dependency is addressed by dependency and output

- **WHEN** a generator declares a dependency
- **THEN** each of that dependency's outputs is readable at the path formed from the input directory, the dependency's name, and the output's name

#### Scenario: The prompts directory exists only when prompts are declared

- **WHEN** a generator declares no prompts
- **THEN** no prompts directory is created and the variable naming it is unset

#### Scenario: The script's working directory is the staging root

- **WHEN** a generator script runs
- **THEN** its working directory is the root containing the input, output and prompts directories

#### Scenario: A generator still cannot consume another's answers

- **WHEN** a generator script reads its standard input to end of input
- **THEN** no later generator's prompt is left unanswered
- **AND** the reason is that answers are files rather than a shared stream

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

### Requirement: One generator may mint several related values, each written to a declared output path

A generator SHALL declare its outputs by name, SHALL write each to the path formed from the output directory and that name, and the runtime SHALL refuse the whole run if any declared output is absent when the script exits.

#### Scenario: Each declared output has its own file

- **WHEN** a generator declaring several outputs runs
- **THEN** each output is read from its own file under the output directory

#### Scenario: A missing output refuses the run and says what was produced

- **WHEN** a declared output is absent after the script exits
- **THEN** the run is refused
- **AND** the refusal names the missing output
- **AND** it lists what the output directory did contain

#### Scenario: Nothing is written until every output is present

- **WHEN** a generator's outputs are checked for presence
- **THEN** the check completes for all of them before any value is encrypted or written

#### Scenario: A generator's outputs share one audience and so one file

- **WHEN** a generator's declared outputs are resolved
- **THEN** they resolve to one audience
- **AND** a multi-output write is therefore one staged document and one rename

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

### Requirement: A generator's share is derived from its outputs and disagreement is refused

A generator SHALL carry a read-only `share` that is true exactly when every entry it writes is shared, and evaluation SHALL refuse a generator whose outputs disagree.

#### Scenario: Share is not authored on the generator

- **WHEN** a consumer attempts to set share on a generator directly
- **THEN** evaluation refuses
- **AND** the refusal names the entry-level field that decides it

#### Scenario: Disagreeing outputs are refused by name

- **WHEN** a generator writes one shared entry and one that is not
- **THEN** evaluation refuses
- **AND** the refusal names both outputs and which side each is on
- **AND** it states the remedy: two generators, the second depending on the first

#### Scenario: The derived value is what the bridge compares

- **WHEN** a generator's share is read for comparison against another system's generator
- **THEN** the value read is the derived one
- **AND** no second authoring surface exists for the same fact

### Requirement: The retired descriptor interface is refused at evaluation with the rewrite

Evaluation SHALL refuse a generator written against the retired descriptor interface, naming this change and stating the mechanical rewrite, rather than allowing it to fail at runtime.

#### Scenario: A descriptor-shaped input reference is refused

- **WHEN** a generator's script references an input by the retired descriptor spelling
- **THEN** evaluation refuses
- **AND** the refusal states the replacement for a prompt and the replacement for a dependency

#### Scenario: A script that writes no output is refused before it runs

- **WHEN** a generator's script never references the output directory
- **THEN** evaluation refuses
- **AND** the reason is recorded: the runtime symptom would otherwise be a missing-output refusal that names the symptom rather than the interface change

#### Scenario: The refusal is retained after the fleet has migrated

- **WHEN** the retention of these refusals is considered
- **THEN** they are kept
- **AND** the reason is recorded: they cost a string comparison during evaluation, and what they prevent is a generator that silently produces no value or reads an empty input

#### Scenario: No compatibility mode exists

- **WHEN** the runtime is searched for a path that executes the retired interface
- **THEN** none is found
- **AND** the reason is recorded: the two interfaces differ in custody rather than in spelling, so a run containing both would hold the weaker property while the per-generator documentation claimed the stronger one

### Requirement: A fragment runs inside the envelope by default

A generator's script and its validation fragments SHALL run inside a sandbox in which the staging root is the only writable path, the nix store is readable, and the network is absent.
The envelope SHALL match the one clan's default executor applies, so that a fragment written against the shared interface runs under either system's default executor without modification.

#### Scenario: A write outside the staging root fails

- **WHEN** a fragment writes to a path outside its staging root
- **THEN** the write fails inside the envelope
- **AND** the run refuses with that fragment's own failure rather than storing a partial result

#### Scenario: The network is absent

- **WHEN** a fragment with no declared escape attempts a network connection
- **THEN** the attempt fails
- **AND** no traffic leaves the envelope

#### Scenario: The declared tools still run

- **WHEN** a fragment invokes a tool its generator's `runtimeInputs` names
- **THEN** the tool runs from the read-only store

#### Scenario: The pipe legs are unchanged

- **WHEN** a validation fragment judges a candidate value
- **THEN** the candidate arrives on standard input exactly as it does outside the envelope
- **AND** a prompted answer still arrives as a file under the prompts directory

### Requirement: The network is granted by declaration and by nothing else

A generator SHALL reach the network only when its own declaration says so.
The declaration SHALL re-share the network and nothing else: the filesystem confinement of the envelope stays in force.
An invocation-level switch for suspending the envelope SHALL NOT exist.

#### Scenario: The declared escape opens the network only

- **WHEN** a generator whose declaration grants the network runs
- **THEN** the fragment reaches the network
- **AND** a write outside the staging root still fails

#### Scenario: No flag suspends the envelope

- **WHEN** an invocation passes `--no-sandbox` or any equivalent
- **THEN** the invocation is refused as carrying an unknown flag

#### Scenario: The audit reads the tree

- **WHEN** an operator asks which generators may reach the network
- **THEN** the declarations answer the question at evaluation, with no runtime consulted

### Requirement: Generation refuses without a sandbox backend

When no sandbox backend is available, generation SHALL refuse before any fragment runs, naming the backend it looked for and what supplies it.
The refusal SHALL NOT be convertible into an unsandboxed run.

#### Scenario: The refusal precedes every fragment

- **WHEN** generation starts on a machine where the platform's backend is unavailable
- **THEN** it refuses before the first fragment runs
- **AND** the refusal names the missing backend and its remedy

#### Scenario: An unsupported platform is told so

- **WHEN** generation starts on a platform with no backend at all
- **THEN** the refusal says the platform has no envelope
- **AND** nothing runs unsandboxed in its place

### Requirement: Minting records the definition it minted under

A generated value SHALL be accompanied by a committed, plaintext record carrying a digest of the generator definition that produced it, written in the same commit as the value and refreshed whenever the value is regenerated.
The digest SHALL be computed over the definition alone; no value and no derivative of a value appears in the record.

#### Scenario: The record rides the mint's own commit

- **WHEN** a generator mints or regenerates a value
- **THEN** the definition record is written in the same commit as the value
- **AND** a mint interrupted before the commit leaves neither

#### Scenario: The record is about the definition, never the value

- **WHEN** the record's content is inspected
- **THEN** it derives from the generator's declaration alone
- **AND** two mints of different values under one definition produce the same record

#### Scenario: The record does not live where its meaning would lie

- **WHEN** the record's path is inspected
- **THEN** it is not under a path that means everything below it is encrypted
- **AND** not under a path that means declared public outputs
