# custody-subjects Specification

## Purpose

The set of things that can hold a key and appear in an audience grows from a person to a subject — a person, a machine, or a group of subjects — while audiences stay derived, placement stays derived from audience, and custody stays with whoever holds the key.

## Requirements

### Requirement: A machine is a declarable subject

A machine SHALL be declarable with a recipient — the age form of the host identity its system scope already decrypts with — an owner, and tags.
Declaring a machine SHALL change nothing by itself: a tree with machines declared and no grant naming them behaves exactly as before.

#### Scenario: The machine's key is the one it already holds

- **WHEN** a machine's recipient is declared
- **THEN** it is the age form of the host identity the machine's system scope decrypts with
- **AND** no second machine identity is minted or required

#### Scenario: Declaration alone is inert

- **WHEN** machines are declared and no audience names one
- **THEN** every generated rule and every governed file is byte-identical to the tree without them

### Requirement: An audience may include machines

A grant SHALL be able to name a machine, and a file whose audience includes one SHALL carry that machine's recipient in its stanzas, with the entry resolving at that machine's system scope.
The existing refusal of hardware recipients for non-interactive decryption paths SHALL NOT apply to machine recipients, because a host identity decrypts non-interactively by nature.

#### Scenario: A person shares with a machine

- **WHEN** a person grants an entry to a declared machine
- **THEN** the derived audience file is encrypted to the machine's recipient beside the person's
- **AND** the machine's system scope resolves the entry at activation with its own identity

### Requirement: A group names subjects, and its audience is its members

A group SHALL be declarable as a set of subjects — people, machines, or other groups — and an audience naming a group SHALL encrypt to the expanded membership's keys at generation time.
Membership growth SHALL be a re-wrap; membership shrink SHALL be reported as the revocation it is, with the same not-retroactive disclosure every other narrowing carries.

#### Scenario: Membership expands at evaluation

- **WHEN** an audience names a group
- **THEN** the file's stanzas are exactly the expanded membership's recipients
- **AND** a cycle among group definitions is refused at evaluation, naming the participating groups

#### Scenario: Leaving a group is a revocation

- **WHEN** a subject is removed from a group whose audience files exist
- **THEN** `check` reports the shrink as a revocation with rotation named as the remedy
- **AND** nothing claims the removed subject has un-read what it could read

### Requirement: Silos are provable non-overlap

Named groups SHALL be declarable as mutually siloed, and evaluation SHALL refuse any file whose audience would include subjects from two silos, naming the file, the subjects, and the silo declaration that forbids it.

#### Scenario: A cross-silo audience never encrypts

- **WHEN** a grant would put subjects from two siloed groups in one audience
- **THEN** evaluation refuses before any file or rule is generated
- **AND** the refusal names both silos and the offending grant

### Requirement: Ownership is a record that grants resolve through

A machine SHALL record its owner, and a grant SHALL be able to name the owner of a machine rather than a person by name; the grant SHALL resolve through the declaration, so a change of owner re-wraps toward the new owner rather than continuing to name the old one.

#### Scenario: The grant follows the ownership record

- **WHEN** a machine's owner changes and `fix` runs
- **THEN** files granted to that machine's owner re-wrap toward the new owner
- **AND** the old owner's loss of future access is reported with the same not-retroactive disclosure as any narrowing

### Requirement: Every subject-model feature is scope-portable

Machines, groups, silos, and ownership SHALL behave identically whether a profile is a NixOS system scope, a home-manager profile inside NixOS, or a standalone home-manager profile on a non-NixOS distribution.

#### Scenario: The standalone profile is not a second-class consumer

- **WHEN** the same declarations are consumed by a standalone home-manager profile on a non-NixOS host
- **THEN** resolution, refusals, and reports are identical to the NixOS-hosted profile's
- **AND** nothing requires NixOS to be present anywhere in the fleet

### Requirement: A service is a declarable subject that resolves to its machines

A service SHALL be declarable with the machines it runs on, an owner, and the unix user and group its landed entries belong to.
A service SHALL resolve to its declared machines' recipients and SHALL NOT carry an identity of its own; the documentation SHALL state, where the option is declared, that the audience names the service while the host identity remains what decrypts, so the machine is the trust boundary for everything running on it.

#### Scenario: The service names machines the fleet declares

- **WHEN** a service names a machine no declaration covers
- **THEN** evaluation refuses, naming the service and the machine

#### Scenario: Declaration alone is inert

- **WHEN** services are declared and no grant names one
- **THEN** every generated rule and every governed file is byte-identical to the tree without them

### Requirement: An audience may include services, as elements that follow the declaration

A grant SHALL be able to name a service anywhere subjects are named, and a file whose audience includes one SHALL carry the recipients of the service's machines at generation time.
The service SHALL appear in the audience as its own rendered element, so a change to its machine set re-wraps the same files rather than moving them, with growth a re-wrap and shrink reported as the revocation it is.

#### Scenario: A machine joins the service and the files follow

- **WHEN** a machine is added to a granted service's set and `fix` runs
- **THEN** the service's audience files re-wrap to include the new machine's recipient
- **AND** no file moves

#### Scenario: A machine leaving is a revocation

- **WHEN** a machine is removed from a granted service's set
- **THEN** `check` reports the shrink as a revocation with rotation named as the remedy
- **AND** the not-retroactive disclosure is carried as it is for every other narrowing

#### Scenario: An empty service cannot be granted to

- **WHEN** a grant names a service whose machine set is empty
- **THEN** evaluation refuses, naming the grant and the empty set, because the file would be encrypted to nobody

### Requirement: Services share the one subject name space

A service's name SHALL live in the same name space as users, machines, and groups, with collisions refused at evaluation, and groups MAY include services as members.

#### Scenario: A collision is refused where it is declared

- **WHEN** a service is declared with a name a user, machine, or group already holds
- **THEN** evaluation refuses, naming both declarations

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

### Requirement: Delegation is recorded on both sides and confers no read

An organization SHALL be able to name its managers, and a person SHALL be able to declare `managedBy` naming an organization; evaluation SHALL refuse either side naming what the fleet does not declare.
Neither record SHALL place any key in any audience: managing confers scaffolding, never reading, and the option documentation SHALL state the boundary — the verbs bind the cooperative path, the tree remains the authorization, and key generation stays with the person.

#### Scenario: The records are declarations, not access

- **WHEN** alice is named a manager of acme and bob declares `managedBy` acme
- **THEN** no generated rule and no governed file changes
- **AND** bob's audience is exactly what it was

#### Scenario: Dangling references refuse

- **WHEN** `managers` names a person the fleet does not declare, or `managedBy` names an undeclared organization
- **THEN** evaluation refuses, naming the record and the missing declaration
