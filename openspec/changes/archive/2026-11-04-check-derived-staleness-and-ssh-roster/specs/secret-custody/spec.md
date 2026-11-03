## MODIFIED Requirements

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
- **AND** for raw age files a leaving carrier's SSH key is reported from its stanza tag, and a recorded audience digest that no longer matches is reported as an audience no longer declared
- **AND** raw age checks report every other recipient of the old audience as unverifiable instead of declaring custody unchanged
