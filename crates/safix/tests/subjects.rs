//! A narrowed audience, and what `check` says about it.
//!
//! Four declarations narrow an audience the same way: a member leaves a group, a
//! grant is dropped, a machine changes hands, a machine leaves a service. None of
//! them is distinguishable from the others here, and that is the model rather than
//! a limitation — an evaluation records the audience that is and never the audience
//! that was, so what reaches this runtime is one state: a key on a governed file
//! that the declared audience does not name.
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
    fixture.set_audience(SHARED_FILE, &["ana"], &[&ana]);
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

/// A widened audience is a re-wrap, and `fix` is the whole of it.
///
/// Growing a group's membership adds a recipient to the audience of a file whose
/// path does not move — that is what naming the directory for the group rather
/// than for its members buys — so the convergence is `sops updatekeys` over a
/// file that already exists. This drives the real one.
#[test]
fn a_widened_audience_is_re_wrapped_by_fix() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);

    // The joining member holds a key nothing has encrypted to yet, and is
    // declared so that the regenerated policy defines the anchor its rule grants.
    let joined = fixture.new_recipient();
    let ana = fixture.ana.clone();
    let bo = fixture.bo.clone();
    fixture.declare_person("cy", &joined);
    assert!(
        !fixture.read(SHARED_FILE).contains(&joined),
        "the fixture file already names the joining member"
    );

    fixture.set_audience(SHARED_FILE, &["ana", "bo", "cy"], &[&ana, &bo, &joined]);
    fixture.write_policy_agreeing(&["ana", "bo", "cy"]);

    fixture
        .run(&["fix", "--yes"])
        .expect_success("fix over a widened audience");

    assert!(
        fixture.read(SHARED_FILE).contains(&joined),
        "fix did not re-wrap the file to the widened audience:\n{}",
        fixture.read(SHARED_FILE)
    );

    // The value is still there and still readable, which is what makes this a
    // re-wrap rather than a rewrite.
    assert_eq!(
        fixture.value(SHARED_FILE, "wifi-psk"),
        "fixture-value-for-wifi-psk"
    );

    // And the report is quiet about that file afterwards: the ciphertext and the
    // declared audience now agree.
    let report = fixture.run(&["check"]);
    report.silent_about(&format!(
        "{SHARED_FILE} is not encrypted to the audience declared for it"
    ));
}

/// A machine leaving a service is a revocation, reported through the service it
/// left.
///
/// The service is what the audience names, so it is what the file's directory is
/// named for and what the headline therefore prints; the machine is what held the
/// key, so it is what the custody line names. Both halves are needed to act on the
/// report: the machine says whose access this was, and the service says which
/// declaration to look at.
#[test]
fn a_machine_leaving_a_service_is_reported_through_the_service() {
    const SERVICE_FILE: &str = "secrets/safix/shared/%nginx,ana/secrets.yaml";

    let mut fixture = Fixture::new();

    // deck's own key, which is the age form of a host identity in the model and
    // just a key here. Declared as a subject so that the report can name the
    // machine rather than print its key and leave the reader to look it up.
    let deck = fixture.new_recipient();
    fixture.declare_subject("deck", &[&deck]);

    fixture.encrypt_to(
        SERVICE_FILE,
        &[&fixture.ana.clone(), &deck],
        "token: \"fixture-value-for-the-service\"\n",
    );
    fixture.seed_output("token", SERVICE_FILE);

    // The service ran on deck and now runs elsewhere. The file does not move —
    // that is what naming the directory for the service buys — so what is left is
    // deck's key on a file the declared audience no longer covers.
    let ana = fixture.ana.clone();
    fixture.set_audience(SERVICE_FILE, &["%nginx", "ana"], &[&ana]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the narrowing");

    report.says(&format!(
        "{SERVICE_FILE} is not encrypted to the audience declared for it"
    ));
    report.says("those keys are the custody of:");
    assert!(
        report
            .stderr
            .lines()
            .any(|line| line.trim() == "- deck" || line.trim() == "  - deck"),
        "check does not name deck as the custody the extra key belongs to:\n{}",
        report.stderr
    );

    // The revocation, and the honesty about what `fix` is: a re-wrap aligns the
    // ciphertext with the shrunk machine set and unreads nothing deck read.
    report.says("so this is a revocation");
    report.says("no re-wrap unreads it");
    report.says("Only a new value revokes.");
    report.says("then, to revoke rather than align, mint new values:");
    report.says("safix set ana token");
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
    fixture.set_audience(SHARED_FILE, &["ana"], &[&ana]);
    fixture.write_policy(&["ana"]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the narrowing");

    report.says("those keys are the custody of:");
    report.says("and of no declared subject:");
    report.says(&stranger);
    report.says("so this is a revocation");
}
