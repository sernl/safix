# bridge-transfer Specification

## Purpose

Moving a value across the boundary: the two verbs, the delegation that keeps clan the authority on its own store, the comparison that makes a run converge instead of churn, the commit discipline, and the divergence a later audit reports.

## Requirements

### Requirement: clan is the authority on its own store and is reached only through its command

Every read of a clan value, every write of one, and every enumeration of a machine's vars SHALL be performed by invoking clan's own command as a subprocess, and the runtime SHALL NOT read, write, decrypt, encrypt or parse clan's stored files.
When a mapping's placement is shared, the machine named on clan's command line to address it SHALL itself be obtained by invoking clan's command, never from a second field a consumer declares.

#### Scenario: Reading a clan value delegates

- **WHEN** a clan-side value is needed
- **THEN** it is obtained by invoking clan's command
- **AND** the value arrives on that process's standard output

#### Scenario: Writing a clan value delegates

- **WHEN** a clan-side value is written
- **THEN** it is written by invoking clan's command
- **AND** the value is supplied on that process's standard input

#### Scenario: Enumerating a machine's vars delegates

- **WHEN** the audit verb determines which vars a machine's namespace holds
- **THEN** it is obtained by invoking clan's command
- **AND** no secret var's value is read to make that determination

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
- **THEN** `sync`, for the clan target, refuses before transferring anything, whatever mix of directions the run would have acted on
- **AND** the refusal states that clan is the authority on its own store
- **AND** the run does not proceed with a subset of its mappings

#### Scenario: A shared mapping's address is discovered from clan, not declared twice

- **WHEN** a mapping's placement is shared
- **THEN** the runtime asks clan which machines it has, and tries them in turn against the mapping's generator until one resolves it
- **AND** no option or field on the mapping names that machine

#### Scenario: An unaddressable shared placement refuses naming the generator

- **WHEN** no machine clan has resolves a shared mapping's generator
- **THEN** the mapping is refused
- **AND** the refusal names the mapping, the placement, the generator and the file, and states that no machine in clan's own fleet exposed it

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

`sync`, for the clan target, SHALL read both sides of a mapping and compare them before writing either, and SHALL write nothing and commit nothing when the two agree.

#### Scenario: An agreeing mapping writes nothing

- **WHEN** both sides of a mapping hold the same bytes
- **THEN** nothing is written and nothing is committed

#### Scenario: The comparison precedes the export write for a stated reason

- **WHEN** the safix-to-clan write path is documented
- **THEN** it records that clan's write command writes unconditionally and that a backend re-encrypting an unchanged value produces fresh ciphertext
- **AND** it states that without the comparison every run would commit in the clan repository for every mapping

#### Scenario: A second run changes nothing

- **WHEN** `sync clan` runs twice with nothing else intervening
- **THEN** the second run writes nothing and commits nothing

#### Scenario: A value the operator cannot read is refused rather than transferred

- **WHEN** a mapping's safix side cannot be decrypted by the operator running the command
- **THEN** the mapping is refused
- **AND** the reason given is that a value that cannot be read cannot be verified

### Requirement: sync toward clan refuses a source that holds no value

`sync`, for the clan target's safix-to-clan direction, SHALL refuse a mapping whose source key is absent from the source file, and the refusal SHALL name the two ways a value gets there.

#### Scenario: An unwritten source refuses rather than exporting nothing

- **WHEN** `sync clan` reaches a safix-to-clan mapping whose source key is not in the source file
- **THEN** the mapping is refused
- **AND** the refusal names the entry and the file that would have held it
- **AND** it names setting the value by hand and generating it as the two remedies

#### Scenario: The refusal is the runtime sibling of an evaluation silence

- **WHEN** the same mapping is evaluated
- **THEN** evaluation produces no message about it
- **AND** the reason is that an entry declares where a value lives rather than that one is there

### Requirement: sync toward clan refuses when clan already considers the generator stale

`sync`, for the clan target's safix-to-clan direction, SHALL refuse a mapping whose clan-side generator has a recorded validation that no longer matches its definition, before writing anything, and SHALL provide no option to proceed.

#### Scenario: A stale generator refuses before the write

- **WHEN** `sync clan` reaches a safix-to-clan mapping whose clan-side generator clan reports as having an outdated validation
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

- **WHEN** `sync`'s arguments are enumerated
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

The audit verb SHALL report each mapping whose two sides no longer agree, and the safix-to-clan direction's report SHALL name the condition under which a value written toward clan is silently discarded.

#### Scenario: A diverged mapping is a finding

- **WHEN** a mapping's two sides hold different values
- **THEN** the audit reports it naming the mapping and which side is newer if that is knowable
- **AND** it names no value

#### Scenario: The silent-discard condition is named

- **WHEN** the safix-to-clan direction's documentation is read
- **THEN** it states that changing the clan-side generator's definition invalidates clan's recorded validation and that clan's next routine generation then replaces the value `sync` wrote toward clan
- **AND** it states that this is why the audit performs this comparison

#### Scenario: The runtime does not write clan's validation record

- **WHEN** the runtime is searched for a write of clan's validation record
- **THEN** none is found
- **AND** the reason is recorded: it would mean writing clan's store directly, and the value written would be a function of clan's own definition

#### Scenario: This comparison is the clan target of audit

- **WHEN** `audit` is invoked bare or with `clan` as its target
- **THEN** this requirement's comparison is what runs, over the mappings declared under `flake.safix.bridge.mappings`
- **AND** naming `keepassxc` as the target instead runs `keepassxc-sync`'s own comparison, not this one

### Requirement: sync moves declared mappings, scoped by target and narrowed by direction

Amended while proposing `rename-transfer-verbs`: this requirement is `rename-transfer-verbs`'s renamed and rewritten successor to the base spec's "Two verbs move declared mappings, one per direction", which `import`/`export` refused a two-way-declared mapping under and a third verb `bridge` would have acted on.
`rename-transfer-verbs` collapses `import`/`export` into `sync clan`'s own per-mapping direction convergence and a `--direction` filter; this change folds its own third verb into that same filter as a third accepted value, `two-way`, rather than adding a fourth CLI form.
Substance is unchanged: a `two-way` mapping still converges by `bridge-sync`'s own rule, still refuses under `--direction clan-to-safix`/`--direction safix-to-clan` by naming its actual declared direction, and that refusal is now the same generic one every direction mismatch already gets from `rename-transfer-verbs`'s filter, with no `two-way`-specific wrong-verb wording left to carry.

The command SHALL provide a `sync` verb whose `clan` target acts on the mappings declared under `flake.safix.bridge.mappings`, each mapping converging in its own declared direction — `clan-to-safix`, `safix-to-clan`, or `two-way` — scoped to one or more named mappings or to every mapping of that target when none is named, and narrowed further by an optional `--direction` option naming one of the three declared direction values.

#### Scenario: Each mapping converges in its own declared direction

- **WHEN** `sync clan` runs
- **THEN** each mapping acted on moves in the direction declared for it: a `clan-to-safix` or `safix-to-clan` mapping writes the side that lags, and a `two-way` mapping converges by the rule `bridge-sync` declares
- **AND** no option or argument overrides a mapping's own declared direction

#### Scenario: A run may be scoped by naming mappings, or complete when none is named

- **WHEN** `sync clan` is given one or more mapping names
- **THEN** it acts on exactly those mappings, of any declared direction
- **AND** when given none it acts on every mapping declared under the clan target

#### Scenario: --direction narrows the run to mappings declared with that value

- **WHEN** `sync clan --direction <value>` runs with no mapping named
- **THEN** only mappings declared with that direction are acted on, `--direction two-way` narrowing to two-way mappings the same way the two one-way values narrow to theirs
- **AND** a mapping declared with a different direction is left untouched, not refused

#### Scenario: A named mapping outside the --direction filter is told its actual direction

- **WHEN** `sync clan --direction <value>` is given the name of a mapping declared with a different direction
- **THEN** it refuses naming the direction the mapping is actually declared with — one of `clan-to-safix`, `safix-to-clan`, or `two-way`
- **AND** the refusal is distinct from the one for a name nothing declares

#### Scenario: sync's help documents the clan target's forms

- **WHEN** the command is asked for help with `sync`
- **THEN** the help text states the `clan` target, its mapping-name scoping, and the three `--direction` values it accepts, `two-way` among them

### Requirement: The audit reports clan vars no declared mapping accounts for

For every machine this capability enumerates for a currently declared mapping — the machine a per-machine-placement mapping declares, or the addressing machine discovered for a shared-placement mapping — the audit verb SHALL enumerate the vars clan's own command reports for that machine, and SHALL report as information each one whose id no currently declared mapping's clan side claims — including a var whose only mapping has since been removed from the declarations.
This enumeration SHALL be scoped to machines enumerated for a currently declared mapping and SHALL NOT extend to a clan machine that no declared mapping names or resolves.
No mode SHALL delete, export, or import a var by virtue of this report alone, and this report SHALL NOT change the audit's exit status, which continues to answer only whether every compared mapping agreed.

#### Scenario: A var no mapping names is reported

- **WHEN** a machine enumerated for a declared mapping holds a var that no currently declared mapping's clan side claims
- **THEN** the audit reports it naming the machine and the var
- **AND** nothing is written on either side of the boundary

#### Scenario: A removed mapping's var keeps appearing until a person acts

- **WHEN** a mapping is removed from the declarations after its var was created
- **THEN** the next audit reports that var among the ones no mapping names
- **AND** the var is not deleted, exported, or imported by the audit

#### Scenario: Enumeration is scoped to the machines the bridge currently names or resolves

- **WHEN** the audit enumerates clan vars
- **THEN** it considers only machines enumerated for a currently declared mapping
- **AND** it does not enumerate a clan machine that no declared mapping names or resolves, even when clan manages one

#### Scenario: Lingering never changes the exit status

- **WHEN** a run finds one or more vars no mapping accounts for and every compared mapping agrees
- **THEN** the audit still exits reporting agreement
- **AND** the vars no mapping accounts for are reported alongside it as information

#### Scenario: This is the clan target's own lingering report, alongside keepassxc's

- **WHEN** `audit` runs bare or with `clan` as its target
- **THEN** this requirement's lingering report is what appears for clan vars
- **AND** naming `keepassxc` as the target instead surfaces `keepassxc-sync`'s own lingering report, which `rename-transfer-verbs` adds as that capability's parallel gap-fill
