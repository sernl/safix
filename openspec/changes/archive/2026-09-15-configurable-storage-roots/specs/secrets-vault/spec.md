## MODIFIED Requirements

### Requirement: Every vault-rooted name is opaque

When a vault is declared, every ciphertext document, public output, generator definition record, and in-document key name the vault holds SHALL be derived from the naming key, a use-specific tag, and the corresponding logical name, rather than from the logical name alone.
The logical name a derivation consumes SHALL be the entry's name relative to its configured storage root, never the root-prefixed name, so that a vault-rooted name is a function of the entry's identity and not of the readable tree's spelling.
No vault-rooted physical name or directory listing SHALL disclose an audience's membership, a secret's name, or a definition record's owner.

#### Scenario: A ciphertext document's name is opaque

- **WHEN** a secret's audience resolves to a file in vault mode
- **THEN** the file's name is a hash of the naming key and the audience's readable identity, not the audience's members or their count
- **AND** the file sits one level under the vault's ciphertext prefix, never nested in an audience-named directory

#### Scenario: A public output's path is opaque

- **WHEN** a public output resolves to a path in vault mode
- **THEN** the path is a hash of the naming key and the output's readable identity, held as a single file rather than a `<name>/value` directory

#### Scenario: A definition record's path is opaque

- **WHEN** a generator's definition record resolves to a path in vault mode
- **THEN** the path is a hash of the naming key and the record's readable owner-or-audience identity, one level under the vault's state prefix

#### Scenario: A document's key name is opaque

- **WHEN** a secret's value is written into a vault-mode document
- **THEN** the top-level key it is stored under is a hash of the naming key, the document's readable identity, and the secret's readable name
- **AND** this holds whether or not the entry declares a custom key name

#### Scenario: Renaming a readable root does not rename a vault name

- **WHEN** a storage root is changed and a vault is declared
- **THEN** every opaque file name, public leaf, definition record and in-document key is unchanged
- **AND** the change costs no decryption, because the derivation's input never carried the root

#### Scenario: The tags remain the separator between the uses

- **WHEN** two entries in different trees share one root-relative readable identity
- **THEN** their opaque names differ
- **AND** the use-specific tag is what distinguishes them, which is why it is load-bearing rather than decorative once the root is no longer part of the input

#### Scenario: The vault's own buckets are not configurable

- **WHEN** the vault's layout is inspected
- **THEN** its ciphertext, public and state buckets sit flat at the vault root under fixed names
- **AND** they are independent of every declaration-side storage root, because the vault is a dedicated repository whose root is its own

#### Scenario: No vault, no opacity

- **WHEN** no vault is declared
- **THEN** every ciphertext document, public output, definition record, and in-document key resolves to its readable name under its configured storage root
