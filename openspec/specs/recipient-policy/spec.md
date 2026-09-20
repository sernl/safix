# recipient-policy Specification

## Purpose

The recipient policy the encryption tool reads off disk: that it is generated from the declarations rather than hand-written, that it carries one rule per audience, that every rule is anchored, extension-terminated and scoped to one directory level, that no catch-all exists so an unmatched path fails closed, and that the committed file is held to the generated one by a check.

## Requirements

### Requirement: The policy file is generated and never hand-edited

The recipient policy SHALL be rendered from the same declarations the resolver reads, SHALL be committed to the declaring repository because the encryption tool reads the committed file from there — whether or not a vault is declared — and SHALL carry a header stating that it is generated and naming the command that regenerates it.
Every path the header's worked examples spell SHALL be derived from the configured storage roots, because a committed header documenting a layout the repository does not have is a false statement in the one file a reviewer reads to learn who can open what.
For a command that needs creation rules to reach a vault-rooted document, the runtime SHALL additionally render a disposable, uncommitted copy of the rules into the vault working tree, scoped to that command alone.

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

#### Scenario: A renamed encrypted root moves the rules and the header together

- **WHEN** the encrypted root is renamed and the policy is regenerated
- **THEN** every rule's pattern is anchored under the new root
- **AND** every path the header's worked examples spell names the new root
- **AND** the drift check reports the committed file until it is regenerated

#### Scenario: A vault does not move the committed file

- **WHEN** a consumer declares a vault
- **THEN** the committed policy file still lives in the declaring repository, exactly where it lives with no vault declared
- **AND** the vault repository never carries a committed policy file of its own

#### Scenario: A vault-rooted command reads a disposable rendering, not the committed file

- **WHEN** a command needs creation rules to encrypt or re-key a vault-rooted document
- **THEN** it reads a rendering produced for that run alone, inside the vault working tree, never the committed file at the declaring root
- **AND** that rendering is never committed

### Requirement: One rule per audience, naming exactly that audience's recipients

Every format-specific file rule SHALL name exactly the recipients of its audience. Native age and explicit GnuPG fingerprints SHALL be rendered in the corresponding recipient groups without introducing extra recipients or a threshold between recipient kinds. Raw age declarations SHALL reject GnuPG recipients and SHALL NOT claim their recipient roster can be inspected from ciphertext. Governed metadata checks SHALL report unsupported recipient providers and threshold groups explicitly rather than treating their recipient union as an ordinary audience.

#### Scenario: Mixed age and GnuPG custody
- **WHEN** an audience declares both kinds of recipient for a SOPS document
- **THEN** either authorized identity kind can decrypt the document
- **AND** metadata drift compares both recipient kinds

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

#### Scenario: Reconciliation cannot inspect a raw-age roster
- **WHEN** fix or enrollment encounters a raw-age file
- **THEN** it requires explicit trust in the declared recipients before rewrapping that file
- **AND** its diagnostics do not describe the previous roster as verified

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

#### Scenario: A vault-mode rule matches one opaque file, not a directory wildcard

- **WHEN** a rule is emitted for a vault-rooted, opaquely named document
- **THEN** its pattern is the literal opaque filename, anchored at both ends
- **AND** it still satisfies every clause of this requirement, because a single-file match is the limiting case of one directory level rather than an exception to it

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

Governed file placement SHALL be derived from the audience, selected format and secret name where the format requires a separate file. Existing keyed YAML placements SHALL retain their paths. Authored governed source files SHALL still be refused; explicit external deployment imports SHALL not pretend to be governed audience placements.

#### Scenario: Format selection preserves audience scope
- **WHEN** an entry changes its storage format
- **THEN** its derived file remains scoped to the same audience
- **AND** no recipient is added because of that format choice

#### Scenario: The derivation

- **WHEN** a secret's audience is known
- **THEN** its file is derived from that audience and the entry's declared format
- **AND** ordinary keyed YAML secrets with the same audience share one file; other formats remain in that audience's derived directory

#### Scenario: An authored file is refused by name

- **WHEN** a declaration names the encrypted file directly
- **THEN** evaluation fails naming the declaration
- **AND** the message states that a file's recipients are a property of the file, and names the option that widens an audience instead

#### Scenario: The refused field stays in the vocabulary

- **WHEN** an author reaches for the field that names a file
- **THEN** the field exists and refuses with an explanation
- **AND** the author is not left with an unknown-option error that says only that they were wrong
