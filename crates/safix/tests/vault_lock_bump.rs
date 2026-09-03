//! The lock-bump disclosure a vault-root commit prints (design V6, task
//! group 9): the requirement that no consuming build sees the change until
//! the declaring flake's lock entry for the vault is updated, and the exact
//! `nix flake lock --update-input <name>` line when the declaring flake's
//! lock file settles on exactly one input matching the vault root.
//!
//! Every test here drives `set`, which reaches `set.rs`'s `run_committing` —
//! one of the three vault-root commit success paths task 9.2 wires the
//! disclosure into, alongside `generate.rs`'s `write` and `enroll/mod.rs`'s
//! `wire()`. Task 9's own fixture shape is a set of `(name, path)` lock
//! nodes — [`Fixture::set_flake_lock_nodes`] — standing in for
//! `nix flake metadata`'s `.locks.nodes`, which the stub answers verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use std::path::Path;

use harness::{ALICE_FILE, Fixture};

/// Task 9.3: a lock naming exactly one input whose path matches the vault
/// root — the disclosure names that input and the exact
/// `nix flake lock --update-input` command.
#[test]
fn a_lock_naming_exactly_one_matching_input_names_it_in_the_disclosure() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(ALICE_FILE, &[&fixture.alice.clone()]);
    fixture.set_flake_lock_nodes(&[("vault", vault.as_path())]);

    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-lock-bump-named"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("setting a value with a vault declared and one matching lock input")
        .says("This change is not visible to any consuming build")
        .says("nix flake lock --update-input vault");
}

/// Task 9.4, the zero-match half: a lock naming no input whose path matches
/// the vault root — the disclosure states the general requirement and names
/// no input.
#[test]
fn a_lock_naming_no_matching_input_falls_back_to_the_general_disclosure() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(ALICE_FILE, &[&fixture.alice.clone()]);
    fixture.set_flake_lock_nodes(&[("nixpkgs", Path::new("/nix/store/unrelated-input"))]);

    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-lock-bump-none"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("setting a value with a vault declared and no matching lock input")
        .says("This change is not visible to any consuming build")
        .silent_about("--update-input");
}

/// Task 9.4, the more-than-one half: a lock naming two inputs whose path
/// both match the vault root — ambiguous, so the disclosure falls back the
/// same way the zero-match case does.
#[test]
fn a_lock_naming_more_than_one_matching_input_falls_back_to_the_general_disclosure() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.set_vault_rules(ALICE_FILE, &[&fixture.alice.clone()]);
    fixture.set_flake_lock_nodes(&[
        ("vault", vault.as_path()),
        ("vault-mirror", vault.as_path()),
    ]);

    fixture
        .run_env(
            &["set", "alice", "api-token"],
            Some("CANARY-lock-bump-ambiguous"),
            &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
        )
        .expect_success("setting a value with a vault declared and two matching lock inputs")
        .says("This change is not visible to any consuming build")
        .silent_about("--update-input");
}

/// A run with no vault declared prints no lock-bump disclosure at all — the
/// guard `vault_root() != root()` every call site applies, held as a
/// standing test rather than inferred from the other three passing.
#[test]
fn a_run_with_no_vault_declared_prints_no_lock_bump_disclosure() {
    let fixture = Fixture::new();

    fixture
        .run_with(&["set", "alice", "api-token"], "CANARY-no-vault-declared")
        .expect_success("setting a value with no vault declared")
        .silent_about("lock entry for the vault");
}
