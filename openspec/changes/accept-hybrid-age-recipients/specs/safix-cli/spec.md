## ADDED Requirements

### Requirement: `keygen --kind age --hybrid` mints a hybrid post-quantum identity

`keygen --kind age --hybrid` SHALL mint one hybrid X25519 + ML-KEM-768 identity through the upstream `age-keygen`, append it to the identity file plain `--kind age` appends to, and print only its public recipient.
Plain `--kind age` SHALL keep minting an X25519 identity.
The help text SHALL state the per-recipient ciphertext cost and the sops and age versions every reader needs.

#### Scenario: A hybrid identity is appended beside an existing one

- **GIVEN** an identity file that already holds an X25519 identity
- **WHEN** `keygen --kind age --hybrid` runs
- **THEN** the file holds the X25519 identity unchanged and one new `AGE-SECRET-KEY-PQ-1…` identity after it
- **AND** the printed recipient begins `age1pq1` and is the one `age-keygen -y` derives from the new identity

#### Scenario: The private half is never printed

- **WHEN** `keygen --kind age --hybrid` runs
- **THEN** neither standard output nor standard error carries the `AGE-SECRET-KEY-PQ-1…` line

#### Scenario: `--show` names the hybrid recipient after a hybrid mint

- **WHEN** `keygen --show` runs after `keygen --kind age --hybrid`
- **THEN** it prints the hybrid recipient, because that identity was appended last

#### Scenario: `--hybrid` outside the age profile is a usage error

- **WHEN** `--hybrid` is given with `--kind pgp`, or without `--kind age`
- **THEN** the command refuses with its usage form and mints nothing

#### Scenario: The cost is stated

- **WHEN** the `keygen` help text is read
- **THEN** it states that a hybrid recipient is about 2,000 characters long and adds about 4.8 KB to every sops file wrapped to it
- **AND** it states that every reader needs sops 3.12.0 or later, or age 1.3.0 or later

### Requirement: `adduser` admits a hybrid recipient by shape

`adduser` SHALL accept `age1pq1` followed by exactly 1952 bech32 characters as a well-formed recipient, beside the X25519, SSH and `pgp:` shapes it already accepts.
It SHALL keep refusing `age1yubikey1…` as a hardware recipient, and SHALL refuse a hybrid-prefixed string of any other length or alphabet as malformed.

#### Scenario: A hybrid recipient is declared

- **WHEN** `adduser` is given a well-formed hybrid recipient
- **THEN** the scaffold records it as the person's recipient and the regenerated policy names it

#### Scenario: A truncated hybrid recipient is refused

- **WHEN** `adduser` is given `age1pq1` followed by fewer or more than 1952 bech32 characters, or by a character outside the bech32 alphabet
- **THEN** it refuses as a malformed recipient and writes nothing

#### Scenario: The refusal names both age shapes

- **WHEN** a malformed recipient is refused
- **THEN** the message describes the X25519 shape and the hybrid shape
