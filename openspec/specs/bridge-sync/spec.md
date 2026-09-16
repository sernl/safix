# bridge-sync Specification

## Purpose
A declared clan var and a declared safix entry stay converged in either direction, with a remembered last agreement that tells a genuine conflict apart from an ordinary one-sided change, so a two-way relationship across the clan boundary is safe rather than merely permitted.

## Requirements

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
This decision SHALL be the one stated once for every target rather than a copy of it, and this target's five outcome words SHALL be this target's own mapping of its verdict.

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

#### Scenario: A shared mapping's address is discovered from clan, not declared twice

- **WHEN** a two-way mapping's placement is shared
- **THEN** the machine used to reach it on clan's command line is discovered the same way `bridge-transfer` requires for every direction, one-way or two-way alike

#### Scenario: The clan target's convergence and the keepassxc target's reach the same decision

- **WHEN** a two-way mapping on either target is judged over the same two values and the same recorded agreement
- **THEN** the verdict is the same
- **AND** it is the same code that produced it, so the two cannot drift apart

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

### Requirement: One judgement decides every target's convergence, and each target words its own outcome

The three-way decision a convergence rests on — both sides absent is nothing to do, exactly one side absent is a bootstrap write toward the empty one with the agreement recorded, both sides equal is nothing to do, both sides differing consults the recorded agreement and converges toward the side that moved, and neither side matching the agreement or no agreement recorded is a conflict — SHALL be stated once and reached by every target, rather than restated per target.
That judgement SHALL yield a verdict, and each target SHALL map the verdict onto its own report words, because the targets do not report the same set of outcomes.
No target SHALL hold a second copy of the decision.

#### Scenario: One decision, reached from every target

- **WHEN** the decision a two-way convergence rests on is located
- **THEN** there is exactly one of it
- **AND** every target's convergence reaches it rather than carrying its own

#### Scenario: A target's report words stay the target's own

- **WHEN** a verdict is reported
- **THEN** the words are the target's own, and a target reporting an outcome no other target has keeps it
- **AND** the shared decision names none of those words, so adding a target's outcome does not change the decision

#### Scenario: The decision consults nothing but the two sides and the agreement

- **WHEN** the decision is given two sides and a recorded agreement
- **THEN** its verdict is a function of those three alone
- **AND** it reads no clock, no ordering and no other state, so no run can pick a winner from one

### Requirement: What a far side is, and what stays outside that contract

A far side of a mapping SHALL be reached through one stated contract: what it needs before the first mapping is touched, the reading of one address as a value and the metadata declared beside it, the writing of both together, the enumeration of what it holds, and what it declares itself able to carry.
A refusal that only one far side can raise SHALL stay outside that contract, be raised by that far side alone, and be documented as belonging to it.
Those refusals SHALL include, and are not widened by this requirement to more than: clan's stale-generator refusal, clan's shared-placement address discovery, clan's committing in its own repository rather than in this one, clan's ungenerated var being an ordinary absence, the keepassxc value channel's single-line limit, and the keepassxc write burst.

#### Scenario: Every far side answers the same five questions

- **WHEN** a far side is reached
- **THEN** it is asked what it needs to unlock, what it holds at an address, what to write there, what it holds in total, and what it can carry
- **AND** nothing else about a far side is asked through that contract

#### Scenario: A per-target refusal is not promoted into the contract

- **WHEN** a refusal only one far side can raise is located
- **THEN** it is raised by that far side, and the contract says nothing about it
- **AND** the reason it cannot be shared is recorded beside it, so a later target does not inherit a rule that was never about it

#### Scenario: What a far side can carry is something it declares

- **WHEN** a far side is asked what it can carry
- **THEN** it answers per field, naming whether it can carry that field at all and over which channel, and whether its value channel accepts more than one line
- **AND** every refusal about a field or a value's shape is derived from that answer rather than written as a special case
