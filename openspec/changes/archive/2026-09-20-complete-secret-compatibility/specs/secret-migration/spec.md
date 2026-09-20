# Spec Delta

## Purpose

Move secrets between supported encrypted containers and recipient kinds without losing bytes, deployment metadata or the original recovery path.

## ADDED Requirements

### Requirement: Migration verifies every destination before publication

Migration SHALL encrypt to explicitly named recipients, decrypt through explicitly named target identities and compare the recovered bytes with the source. Ambient identities SHALL NOT satisfy independent verification. Existing destinations and aliases of sources SHALL be refused. Sources SHALL always be retained. A verification failure SHALL publish no new ciphertext or deployment artifacts. Recoverable publication errors SHALL roll back outputs created by that run. Publication across multiple filesystem paths is not crash-atomic: interruption MAY leave a subset of already verified outputs, but SHALL NOT remove the original sources.

#### Scenario: Binary migration and reverse migration
- **WHEN** a value containing NUL, invalid UTF-8 and a trailing newline migrates between raw age and SOPS binary, then back
- **THEN** each verified destination contains exactly the source bytes
- **AND** every source remains available

#### Scenario: The target cannot decrypt
- **WHEN** target verification uses an identity outside the destination audience
- **THEN** migration refuses before publication even if an ambient source identity could decrypt it

### Requirement: Migration preserves deployment declarations

A successful migration SHALL publish usable declarations for the chosen safix, sops-nix or agenix consumer and a value-free receipt preserving source and destination mappings, paths, modes, owners, groups and requested lifecycle hooks. Generated declarations SHALL copy ciphertext into the Nix store when their consumer requires store paths, without copying plaintext or identities. A target that cannot express a requested semantic, including unsupported hooks, templates or early-user permissions, SHALL be refused before publication rather than silently dropping that semantic.

#### Scenario: Custom installation contract survives conversion
- **WHEN** a secret with a custom path, mode and supported service hook is migrated
- **THEN** the generated consumer declaration retains those settings
- **AND** the receipt contains no plaintext or low-entropy plaintext digest

#### Scenario: A later candidate fails verification
- **WHEN** an earlier candidate verifies but a later candidate cannot be decrypted by its target identity
- **THEN** no candidate, declaration or receipt is published
- **AND** all source ciphertext remains unchanged
