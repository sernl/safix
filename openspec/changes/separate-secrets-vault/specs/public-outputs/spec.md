## MODIFIED Requirements

### Requirement: The plaintext store is separable from the ciphertext tree by path prefix

Public outputs SHALL be stored under a top-level prefix distinct from the one holding encrypted material, and SHALL NOT be nested inside it.

#### Scenario: The two trees do not overlap

- **WHEN** the public store's prefix and the encrypted tree's prefix are compared
- **THEN** neither is a prefix of the other

#### Scenario: A prefix-scoped rule can address exactly one of them

- **WHEN** an exclusion, a backup policy or a search is scoped to the encrypted tree's prefix
- **THEN** no plaintext output is inside its scope

#### Scenario: The reason is recorded

- **WHEN** the location decision is documented
- **THEN** it states that a path named for secrets must mean that everything under it is encrypted, without qualification
- **AND** it records the alternative that was refused and why

#### Scenario: The layout distinguishes shared from per-user

- **WHEN** a public output's path is computed
- **THEN** a shared entry's path is keyed by its audience and a per-user entry's by its carrier
- **AND** the leaf carries the output's name

#### Scenario: A vault-mode leaf is opaque, not keyed by audience or carrier

- **WHEN** a public output's path is computed and a vault is declared
- **THEN** the leaf under the public prefix is a hash of the naming key and the output's readable identity, held as a single file rather than a `<name>/value` directory
- **AND** prefix separation from the encrypted tree still holds: neither prefix is a prefix of the other, opaque or not
