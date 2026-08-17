# recipient-policy Specification

## Purpose

The recipient policy the encryption tool reads off disk: that it is generated from the declarations rather than hand-written, that it carries one rule per audience, that every rule is anchored, extension-terminated and scoped to one directory level, that no catch-all exists so an unmatched path fails closed, and that the committed file is held to the generated one by a check.

## Requirements

### Requirement: The policy file is generated and never hand-edited

The recipient policy SHALL be rendered from the same declarations the resolver reads, SHALL be committed to the consumer's repository because the encryption tool reads it from the filesystem, and SHALL carry a header stating that it is generated and naming the command that regenerates it.

#### Scenario: One declaration, two projections

- **WHEN** the resolver computes a secret's audience and the renderer emits that audience's rule
- **THEN** both are derived from the same declarations
- **AND** no arrangement of declarations produces a file whose rule and whose resolved audience disagree

#### Scenario: Drift from the committed file is a finding

- **WHEN** the committed policy file differs from the one the declarations imply
- **THEN** a check fails
- **AND** the failure names the command that regenerates the file

#### Scenario: The generator does not overwrite the people

- **WHEN** the policy is regenerated
- **THEN** every declared person's rule is present in the output, because the people are the generator's input
- **AND** regeneration cannot drop a rule that no declaration removed

### Requirement: One rule per audience, naming exactly that audience's recipients

The policy SHALL contain exactly one rule per distinct audience, and that rule's recipient list SHALL be exactly the recipients the audience's members hold.

#### Scenario: A rule's recipient list

- **WHEN** a rule is emitted for an audience
- **THEN** it names every recipient held by every member of that audience, including their further recipients of their own custody
- **AND** it names no recipient held by anyone outside the audience

#### Scenario: Recipients are declared once and referenced

- **WHEN** a recipient appears in more than one rule
- **THEN** the key is declared once under a stable anchor and referenced from each rule
- **AND** an anchor's name is subject to the same alphabet every other name is

#### Scenario: A widened rule is a disclosure rather than untidiness

- **WHEN** a rule is widened such that it matches another person's file
- **THEN** a recipient-update sweep would re-encrypt that file to recipients its owner did not choose
- **AND** the arrangement records that the owner's operator cannot undo it, because they cannot decrypt the file to restore it

### Requirement: Every rule is anchored, extension-terminated, and scoped to one directory level

Every path pattern SHALL begin with a start-of-string anchor, SHALL end with the literal file extension anchored at end of string, and SHALL match exactly one directory level.
A pattern violating any of the three SHALL fail a check that names the offending rule.

#### Scenario: A well-formed rule

- **WHEN** a rule is emitted
- **THEN** its pattern begins with a start-of-string anchor
- **AND** ends with the literal file extension anchored at end of string
- **AND** matches a single directory level, admitting no nested path and no prefixed path

#### Scenario: Why the start anchor is load-bearing

- **WHEN** a pattern omits the start anchor
- **THEN** it also matches the same suffix under any prefix, because the encryption tool matches patterns unanchored against the path relative to the policy file
- **AND** the check fails naming the rule

#### Scenario: Why the extension anchor is load-bearing

- **WHEN** a pattern omits the extension anchor
- **THEN** it can match encrypted material this package did not place, whose recipients a sweep would silently rewrite
- **AND** the check fails naming the rule

#### Scenario: Why one directory level is load-bearing

- **WHEN** a pattern admits nested paths through an unrestricted wildcard
- **THEN** a file dropped in a subdirectory would inherit a person's recipients rather than failing closed
- **AND** the check fails naming the rule

#### Scenario: A file placed beside a person's secrets rides their custody

- **WHEN** a file is added inside a directory an existing rule already covers
- **THEN** it is governed by that rule with no new rule required
- **AND** no review round is needed for it

#### Scenario: Anchoring makes rule order immaterial

- **WHEN** two people's rules coexist
- **THEN** their patterns match disjoint directories
- **AND** reordering the rules changes no file's recipients

### Requirement: No catch-all exists and an unmatched path fails closed

The policy SHALL contain no catch-all rule and the generator SHALL emit none.
A path matching no rule SHALL fail encryption with the tool's own no-matching-rule error.

#### Scenario: An unmatched path

- **WHEN** encryption is attempted against a path no rule matches
- **THEN** it fails with an explicit no-matching-rule error
- **AND** the file does not acquire a default recipient set

#### Scenario: How a new person acquires a rule

- **WHEN** a person is added
- **THEN** their rule appears because a user record with a recipient was declared
- **AND** there is no second registration step and no fallback rule to rely on

#### Scenario: A declared person holding nothing yet

- **WHEN** a person is declared with a recipient but holds no secret
- **THEN** their key appears as an anchor
- **AND** no rule is emitted for them until they hold something, since no audience includes them

### Requirement: Placement is derived and an authored file is refused

Which encrypted file holds a secret SHALL be computed from that secret's audience.
A declaration attempting to name the file directly SHALL fail evaluation.

#### Scenario: The derivation

- **WHEN** a secret's audience is known
- **THEN** its file is determined by that audience alone
- **AND** two secrets with the same audience share one file

#### Scenario: An authored file is refused by name

- **WHEN** a declaration names the encrypted file directly
- **THEN** evaluation fails naming the declaration
- **AND** the message states that a file's recipients are a property of the file, and names the option that widens an audience instead

#### Scenario: The refused field stays in the vocabulary

- **WHEN** an author reaches for the field that names a file
- **THEN** the field exists and refuses with an explanation
- **AND** the author is not left with an unknown-option error that says only that they were wrong
