## RENAMED Requirements

- FROM: `### Requirement: An export refuses a source that holds no value`
- TO: `### Requirement: sync toward clan refuses a source that holds no value`
- FROM: `### Requirement: An export refuses when clan already considers the generator stale`
- TO: `### Requirement: sync toward clan refuses when clan already considers the generator stale`

## MODIFIED Requirements

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
- **THEN** `sync`, for the clan target, refuses before transferring anything
- **AND** the refusal states that clan is the authority on its own store
- **AND** the run does not proceed with a subset of its mappings

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

## REMOVED Requirements

### Requirement: Two verbs move declared mappings, one per direction

**Reason**: Two verbs invoked separately, one per direction, are superseded by a single `sync` verb whose `clan` target converges every declared mapping in its own declared direction in one run, narrowed by an optional `--direction` filter rather than by choosing which of two verbs to run.

**Migration**: Run `safix sync clan` in place of `safix import` followed by `safix export`.
Narrow to one direction with `safix sync clan --direction clan-to-safix` or `--direction safix-to-clan` in place of running only one of the two retired verbs.
Narrow to specific mappings by naming them after the target — `safix sync clan <mapping>...` — which now accepts more than one name in a single run, where each retired verb accepted at most one.
See the ADDED requirement "sync moves declared mappings, scoped by target and narrowed by direction" in this same delta.

## ADDED Requirements

### Requirement: sync moves declared mappings, scoped by target and narrowed by direction

The command SHALL provide a `sync` verb whose `clan` target acts on the mappings declared under `flake.safix.bridge.mappings`, each mapping converging in its own declared direction, scoped to one or more named mappings or to every mapping of that target when none is named, and narrowed further by an optional `--direction` option naming one of the declared direction values.

#### Scenario: Each mapping converges in its own declared direction

- **WHEN** `sync clan` runs
- **THEN** each mapping acted on moves in the direction declared for it
- **AND** no option or argument overrides a mapping's own declared direction

#### Scenario: A run may be scoped by naming mappings, or complete when none is named

- **WHEN** `sync clan` is given one or more mapping names
- **THEN** it acts on exactly those mappings
- **AND** when given none it acts on every mapping declared under the clan target

#### Scenario: --direction narrows the run to mappings declared with that value

- **WHEN** `sync clan --direction <value>` runs with no mapping named
- **THEN** only mappings declared with that direction are acted on
- **AND** a mapping declared with a different direction is left untouched, not refused

#### Scenario: A named mapping outside the --direction filter is told its actual direction

- **WHEN** `sync clan --direction <value>` is given the name of a mapping declared with a different direction
- **THEN** it refuses naming the direction the mapping is actually declared with
- **AND** the refusal is distinct from the one for a name nothing declares

#### Scenario: sync's help documents the clan target's forms

- **WHEN** the command is asked for help with `sync`
- **THEN** the help text states the `clan` target, its mapping-name scoping, and the `--direction` values it accepts
