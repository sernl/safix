//! The vault-first, two-commit sequence group 6 adds, driven end to end
//! through `enroll` — the one operation that genuinely writes at both roots
//! (a re-wrapped governed file at the vault root, the recovery-recipient
//! scaffold at the declaration root) — with `tests/support/git-shim.rs`
//! standing in for `git` to refuse the declaration-root commit on purpose.
//!
//! `adduser` and `group` write nothing at the vault root (their `.sops.yaml`
//! stays at the declaration root per design V10), so a half-landed state is
//! not reachable through either of them; `git.rs`'s own unit tests hold the
//! underlying `commit_two_roots` logic (tasks 6.7-6.9) directly, and this
//! file is the one integration-level check that `enroll`'s call site is
//! wired to it correctly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::Fixture;

const SERIAL: &str = "12345678";
const CARD: &str = "age1yubikey1qfixture000000000000000000000000000000000000000000000000";

/// The environment one enrollment run needs, with the card's own switches and
/// the vault named.
fn enroll_env(fixture: &Fixture, vault: &std::path::Path) -> Vec<(String, String)> {
    let mut environment = fixture.card_env();
    environment.push(("SAFIX_CARD_STUB_SERIALS".to_owned(), SERIAL.to_owned()));
    environment.push(("SAFIX_CARD_STUB_RECIPIENT".to_owned(), CARD.to_owned()));
    environment.push((
        "SAFIX_VAULT_ROOT".to_owned(),
        vault.to_str().expect("a utf-8 path").to_owned(),
    ));
    environment
}

fn as_pairs(environment: &[(String, String)]) -> Vec<(&str, &str)> {
    environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// Task 6.7's drill, driven through `enroll`: the vault-root commit (the
/// re-wrapped governed file) lands, the declaration-root commit (the
/// recovery-recipient scaffold) is forced to fail by the git shim, and the
/// run reports the half-landed state naming the vault commit and the paths
/// still pending. Task 6.8's drill follows in the same test: a retry with the
/// shim no longer refusing makes no second vault commit and the declaration
/// commit lands carrying the `Safix-Vault:` trailer.
#[test]
fn a_forced_declaration_commit_failure_reports_the_half_landed_state_and_a_retry_completes_it() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(harness::ALICE_FILE, &[&fixture.alice.clone()]);

    // A governed file for `fix`'s re-wrap (inside `enroll`'s ceremony) to have
    // something to write at the vault root — an enrollment over a fleet
    // holding nothing yet re-wraps nothing, and `written_vault` would be
    // empty, which is 6.9's single-root case rather than this one.
    fixture
        .run_env(
            &["set", "alice", "mail-password"],
            Some("before-the-card"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("seeding a governed file for the ceremony to re-wrap");

    // Grant a second, plain recipient in the vault-mode creation rule, ahead
    // of the ceremony: the fixture's stubbed nix half answers a fixed rules
    // text rather than recomputing one from the declaration edit `enroll`
    // makes, so the rule has to already grant a wider audience for `fix`'s
    // re-wrap — inside the ceremony — to be a genuine content change rather
    // than a no-op `sops updatekeys` finds nothing to add. A plain X25519
    // recipient rather than the card's own `age1yubikey1…` one: a real
    // `sops updatekeys` wrapping to a yubikey-shaped recipient runs the age
    // plugin, which this suite's fixture recipient is not a key for — see
    // `enrollment.rs`'s own note on why no file here carries the card's
    // stanza in a creation rule.
    let widened = fixture.new_recipient();
    fixture.set_vault_rules(harness::ALICE_FILE, &[&fixture.alice.clone(), &widened]);
    let vault_head_before = fixture.vault_git(&["rev-parse", "--short", "HEAD"]);
    let declaration_head_before = fixture.head();

    let mut environment = enroll_env(&fixture, &vault);
    environment.push(("SAFIX_GIT".to_owned(), harness::git_shim().to_owned()));
    environment.push(("SAFIX_SHIM_GIT".to_owned(), harness::real_git()));
    environment.push((
        "SAFIX_GIT_SHIM_REFUSE_ROOT".to_owned(),
        fixture.repo.to_str().expect("a utf-8 path").to_owned(),
    ));
    let extra = as_pairs(&environment);

    let run = fixture.run_on_terminal(&["enroll", "alice", "--no-store-pin"], "", &extra);
    run.says("the vault committed");
    run.says("Still staged at the declaration root");
    run.says("re-running the same");

    let vault_head_after_first = fixture.vault_git(&["rev-parse", "--short", "HEAD"]);
    assert_ne!(
        vault_head_before, vault_head_after_first,
        "the vault-root commit landed despite the declaration-root refusal"
    );
    assert_eq!(
        fixture.head(),
        declaration_head_before,
        "the declaration-root commit did not land"
    );

    // The retry: the same card, the same environment, but no shim — the
    // vault content the retry stages is unchanged, so no second vault commit
    // is made, and the declaration-root commit that never landed proceeds.
    let retry_environment = enroll_env(&fixture, &vault);
    let extra = as_pairs(&retry_environment);
    fixture.run_on_terminal(&["enroll", "alice", "--no-store-pin"], "", &extra);

    let vault_head_after_retry = fixture.vault_git(&["rev-parse", "--short", "HEAD"]);
    assert_eq!(
        vault_head_after_first, vault_head_after_retry,
        "the retry made no second vault-root commit"
    );
    assert_ne!(
        fixture.head(),
        declaration_head_before,
        "the declaration-root commit landed on the retry"
    );
    let message = fixture.message("HEAD");
    assert!(
        message.contains(&format!("Safix-Vault: {vault_head_after_retry}")),
        "the declaration commit carries the trailer naming the vault commit: {message}"
    );
}

/// Task 6.9's single-root case, driven through `adduser`: a vault is
/// declared, but `adduser` writes nothing at the vault root — `.sops.yaml`
/// stays at the declaration root per design V10 — so exactly one commit
/// lands, at the declaration root, and it carries no `Safix-Vault:` trailer.
#[test]
fn adduser_with_a_vault_declared_still_commits_once_with_no_trailer() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let vault = fixture.declare_vault();
    let vault_empty_before = std::fs::read_dir(&vault)
        .expect("the fresh vault directory can be listed")
        .filter(|entry| entry.as_ref().expect("a directory entry").file_name() != ".git")
        .count();
    let recipient = fixture.new_recipient();

    fixture
        .run_env(
            &["adduser", "carol", &recipient, "--yes"],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("onboarding with a vault declared but nothing to re-wrap");

    let vault_empty_after = std::fs::read_dir(&vault)
        .expect("the vault directory can be listed")
        .filter(|entry| entry.as_ref().expect("a directory entry").file_name() != ".git")
        .count();
    assert_eq!(
        vault_empty_before, vault_empty_after,
        "adduser writes nothing at the vault root, so its tree is unchanged"
    );
    let message = fixture.message("HEAD");
    assert!(
        !message.contains("Safix-Vault:"),
        "a single-root commit carries no trailer: {message}"
    );
}
