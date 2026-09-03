//! The migration mechanism design V13's dated note settles on: `check`'s
//! `Finding::VaultRelocationPending` plus `fix`'s relocate phase, task group
//! 11 (and task 5.8's migration-write half).
//!
//! Every fixture here starts from a populated *readable* layout — the state
//! a consumer declaring a vault for the first time on an existing repository
//! is in — and drives `safix fix` (forward) or `safix fix --vault-rollback`
//! (backward) against it with a vault declared. The opaque names below are
//! not `opaqueOf`'s real output — the fixture's stubbed `nix` never computes
//! one — but fixed hex-shaped literals standing in for it, exactly as
//! `vault_opaque_names.rs` already does.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::{Fixture, real_sops, shim};
use serde_json::json;

const LOGICAL_PRIVATE_FILE: &str = "secrets/safix/users/alice/secrets.yaml";
const OPAQUE_PRIVATE_FILE: &str =
    "secrets/1111111111111111111111111111111111111111111111111111111111111111.yaml";
const OPAQUE_PRIVATE_KEY: &str = "opaque-key-api-token";
const OPAQUE_PRIVATE_RECORD: &str =
    "state/1111111111111111111111111111111111111111111111111111111111111112";
const LOGICAL_PRIVATE_RECORD: &str = "state/safix/definitions/alice/api-token";

const LOGICAL_SHARED_FILE: &str = "secrets/safix/shared/alice,bob/secrets.yaml";
const OPAQUE_SHARED_FILE: &str =
    "secrets/2222222222222222222222222222222222222222222222222222222222222222.yaml";
const OPAQUE_SHARED_KEY: &str = "opaque-key-fleet-token";
const OPAQUE_SHARED_RECORD: &str =
    "state/2222222222222222222222222222222222222222222222222222222222222223";
const LOGICAL_SHARED_RECORD: &str = "state/safix/definitions/shared/alice,bob/fleet-token";

const LOGICAL_PUBLIC: &str = "public/safix/users/alice/host-key/value";
const OPAQUE_PUBLIC: &str =
    "public/3333333333333333333333333333333333333333333333333333333333333333";

const PRIVATE_VALUE: &str = "CANARY-private-api-token";
const SHARED_VALUE: &str = "CANARY-shared-fleet-token";
const PUBLIC_VALUE: &str = "CANARY-public-host-key";
const PRIVATE_RECORD_TEXT: &str =
    "safix-definition-v2 1111111111111111111111111111111111111111111111111111111111111111\n";
const SHARED_RECORD_TEXT: &str =
    "safix-definition-v2 2222222222222222222222222222222222222222222222222222222222222222\n";

/// One private entry, one shared entry (both carriers), and one public
/// output, declared with both their opaque and readable forms — the shape
/// `flake.safix.lib.placements` carries once a vault is declared, over a
/// fleet whose ciphertext, plaintext outputs and definition records still
/// sit at the declaration root because nothing has relocated them yet.
fn declare_vault_placements(fixture: &mut Fixture) {
    let private = json!({
        "file": OPAQUE_PRIVATE_FILE, "key": OPAQUE_PRIVATE_KEY, "origin": "private",
        "owner": "alice", "shared": false, "generator": null, "public": null,
        "definitionRecord": OPAQUE_PRIVATE_RECORD,
        "logicalFile": LOGICAL_PRIVATE_FILE, "logicalKey": "api-token", "logicalPublic": null,
    });
    fixture.seed_vault_placement("alice", "api-token", private);

    let public = json!({
        "file": OPAQUE_PRIVATE_FILE, "key": "host-key-unused", "origin": "private",
        "owner": "alice", "shared": false, "generator": null,
        "public": OPAQUE_PUBLIC,
        "definitionRecord": null,
        "logicalFile": LOGICAL_PRIVATE_FILE, "logicalKey": "host-key-unused",
        "logicalPublic": LOGICAL_PUBLIC,
    });
    fixture.seed_vault_placement("alice", "host-key", public);

    for owner in ["alice", "bob"] {
        let shared = json!({
            "file": OPAQUE_SHARED_FILE, "key": OPAQUE_SHARED_KEY, "origin": "carries",
            "owner": owner, "shared": true, "generator": null, "public": null,
            "definitionRecord": OPAQUE_SHARED_RECORD,
            "logicalFile": LOGICAL_SHARED_FILE, "logicalKey": "fleet-token",
            "logicalPublic": null,
        });
        fixture.seed_vault_placement(owner, "fleet-token", shared);
    }
}

/// A vault declared, its placements carrying both name forms, and every
/// readable-layout leaf actually present at the declaration root — nothing
/// relocated yet.
fn populated_readable_fixture() -> (Fixture, std::path::PathBuf) {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let vault = fixture.declare_vault();
    declare_vault_placements(&mut fixture);

    let (alice, bob) = (fixture.alice.clone(), fixture.bob.clone());
    fixture.encrypt_to(
        LOGICAL_PRIVATE_FILE,
        &[&alice],
        &format!("api-token: {PRIVATE_VALUE}\n"),
    );
    fixture.encrypt_to(
        LOGICAL_SHARED_FILE,
        &[&alice, &bob],
        &format!("fleet-token: {SHARED_VALUE}\n"),
    );
    fixture.write(LOGICAL_PUBLIC, PUBLIC_VALUE);
    fixture.write(LOGICAL_PRIVATE_RECORD, PRIVATE_RECORD_TEXT);
    fixture.write(LOGICAL_SHARED_RECORD, SHARED_RECORD_TEXT);

    fixture.set_vault_rules_many(&[
        (OPAQUE_PRIVATE_FILE, &[alice.as_str()]),
        (OPAQUE_SHARED_FILE, &[alice.as_str(), bob.as_str()]),
    ]);

    (fixture, vault)
}

/// Task 11.3: `fix` moves every readable-layout leaf into its opaque vault
/// destination — every secret decrypts to the same plaintext, every public
/// output and definition record copies byte for byte, every physical name
/// is the opaque one the placements declared, and the readable-layout
/// source is gone. Folds task 11.6's drill: `check` reports the pending
/// relocations and the missing `.gitignore` entry beforehand, and reports
/// neither afterward.
#[test]
fn a_populated_readable_fixture_migrates_into_a_vault() {
    let (fixture, vault) = populated_readable_fixture();
    let extra = [("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))];

    let before = fixture.run_env(&["check"], None, &extra);
    before.says(LOGICAL_PRIVATE_FILE);
    before.says(LOGICAL_SHARED_FILE);
    before.says(LOGICAL_PUBLIC);
    before.says(LOGICAL_PRIVATE_RECORD);
    before.says(LOGICAL_SHARED_RECORD);
    before.says("has not yet moved it into the vault");
    before.says(".sops-vault-rules.yaml");
    before.says("does not cover");

    fixture
        .run_env(&["fix", "--yes"], None, &extra)
        .expect_success("migrating the readable layout into the vault");

    // Every leaf decrypts to, or copies, the same plaintext.
    assert_eq!(
        fixture.vault_value(OPAQUE_PRIVATE_FILE, OPAQUE_PRIVATE_KEY),
        PRIVATE_VALUE,
        "the private secret's value did not survive the move"
    );
    assert_eq!(
        fixture.vault_value(OPAQUE_SHARED_FILE, OPAQUE_SHARED_KEY),
        SHARED_VALUE,
        "the shared secret's value did not survive the move"
    );
    assert_eq!(
        fixture.vault_read(OPAQUE_PUBLIC),
        PUBLIC_VALUE,
        "the public output's bytes did not survive the move"
    );
    assert_eq!(
        fixture.vault_read(OPAQUE_PRIVATE_RECORD),
        PRIVATE_RECORD_TEXT,
        "the private record's bytes did not survive the move"
    );
    assert_eq!(
        fixture.vault_read(OPAQUE_SHARED_RECORD),
        SHARED_RECORD_TEXT,
        "the shared record's bytes did not survive the move"
    );

    // Every physical name matches the opaque form the placements declared,
    // and the readable-layout source is gone.
    for opaque in [
        OPAQUE_PRIVATE_FILE,
        OPAQUE_SHARED_FILE,
        OPAQUE_PUBLIC,
        OPAQUE_PRIVATE_RECORD,
        OPAQUE_SHARED_RECORD,
    ] {
        assert!(
            fixture.vault_exists(opaque),
            "{opaque} does not exist in the vault"
        );
    }
    for logical in [
        LOGICAL_PRIVATE_FILE,
        LOGICAL_SHARED_FILE,
        LOGICAL_PUBLIC,
        LOGICAL_PRIVATE_RECORD,
        LOGICAL_SHARED_RECORD,
    ] {
        assert!(
            !fixture.exists(logical),
            "{logical} still exists at the declaration root"
        );
    }

    // The vault's own `.gitignore` now covers the scratch rules file —
    // task 5.8's migration-write half.
    assert!(
        fixture
            .vault_read(".gitignore")
            .contains(".sops-vault-rules.yaml"),
        "fix did not write the vault's .gitignore entry"
    );

    let after = fixture.run_env(&["check"], None, &extra);
    after.silent_about("has not yet moved it into the vault");
    after.silent_about("does not cover");
}

/// Task 11.4: rolling the migration back with `--vault-rollback` restores
/// the readable layout. Secrets are compared by decrypted value rather than
/// by ciphertext bytes — a fresh encryption of the same plaintext carries a
/// fresh nonce and `lastmodified`, so byte-identical ciphertext is not the
/// achievable claim; the plaintext leaves (the public output and both
/// definition records) are compared byte for byte, since a rollback copies
/// them rather than re-encrypting anything.
#[test]
fn a_vault_rollback_restores_the_readable_layout() {
    let (fixture, vault) = populated_readable_fixture();
    let extra = [("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))];

    fixture
        .run_env(&["fix", "--yes"], None, &extra)
        .expect_success("migrating the readable layout into the vault");

    fixture
        .run_env(&["fix", "--vault-rollback"], None, &extra)
        .expect_success("rolling the migration back");

    assert_eq!(
        fixture.value(LOGICAL_PRIVATE_FILE, "api-token"),
        PRIVATE_VALUE,
        "the private secret's value did not survive the round trip"
    );
    assert_eq!(
        fixture.value(LOGICAL_SHARED_FILE, "fleet-token"),
        SHARED_VALUE,
        "the shared secret's value did not survive the round trip"
    );
    assert_eq!(
        fixture.read(LOGICAL_PUBLIC),
        PUBLIC_VALUE,
        "the public output is not byte-identical after the round trip"
    );
    assert_eq!(
        fixture.read(LOGICAL_PRIVATE_RECORD),
        PRIVATE_RECORD_TEXT,
        "the private record is not byte-identical after the round trip"
    );
    assert_eq!(
        fixture.read(LOGICAL_SHARED_RECORD),
        SHARED_RECORD_TEXT,
        "the shared record is not byte-identical after the round trip"
    );

    for opaque in [
        OPAQUE_PRIVATE_FILE,
        OPAQUE_SHARED_FILE,
        OPAQUE_PUBLIC,
        OPAQUE_PRIVATE_RECORD,
        OPAQUE_SHARED_RECORD,
    ] {
        assert!(
            !fixture.vault_exists(opaque),
            "{opaque} still exists in the vault after the rollback"
        );
    }
}

/// Task 11.5: interrupted mid-document, the destination is left absent and
/// the source untouched, and a re-run completes it — the same
/// `SAFIX_SHIM_HOLD` pattern `vault_scratch_rules.rs`'s
/// `the_scratch_rules_file_never_survives_a_run` uses, held on the final
/// `sops set` call so the candidate document exists but is never renamed
/// into place.
#[test]
fn an_interrupted_relocation_leaves_the_destination_absent_and_a_re_run_completes_it() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let vault = fixture.declare_vault();
    let private = json!({
        "file": OPAQUE_PRIVATE_FILE, "key": OPAQUE_PRIVATE_KEY, "origin": "private",
        "owner": "alice", "shared": false, "generator": null, "public": null,
        "definitionRecord": null,
        "logicalFile": LOGICAL_PRIVATE_FILE, "logicalKey": "api-token", "logicalPublic": null,
    });
    fixture.seed_vault_placement("alice", "api-token", private);
    let alice = fixture.alice.clone();
    fixture.encrypt_to(
        LOGICAL_PRIVATE_FILE,
        &[&alice],
        &format!("api-token: {PRIVATE_VALUE}\n"),
    );
    fixture.set_vault_rules(OPAQUE_PRIVATE_FILE, &[&alice]);

    let sops = real_sops();
    let run = fixture.run_env(
        &["fix", "--yes"],
        None,
        &[
            ("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path")),
            ("SAFIX_SOPS", shim()),
            ("SAFIX_SHIM_ROLE", "interrupt"),
            ("SAFIX_SHIM_SOPS", &sops),
            ("SAFIX_SHIM_HOLD", "set"),
        ],
    );
    assert_eq!(run.code, Some(130), "an interrupted run exits 130");
    assert!(
        !fixture.vault_exists(OPAQUE_PRIVATE_FILE),
        "the destination exists despite the interruption"
    );
    assert!(
        fixture.exists(LOGICAL_PRIVATE_FILE),
        "the readable-layout source was removed despite the interruption"
    );

    fixture
        .run_env(
            &["fix", "--yes"],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("the re-run completing the interrupted relocation");
    assert_eq!(
        fixture.vault_value(OPAQUE_PRIVATE_FILE, OPAQUE_PRIVATE_KEY),
        PRIVATE_VALUE,
        "the re-run did not complete the relocation"
    );
    assert!(
        !fixture.exists(LOGICAL_PRIVATE_FILE),
        "the readable-layout source survived the completed re-run"
    );
}
