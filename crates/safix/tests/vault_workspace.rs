//! `Workspace::discover_with`'s vault-root resolution and cross-validation
//! (design V1), and the vault-is-a-git-repository refusal (design V2).
//!
//! `list` is the verb every case below drives: it needs nothing beyond
//! `Workspace::discover_with` succeeding, so a refusal here is always
//! discovery's own rather than something the verb itself decided.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::Fixture;

/// Neither `SAFIX_VAULT_ROOT` nor a declared vault: `vault_root` resolves to
/// `root`, today's behaviour, unchanged.
#[test]
fn neither_signal_set_resolves_the_vault_root_to_the_declaration_root() {
    let fixture = Fixture::new();
    fixture
        .run(&["list"])
        .expect_success("a bare list with no vault declared and no root named");
}

/// Both signals set and agreeing: the named path is used as `vault_root`, and
/// is itself a repository's top level (design V2's passing case).
#[test]
fn a_declared_vault_with_a_named_root_resolves_to_it() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture
        .run_env(
            &["list"],
            None,
            &[("SAFIX_VAULT_ROOT", vault.to_str().expect("a utf-8 path"))],
        )
        .expect_success("a declared vault with SAFIX_VAULT_ROOT set to its own top level");
}

/// `vaultDeclared` true, `SAFIX_VAULT_ROOT` unset: refused before anything
/// evaluates or writes, naming both `flake.safix.vault` and the environment
/// variable.
#[test]
fn a_declared_vault_with_no_named_root_is_refused() {
    let mut fixture = Fixture::new();
    fixture.declare_vault();

    let refused = fixture.run_graphical_env(&["list"], &[]);
    assert_eq!(refused.refusal_code(), "vault_declared_without_root");
    refused.says("flake.safix.vault");
    refused.says("SAFIX_VAULT_ROOT");
}

/// `SAFIX_VAULT_ROOT` set, `vaultDeclared` false: refused, naming both the
/// path named and `flake.safix.vault` — a root named for a vault nix does
/// not know about would silently do nothing.
#[test]
fn a_named_root_with_no_declared_vault_is_refused() {
    let fixture = Fixture::new();
    let named = fixture.work.join("undeclared");
    std::fs::create_dir_all(&named).expect("a temporary directory can be made");
    let extra = [("SAFIX_VAULT_ROOT", named.to_str().expect("a utf-8 path"))];

    assert_eq!(
        fixture.run_graphical_env(&["list"], &extra).refusal_code(),
        "vault_root_without_declaration"
    );

    // The plain reporter wraps nothing, so the full runtime path — which the
    // graphical reporter's line wrap can split mid-word — is checked here.
    let refused = fixture.run_env(&["list"], None, &extra);
    refused.says("flake.safix.vault");
    refused.says(named.to_str().expect("a utf-8 path"));
}

/// `SAFIX_VAULT_ROOT` naming a plain directory that is not a git repository
/// at all — design V2's first refusal, distinguished from the top-level
/// mismatch below.
#[test]
fn a_root_naming_a_plain_directory_is_refused() {
    let mut fixture = Fixture::new();
    // `declare_vault` only needs to make `vaultDeclared` true here; the
    // git-backed path it returns is deliberately not the one named below.
    fixture.declare_vault();
    let plain = fixture.work.join("not-a-repository");
    std::fs::create_dir_all(&plain).expect("a temporary directory can be made");

    let refused = fixture.run_graphical_env(
        &["list"],
        &[("SAFIX_VAULT_ROOT", plain.to_str().expect("a utf-8 path"))],
    );
    assert_eq!(refused.refusal_code(), "vault_not_a_repository");
    refused.says(plain.to_str().expect("a utf-8 path"));
}

/// `SAFIX_VAULT_ROOT` naming a subdirectory of a real git repository — design
/// V2's second refusal, distinguished from the plain-directory case above:
/// git finds a top level, and it disagrees with the path named.
#[test]
fn a_root_naming_a_subdirectory_of_a_repository_is_refused() {
    let mut fixture = Fixture::new();
    fixture.declare_vault();
    let sub = fixture.repo.join("secrets");
    std::fs::create_dir_all(&sub).expect("a temporary directory can be made");

    let refused = fixture.run_graphical_env(
        &["list"],
        &[("SAFIX_VAULT_ROOT", sub.to_str().expect("a utf-8 path"))],
    );
    assert_eq!(refused.refusal_code(), "vault_root_not_top_level");
    refused.says(sub.to_str().expect("a utf-8 path"));
    refused.says(fixture.repo.to_str().expect("a utf-8 path"));
}

/// Severity drill for the two V2 refusals above: pointing `vault_root` at the
/// subdirectory case while asserting only the plain-directory code fires
/// turns this red, which is the evidence the two refusals are distinguished
/// rather than collapsed into one message.
///
/// This is task 3.5's drill, encoded as a standing regression rather than a
/// one-off manual check: reverting `verify_vault_repository`'s canonicalized
/// comparison to always raising `VaultNotARepository` (as if
/// `Git::show_toplevel` could never itself succeed) turns this assertion red.
#[test]
fn the_two_git_repository_refusals_are_distinguished_not_collapsed() {
    let mut fixture = Fixture::new();
    fixture.declare_vault();
    let sub = fixture.repo.join("secrets");
    std::fs::create_dir_all(&sub).expect("a temporary directory can be made");

    let refused = fixture.run_graphical_env(
        &["list"],
        &[("SAFIX_VAULT_ROOT", sub.to_str().expect("a utf-8 path"))],
    );
    assert_ne!(
        refused.refusal_code(),
        "vault_not_a_repository",
        "a subdirectory of a real repository must not read as no repository at all"
    );
    assert_eq!(refused.refusal_code(), "vault_root_not_top_level");
}
