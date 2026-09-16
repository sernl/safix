## ADDED Requirements

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

## MODIFIED Requirements

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
