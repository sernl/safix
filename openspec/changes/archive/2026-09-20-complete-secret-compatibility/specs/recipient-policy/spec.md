# Spec Delta

## MODIFIED Requirements

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

