## ADDED Requirements

### Requirement: Hybrid age identities are a supported opt-in profile

Key generation SHALL offer a hybrid X25519 + ML-KEM-768 age identity as an opt-in profile beside the native X25519 default.
An age identity file holding `AGE-SECRET-KEY-PQ-1…` identities, alone or beside `AGE-SECRET-KEY-1…` identities, SHALL be accepted by backup, restore and the recovery separation check, which SHALL derive each identity's recipient through the upstream `age-keygen`.

#### Scenario: A mixed identity file is backed up and restored

- **GIVEN** a managed age identity file holding one X25519 identity and one hybrid identity
- **WHEN** it is backed up to an independent recovery recipient and restored into an empty protected location
- **THEN** the backup's recipient roster names both public recipients
- **AND** the restored file decrypts a file wrapped only to the hybrid recipient

#### Scenario: A hybrid recovery recipient is independent

- **WHEN** the recovery recipient offered to backup is a hybrid recipient whose identity shares nothing with the backed-up file
- **THEN** backup accepts it and verifies decryption with that identity

#### Scenario: An unrecognised private line is still refused

- **WHEN** the identity file holds a non-comment line that is neither an `AGE-SECRET-KEY-1…` nor an `AGE-SECRET-KEY-PQ-1…` identity
- **THEN** backup refuses without publishing an artifact
