## Purpose

Organizations join the model as principals: holders of recovery custody that people consent to by name, owners that grants resolve through, and one more thing an audience can say — while the person's own drawer stays the person's.

## ADDED Requirements

### Requirement: An organization is a declarable principal with custody of its own

An organization SHALL be declarable with its recovery custody — escrow identities, anchored and noted — and nothing else in this phase.
A declared organization nothing references SHALL be byte-inert, and organizations SHALL share the one subject name space with collisions refused.

#### Scenario: Declaration alone is inert

- **WHEN** organizations are declared and nothing references one
- **THEN** every generated rule and every governed file is byte-identical to the tree without them

#### Scenario: Empty custody cannot act

- **WHEN** an escrow declaration, a grant, or an ownership resolution reaches an organization whose custody is empty
- **THEN** evaluation refuses, naming the organization and what reached it, because nothing would be encrypted to it

### Requirement: Escrow is the person's own declaration, naming the organization

A person SHALL be able to declare `escrowedTo` naming a declared organization, and every file the person's audience covers SHALL gain the organization's custody keys at the next re-wrap.
The organization SHALL NOT be able to establish escrow over anyone from its own side, and the option's documentation SHALL carry the trade-off in the person's view: the organization's custody can open everything this person holds, and withdrawing the declaration revokes nothing already readable.

#### Scenario: Consent is written where it acts

- **WHEN** alice declares `escrowedTo` naming `acme`
- **THEN** her files re-wrap to include acme's custody keys
- **AND** the declaration sits in alice's record, not acme's

#### Scenario: Rotation happens in one place

- **WHEN** acme rotates a custody key and `fix` runs
- **THEN** every consenting person's files re-wrap toward the new key
- **AND** no person's declaration changes

#### Scenario: Withdrawal is a revocation

- **WHEN** alice removes her `escrowedTo` declaration
- **THEN** `check` reports the narrowing as the revocation it is, with rotation named as the remedy
- **AND** the not-retroactive disclosure is carried as everywhere else

### Requirement: Ownership resolves through organizations, and grants may name them

A machine's or service's owner SHALL be able to name an organization, with `ownerOf` grants resolving to the organization's custody keys, and a grant SHALL be able to name an organization directly as its own audience element expanding to those keys.
Groups SHALL NOT contain organizations.

#### Scenario: The owner of an acme machine is acme

- **WHEN** a grant names the owner of a machine acme owns
- **THEN** the file's stanzas carry acme's custody keys
- **AND** a later change of that machine's owner re-wraps toward the new owner as ownership already does

#### Scenario: A principal is not a member

- **WHEN** a group declares an organization among its members
- **THEN** evaluation refuses, naming the group and the organization
