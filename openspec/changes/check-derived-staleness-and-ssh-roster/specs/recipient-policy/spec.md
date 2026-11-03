## MODIFIED Requirements

### Requirement: One rule per audience, naming exactly that audience's recipients

Every format-specific file rule SHALL name exactly the recipients of its audience. Native age and explicit GnuPG fingerprints SHALL be rendered in the corresponding recipient groups without introducing extra recipients or a threshold between recipient kinds. Raw age declarations SHALL reject GnuPG recipients and SHALL NOT claim a raw-age recipient roster is verified: SSH recipients SHALL be compared only by the tag age writes into their stanzas, and every other recipient kind SHALL be reported as unverifiable. Governed metadata checks SHALL report unsupported recipient providers and threshold groups explicitly rather than treating their recipient union as an ordinary audience.

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
- **WHEN** fix or enrollment is about to re-wrap a raw-age file
- **THEN** it requires explicit trust in the declared recipients before rewrapping that file
- **AND** its diagnostics do not describe the previous roster as verified

#### Scenario: An SSH tag match is not a verified roster
- **WHEN** a raw-age file's SSH stanzas match the tags of its declared SSH recipients
- **THEN** every report of that match states that it is by a four-byte tag and does not establish the keys
- **AND** no report describes an X25519, plugin or scrypt recipient as verified or as absent

## ADDED Requirements

### Requirement: An unchanged raw-age audience is not re-wrapped

Each declared raw-age entry SHALL have a committed plaintext audience record holding a format tag, a digest of the audience the file was last wrapped to, and the SHA-256 of the bytes that wrap produced.
`fix` SHALL leave a raw-age file untouched when its record reads, matches the declared audience and matches the file's current bytes, and SHALL rewrite the record after every raw-age re-wrap it performs.

#### Scenario: A second fix run changes nothing

- **WHEN** `safix fix --yes` runs twice with no change to the declarations or to any raw-age file between the runs
- **THEN** the second run rewrites no raw-age file and no audience record
- **AND** says, per skipped file, that it is already wrapped to the declared audience

#### Scenario: A skip needs no authorization

- **WHEN** `safix fix` runs without `--yes` and every raw-age file's record matches
- **THEN** it does not refuse over those files, because nothing about them is replaced

#### Scenario: A changed audience is re-wrapped

- **WHEN** a raw-age file's declared audience differs from the one its record names
- **THEN** `fix --yes` re-wraps it to the declared audience and writes a record naming that audience and the new bytes

#### Scenario: Bytes the record does not describe are re-wrapped

- **WHEN** a raw-age file's bytes differ from those its record names, or its record is absent, in another format, or does not parse
- **THEN** `fix --yes` re-wraps it as it does today and writes a fresh record

#### Scenario: A file no declaration places has no record

- **WHEN** a raw-age file is governed only through `extraGovernedFiles`
- **THEN** it has no audience record and `fix --yes` re-wraps it on every run

#### Scenario: The record carries no value

- **WHEN** an audience record is inspected
- **THEN** it holds the format tag, a digest of recipient strings and a digest of ciphertext, and nothing derived from the plaintext

### Requirement: The audience record's path is the resolver's

The audience record's path SHALL be emitted by the declaration-side resolver on the placement of every raw-age entry, under the generator-record root, and SHALL be absent on every other placement.
In vault mode it SHALL be an opaque name with its readable form carried beside it, and a vault migration or rollback SHALL move it with the value.

#### Scenario: The readable path sits beside the stamps

- **WHEN** no vault is declared and an entry's format is `age`
- **THEN** its audience record is its definition record path with an `.audience` suffix
- **AND** the runtime reconstructs no part of it

#### Scenario: A vault moves the record with the file

- **WHEN** `fix` relocates a raw-age entry into or out of a vault
- **THEN** its audience record moves with it, and `check` reports a readable-layout record left at the declaration root as pending relocation
