# Proposal

## Why

Safix currently corrupts non-UTF-8 values on its structured write path. Its YAML/age-only deployment interface also prevents safe migration from otherwise supported age and SOPS installations.

## What Changes

- Preserve arbitrary secret bytes, distinguish intentional empty values from missing values, and refuse unrepresentable text-only transfers before mutation.
- Support raw age and SOPS YAML, JSON, dotenv, INI and binary documents with explicit age/GnuPG identities.
- Extend generated recipient policy to GnuPG without weakening audience checks; report raw-age roster unverifiability explicitly.
- Add whole-document installation, runtime templates, early-user secrets and checked user service hooks.
- Provide verified, source-preserving migration with deployment declarations and rollback on failed verification.
- Provide upstream-backed age and modern GnuPG key generation with independent encrypted recovery.

## Capabilities

### New Capabilities

- `secret-migration`: Interface-level verified bidirectional conversion and deployment mapping.
- `identity-custody`: Interface-level secure generation and independently recoverable private-key custody.

### Modified Capabilities

- `secret-installation`: Formats, identities, templates, early-user ordering and service hooks.
- `recipient-policy`: Format-aware file rules and explicit GnuPG recipients.
- `secret-custody`: Format-specific files remain audience-scoped.
- `safix-cli`: Byte-preserving values, format-aware operations and new commands.
- `rust-runtime`: Stable refusal codes without freezing diagnostic wording in snapshots.

## Impact

Rust core drivers, CLI, Nix registry and consumer modules, existing integration and VM checks, package dependencies, README and changelog. Existing YAML/age declarations retain their default paths and behavior. No private material enters Git or the Nix store.