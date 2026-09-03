//! The vault's disposable creation rules (design V10, task group 5): a
//! scratch rendering that reaches `encrypt` and `updatekeys` through
//! `--config`, never committed, never present outside the run that needed it.
//!
//! `.sops.yaml`'s own write and read sites stay at the declaration root
//! unconditionally (task 5.1) — every test here that declares a vault also
//! confirms the committed policy is untouched by it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use std::io::Write as _;

use harness::{ALICE_FILE, Fixture, real_sops, shim};

/// `.sops.yaml`'s write and read sites never read `vault_root`, with or
/// without a vault declared.
///
/// `adduser` is what writes and regenerates it; a vault declared alongside it
/// changes nothing about where the file lands, because [`fix::write_policy`]
/// and [`check::policy`] are unedited by design.
#[test]
fn the_committed_policy_stays_at_the_declaration_root_with_a_vault_declared() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();

    fixture
        .run_env(
            &["adduser", "--yes", "dave", &fixture.new_recipient()],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("declaring a person with a vault declared");

    assert!(
        fixture.exists(".sops.yaml"),
        "the committed policy did not land at the declaration root"
    );
    assert!(
        !fixture.vault_exists(".sops.yaml"),
        "the committed policy leaked into the vault"
    );
}

/// `encrypt` and `updatekeys` against a vault-rooted document succeed with
/// the scratch config, and the vault fixture carries no committed
/// `.sops.yaml` of its own.
#[test]
fn set_creates_a_vault_rooted_document_through_the_scratch_config() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(ALICE_FILE, &[&fixture.alice.clone()]);

    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-vault-created\n"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("creating a vault-rooted document through the scratch rules");

    assert!(
        fixture.vault_exists(ALICE_FILE),
        "the document was not created inside the vault"
    );
    assert!(
        !fixture.vault_exists(".sops.yaml"),
        "the vault fixture carries a committed policy of its own"
    );
    assert_eq!(
        fixture.vault_value(ALICE_FILE, "api-token"),
        "CANARY-vault-created",
        "the value does not round-trip"
    );
}

/// The scratch rules file does not exist after the run: on normal return, on
/// a forced sops failure, and on a signal mid-call.
#[test]
fn the_scratch_rules_file_never_survives_a_run() {
    // Normal return.
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(ALICE_FILE, &[&fixture.alice.clone()]);
    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-normal\n"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("a normal run");
    assert!(
        !fixture.vault_exists(".sops-vault-rules.yaml"),
        "the scratch rules file survived a normal return"
    );

    // A forced sops failure: no rule grants alice's own key, so `encrypt`
    // itself refuses.
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(
        "secrets/safix/users/bob/other.yaml",
        &[&fixture.alice.clone()],
    );
    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-refused\n"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_refusal("a rule that does not cover the document being created");
    assert!(
        !fixture.vault_exists(".sops-vault-rules.yaml"),
        "the scratch rules file survived a forced sops failure"
    );

    // A signal mid-call: interrupted while `sops set` holds the candidate the
    // earlier `sops encrypt` created through the scratch config.
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(ALICE_FILE, &[&fixture.alice.clone()]);
    let sops = real_sops();
    let run = fixture.run_env(
        &["set", "alice", "api-token"],
        Some("CANARY-interrupted\nCANARY-interrupted\n"),
        &[
            ("SAFIX_VAULT_ROOT", vault.to_str().unwrap()),
            ("SAFIX_SOPS", shim()),
            ("SAFIX_SHIM_ROLE", "interrupt"),
            ("SAFIX_SHIM_SOPS", &sops),
            ("SAFIX_SHIM_HOLD", "set"),
        ],
    );
    assert_eq!(run.code, Some(130), "an interrupted run exits 130");
    assert!(
        !fixture.vault_exists(".sops-vault-rules.yaml"),
        "the scratch rules file survived a signal mid-call"
    );
}

/// `decrypt` and `set` against an already-encrypted vault-rooted document
/// succeed with no scratch rules file ever written — no rule is declared for
/// the fixture at all, which is what proves neither call needed one.
#[test]
fn setting_an_existing_vault_rooted_document_needs_no_scratch_rules() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.encrypt_to_vault(ALICE_FILE, &[&fixture.alice.clone()], "api-token: before\n");
    fixture.vault_git(&["config", "user.email", "selftest@example.com"]);
    fixture.vault_git(&["config", "user.name", "selftest"]);
    fixture.vault_git(&["add", "--", ALICE_FILE]);
    fixture.vault_git(&[
        "commit",
        "-q",
        "-m",
        "fixture: seed a vault-rooted document",
    ]);

    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-already-encrypted\n"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("setting a value into an already-encrypted vault-rooted document");

    assert!(
        !fixture.vault_exists(".sops-vault-rules.yaml"),
        "a run against an already-encrypted document left a scratch rules file"
    );
    assert_eq!(
        fixture.vault_value(ALICE_FILE, "api-token"),
        "CANARY-already-encrypted",
        "the value does not round-trip"
    );
}

/// Severity drill for task 5.12: creation rules pointed at the wrong
/// directory turn the create into a no-matching-creation-rules refusal,
/// which is the evidence sops's directory-relative `path_regex` matching is
/// exercised rather than assumed.
///
/// [`Fixture::encrypt_to`] is the fixture's own straight-to-recipients
/// helper, standing in for a config file that sits at the declaration root
/// instead of the vault root: `create_empty_document`'s `--config` is always
/// the path [`safix_core::workspace::Workspace::stage_vault_rules`] writes
/// inside the vault, and this drill is what proves the vault-rootedness of
/// that path is load-bearing rather than incidental, by driving the same
/// rule text through the declaration root instead.
#[test]
fn rules_staged_outside_the_vault_root_do_not_match() {
    let mut fixture = Fixture::new();
    let _vault = fixture.declare_vault();

    let rules = fixture.work.join("misplaced-rules.yaml");
    std::fs::write(
        &rules,
        format!(
            "creation_rules:\n  - path_regex: ^{ALICE_FILE}$\n    key_groups:\n      - age:\n          - {}\n",
            fixture.alice
        ),
    )
    .unwrap();

    let mut command = std::process::Command::new("sops");
    command
        .arg("--config")
        .arg(&rules)
        .arg("encrypt")
        .arg("--filename-override")
        .arg(ALICE_FILE)
        .arg("--input-type")
        .arg("json")
        .arg("--output-type")
        .arg("yaml")
        .arg("/dev/stdin")
        .current_dir(&fixture.repo)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("could not run sops");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"api-token\":\"\"}")
        .unwrap();
    let finished = child.wait_with_output().expect("sops did not finish");

    assert!(
        !finished.status.success(),
        "a config rooted outside the vault matched the document anyway"
    );
    assert!(
        String::from_utf8_lossy(&finished.stderr).contains("no matching creation rules found"),
        "the failure is not the no-matching-creation-rules one:\n{}",
        String::from_utf8_lossy(&finished.stderr)
    );
}

/// Task 5.8: `check` reports a finding when a vault's `.gitignore` does not
/// cover the scratch rules file, and reports nothing once it does.
#[test]
fn check_reports_a_vault_with_no_gitignore_entry_for_the_scratch_rules() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();

    let uncovered = fixture.run_env(
        &["check"],
        None,
        &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
    );
    uncovered.says(".sops-vault-rules.yaml");
    uncovered.says("does not cover");

    std::fs::write(vault.join(".gitignore"), "/.sops-vault-rules.yaml\n").unwrap();
    let covered = fixture.run_env(
        &["check"],
        None,
        &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
    );
    covered.silent_about(".sops-vault-rules.yaml");
}
