# Spec Delta

## MODIFIED Requirements

### Requirement: The audience picks the file, and one audience gets one file

Every encrypted file SHALL belong to exactly one audience. Ordinary keyed YAML entries of one audience SHALL share their existing file; other formats and whole-document entries SHALL use separately derived files within that same audience. Vault filenames SHALL remain opaque and preserve the format needed to interpret them.

#### Scenario: Separating containers does not widen custody
- **WHEN** two entries of one audience use different formats
- **THEN** their files name exactly the same declared recipients
- **AND** an entry of another audience cannot inherit their rules

#### Scenario: A sole-owner audience

- **WHEN** a secret's audience is one person
- **THEN** it resolves to that person's own file
- **AND** no other person's recipient appears on it

#### Scenario: A multi-member audience

- **WHEN** a secret's audience is several people
- **THEN** it resolves to a file belonging to exactly that audience, in a directory named for its members in sorted order
- **AND** the path states who can open the file without opening it

#### Scenario: Sharing moves the value rather than widening a personal file

- **WHEN** an owner grants one of several secrets to another person
- **THEN** the granted secret moves to the audience's own file
- **AND** the owner's remaining secrets stay in the owner's file, unreadable by the grantee

#### Scenario: A multi-member audience in vault mode

- **WHEN** a secret's audience is several people and a vault is declared
- **THEN** it still resolves to a file belonging to exactly that audience
- **AND** the file's name is opaque rather than a directory named for its members in sorted order, so the path no longer states who can open it — that guarantee is scoped to the case with no vault declared

### Requirement: Carrying is not sharing

Two users carrying one catalogue entry SHALL by default hold independent values in separate files.
An entry marked shared SHALL resolve to one value in one file whose audience is every user who carries it.

#### Scenario: The default is independent values

- **WHEN** two users carry the same catalogue entry and it is not marked shared
- **THEN** each resolves it to their own audience's file
- **AND** setting one value leaves the other untouched

#### Scenario: A shared entry is one value

- **WHEN** two users carry an entry marked shared
- **THEN** both resolve the same file and the same bytes
- **AND** a user joining the carriers can read the existing value

#### Scenario: A shared carrier leaving requires rotation, not a re-wrap

- **WHEN** a user stops carrying a shared entry
- **THEN** the arrangement records that the value needs minting anew rather than merely re-wrapping
- **AND** for inspectable SOPS files that finding is derived from recipient stanzas rather than a stored record of the former audience
- **AND** raw age checks report that the old audience cannot be verified instead of declaring custody unchanged

