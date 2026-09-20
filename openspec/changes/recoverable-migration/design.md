# Design

## Context

See proposal.md. Today `migrate::run` stages one private candidate per output beside its destination, verifies it, then `Transaction::publish_all` hard-links every candidate into place and `Drop` rolls back an uncommitted transaction by unlinking outputs whose `(device, inode)` still match. Nothing durable records which outputs a run created, staging directories are named by pid only, `require_absent` cannot tell a leftover from an unrelated file, and a signal bypasses `Drop`. There are no integration tests for the verb.

## Goals / Non-Goals

**Goals:**
- A rerun of the same plan after any interruption either finishes the migration or refuses naming exactly one path and why.
- Every claim the journal makes is checked before it is acted on: identity and bytes, never the path alone.
- The existing guarantees stand unchanged: no overwrite, no source mutation, no plaintext on disk, isolated identities.

**Non-Goals:**
- All-or-nothing visibility across outputs (a single directory swap). Destinations are operator-chosen paths on possibly different filesystems; the journal makes interruption recoverable, not invisible.
- Resuming across a changed plan. A digest mismatch is a refusal, not a merge.

## Decisions

### D1. The journal is a reserved output beside the receipt

Path: `<receipt>.journal`, reserved through `OutputNames::reserve` so it can neither alias a source nor collide with another output. It is written with the same create-new/fsync/rename discipline as the receipt, in a private staging directory, and rewritten atomically after every published output. Alternative considered: a journal under the plan's directory keyed by digest — rejected because a plan's directory may be read-only or shared, and the receipt's directory is already the place the operator agreed to receive outputs.

### D2. The journal's content

JSON, `deny_unknown_fields`, version 1:

- `planDigest`: SHA-256 over the plan file's bytes and its canonical directory, so the same JSON at another path is a different plan.
- `plan`: the canonical plan path, for the refusal message.
- `staging`: every staging directory this run created, so abandonment and resumption can sweep them without guessing names.
- `outputs`: one record per published output — `path`, `kind` (ciphertext, declarations, receipt), `device`, `inode`, `sha256` of the published bytes.

No plaintext, no plaintext digest: ciphertext digests are of what is already on disk. The `Identities` paths are not repeated; the receipt carries them once the run completes.

### D3. Publish order and the commit point

Publication becomes: write the journal (empty `outputs`) → for each candidate, hard-link, record `(device, inode, sha256)`, rewrite the journal → publish declarations → publish receipt → remove the journal → fsync parents. Removing the journal is the commit point; its presence is the whole meaning of "interrupted".

### D4. Resumption re-verifies rather than trusts

On `run`, if the journal exists: parse it, compare `planDigest`; then for each recorded output require `symlink_metadata` to report a regular file with the recorded identity and bytes hashing to the recorded digest. Any mismatch refuses naming the path before anything is published. Kept ciphertext outputs are re-read through the target identities and compared with the source exactly as a fresh candidate is; kept declarations and receipt are recomputed and their digests compared. Only then are the remaining candidates staged and published. Alternative considered: trusting the digest alone — rejected because the verification step is cheap and it is the guarantee the receipt advertises.

### D5. Abandonment removes what matches and nothing else

`--abandon` runs the same journal checks, refuses on any mismatch, then unlinks recorded outputs (identity re-checked immediately before each unlink), removes recorded staging directories, removes the journal, fsyncs parents. A missing journal is a refusal, so `--abandon` cannot be used to delete a completed migration's outputs.

### D6. Signals between steps are ordinary errors

Register SIGINT/SIGTERM through the `scratch` module's existing handler so a flag is set; `run` checks the flag before each stage, each publish and before removing the journal, returning an interruption error that drops the transaction. `Drop` gains journal removal for the uncommitted case. Interruption during a syscall is what D3 covers; nothing tries to make a hard link cancellable.

### D7. Tests build the on-disk contract by hand

A new `crates/safix/tests/migrate.rs` suite with a plan fixture (age identities minted per test, sources encrypted with the real `age`/`sops`, destinations in a fresh directory). Resumption is tested by writing a partial state exactly as the contract describes — a journal recording one hard-linked output — and rerunning; no halt hook is added to the production binary. The interruption-between-steps path is tested through the existing `SAFIX_SHIM_HOLD` shim on the encrypting subprocess, which is the one interruptible step before publication. The spec's "later candidate fails verification" scenario gets its runner by pointing one entry's recipient at an identity the target set does not hold.

## Risks / Trade-offs

- [Journal and output on different filesystems] → The journal lives beside the receipt; outputs elsewhere are still identified by device and inode, which are filesystem-local. Resumption never assumes a shared filesystem.
- [An operator edits a published output before rerun] → Refusal by name, nothing removed; `--abandon` refuses the same way. The operator's file is never the tool's to delete.
- [Journal rewritten after each output costs an fsync per output] → Migrations are operator ceremonies over tens of files, not hot paths.
- [A reader mistakes the journal for a receipt] → Different name, different `version` semantics, and the help names both.

## Migration Plan

No data migration. Existing completed migrations have no journal and behave exactly as before. The README's "inspect partial outputs by hand" paragraph is replaced in the sibling `quickstart-documentation` change; this change updates the verb's help.
