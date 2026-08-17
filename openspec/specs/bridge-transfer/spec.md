# bridge-transfer Specification

## Purpose

Moving a value across the boundary: the two verbs, the delegation that keeps clan the authority on its own store, the comparison that makes a run converge instead of churn, the commit discipline, and the divergence a later audit reports.

## Requirements

### Requirement: Two verbs move declared mappings, one per direction

The command SHALL provide a verb that acts on clan-to-safix mappings and a verb that acts on safix-to-clan mappings, each acting on one named mapping or on all mappings of its direction.

#### Scenario: Each verb acts on its own direction only

- **WHEN** a verb runs
- **THEN** it acts on mappings of its direction
- **AND** it does not act on mappings of the other

#### Scenario: A run may be scoped or complete

- **WHEN** a verb is given a mapping's name
- **THEN** it acts on that mapping alone
- **AND** when given no mapping's name it acts on every mapping of its direction

#### Scenario: A mapping named to the verb of the other direction is told which verb acts on it

- **WHEN** a verb is given the name of a mapping declared in the other direction
- **THEN** it refuses naming the direction the mapping is declared with
- **AND** it names the verb that does act on it
- **AND** the refusal is distinct from the one for a name nothing declares

#### Scenario: The verbs appear in the command's help

- **WHEN** the command is asked for help with no verb
- **THEN** both verbs appear with their directions stated

### Requirement: clan is the authority on its own store and is reached only through its command

Every read of a clan value and every write of one SHALL be performed by invoking clan's own command as a subprocess, and the runtime SHALL NOT read, write, decrypt, encrypt or parse clan's stored files.

#### Scenario: Reading a clan value delegates

- **WHEN** a clan-side value is needed
- **THEN** it is obtained by invoking clan's command
- **AND** the value arrives on that process's standard output

#### Scenario: Writing a clan value delegates

- **WHEN** a clan-side value is written
- **THEN** it is written by invoking clan's command
- **AND** the value is supplied on that process's standard input

#### Scenario: No store implementation exists in the runtime

- **WHEN** the runtime is searched for clan's store layout, its recipient handling, or any of its backends
- **THEN** none is found
- **AND** the reason is recorded: the consumer's backend is a choice clan owns, and reimplementing one would silently support only that one

#### Scenario: The raw value is captured rather than a rendered one

- **WHEN** a clan value is read
- **THEN** the runtime establishes that it received the raw bytes rather than a rendering intended for a terminal
- **AND** the reason is recorded: clan's read command substitutes a printable form when its output is a terminal

#### Scenario: An absent clan command refuses the whole run

- **WHEN** clan's command is not available
- **THEN** both verbs refuse before transferring anything
- **AND** the refusal states that clan is the authority on its own store
- **AND** the run does not proceed with a subset of its mappings

### Requirement: A safix-side write goes through the existing write path

A value entering safix SHALL be written by the same path a hand-supplied value takes, and SHALL acquire that path's refusals without exception.

#### Scenario: The drift refusal applies

- **WHEN** the destination file's recipients have drifted from the audience declared for it
- **THEN** the write is refused before anything is renamed
- **AND** the refusal is the one that already exists for that condition

#### Scenario: The staged write and rename apply

- **WHEN** a value is written into safix
- **THEN** it is staged beside its target and renamed into place

#### Scenario: The value does not reach argv or the environment

- **WHEN** the write is performed
- **THEN** the value travels a pipe
- **AND** it appears in no argument vector and no environment

### Requirement: A transfer compares before it writes, and an agreeing mapping is left alone

Both verbs SHALL read both sides of a mapping and compare them before writing either, and SHALL write nothing and commit nothing when the two agree.

#### Scenario: An agreeing mapping writes nothing

- **WHEN** both sides of a mapping hold the same bytes
- **THEN** nothing is written and nothing is committed

#### Scenario: The comparison precedes the export write for a stated reason

- **WHEN** the export path is documented
- **THEN** it records that clan's write command writes unconditionally and that a backend re-encrypting an unchanged value produces fresh ciphertext
- **AND** it states that without the comparison every run would commit in the clan repository for every mapping

#### Scenario: A second run changes nothing

- **WHEN** a verb runs twice with nothing else intervening
- **THEN** the second run writes nothing and commits nothing

#### Scenario: A value the operator cannot read is refused rather than transferred

- **WHEN** a mapping's safix side cannot be decrypted by the operator running the command
- **THEN** the mapping is refused
- **AND** the reason given is that a value that cannot be read cannot be verified

### Requirement: An export refuses a source that holds no value

The export verb SHALL refuse a mapping whose source key is absent from the source file, and the refusal SHALL name the two ways a value gets there.

#### Scenario: An unwritten source refuses rather than exporting nothing

- **WHEN** an export reaches a mapping whose source key is not in the source file
- **THEN** the mapping is refused
- **AND** the refusal names the entry and the file that would have held it
- **AND** it names setting the value by hand and generating it as the two remedies

#### Scenario: The refusal is the runtime sibling of an evaluation silence

- **WHEN** the same mapping is evaluated
- **THEN** evaluation produces no message about it
- **AND** the reason is that an entry declares where a value lives rather than that one is there

### Requirement: An export refuses when clan already considers the generator stale

The export verb SHALL refuse a mapping whose clan-side generator has a recorded validation that no longer matches its definition, before writing anything, and SHALL provide no option to proceed.

#### Scenario: A stale generator refuses before the write

- **WHEN** an export reaches a mapping whose clan-side generator clan reports as having an outdated validation
- **THEN** the mapping is refused and nothing is written into clan
- **AND** the refusal names the machine and the generator

#### Scenario: The refusal names both remedies

- **WHEN** the refusal is read
- **THEN** it names updating the clan-side definition
- **AND** it names declaring the mapping in the other direction instead

#### Scenario: The staleness is clan's answer rather than safix's computation

- **WHEN** the runtime establishes whether a generator is stale
- **THEN** it obtains the answer by invoking clan's own command
- **AND** it reads no recorded validation and computes no hash

#### Scenario: No option defeats the refusal

- **WHEN** the export verb's arguments are enumerated
- **THEN** none of them proceeds past this refusal

### Requirement: A run reports each mapping's outcome as one of four states

Each mapping acted on SHALL be reported as unchanged, updated, absent at source, or refused with its reason, and no value SHALL appear in any report.

#### Scenario: The four states are distinguished

- **WHEN** a run completes
- **THEN** each mapping it acted on carries exactly one of the four outcomes

#### Scenario: An ungenerated source is a state rather than a failure

- **WHEN** a mapping's source holds no value yet
- **THEN** the outcome is absent at source
- **AND** the run continues with the remaining mappings

#### Scenario: No report names a value

- **WHEN** any report, refusal or commit message is produced
- **THEN** it names mappings, endpoints and outcomes
- **AND** it names no value

### Requirement: Each transferred mapping is its own commit

A transfer SHALL commit each mapping separately, naming the mapping, and SHALL NOT combine several mappings into one commit.

#### Scenario: One mapping, one commit

- **WHEN** a run transfers several mappings into safix
- **THEN** each produces its own commit

#### Scenario: The export direction commits where the value landed

- **WHEN** a run transfers a mapping into clan
- **THEN** it makes no commit in the consumer's own repository, where nothing changed
- **AND** each mapping is one invocation of clan's write, which commits what it wrote

#### Scenario: The commit names the mapping and not the value

- **WHEN** a transfer's commit message is read
- **THEN** it names the mapping and the direction
- **AND** it names no value

#### Scenario: The reason for single-intent commits is recorded

- **WHEN** the commit discipline is documented
- **THEN** it states that a combined commit's message cannot say what it did without naming values
- **AND** it states that reverting one mapping should not revert the others

### Requirement: Divergence after a transfer is reported by the audit verb

The audit verb SHALL report each mapping whose two sides no longer agree, and the export direction's report SHALL name the condition under which an exported value is silently discarded.

#### Scenario: A diverged mapping is a finding

- **WHEN** a mapping's two sides hold different values
- **THEN** the audit reports it naming the mapping and which side is newer if that is knowable
- **AND** it names no value

#### Scenario: The silent-discard condition is named

- **WHEN** the export direction's documentation is read
- **THEN** it states that changing the clan-side generator's definition invalidates clan's recorded validation and that clan's next routine generation then replaces the exported value
- **AND** it states that this is why the audit performs this comparison

#### Scenario: The runtime does not write clan's validation record

- **WHEN** the runtime is searched for a write of clan's validation record
- **THEN** none is found
- **AND** the reason is recorded: it would mean writing clan's store directly, and the value written would be a function of clan's own definition
