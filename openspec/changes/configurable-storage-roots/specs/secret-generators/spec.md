## MODIFIED Requirements

### Requirement: Minting records the definition it minted under

A generated value SHALL be accompanied by a committed, plaintext record carrying a digest of the generator definition that produced it, written in the same commit as the value and refreshed whenever the value is regenerated.
The digest SHALL be computed over the definition alone; no value and no derivative of a value appears in the record.
The record's path SHALL be computed in exactly one place — the declaration-side resolver — and carried to the runtime on every placement, so that the runtime never constructs a storage path of its own.

#### Scenario: The record rides the mint's own commit

- **WHEN** a generator mints or regenerates a value
- **THEN** the definition record is written in the same commit as the value
- **AND** a mint interrupted before the commit leaves neither

#### Scenario: The record is about the definition, never the value

- **WHEN** the record's content is inspected
- **THEN** it derives from the generator's declaration alone
- **AND** two mints of different values under one definition produce the same record

#### Scenario: The record does not live where its meaning would lie

- **WHEN** the record's path is inspected
- **THEN** it is under the generator-record root and under neither of the other two storage roots
- **AND** the three roots' non-overlap is refused at evaluation, so this holds for every configuration rather than only for the default spellings

#### Scenario: The record's root is the consumer's to name

- **WHEN** the generator-record root is set to a value other than its default
- **THEN** every definition record resolves under it
- **AND** the record's shape below the root — one leaf per generated value, keyed by owner or audience and the output's name — is unchanged

#### Scenario: One implementation of the layout

- **WHEN** a definition record's path is produced for a placement
- **THEN** it is the path the resolver emitted on that placement
- **AND** the runtime reconstructs no part of it, whether or not a vault is declared
