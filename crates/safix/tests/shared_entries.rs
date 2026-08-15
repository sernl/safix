//! An entry both people carry, and the two ways its declaration can move.
//!
//! A shared entry is one value: both carriers' placements name one file and one
//! key. Dropping a carrier is a revocation and flipping an entry to shared over
//! values already present is a migration, and the difference between the two is
//! what `check` has to get right — one has handed someone a value they should
//! not have, and the other has not.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{ANA_FILE, Fixture, SHARED_FILE};

/// One value, one file, one key, for every carrier: written by one of them and
/// read back by the other.
#[test]
fn both_carriers_resolve_one_file_and_read_one_value() {
    let mut fixture = Fixture::new();
    fixture.seed_shared("fleet-token", SHARED_FILE);
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);

    for user in ["ana", "bo"] {
        let listing = fixture.run(&["list", user]).expect_success("list");
        let row: Vec<String> = listing
            .output()
            .lines()
            .find(|line| line.split_whitespace().next() == Some("fleet-token"))
            .unwrap_or_else(|| panic!("no fleet-token row for {user}"))
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            row,
            vec![
                "fleet-token",
                "carries",
                "yes",
                "-",
                "fleet-token",
                SHARED_FILE
            ],
            "{user}'s row does not mark fleet-token shared against the audience file"
        );
    }

    fixture
        .set("ana", "fleet-token", "CANARY-one-value-for-both")
        .expect_success("a carrier setting the shared value");

    // bo's placement resolves the file ana wrote, so bo reads what ana minted. A
    // per-carrier copy would leave this reading nothing at all.
    let read = fixture
        .run(&["get", "bo", "fleet-token"])
        .expect_success("bo reading his fellow carrier's value");
    assert_eq!(read.stdout, b"CANARY-one-value-for-both");

    // And exactly one file holds it: a second copy anywhere is the defect the
    // revocation check exists to report, and must not be what a plain `set`
    // creates.
    let holders: Vec<String> = fixture
        .git(&["ls-files", "--", "secrets"])
        .lines()
        .filter(|path| path.ends_with(".yaml"))
        .filter(|path| {
            fixture
                .read(path)
                .lines()
                .any(|line| line.starts_with("fleet-token:"))
        })
        .map(str::to_owned)
        .collect();
    assert_eq!(
        holders,
        vec![SHARED_FILE.to_owned()],
        "the shared key is held by more than one file"
    );
}

/// A carrier dropped from a shared entry is a revocation, and the signal is the
/// ciphertext rather than a record of what the audience used to be.
#[test]
fn a_dropped_carrier_is_reported_as_a_revocation_naming_the_file_and_the_person() {
    let mut fixture = Fixture::new();
    fixture.seed_shared("fleet-token", SHARED_FILE);
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk", "fleet-token"]);
    fixture.make_sops_file(ANA_FILE, &["api-token", "mail-password"]);

    fixture.unshare_from("fleet-token", "ana", ANA_FILE);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the revocation");

    report.says("This is a revocation.");
    report.says(&format!(
        "{SHARED_FILE} still holds a value under 'fleet-token'"
    ));
    // Named, not printed as a key: an operator reading an age public key has to
    // go and look up whose it is, which is the moment a revocation is misjudged.
    assert!(
        report
            .stderr
            .lines()
            .any(|line| line.trim() == "- bo" || line.trim() == "  - bo"),
        "check does not name bo as the reader outside the audience:\n{}",
        report.stderr
    );
    report.silent_about(&fixture.bo);

    // The remedy is a new value, and it is the `set` form because this entry has
    // no generator. `fix` may appear only as the last convergence step, never as
    // the answer: re-wrapping a data key bo has already held revokes nothing.
    report.says("safix set ana fleet-token");
    report.says("fix is not the remedy");

    // Reported once. The stray is an unclaimed value too, and the two remedies
    // disagree — one says delete it, the other says declare it.
    report.silent_about("and no declaration claims it");
}

/// Flipping an entry to shared over values already present is a migration rather
/// than a disclosure: every reader of the copy left behind is still in the
/// audience.
#[test]
fn a_flip_to_shared_over_existing_values_is_reported_as_a_migration() {
    let mut fixture = Fixture::new();
    fixture.seed_shared("fleet-token", SHARED_FILE);
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);
    fixture.make_sops_file(ANA_FILE, &["api-token", "fleet-token"]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the migration");

    report.says(&format!(
        "{ANA_FILE} holds a value under 'fleet-token' of its own"
    ));
    report.says("migration rather than a disclosure");
    report.silent_about("This is a revocation.");
    // What is wrong is that the audience's own file holds no value and the
    // per-carrier copies can disagree with each other. The tool must not pick
    // which one wins.
    report.says("Which one should win is yours to say");
    report.says("safix set ana fleet-token");

    // The audience's own file is reported valueless for every carrier as well:
    // the migration is not done until a value is there.
    report.says(&format!(
        "flake.safix.users.bo declares 'fleet-token' and {SHARED_FILE} holds no value"
    ));
}
