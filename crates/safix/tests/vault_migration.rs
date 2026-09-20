//! The migration mechanism design V13's dated note settles on: `check`'s
//! `Finding::VaultRelocationPending` plus `fix`'s relocate phase, task group
//! 11 (and task 5.8's migration-write half).
//!
//! Every fixture here starts from a populated *readable* layout — the state
//! a consumer declaring a vault for the first time on an existing repository
//! is in — and drives `safix fix` (forward) or `safix fix --vault-rollback`
//! (backward) against it with a vault declared. The opaque names below are
//! `opaqueOf`'s real output, recomputed rather than hand-written: each is
//! `sha256("<namingKey>|<tag>|<root-relative readable path>")` over
//! `modules/flake/checks/vault.nix`'s fixture naming key
//! `fc84b416cd03fedc2c02116b068d75c543d327361f19e3d18d9e8aa3de0f4ad2`, with
//! the five tags `secrets`, `key`, `public`, `state` and `stamps`. The
//! stubbed `nix` computes none of them — it hands them over as data,
//! exactly as the resolver does — but they are the resolver's real bytes,
//! so a change to the hash input this suite's constants did not follow
//! shows up as a mismatch rather than passing silently.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::{Fixture, real_sops, shim};
use serde_json::json;

const LOGICAL_PRIVATE_FILE: &str = "secrets/safix/users/alice/secrets.yaml";
/// `secrets|users/alice/secrets.yaml`.
const OPAQUE_PRIVATE_FILE: &str =
    "secrets/49de471561b63dceeb0edc0b29d324f49d1ec33bd7f21a2f312b47e8836248ea.yaml";
/// `key|users/alice/secrets.yaml#api-token`.
const OPAQUE_PRIVATE_KEY: &str = "6aec486973c1530ed15e672a3e69f95d5030dbd2f808a3f43bebd238331d5fd1";
/// `state|alice/api-token` — the record's hash input was already relative
/// to the generator-record root, so this one name survives the break.
const OPAQUE_PRIVATE_RECORD: &str =
    "state/bf23927f1bb02936d41fc8f9730f5e98431724e6ff6bfe7b0f1da168d71450b6";
const LOGICAL_PRIVATE_RECORD: &str = "state/safix/definitions/alice/api-token";
/// `stamps|alice/api-token.stamps`.
const OPAQUE_PRIVATE_STAMP: &str =
    "state/2e54ed1efaa901181510f06d0db9af37058d852ae0ccd08b5b01ca6e42c5ec2d";
const LOGICAL_PRIVATE_STAMP: &str = "state/safix/definitions/alice/api-token.stamps";

const LOGICAL_SHARED_FILE: &str = "secrets/safix/shared/alice,bob/secrets.yaml";
/// `secrets|shared/alice,bob/secrets.yaml`.
const OPAQUE_SHARED_FILE: &str =
    "secrets/873b6adfa41ee669e5896f0fe90bee8c5738b31b8491b39ade5c690a02741b26.yaml";
/// `key|shared/alice,bob/secrets.yaml#fleet-token`.
const OPAQUE_SHARED_KEY: &str = "921c7ad8e1fe08132fafe21cb993c15ed5b4b1654f251063976a9c989ba7b2c4";
/// `state|shared/alice,bob/fleet-token`.
const OPAQUE_SHARED_RECORD: &str =
    "state/63ab6079cff5facdfe01730d47c6dd0177ff95e86f831e6ee4db7c37ed9a5437";
const LOGICAL_SHARED_RECORD: &str = "state/safix/definitions/shared/alice,bob/fleet-token";
/// `stamps|shared/alice,bob/fleet-token.stamps`.
const OPAQUE_SHARED_STAMP: &str =
    "state/fb678949eebf737dd873665255ff8054d5197b366b7172ab7289e597eb10c19b";
const LOGICAL_SHARED_STAMP: &str = "state/safix/definitions/shared/alice,bob/fleet-token.stamps";

const LOGICAL_PUBLIC: &str = "public/safix/users/alice/host-key/value";
/// `public|users/alice/host-key/value`.
const OPAQUE_PUBLIC: &str =
    "public/662de5cdc16fc6e9d1f0809b5b6a22ce74aee202ef163267556904884892b81f";
/// `state|alice/host-key`. The public output's own record: every placement
/// carries one, whether its value is encrypted or not.
const OPAQUE_PUBLIC_RECORD: &str =
    "state/676dbf71c0c3280eeb04d631230df1cac6eed5d1ee4f00b83df5c0019d96e064";
const LOGICAL_PUBLIC_RECORD: &str = "state/safix/definitions/alice/host-key";
/// `stamps|alice/host-key.stamps`.
const OPAQUE_PUBLIC_STAMP: &str =
    "state/ad498ea92592d2936ab0d85d2a36896c70cf48d48dad4120e6f9356e41b5527b";
const LOGICAL_PUBLIC_STAMP: &str = "state/safix/definitions/alice/host-key.stamps";

/// The pre-change names, hashed from the *root-prefixed* readable path the
/// resolver used to feed `opaqueOf`, for the one-time-break drill below.
/// The three records are absent from this list on purpose: their input was
/// already root-relative, so they did not move.
const PREVIOUS_PRIVATE_FILE: &str =
    "secrets/374e7f916175346f12a5e181a3d58b6ebfd729cde1330a57ea0bf6b80a482254.yaml";
const PREVIOUS_PRIVATE_KEY: &str =
    "2c09b1be9e41eb6c358992919e270e95ca42ba83a420e7b40f336ac3e61f00d9";
const PREVIOUS_SHARED_FILE: &str =
    "secrets/d9384552d4cca82cbda1f5ba0397fb3d890fb0efaa8f78d9144a09ff945b10c2.yaml";
const PREVIOUS_SHARED_KEY: &str =
    "42e6df0ee468da93ec5a30562fda302e56f643a90568f4c1f7749fa842e8c0a1";
const PREVIOUS_PUBLIC: &str =
    "public/07e39eb89c181e396c52e61a48c5cde2b496f26cf40570f2d2e1ed868698e146";

const PRIVATE_VALUE: &str = "CANARY-private-api-token";
const SHARED_VALUE: &str = "CANARY-shared-fleet-token";
const PUBLIC_VALUE: &str = "CANARY-public-host-key";
const PRIVATE_RECORD_TEXT: &str =
    "safix-definition-v2 1111111111111111111111111111111111111111111111111111111111111111\n";
const SHARED_RECORD_TEXT: &str =
    "safix-definition-v2 2222222222222222222222222222222222222222222222222222222222222222\n";
/// Two distinct stamp lines, so a relocation that moved one record's bytes
/// into the other's destination is a mismatch rather than a coincidence.
const PRIVATE_STAMP_TEXT: &str = "v1 created=1700000001 updated=1700000002\n";
const SHARED_STAMP_TEXT: &str = "v1 created=1700000003 updated=1700000004\n";

/// One private entry, one shared entry (both carriers), and one public
/// output, declared with both their opaque and readable forms — the shape
/// `flake.safix.lib.placements` carries once a vault is declared, over a
/// fleet whose ciphertext, plaintext outputs, definition records and stamp
/// records still sit at the declaration root because nothing has relocated
/// them yet.
fn declare_vault_placements(fixture: &mut Fixture) {
    let private = json!({
        "file": OPAQUE_PRIVATE_FILE, "key": OPAQUE_PRIVATE_KEY, "origin": "private",
        "owner": "alice", "shared": false, "generator": null, "public": null,
        "definitionRecord": OPAQUE_PRIVATE_RECORD,
        "logicalFile": LOGICAL_PRIVATE_FILE, "logicalKey": "api-token", "logicalPublic": null,
        "logicalRecord": LOGICAL_PRIVATE_RECORD,
        "stampRecord": OPAQUE_PRIVATE_STAMP,
        "logicalStamp": LOGICAL_PRIVATE_STAMP,
    });
    fixture.seed_vault_placement("alice", "api-token", private);

    let public = json!({
        "file": OPAQUE_PRIVATE_FILE, "key": "host-key-unused", "origin": "private",
        "owner": "alice", "shared": false, "generator": null,
        "public": OPAQUE_PUBLIC,
        // A public output carries a record of its own, opaque like every
        // other vault-rooted name; what makes this placement public is
        // `public`, not the absence of a record.
        "definitionRecord": OPAQUE_PUBLIC_RECORD,
        "logicalFile": LOGICAL_PRIVATE_FILE, "logicalKey": "host-key-unused",
        "logicalPublic": LOGICAL_PUBLIC,
        "logicalRecord": LOGICAL_PUBLIC_RECORD,
        "stampRecord": OPAQUE_PUBLIC_STAMP,
        "logicalStamp": LOGICAL_PUBLIC_STAMP,
    });
    fixture.seed_vault_placement("alice", "host-key", public);

    for owner in ["alice", "bob"] {
        let shared = json!({
            "file": OPAQUE_SHARED_FILE, "key": OPAQUE_SHARED_KEY, "origin": "carries",
            "owner": owner, "shared": true, "generator": null, "public": null,
            "definitionRecord": OPAQUE_SHARED_RECORD,
            "logicalFile": LOGICAL_SHARED_FILE, "logicalKey": "fleet-token",
            "logicalPublic": null,
            "logicalRecord": LOGICAL_SHARED_RECORD,
            "stampRecord": OPAQUE_SHARED_STAMP,
            "logicalStamp": LOGICAL_SHARED_STAMP,
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
    fixture.set_audience(OPAQUE_PRIVATE_FILE, &["alice"], &[&alice]);
    fixture.set_audience(OPAQUE_SHARED_FILE, &["alice", "bob"], &[&alice, &bob]);
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
    fixture.write(LOGICAL_PRIVATE_STAMP, PRIVATE_STAMP_TEXT);
    fixture.write(LOGICAL_SHARED_STAMP, SHARED_STAMP_TEXT);

    fixture.set_vault_rules_many(&[
        (OPAQUE_PRIVATE_FILE, &[alice.as_str()]),
        (OPAQUE_SHARED_FILE, &[alice.as_str(), bob.as_str()]),
    ]);

    (fixture, vault)
}

/// Task 11.3: `fix` moves every readable-layout leaf into its opaque vault
/// destination — every secret decrypts to the same plaintext, every public
/// output, definition record and stamp record copies byte for byte, every
/// physical name is the opaque one the placements declared, and the
/// readable-layout source is gone. Folds task 11.6's drill: `check` reports
/// the pending relocations and the missing `.gitignore` entry beforehand,
/// and reports neither afterward.
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
    before.says(LOGICAL_PRIVATE_STAMP);
    before.says(LOGICAL_SHARED_STAMP);
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
    assert_eq!(
        fixture.vault_read(OPAQUE_PRIVATE_STAMP),
        PRIVATE_STAMP_TEXT,
        "the private stamp's dates did not survive the move"
    );
    assert_eq!(
        fixture.vault_read(OPAQUE_SHARED_STAMP),
        SHARED_STAMP_TEXT,
        "the shared stamp's dates did not survive the move"
    );

    // Every physical name matches the opaque form the placements declared,
    // and the readable-layout source is gone.
    for opaque in [
        OPAQUE_PRIVATE_FILE,
        OPAQUE_SHARED_FILE,
        OPAQUE_PUBLIC,
        OPAQUE_PRIVATE_RECORD,
        OPAQUE_SHARED_RECORD,
        OPAQUE_PRIVATE_STAMP,
        OPAQUE_SHARED_STAMP,
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
        LOGICAL_PRIVATE_STAMP,
        LOGICAL_SHARED_STAMP,
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
/// achievable claim; the plaintext leaves (the public output, both
/// definition records and both stamps) are compared byte for byte, since a
/// rollback copies them rather than re-encrypting anything.
#[test]
fn a_vault_rollback_restores_the_readable_layout() {
    let (fixture, vault) = populated_readable_fixture();
    let extra = [("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))];

    fixture
        .run_env(&["fix", "--yes"], None, &extra)
        .expect_success("migrating the readable layout into the vault");
    fixture.vault_git(&["add", "--all"]);
    fixture.vault_git(&[
        "commit",
        "-qm",
        "Keep the verified migration before rollback",
    ]);

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
    assert_eq!(
        fixture.stamps_of(LOGICAL_PRIVATE_STAMP),
        Some((1_700_000_001, 1_700_000_002)),
        "the private stamp's dates did not survive the round trip"
    );
    assert_eq!(
        fixture.stamps_of(LOGICAL_SHARED_STAMP),
        Some((1_700_000_003, 1_700_000_004)),
        "the shared stamp's dates did not survive the round trip"
    );

    for opaque in [
        OPAQUE_PRIVATE_FILE,
        OPAQUE_SHARED_FILE,
        OPAQUE_PUBLIC,
        OPAQUE_PRIVATE_RECORD,
        OPAQUE_SHARED_RECORD,
        OPAQUE_PRIVATE_STAMP,
        OPAQUE_SHARED_STAMP,
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
        "definitionRecord": OPAQUE_PRIVATE_RECORD,
        "logicalFile": LOGICAL_PRIVATE_FILE, "logicalKey": "api-token", "logicalPublic": null,
        "logicalRecord": LOGICAL_PRIVATE_RECORD,
        "stampRecord": OPAQUE_PRIVATE_STAMP,
        "logicalStamp": LOGICAL_PRIVATE_STAMP,
    });
    fixture.seed_vault_placement("alice", "api-token", private);
    let alice = fixture.alice.clone();
    fixture.encrypt_to(
        LOGICAL_PRIVATE_FILE,
        &[&alice],
        &format!("api-token: {PRIVATE_VALUE}\n"),
    );
    fixture.set_vault_rules(OPAQUE_PRIVATE_FILE, &[&alice]);
    fixture.set_audience(OPAQUE_PRIVATE_FILE, &["alice"], &[&alice]);

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

/// Seed the vault with a fleet already relocated under the names given, and
/// leave the declaration root empty — the state a consumer who ran
/// `safix fix` under some derivation of `opaqueOf` is in. The two records
/// are seeded at their post-change names in both halves below, because the
/// record's hash input was already relative to the generator-record root
/// and so did not move.
fn vault_seeded_at(
    fixture: &mut Fixture,
    secret_file: &str,
    secret_key: &str,
    shared_file: &str,
    shared_key: &str,
    public: &str,
) {
    let (alice, bob) = (fixture.alice.clone(), fixture.bob.clone());
    fixture.encrypt_to_vault(
        secret_file,
        &[&alice],
        &format!("{secret_key}: {PRIVATE_VALUE}\n"),
    );
    fixture.encrypt_to_vault(
        shared_file,
        &[&alice, &bob],
        &format!("{shared_key}: {SHARED_VALUE}\n"),
    );
    vault_write(fixture, public, PUBLIC_VALUE);
    vault_write(fixture, OPAQUE_PRIVATE_RECORD, PRIVATE_RECORD_TEXT);
    vault_write(fixture, OPAQUE_SHARED_RECORD, SHARED_RECORD_TEXT);
    vault_write(fixture, ".gitignore", "/.sops-vault-rules.yaml\n");
}

/// Write a plaintext leaf straight into the vault repository.
fn vault_write(fixture: &Fixture, relative: &str, contents: &str) {
    let path = fixture.vault_root().join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

/// A vault fixture whose documents sit at the names given, with the
/// post-change placements declared over them.
fn stale_vault_fixture(
    secret_file: &str,
    secret_key: &str,
    shared_file: &str,
    shared_key: &str,
    public: &str,
) -> (Fixture, std::path::PathBuf) {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let vault = fixture.declare_vault();
    declare_vault_placements(&mut fixture);
    vault_seeded_at(
        &mut fixture,
        secret_file,
        secret_key,
        shared_file,
        shared_key,
        public,
    );
    (fixture, vault)
}

/// Tasks 8.3 and 8.7: the one-time opaque-name break, drilled.
///
/// The break is observable, which is the half of task 8.7 that holds. A
/// vault seeded under the pre-change, root-prefixed names reports every
/// ciphertext document as holding no value, because each placement now
/// names a post-change destination that is empty; the same vault seeded
/// under the post-change names reports none of them. A `check` silent in
/// both cases would mean the opaque names are not being compared at all.
///
/// Two things this drill establishes that design S5 does not say, recorded
/// here rather than asserted away:
///
/// The break is *partial*, not total. `opaqueOf`'s `state` input was
/// already relative to the generator-record root — `alice/api-token`, not
/// `state/safix/definitions/alice/api-token` — so every definition record
/// keeps its name across the change while every document, in-document key
/// and public leaf loses its. That asymmetry is what the pre-change
/// constants above record, and it is why the rollback below brings the two
/// records back and nothing else.
///
/// And the break does not relocate itself. S5's migration plan says
/// `safix check` reports every affected document as *pending relocation*
/// and `safix fix` relocates; it does not, and this machinery cannot.
/// `relocation`'s candidates are `(opaque, readable)` pairs and
/// `named_move` only ever moves between the declaration root and the vault
/// root, so an old-opaque-to-new-opaque move is not a move this code
/// expresses: `check` finds nothing at the declaration root to queue, and
/// `fix --vault-rollback` finds only the records, whose names did not
/// move. The operational consequence is that the rollback has to be run
/// *before* the input carrying this change is taken. The forward half is
/// [`a_populated_readable_fixture_migrates_into_a_vault`], whose
/// destinations are now the root-relative names.
#[test]
fn the_opaque_name_break_is_visible_but_does_not_relocate_itself() {
    let (stale, stale_vault) = stale_vault_fixture(
        PREVIOUS_PRIVATE_FILE,
        PREVIOUS_PRIVATE_KEY,
        PREVIOUS_SHARED_FILE,
        PREVIOUS_SHARED_KEY,
        PREVIOUS_PUBLIC,
    );
    let stale_env = [(
        "SAFIX_VAULT_ROOT",
        stale_vault.to_str().expect("a utf-8 path"),
    )];

    let before = stale.run_env(&["check"], None, &stale_env);
    assert_eq!(before.code, Some(1), "the break went unreported");
    // The two entries whose value lives in a ciphertext document: each is
    // reported against the post-change name its placement now carries,
    // which is empty. `host-key` is not part of the signal — its
    // placement's `file` is the private document and `check`'s no-value
    // pass reports it in both halves, unchanged by the break.
    for entry in ["declares 'api-token'", "declares 'fleet-token'"] {
        before.says(entry);
    }
    before.says(&format!("{OPAQUE_SHARED_FILE} holds no value for it"));
    // Nothing is pending: the readable layout is not at the declaration
    // root, so there is nothing for the relocation to queue.
    before.silent_about("has not yet moved it into the vault");

    // A rollback from here recovers exactly the names the break left
    // alone. The two records come back, because their hash input was
    // already root-relative; the documents and the public leaf do not,
    // because the names the rollback looks for are the post-change ones
    // and the vault holds the pre-change ones.
    stale
        .run_env(&["fix", "--yes", "--vault-rollback"], None, &stale_env)
        .expect_success("a rollback over a vault holding only pre-change names");
    for recovered in [LOGICAL_PRIVATE_RECORD, LOGICAL_SHARED_RECORD] {
        assert!(
            stale.exists(recovered),
            "{recovered} did not come back, so its name moved after all"
        );
    }
    for stranded in [LOGICAL_PRIVATE_FILE, LOGICAL_SHARED_FILE, LOGICAL_PUBLIC] {
        assert!(
            !stale.exists(stranded),
            "{stranded} came back from a vault holding only pre-change names"
        );
    }

    // The other half of the drill: the same fleet under the post-change
    // names reports neither finding.
    let (current, current_vault) = stale_vault_fixture(
        OPAQUE_PRIVATE_FILE,
        OPAQUE_PRIVATE_KEY,
        OPAQUE_SHARED_FILE,
        OPAQUE_SHARED_KEY,
        OPAQUE_PUBLIC,
    );
    let after = current.run_env(
        &["check"],
        None,
        &[(
            "SAFIX_VAULT_ROOT",
            current_vault.to_str().expect("a utf-8 path"),
        )],
    );
    for entry in ["declares 'api-token'", "declares 'fleet-token'"] {
        after.silent_about(entry);
    }
    after.silent_about(&format!("{OPAQUE_SHARED_FILE} holds no value for it"));
    after.silent_about("has not yet moved it into the vault");
}
