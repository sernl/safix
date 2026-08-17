//! A narrowed audience, and what `check` says about it.
//!
//! Three declarations narrow an audience the same way: a member leaves a group, a
//! grant is dropped, a machine changes hands. None of them is distinguishable
//! from the others here, and that is the model rather than a limitation — an
//! evaluation records the audience that is and never the audience that was, so
//! what reaches this runtime is one state: a key on a governed file that the
//! declared audience does not name.
//!
//! What the report has to do with that state is say whose key it is, call it the
//! revocation it is, and be honest that `fix` is the alignment and not the
//! remedy. The last is the part worth a test: `fix` re-wraps the file to the
//! narrowed audience, which is right and is not revocation, and a report that
//! offered it as the answer would tell an operator their contractor had lost
//! access to a value the contractor has already read.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{Fixture, SHARED_FILE};

/// The ciphertext still names bo and the declarations no longer do.
#[test]
fn a_narrowed_audience_is_reported_as_the_revocation_it_is() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);

    // The file was created through the rule granting both, so bo's key is on it.
    // The declarations now name ana alone, and the committed policy is
    // regenerated with them so that the drift under test is the ciphertext's and
    // not a stale artifact's.
    let ana = fixture.ana.clone();
    fixture.narrow_audience(SHARED_FILE, &["ana"], &[&ana]);
    fixture.write_policy(&["ana"]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the narrowing");

    report.says(&format!(
        "{SHARED_FILE} is not encrypted to the audience declared for it"
    ));
    report.says(&fixture.bo);

    // Named, not printed as a key. An operator reading an age public key has to
    // go and look up whose it is, which is the moment a revocation is misjudged.
    assert!(
        report
            .stderr
            .lines()
            .any(|line| line.trim() == "- bo" || line.trim() == "  - bo"),
        "check does not name bo as the custody the extra key belongs to:\n{}",
        report.stderr
    );

    report.says("so this is a revocation");
    report.says("no re-wrap unreads it");
    report.says("Only a new value revokes.");

    // `fix` is offered as the alignment, and a new value as the revocation. Both
    // appear, and the report says which is which.
    report.says("safix fix");
    report.says("then, to revoke rather than align, mint new values:");
    report.says("safix set ana wifi-psk");

    // The narrowed name itself is neither valueless nor unclaimed: its value is
    // in the file the declarations place it in, and the only thing wrong is who
    // can open that file. The fixture's other declared names have no values and
    // are reported for it, which is what says this report is not one finding
    // wide.
    report.silent_about("declares 'wifi-psk'");
    report.silent_about("and no declaration claims it");
}

/// A key on the file that answers to no declared subject is the more alarming
/// half, and must not be swallowed by naming only the subjects that matched.
#[test]
fn a_key_answering_to_nobody_is_reported_apart_from_the_named_subjects() {
    let mut fixture = Fixture::new();
    let stranger = fixture.new_recipient();
    fixture.encrypt_to(
        SHARED_FILE,
        &[&fixture.ana, &fixture.bo, &stranger],
        "wifi-psk: \"fixture-value-for-the-network\"\n",
    );

    let ana = fixture.ana.clone();
    fixture.narrow_audience(SHARED_FILE, &["ana"], &[&ana]);
    fixture.write_policy(&["ana"]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the narrowing");

    report.says("those keys are the custody of:");
    report.says("and of no declared subject:");
    report.says(&stranger);
    report.says("so this is a revocation");
}
