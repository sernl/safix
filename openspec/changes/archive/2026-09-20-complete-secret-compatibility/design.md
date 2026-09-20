# Design

## Context

See proposal.md for scope. Existing declarations derive custody and file paths in Nix; Rust executes effects through upstream tools. A real pre-change probe showed `00 ff fe 41 0a` becoming `00 ef bf bd ef bf bd 41 0a` on write and installation. Existing activation rejects the requested non-YAML formats and deployment fields.

## Decisions

- Keep existing nonempty UTF-8 storage strings. Store binary and intentional empty values as the exact single-member object `{"__safix_bytes_v1":[bytes...]}`. A literal string is never interpreted as an envelope. Text-only APIs reject invalid UTF-8 before mutation.
- Keep upstream age, SOPS and GnuPG as cryptographic authorities. Explicit format and identity records cross the runtime boundary. GnuPG recipients use `pgp:` followed by an uppercase full fingerprint; native age spellings remain unchanged.
- Preserve default YAML paths. Other formats use one derived per-secret file inside the audience directory, with opaque names in vault mode. Raw age cannot disclose its recipient roster; checks must distinguish unverifiable custody from verified agreement, and reconciliation requires explicit trust in the declaration. Unsupported SOPS providers and threshold groups remain findings, not flattened ordinary audiences.
- A normal install remains after user creation. Early-user secrets use a separate root-owned manifest and generation store, installed before user creation. Templates render only inside private runtime state; unknown placeholders fail before promotion. Home service hooks use checked user-manager commands.
- Migration stages ciphertext, decrypts it with target identities and compares bytes before publication. Sources remain untouched. Declarations and receipts publish only after every candidate verifies; destinations cannot be overwritten. Ordinary publication errors roll back outputs, but the multi-path transaction is not crash-atomic and an interrupted process may leave verified outputs beside the retained sources. Format representability is an explicit refusal, not a conversion guess.
- Native age is the generation default. GnuPG uses Ed25519 certification and cv25519 encryption, expiry and upstream pinentry. Private material stays in protected local homes, outside Git and the Nix store. Encrypted recovery requires independent recipients and a demonstrated decrypt path. GnuPG backup verifies all private components; restore refuses both existing fingerprints and private-keygrip aliases before import.

## Gate 1 modality verdicts

The affected capabilities describe machine interfaces. Retain the repository's existing Rust real-tool acceptance runner rather than introduce a second runner solely for this change.

| Requirement | stratum | modality | Witness |
| --- | --- | --- | --- |
| Byte-preserving values | interface | regression | Binary round trip, literal strings, intentional empty, strict text refusal |
| Explicit format and identity support | interface | integration-smoke | Real age/SOPS/GnuPG decrypt/encrypt/install |
| Custody-preserving format placement | interface | integration-smoke | Nix evaluation and generated policy |
| Complete deployment semantics | interface | integration-smoke | Runtime installation and NixOS VM ordering |
| Verified source-preserving migration | interface | regression | Independent target decryption and rollback |
| Secure key generation and recovery | interface | integration-smoke | Disposable keys and independent restore |

## Risks / Trade-offs

The byte envelope increases ciphertext size for binary values; raw age or SOPS binary avoids that overhead. SOPS structured formats may canonicalize whole-document whitespace; migration refuses any requested conversion that fails exact-byte verification. Raw age has no inspectable public recipient roster, so declaration consistency cannot be advertised as cryptographic roster verification. GPG pinentry and hardware-backed identities require an interactive operator; verification uses disposable identities only.