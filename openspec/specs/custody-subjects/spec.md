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
