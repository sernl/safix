# Proposal

## Why

The pinned sops (3.13.3) and age (1.3.1) accept hybrid post-quantum X25519 + ML-KEM-768 recipients (`age1pq1…`) and identities (`AGE-SECRET-KEY-PQ-1…`), and sops has done so since 3.12.0.
safix refuses both by shape: `adduser` admits only `age1` followed by 58 bech32 characters (`crates/safix-core/src/adduser.rs:247-249`), and the identity-file reader behind `keygen` and `identity backup` refuses any private line that does not start with `AGE-SECRET-KEY-1` (`crates/safix-core/src/identity.rs:738-740`).
A person who wants files that stay confidential against a future quantum adversary cannot declare such a recipient through safix, and cannot mint or back up such an identity with it (decision D3).

## What Changes

- `safix adduser` accepts a hybrid recipient: `age1pq1` followed by exactly 1952 bech32 characters.
  `age1yubikey1…` stays refused as a hardware recipient, and the hybrid test runs before any test that would read a second `1` as a plugin recipient.
- Every other place safix judges an age recipient's shape is reviewed for the hybrid form, and each gets a test that holds it.
  The nix resolver, the policy renderer, the explicit-recipient write path, the migration plan reader, recovery-recipient normalisation and the raw-recipient metadata gate already admit it.
- The private-identity reader admits `AGE-SECRET-KEY-PQ-1…` lines beside `AGE-SECRET-KEY-1…` lines, so `identity backup age`, `identity restore` and the separation check work on an identity file that holds a hybrid identity.
- A native-age (`format = "age"`) write whose recipient set mixes hybrid and classic recipients is refused before `age` runs, naming the cause.
  age itself refuses that mix, and the refusal is now safix's own.
- `safix keygen --kind age --hybrid` mints a hybrid identity with the pinned `age-keygen -pq` and appends it to the same identity file plain `--kind age` appends to.
  The plain form is unchanged and stays the default.
- Docs: the `keygen` and `adduser` sections of `docs/reference/cli.md`, "Recipients and identities" in `docs/concepts/custody.md`, the native-age recipient line in `docs/reference/migration-plan.md`, the `safix::bad_recipient` row in `docs/reference/refusals.md`, and the help text and refusal prose.
  The docs state the per-recipient ciphertext cost and the reader floor (sops >= 3.12.0, age >= 1.3.0).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-custody`: a recipient may be a hybrid post-quantum age recipient, and the mix rule for native-age files is stated.
- `safix-cli`: `keygen --kind age --hybrid` is a form of `keygen`, and `adduser`'s shape check admits the hybrid recipient.
- `identity-custody`: hybrid age identities are a supported profile that backup, restore and the separation check handle like native X25519 identities.

## Impact

- Code: `crates/safix-core/src/adduser.rs`, `crates/safix-core/src/identity.rs`, `crates/safix-core/src/ciphertext.rs`, `crates/safix-core/src/error/prose.rs`, `crates/safix/src/main.rs` (`keygen_command`), `crates/safix/src/usage.rs` (`KEYGEN`, `ADDUSER`).
- Tests: unit tests in `adduser.rs`, `identity.rs` and `ciphertext.rs`; a new integration target `crates/safix/tests/keygen.rs`; two new checks in `modules/flake/checks/cli.nix`.
- Nix: no behavioural change; `modules/flake/checks/custody.nix` gains a resolver case that holds the hybrid recipient's acceptance.
- Dependencies: none added.
  The hybrid key path requires the pinned `age-keygen` to support `-pq`, which age 1.3.0 and later do.
- Interoperability: a file wrapped to a hybrid recipient is readable only by sops >= 3.12.0 or age >= 1.3.0.
  Each hybrid recipient costs about 4.8 KB of metadata in every sops file it is wrapped into, against about 0.55 KB for an X25519 recipient (derived in design.md D6; measured by a task).
