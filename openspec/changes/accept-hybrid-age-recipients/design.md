# Design

## Context

See proposal.md, "Why".

The hybrid formats come from age's `pq.go`, read at the local clone of `FiloSottile/age` (tag v1.3.2; nixpkgs pins age 1.3.1, and sops 3.13.3 vendors `filippo.io/age v1.3.1` at `go.mod:8`).
A recipient is bech32 with human-readable part `age1pq` over the 1216-byte ML-KEM-768 + X25519 public key (`pq.go:45-58`, `:91-94`).
That is 1946 data characters and a 6-character checksum, so `age1pq1` is followed by 1952 characters and the whole string is 1959 long.
age's own usage text agrees: `age1pq1pd[... 1950 more characters ...]` (`cmd/age-keygen/keygen.go:50`).
An identity is bech32 with human-readable part `AGE-SECRET-KEY-PQ-` over a 32-byte seed (`pq.go:177-180`), 77 characters in all, matching the example at `keygen.go:51`.
`age-keygen -pq` mints one, and `age-keygen -y` converts either identity type (`keygen.go:133-160`, `:167-187`).

sops routes `age1pq1…` to `age.ParseHybridRecipient` before its plugin branch, which matches any `age1` string with a second `1` (`age/keysource.go:491-503`), and routes `AGE-SECRET-KEY-PQ-1…` to `age.ParseHybridIdentity` (`:566-567`).
sops added this in 3.12.0 (`CHANGELOG.md:217`); age added it in 1.3.0 (`README.md:248`).

sops wraps the data key to each age recipient in its own age file (`age/keysource.go:163-193`), so a sops file can name hybrid and classic recipients together.
The `age` binary encrypts one file to all recipients at once, and refuses to mix a recipient labelled `postquantum` with one that is not (`age.go:212-215`, `pq.go:65-88`).

In safix the shape judgements are:

| Site | Today | Hybrid |
|---|---|---|
| `crates/safix-core/src/adduser.rs:238-261` `refuse_bad_recipient` | `age1` + 58 bech32 | refused |
| `crates/safix-core/src/identity.rs:729-758` `age_recipients` | private lines must start `AGE-SECRET-KEY-1` | refused |
| `crates/safix-core/src/identity.rs:789-801` `normalize_recipient` | `age1` prefix, no whitespace | accepted |
| `crates/safix-core/src/ciphertext.rs:436-441` `require_local_recipients` | `age1` prefix | accepted |
| `crates/safix-core/src/ciphertext.rs:646-700` `recipient_sets` | `age1` + lowercase alphanumeric | accepted, mixing unchecked |
| `crates/safix-core/src/ciphertext.rs:634-644` `is_ssh_recipient` | SSH only | not involved |
| `crates/safix-core/src/migrate.rs:427-439` | `age1` + lowercase alphanumeric | accepted |
| `crates/safix-core/src/enroll/identity.rs:160-193` `capture` | recipient must start `age1yubikey1` | not captured, correctly |
| `modules/flake/safix/resolve.nix:679-685` `validRecipient` | `[A-Za-z0-9+/= ._:@-]+` | accepted |
| `modules/flake/safix/policy.nix:226-233` `recipientScalar` | bare when `[A-Za-z0-9_-]+` | emitted bare |

`types.nix` (`:637-645`, `:925-956`) gives examples and prose but validates nothing.

## Goals / Non-Goals

**Goals:**

- Every site in the table either accepts the hybrid form or refuses it for a stated reason, and a test holds each one.
- `keygen` can mint a hybrid identity without changing what it mints by default.

**Non-Goals:**

- Checking the bech32 checksum.
  safix checks shape only, as it does for X25519 (`adduser.rs:236-237`); sops parses the key and reports a bad checksum.
- Making hybrid the default, or warning about classic recipients.
- Rewrapping existing files to hybrid recipients.
  That is the existing `fix` path once a declaration changes.
- FIDO2 hmac-secret keys in `safix enroll` (project owner's decision D4, D7).

## Decisions

### D1. The hybrid test runs before every other age test in `refuse_bad_recipient`

The order becomes: hardware prefix `age1yubikey1`, then `age1pq1` plus exactly 1952 bech32 characters, then the SSH shape, then `age1` plus 58 bech32 characters, then `pgp:`.
`age1pq1` cannot start with `age1yubikey1`, so the hardware test is unaffected.
`age1pq1` holds a second `1`, which is sops' test for a plugin recipient; placing the hybrid branch first means any later plugin-shaped test cannot claim it.
The X25519 branch cannot accept a hybrid string, because 1952 is not 58 and `1` is outside the bech32 alphabet constant (`adduser.rs:44`).
The body length is a named constant, `HYBRID_RECIPIENT_BODY: usize = 1952`, beside `RECIPIENT_BODY` (`adduser.rs:47`), with the derivation in its doc comment.

Rejected: accepting any `age1pq1` string of bech32 characters.
A truncated paste would pass and fail only at encryption time on someone else's machine, which is the failure the length check exists to stop for X25519.

### D2. The other recipient sites keep their looser test

`normalize_recipient`, `require_local_recipients`, `recipient_sets`, `migrate.rs:427` and the nix resolver already accept the hybrid string, and sops or age validates it fully when it wraps.
Tightening them to exact lengths is a separate change with its own compatibility cost.
Each gets a test that a 1959-character hybrid recipient passes, so a later tightening cannot drop the form silently.

### D3. Mixed native-age writes are refused by safix

One predicate, `pub(crate) fn is_hybrid_recipient(recipient: &str) -> bool` in `crates/safix-core/src/ciphertext.rs` beside `is_ssh_recipient`, answers "starts with `age1pq1`".
`recipient_sets` uses it to classify each age-side recipient as hybrid or classic, and for `Format::Age` refuses a set that holds both, through the existing `document_error("encrypt", …)` path, with the cause `native age cannot mix post-quantum and classic recipients`.
`migrate.rs` `validate_recipients` (`:399-447`) applies the same rule to a plan's `age`-format destination, beside its existing refusal of PGP recipients for raw age (`:412-414`), so a plan that cannot be written is refused when it is read rather than part-way through a migration.
age would refuse the write anyway; refusing first gives a stable message and means no child process runs for a write that cannot succeed.
sops-format destinations are not checked, because sops wraps per recipient.

Rejected: bypassing age's label check.
age documents a bypass (`pq.go:69-70`), but only through its Go API, and it would leave a file that looks post-quantum and is not.

### D4. `age_recipients` admits `AGE-SECRET-KEY-PQ-1` lines

The prefix test at `identity.rs:738` becomes "starts with `AGE-SECRET-KEY-1` or `AGE-SECRET-KEY-PQ-1`".
Recipients are still derived by piping each line to `age-keygen -y` (`:741-745`), which handles both types, so safix computes no key material.
The derived recipient check at `:749` already admits `age1pq1…`.
Lines beginning `AGE-PLUGIN-` stay refused here as they are today; that is outside this change.

### D5. `--hybrid` is a flag of `--kind age` only

The form is `safix keygen --kind age --hybrid`, matching `age-keygen -pq` in meaning and spelled for what it produces.
`crate::identity::generate_age` gains a `hybrid: bool` parameter that adds `-pq` to the `age-keygen` invocation.
Its existing callers, `keygen::run` (`keygen.rs:88`) and plain `--kind age` (`main.rs:916`), pass `false`.
The workspace form `keygen [<user>]` does not take `--hybrid`, because the project owner's decision D3 names `--kind age`.
The flag is opt-in because of the size and reader-floor costs in D6.

`keygen --show` needs no change: `age-keygen` writes the full recipient in the `# public key:` comment (`keygen.go:157`), and `show` prints the last such comment (`keygen.rs:113-117`).

Rejected: `--pq`.
It copies age's flag name, but `--hybrid` says that the key is still X25519 as well, which is the property readers ask about.

### D6. The cost is derived here and measured by a task

For a sops file, each age recipient adds a `recipient` scalar and an armored age file under `enc`.
Hybrid: the recipient is 1959 bytes.
The age file is a 22-byte version line, a `mlkem768x25519` stanza whose argument is the 1120-byte encapsulation in unpadded base64 (1494 characters), a 48-byte wrapped file key (64 characters plus the empty line a full final line requires), a 48-byte MAC line, and a 64-byte payload: 1713 bytes.
Armored, that is 2284 base64 characters in 36 lines plus the two armor lines, about 2.4 KB, and about 2.8 KB once indented as a YAML block.
In total, about 4.8 KB per hybrid recipient per file.
X25519: the recipient is 62 bytes and the armored file about 0.4 KB, about 0.55 KB in total.
A task measures both with the pinned sops, and the docs quote the measured figures, not these.

Readers: any tool that decrypts a file wrapped to a hybrid recipient with the matching identity needs sops 3.12.0 or later, or age 1.3.0 or later.
A reader holding only a classic identity in a mixed sops file is unaffected.

### D7. FIDO2 hmac-secret keys are out of scope for `enroll`

Recorded from the project owner's decision D4.
This change makes no code or documentation change for it.
The sentence in `docs/guides/hardware-keys.md` stating it is owned by the change `sops-environment-and-darwin-paths`.

## Risks / Trade-offs

- [A future age release changes the hybrid key size] → The length is one constant with its derivation beside it, and the `safix-keygen-hybrid` check asserts that the pinned `age-keygen -pq` output is accepted by `adduser`, so a nixpkgs bump that changes the size turns that check red.
- [A mixed sops file is only as strong as its classic recipients] → Stated in `docs/concepts/custody.md`; D3 does not refuse it, because sops' per-recipient wrapping makes it valid and refusing would block incremental adoption.
- [A reader below the floor cannot decrypt] → The floor is stated in the `keygen` help text and in the CLI reference; recipients outside safix (for example a migration target consumed by agenix with `rage`) are the operator's to check.
- [The policy line is 1959 characters] → `recipientScalar` emits it bare and YAML has no line limit; `safix-keygen-hybrid` drives the regenerated policy through real sops.

## Migration Plan

Additive.
No existing declaration, identity file or ciphertext changes meaning, and plain `keygen` output is unchanged.
Rollback is reverting the change; a hybrid recipient already declared would then be refused by `adduser` but still accepted by evaluation, as today.
