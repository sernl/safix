# identity-custody Specification

## Purpose

Create and retain usable private identities through upstream cryptographic tools, with independently verified encrypted recovery.

## Requirements

### Requirement: New keys use conservative supported profiles

Key generation SHALL default to native age and SHALL offer an expiring GnuPG certification/encryption profile using Ed25519 and cv25519. Safix SHALL NOT offer obsolete algorithms for new identities. GnuPG passphrases SHALL be handled by upstream pinentry, never command arguments or environment variables. Only public recipients and custody instructions SHALL be displayed.

#### Scenario: A person creates a GnuPG identity
- **WHEN** a person requests the GnuPG profile
- **THEN** the protected local keyring contains an expiring certification primary and encryption subkey
- **AND** the output provides the full public fingerprint and declaration instruction

### Requirement: Private custody cannot depend on itself for recovery

Unencrypted private identities SHALL remain outside repositories and the Nix store in owner-protected locations. Backup SHALL require an independent recovery recipient and verify decryption using an identity sharing no private component with the backed-up keys. It SHALL account for every private-key file, refusing unlisted components that cannot be completely recovered. Restore SHALL refuse to overwrite an existing private identity. Compatible existing keys MAY be imported without changing their algorithms.

#### Scenario: A circular backup is refused
- **WHEN** the only backup recipient belongs to the identity being backed up
- **THEN** backup refuses without publishing a recovery artifact

#### Scenario: Recovery is demonstrated
- **WHEN** an independently encrypted backup is restored into an empty protected identity location
- **THEN** the restored identity decrypts the original ciphertext

#### Scenario: A GnuPG recovery artifact is incomplete
- **WHEN** an export lacks a local private primary or any private subkey
- **THEN** backup refuses without publishing an artifact advertised as a complete recovery

#### Scenario: A different certificate aliases an existing private component
- **WHEN** a GnuPG restore would reuse a private keygrip already present in the target home
- **THEN** restore refuses before importing
- **AND** the existing private component files remain unchanged

#### Scenario: Different certificates share a recovery component
- **WHEN** the source and proposed recovery certificates have different fingerprints but share a private encryption keygrip
- **THEN** backup refuses without publishing a recovery artifact

#### Scenario: A private component lacks its public certificate
- **WHEN** the managed keyring contains a private-key file absent from its public certificate inventory
- **THEN** backup refuses rather than silently omitting that component
- **AND** restore refuses to overwrite that private component even though no existing certificate names it
