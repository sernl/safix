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
#[test]
fn a_dropped_carrier_is_reported_as_a_revocation_over_opaque_paths_too() {
    let opaque_shared =
        "secrets/374e7f916175346f12a5e181a3d58b6ebfd729cde1330a57ea0bf6b80a482254.yaml";
    let opaque_alice =
        "secrets/82e305b5e0542ccd740dea5fac69547697e3bda4d51f7f5ee65dacd53179e0d9.yaml";

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
