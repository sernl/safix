//! What a declared vault must not disturb — design V3's touch points 6, 10
//! and 14 that stay at `declaration_root` regardless of `vault_root` — held
//! rather than assumed (task group 8).
//!
//! Each test drives an actual command over a fixture where the two roots
//! differ, rather than asserting on source text, so a future edit that moved
//! one of these sites to `vault_root` fails here rather than passing
//! silently. `list` and `adduser` are the two verbs reachable without group
//! 6's cross-root commit ordering: neither writes anything at `vault_root`
//! today, so a vault declared alongside them changes nothing about what they
//! do, which is exactly the property under test.
//!
//! Full coverage of touch point 7 (`Git`'s commit driver) and touch point 11
//! (the scratch sweep floor) for an operation whose commit lands at
//! `vault_root` is wave two's to add once group 6 lands `set`'s cross-root
//! commit: today nothing commits at `vault_root` yet for such a test to
//! observe.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::Fixture;

/// Every `nix eval` target is built from `declaration_root` alone.
///
/// `tests/support/nix-stub.rs::eval` refuses an evaluation naming any root
/// but `SAFIX_REPO_ROOT` — see its `expected_root` check — so a run that
/// succeeds with `vault_root` set to a different path than `root` is itself
/// the proof that nothing routed an evaluation through the vault: had
/// `Nix::target` or `Nix::shell` ever closed over `vault_root`, this run
/// would have been refused by the stub rather than have succeeded.
#[test]
fn every_evaluation_targets_the_declaration_root_regardless_of_the_vault_root() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    assert_ne!(
        vault, fixture.repo,
        "the fixture's own vault must differ from its declaration root"
    );

    fixture
        .run_env(
            &["list"],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("list with a vault declared at a different root");
}

/// Onboarding still commits at `declaration_root`, authored per
/// `declaration_root`'s own git configuration, with a vault declared at a
/// different root.
///
/// The vault repository `declare_vault` stands up carries no `user.name` or
/// `user.email` of its own, so a commit that somehow read the vault's
/// identity would fail to resolve one at all rather than read the fixture's
/// name under a different root: this is the sharpest observable difference
/// available without group 6's own commit landing at `vault_root`.
#[test]
fn onboarding_commits_at_the_declaration_root_with_a_vault_declared() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.seed_declarations();
    let recipient = fixture.new_recipient();
    let head = fixture.head();

    fixture
        .run_env(
            &["adduser", "carol", &recipient, "--yes"],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("onboarding with a vault declared");
    assert_ne!(
        fixture.head(),
        head,
        "the declaration-root commit did not land"
    );

    let author = fixture.git(&["log", "-1", "--format=%an <%ae>", "HEAD"]);
    assert_eq!(
        author, "selftest <selftest@example.com>",
        "the commit was not authored per the declaration root's own identity"
    );
}

/// The onboarding hook runs with `current_dir` at `declaration_root`,
/// unchanged, with a vault declared at a different root.
///
/// The hook writes a file named repository-relative to whatever directory it
/// ran in; it lands where only a `declaration_root`-rooted `current_dir`
/// would put it, so `fixture.read`, which is itself `declaration_root`
/// relative, finding it at all is the assertion.
#[test]
fn the_onboarding_hook_runs_at_the_declaration_root_with_a_vault_declared() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.seed_declarations();
    let recipient = fixture.new_recipient();
    fixture.set_hook(Some("pwd >hook-cwd.txt\n"));

    fixture
        .run_env(
            &["adduser", "carol", &recipient, "--yes"],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("onboarding with a hook and a vault declared");

    let cwd = fixture.read("hook-cwd.txt");
    assert_eq!(
        cwd.trim_end(),
        fixture.repo.to_str().expect("a utf-8 path"),
        "the onboarding hook did not run at the declaration root"
    );
}

/// `enroll/mod.rs`'s own hook runs with `current_dir` at `declaration_root`,
/// unchanged, with a vault declared at a different root — the counterpart of
/// [`the_onboarding_hook_runs_at_the_declaration_root_with_a_vault_declared`]
/// above for the enroll hook.
///
/// The card surface is stubbed throughout, per this suite's own rule: see
/// `tests/support/card-stubs.rs`. The run's own proof does not have to pass
/// for this claim — the hook fires once the ceremony's commit lands, before
/// the proof step that follows it, exactly as `enrollment.rs`'s own hook test
/// observes.
#[test]
fn the_enroll_hook_runs_at_the_declaration_root_with_a_vault_declared() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let vault = fixture.declare_vault();
    fixture.set_enroll_hook(Some("pwd >enroll-hook-cwd.txt\n"));

    let mut environment = fixture.card_env();
    environment.push(("SAFIX_CARD_STUB_SERIALS".to_owned(), "12345678".to_owned()));
    environment.push((
        "SAFIX_CARD_STUB_RECIPIENT".to_owned(),
        "age1yubikey1qfixture000000000000000000000000000000000000000000000000".to_owned(),
    ));
    environment.push((
        "SAFIX_VAULT_ROOT".to_owned(),
        vault.to_str().expect("a utf-8 path").to_owned(),
    ));
    let extra: Vec<(&str, &str)> = environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();

    fixture.run_on_terminal(&["enroll", "alice", "--no-store-pin"], "", &extra);

    let cwd = fixture.read("enroll-hook-cwd.txt");
    assert_eq!(
        cwd.trim_end(),
        fixture.repo.to_str().expect("a utf-8 path"),
        "the enroll hook did not run at the declaration root"
    );
}
