## ADDED Requirements

### Requirement: check reports a derived value whose dependency changed after it

`safix check` SHALL report a finding for each generator, per holder, when a dependency it declares has an `updated` stamp later than the earliest `updated` stamp among the values that generator writes.
The finding SHALL name the holder, the generator's entry and the dependency, SHALL name `safix generate --regenerate <user> <entry>` as the remedy, and SHALL fail the run.
It SHALL be answered from the stamp records alone.

#### Scenario: A dependency set by hand after the derivation is reported

- **WHEN** a value a generator reads is written by `set`, `edit` or `sync` after that generator last ran
- **THEN** `check` reports the generator's entry as derived from a dependency that has changed since, naming the dependency
- **AND** the remedy is `safix generate --regenerate <user> <entry>` naming the dependent entry, never the dependency
- **AND** `check` exits non-zero
- **AND** no ciphertext is opened and the finding carries no value

#### Scenario: A regeneration cascade leaves nothing to report

- **WHEN** a dependency is regenerated through `generate --regenerate` or `rotate`, which re-run every generator derived from it
- **THEN** no staleness finding exists for any generator in that cascade

#### Scenario: The remedy clears the finding

- **WHEN** the named remedy runs
- **THEN** the finding is gone on the next `check`

#### Scenario: A missing stamp makes no claim

- **WHEN** the dependency or every value the generator writes has no stamp record
- **THEN** no staleness finding exists for that pair

#### Scenario: Equal stamps are not stale

- **WHEN** a dependency's `updated` stamp equals the dependent's, as a cascade completing within one second leaves them
- **THEN** no staleness finding exists for that pair

### Requirement: check judges a raw-age file's SSH recipients by their tags

`safix check` SHALL read each existing raw-age governed file's header, binary or ASCII-armored, without decrypting it, and SHALL compare each SSH stanza's tag against the tags of the SSH recipients the file's declared audience names.
A missing, extra or duplicated SSH stanza SHALL be a failing finding naming `safix fix --yes`.
A header that cannot be read SHALL refuse the run naming the file.

#### Scenario: An extra SSH recipient is reported

- **WHEN** a raw-age file carries an `ssh-ed25519` or `ssh-rsa` stanza whose tag matches no declared SSH recipient of its audience
- **THEN** `check` reports the file's SSH recipients as drifted, listing the stanza as extra
- **AND** names the declared subject holding a key with that tag when exactly one declared key anywhere has it, and the stanza's type and tag otherwise
- **AND** states that re-wrapping is not revocation and names what mints a new value
- **AND** `check` exits non-zero

#### Scenario: A missing SSH recipient is reported

- **WHEN** a declared SSH recipient of the audience has no stanza with its tag in the file
- **THEN** `check` reports that recipient as missing and exits non-zero

#### Scenario: A duplicated SSH stanza is reported

- **WHEN** more stanzas carry one type and tag than the audience declares distinct keys with that type and tag
- **THEN** `check` reports the surplus as duplicated and exits non-zero

#### Scenario: Armored and binary files are read alike

- **WHEN** a raw-age file is ASCII-armored
- **THEN** its header is decoded from the armor and judged exactly as the binary form would be

#### Scenario: Grease stanzas are ignored

- **WHEN** a stanza's type ends in `-grease`
- **THEN** it is neither counted as a recipient nor as unverifiable

#### Scenario: An unreadable header refuses

- **WHEN** a raw-age governed file does not begin with the age v1 header, its armor does not decode, or its header has no closing MAC line
- **THEN** `check` refuses with `safix::recipients_unreadable` naming the file, caused by `safix::age_header_unreadable`

### Requirement: An exact SSH tag match is a note, never a pass

When every stanza of a raw-age file is an SSH stanza and the stanzas match the declared SSH recipients exactly, `safix check` SHALL report a note that the roster matched by 32-bit tag only and that a tag match does not establish which keys the file is wrapped to.
The note SHALL NOT fail the run.

#### Scenario: A matching all-SSH file is a note

- **WHEN** a raw-age file's audience declares only SSH recipients and the file's stanzas match their tags one for one
- **THEN** `check` prints a note naming the file and stating that the match is by a four-byte tag anyone can collide with
- **AND** that note alone does not make `check` exit non-zero

#### Scenario: Any non-SSH recipient keeps the file unverifiable

- **WHEN** a raw-age file carries an X25519, plugin, scrypt or other non-SSH stanza, or its audience declares a non-SSH recipient
- **THEN** `check` reports the file's roster as unverifiable, failing the run as it does today
- **AND** any SSH stanza disagreement in the same file is reported as well, as its own finding

### Requirement: check's exit status follows its failing findings

`safix check` SHALL exit non-zero when at least one failing finding is reported and zero otherwise.
Every finding SHALL fail except the SSH tag-match note.
The closing line SHALL count failing findings only.

#### Scenario: Notes alone exit zero

- **WHEN** the only findings are tag-match notes
- **THEN** `check` prints them, closes by stating that no drift was found and that the notes above qualify it, and exits zero

#### Scenario: A failing finding beside a note exits non-zero

- **WHEN** a note and a failing finding are both reported
- **THEN** the closing line counts the failing finding and not the note
- **AND** `check` exits non-zero

### Requirement: check reports a raw-age file wrapped to an audience that is no longer declared

When a raw-age file's audience record exists, is in a format this version reads, and records the SHA-256 of the file's current bytes, `safix check` SHALL report a failing finding if the audience digest it records differs from the digest of the audience declared now, naming the file and `safix fix --yes`.
An absent or unreadable record, or one recording other bytes, SHALL produce no finding.

#### Scenario: A narrowed audience over a recorded raw-age file is reported

- **WHEN** `fix --yes` re-wrapped a raw-age file and wrote its record, and a recipient is then removed from the file's declared audience
- **THEN** `check` reports the file as wrapped to an audience that is no longer declared
- **AND** states that re-wrapping is not revocation
- **AND** exits non-zero

#### Scenario: An absent record is not a finding

- **WHEN** a raw-age file has no audience record, as every file written before this change has none
- **THEN** no audience-record finding exists for it

#### Scenario: A record about other bytes is silent

- **WHEN** the file was rewritten after its record, by `set`, `edit`, `generate`, `sync` or a tool outside safix
- **THEN** the record says nothing about the current file and no audience-record finding exists for it
