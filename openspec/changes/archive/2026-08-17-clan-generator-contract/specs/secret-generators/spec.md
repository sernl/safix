## Purpose

The generator executor's interface, changed from read-once descriptors to the directory contract clan-core's executor implements, so that a generator written for either system runs under the other.

## MODIFIED Requirements

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

- **WHEN** a generator script copies a value it was handed to a location of its own choosing
- **THEN** that location is outside what the runtime shreds
- **AND** the interface documentation states this rather than implying containment it does not provide

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

## ADDED Requirements

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
