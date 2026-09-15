//! `check.rs`'s shared/stray logic (`shared`, `check.rs:291-375`) reads
//! `placement.file` and `placement.key` as opaque strings and never parses
//! either — every comparison is string equality against other nix-provided
//! strings — so design V14 states it needs no change in either mode.
//!
//! This drives the same revocation scenario `shared_entries.rs` drives with
//! readable paths, over opaque-hex-shaped ones instead, via
//! [`harness::Fixture::encrypt_to`] rather than [`harness::Fixture::make_sops_file`]
//! so no committed creation rule — which names a readable directory — has to
//! match an opaque path for the ciphertext to exist at all: the claim under
//! test is about `check`'s own logic, not about the recipient policy.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::Fixture;

/// An audience file and a stray file both named by opaque hex, exactly as
/// `secretsFileOf`/`opaqueOf` would name them in vault mode: `check` reports
/// the dropped carrier as a revocation the same way it does over readable
/// paths in `shared_entries.rs`'s
/// `a_dropped_carrier_is_reported_as_a_revocation_naming_the_file_and_the_person`.
///
/// The two names are the resolver's real output rather than hex-shaped
/// stand-ins, recomputed here as `opaqueOf` computes them —
/// `sha256("<namingKey>|secrets|<root-relative readable path>")` — over
/// `modules/flake/checks/vault.nix`'s fixture naming key
/// `fc84b416cd03fedc2c02116b068d75c543d327361f19e3d18d9e8aa3de0f4ad2` and
/// the readable paths `harness::SHARED_FILE` and `harness::ALICE_FILE`
/// carry relative to `flake.safix.storage.encrypted`:
/// `shared/alice,bob/secrets.yaml` and `users/alice/secrets.yaml`. Root
/// relativity is what makes each a function of the entry's identity rather
/// than of the encrypted tree's spelling, so renaming that root leaves both
/// of these bytes alone.
#[test]
fn a_dropped_carrier_is_reported_as_a_revocation_over_opaque_paths_too() {
    let opaque_shared =
        "secrets/873b6adfa41ee669e5896f0fe90bee8c5738b31b8491b39ade5c690a02741b26.yaml";
    let opaque_alice =
        "secrets/49de471561b63dceeb0edc0b29d324f49d1ec33bd7f21a2f312b47e8836248ea.yaml";

    let mut fixture = Fixture::new();
    fixture.seed_shared("fleet-token", opaque_shared);
    let (alice_key, bob_key) = (fixture.alice.clone(), fixture.bob.clone());
    fixture.set_audience(opaque_shared, &["alice", "bob"], &[&alice_key, &bob_key]);
    fixture.encrypt_to(
        opaque_shared,
        &[&alice_key, &bob_key],
        "wifi-psk: CANARY-shared\nfleet-token: CANARY-fleet\n",
    );

    // alice is dropped from the shared entry and re-declared privately at her
    // own opaque file — but the shared value's ciphertext is left behind at
    // her old, now-private file, which is the stray `check` must catch.
    fixture.unshare_from("fleet-token", "alice", opaque_alice);
    fixture.set_audience(opaque_alice, &["alice"], &[&alice_key]);
    fixture.encrypt_to(
        opaque_alice,
        &[&alice_key],
        "api-token: CANARY-alice\nfleet-token: CANARY-fleet\n",
    );

    let report = fixture.run(&["check"]);
    assert_eq!(
        report.code,
        Some(1),
        "check did not report the revocation over opaque paths"
    );
    report.says("This is a revocation.");
    report.says(&format!(
        "{opaque_shared} still holds a value under 'fleet-token'"
    ));
    assert!(
        report
            .stderr
            .lines()
            .any(|line| line.trim() == "- bob" || line.trim() == "  - bob"),
        "check does not name bob as the reader outside the audience over opaque paths:\n{}",
        report.stderr
    );
}
