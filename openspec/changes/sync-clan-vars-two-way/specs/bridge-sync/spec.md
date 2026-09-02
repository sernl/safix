## Purpose

A declared clan var and a declared safix entry stay converged in either direction, with a remembered last agreement that tells a genuine conflict apart from an ordinary one-sided change, so a two-way relationship across the clan boundary is safe rather than merely permitted.

## ADDED Requirements

### Requirement: A two-way mapping's agreement lives in a companion entry it mints, not one a consumer declares

Every mapping whose direction is two-way SHALL have a companion safix entry that records the last agreed value, sharing the mapped entry's file and audience, minted automatically rather than declared, and evaluation SHALL refuse a hand-declared entry whose name collides with a companion's.

#### Scenario: The companion shares the mapped entry's file and audience

- **WHEN** a two-way mapping is resolved
- **THEN** its companion entry resolves to the same file and the same audience as the mapped entry
- **AND** it is distinguished from the mapped entry by a reserved key suffix alone

#### Scenario: The reserved name cannot be declared by hand

- **WHEN** a consumer declares an entry whose name carries the suffix a two-way mapping's companion reserves
- **THEN** evaluation refuses, naming the entry, the mapping that reserves the suffix, and the suffix itself

#### Scenario: A mapping with no two-way declaration mints no companion

- **WHEN** a mapping's direction is clan-to-safix or safix-to-clan
- **THEN** no companion entry is minted for it

### Requirement: A two-way mapping converges toward whichever side changed since the last agreement

`sync`, converging a mapping declared `two-way` under the clan target, SHALL read both sides of every declared two-way mapping — or the one named — and: write nothing where the two sides already agree; write the side that has not moved to match the side that has, recording the new agreement, where exactly one side differs from the last-recorded agreement; write nothing and report a conflict where both sides differ from the last-recorded agreement or from each other with no agreement yet recorded; and, where exactly one side has never held a value, write that side from the other and record the agreement, treating that as ordinary convergence rather than a failure.

#### Scenario: Agreement writes nothing

- **WHEN** a two-way mapping's two sides already hold the same value
- **THEN** neither side is written
- **AND** the report says unchanged

#### Scenario: One side moved, and the other converges to it

- **WHEN** exactly one side's current value no longer matches the last-recorded agreement
- **THEN** the other side is written to match it
- **AND** the new agreement is recorded

#### Scenario: Both moved is a conflict, not a guess

- **WHEN** both sides' current values differ from the last-recorded agreement, or the two sides differ from each other and no agreement has ever been recorded
- **THEN** nothing is written
- **AND** the finding names the mapping and the two one-way remedies: narrowing a `sync clan` run to it with `--direction clan-to-safix` or `--direction safix-to-clan` and running once, before the mapping's declared direction reverts to two-way

#### Scenario: One side has never held a value, and that is bootstrap rather than a failure

- **WHEN** exactly one side of a two-way mapping has never held a value
- **THEN** the empty side is written from the other
- **AND** the agreement is recorded, the same as any other convergence

#### Scenario: Neither side has ever held a value

- **WHEN** neither side of a two-way mapping holds a value
- **THEN** nothing is written
- **AND** the report says unchanged

#### Scenario: A shared or export-scoped mapping's address is discovered from clan, not declared twice

- **WHEN** a two-way mapping's placement is shared or per-export
- **THEN** the machine used to reach it on clan's command line is discovered the same way `bridge-transfer` requires for every direction, one-way or two-way alike

### Requirement: The agreement is written after the value it describes, and nowhere a plaintext digest would be an oracle

A two-way convergence that writes a side SHALL write the agreement only after that write has landed, SHALL record it as a digest inside the sops-encrypted companion entry, and SHALL NOT record any value-derived state in clan's own store or in a plaintext, committed tree.

#### Scenario: The companion write follows the value write

- **WHEN** a two-way convergence writes a side and its agreement
- **THEN** the value lands first, as its own commit or its own invocation of clan's write
- **AND** the agreement is written afterward, as a separate act

#### Scenario: An interruption between the two leaves the safe reading

- **WHEN** a run is interrupted after writing a side and before recording the agreement
- **THEN** the next run reads the recorded agreement as the older one
- **AND** the next divergence on that mapping is reported as a conflict rather than resolved by a guess

#### Scenario: Nothing value-derived reaches clan's store or the plaintext definitions tree

- **WHEN** the runtime is searched for a write of the agreement to a file clan placed, or to a path under the definitions tree safix commits in the clear
- **THEN** neither is found
- **AND** the reason is recorded for each: clan's store is reached only through its command, and a digest of a secret value committed in the clear is an offline-confirmable oracle for anyone holding the tree

### Requirement: A two-way push into clan carries sync's safix-to-clan discipline, with no override

Writing a two-way mapping's clan side SHALL compare against clan's current value before writing, for the reason `bridge-transfer` already gives for its safix-to-clan direction, and SHALL refuse when clan reports the generator's recorded validation stale, with no option that proceeds past that refusal.

#### Scenario: The comparison is asked of the same code path a safix-to-clan write uses

- **WHEN** a two-way convergence decides to write clan's side
- **THEN** it is refused under the identical condition, and with the identical message, a safix-to-clan write of the same mapping would be refused under

#### Scenario: No flag or mode defeats the stale-generator refusal

- **WHEN** `sync`'s arguments are enumerated
- **THEN** none of them proceeds past a stale-generator refusal

### Requirement: The report names mappings and their outcome, never a value

Each two-way mapping `sync`'s clan target acts on SHALL be reported as unchanged, updated toward safix, updated toward clan, conflict, or refused with its reason, and no value and no digest SHALL appear in any report, refusal, or commit message.
Rendered rather than structured, an `updated toward safix` outcome reads `pulled <mapping> ← clan`, an `updated toward clan` outcome reads `pushed <mapping> → clan`, and — because a two-way convergence names no source and no destination — its own outcome reads `converged <mapping>` rather than reusing either arrow, matching the rendering `rename-transfer-verbs` establishes for the two one-way directions.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every mapping it acted on appears with exactly one of the five outcomes
- **AND** no output names a value or the digest recorded for it

#### Scenario: Each write is its own commit

- **WHEN** a two-way convergence writes safix's side and records the agreement
- **THEN** the value and the agreement are two separate commits, each naming only the mapping and what it did
