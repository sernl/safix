# Tasks

## 1. Test harness for the verb

- [x] 1.1 Add `crates/safix/tests/migrate.rs` with a plan fixture: per-test age identities, real `age`/`sops` sources, a destination directory, a plan writer; verify a straight migration publishes ciphertext, declarations and receipt, retains sources and leaves no journal.
- [x] 1.2 Give the spec's "later candidate fails verification" scenario its runner: one entry encrypted to a recipient the target identities cannot open; verify nothing is published and sources are unchanged.

## 2. Journal

- [x] 2.1 Reserve `<receipt>.journal` through the output-name reservation and add the journal record type (plan digest, plan path, staging directories, outputs with path, kind, device, inode, sha256); verify a unit test round-trips it and refuses unknown fields.
- [x] 2.2 Write the journal before the first hard link and rewrite it atomically after each published output; remove it after the receipt is published; verify by reading the journal mid-run in a test that publishes through a hand-constructed transaction.
- [x] 2.3 Remove the journal in the uncommitted `Drop` path; verify the rollback test leaves no journal.

## 3. Resume and abandon

- [x] 3.1 On `run`, detect the journal, compare the plan digest, verify every recorded output's identity and bytes, re-verify kept ciphertext through the target identities, publish the rest, remove the journal; verify with a test that builds the partial state by hand (one output hard-linked, journal recording it) and reruns.
- [x] 3.2 Refuse a journal from a different plan naming both paths, and a recorded output that no longer matches naming its path; verify both refusals leave every file in place.
- [x] 3.3 Implement `--abandon`: same checks, unlink matching outputs with identity re-checked before each unlink, remove staging directories and the journal; verify sources and unrecorded files survive and a replaced output refuses.
- [x] 3.4 Register the interruption flag through the scratch signal handler and check it before each stage, each publish and journal removal; verify with the encrypt-hold shim that an interrupted run exits with the interruption status and leaves no outputs and no journal.

## 4. Command surface

- [x] 4.1 Add `--abandon` to `migrate_command` with the arity refusal, and rewrite the `migrate` help to state the journal, resume-on-rerun and abandon; verify `safix migrate --help` in a CLI test.
- [x] 4.2 Update the `secret-migration` scenario text in the CHANGELOG entry for this change; verify `openspec validate --strict` passes. (The changelog entry itself is the orchestrator's to write from this change's report; the spec delta and design were amended for the publish-order correction task 5.2 forced.)

## 5. Verification

- [x] 5.1 Run the `migrate.rs` suite and the full workspace tests; verify all pass and clippy is clean.
- [x] 5.2 Run the real-tool migration probe from the previous change against a plan with three entries, kill the process after the first publish, rerun, and verify the receipt exists, the journal is gone and every output decrypts to its source.
