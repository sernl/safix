//! What `get`, `list` and `check` report, and what they must never render.
//!
//! `get`'s standard output is the value and nothing else, which is what makes it
//! pipeable; `list` reports where every name lives without rendering one; and
//! `check` judges the union of files the declarations imply and the files the
//! consumer named, which are different claims and are reported differently.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use harness::{ANA_FILE, Fixture, SHARED_FILE};

/// A value round-trips byte for byte, and `list` says where each name lives
/// without saying what it is.
#[test]
fn get_round_trips_a_value_and_list_reports_where_it_lives() {
    let fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token", "mail-password"]);
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);

    let read = fixture
        .run(&["get", "ana", "api-token"])
        .expect_success("get");
    assert_eq!(
        read.stdout, b"fixture-value-for-api-token",
        "get did not round-trip the fixture value"
    );

    // Byte for byte, including the absence of a trailing newline: a value stored
    // exactly as typed comes back exactly as stored, and a stream that gained a
    // newline on the way out would still match a line-wise comparison. Nothing
    // but the value reaches standard output, which is what lets a pipe carry the
    // secret alone.
    fixture
        .set("ana", "mail-password", "CANARY-round-trip")
        .expect_success("the round-trip set");
    let read = fixture
        .run(&["get", "ana", "mail-password"])
        .expect_success("get");
    assert_eq!(read.stdout, b"CANARY-round-trip", "not byte-identical");

    // A secret shared from another owner resolves to the shared file for the
    // recipient too, so both parties read one file.
    let read = fixture
        .run(&["get", "ana", "wifi-psk"])
        .expect_success("get on a granted secret");
    assert_eq!(read.stdout, b"fixture-value-for-wifi-psk");

    // The default user is $USER when it is a declared user.
    let read = fixture.run(&["get", "api-token"]).expect_success("get");
    assert_eq!(read.stdout, b"fixture-value-for-api-token");

    let listing = fixture.run(&["list", "ana"]).expect_success("list");
    let table = listing.output();
    assert_eq!(
        row(&table, "NAME"),
        vec!["NAME", "ORIGIN", "SHARED", "GENERATOR", "KEY", "FILE"],
        "list does not head the SHARED and GENERATOR columns"
    );
    // ORIGIN says how the name reached this user and SHARED says whether the
    // entry is one value. A secret granted through sharedWith is shared in the
    // first sense and not in the second, so the two columns disagree here.
    assert_eq!(
        row(&table, "api-token"),
        vec!["api-token", "carries", "-", "-", "api-token", ANA_FILE],
    );
    assert_eq!(
        row(&table, "wifi-psk"),
        vec!["wifi-psk", "shared", "-", "-", "wifi-psk", SHARED_FILE],
    );
    // An entry may be read under a key that is not its name, and the KEY column
    // is what tells an operator which.
    assert_eq!(
        row(&table, "aliased-secret"),
        vec![
            "aliased-secret",
            "private",
            "-",
            "-",
            "custom-key",
            ANA_FILE
        ],
    );
    listing.silent_about("fixture-value-for");
}

/// The governed set is the union of what the declarations imply and what the
/// consumer named, and the two halves are judged differently.
///
/// A file named through `extraGovernedFiles` rides an existing rule and no
/// declaration places a secret in it, so its keys are unclaimed by construction
/// and must not be reported — while its stanzas are still held to the rule that
/// covers it, which is exactly what `fix` re-wraps it to.
#[test]
fn a_governed_extra_is_held_to_its_rule_and_not_to_the_declarations() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.make_sops_file(ANA_FILE, &["api-token", "mail-password", "custom-key"]);

    let extra = "secrets/safix/users/ana/ops-tooling.yaml";
    fixture.govern_extra(extra);
    fixture.make_sops_file(extra, &["shared-tooling-token"]);

    // In step with its rule, it is not a finding of any kind. Reporting its keys
    // as unclaimed would be a finding no declaration could ever resolve — not
    // naming them is what naming the file in extraGovernedFiles means.
    let report = fixture.run(&["check"]);
    report.silent_about(extra);
    report.silent_about("shared-tooling-token");

    // Drifted from the rule that covers it, it is drift in exactly the sense a
    // required file's would be.
    let stranger = fixture.new_recipient();
    fixture.encrypt_to(
        extra,
        &[&fixture.ana, &stranger],
        "shared-tooling-token: \"fixture-value-for-tooling\"\n",
    );
    fixture.git(&["add", "--", extra]);
    fixture.git(&[
        "commit",
        "-q",
        "-m",
        "fixture: the extra file drifted from the rule that covers it",
    ]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the drift");
    report.says(&format!(
        "{extra} is not encrypted to the audience declared for it"
    ));
    report.says(&stranger);

    // `fix` re-wraps it, which is the whole reason the union exists: driving the
    // re-wrap from the declared half alone would leave a consumer-named file
    // encrypted to whoever it was encrypted to when it was written.
    fixture
        .run(&["fix", "--yes"])
        .expect_success("fix over the governed set");
    assert!(
        !fixture.read(extra).contains(&stranger),
        "fix did not re-wrap the consumer-named file"
    );

    // A path no rule's directory covers is its own finding: naming a file
    // creates no rule for it.
    let unruled = "secrets/safix/users/cy/stranded.yaml";
    fixture.govern_extra(unruled);
    fixture.encrypt_to(
        unruled,
        &[&fixture.ana],
        "shared-tooling-token: \"fixture-value-for-tooling\"\n",
    );
    let report = fixture.run(&["check"]);
    assert_eq!(
        report.code,
        Some(1),
        "check did not report the unruled path"
    );
    report.says("no creation rule's directory covers it");
}

/// What the consumption module's identity preflight checks, and what it says it
/// does not: an identity present and readable and not a recipient.
///
/// `modules/consume/home.nix` refuses a home activation whose identity paths are
/// missing or unreadable, and its own message states the limit of that — "a key
/// that exists and is readable but is not a recipient of these files still fails
/// later, in sops-install-secrets". Everything `add-consumption-modules` verified
/// was an evaluation, so the sentence about what happens later was the one claim
/// on that path no check held.
///
/// This holds it against fixture ciphertext instead of against an activation:
/// nothing here switches a profile, and the decryption boundary a run reaches is
/// the same sops reading the same `SOPS_AGE_KEY_FILE` that `sops-install-secrets`
/// reads. What is not asserted is the activation itself — the ordering is
/// `safix-consumption-ordering`'s, against a real home-manager evaluation.
///
/// The stranger's identity is shown to open a document it is a recipient of
/// before it is shown not to open one it is not. Without that, the refusal would
/// hold just as well over a key file that was malformed, empty, or not a key at
/// all, and the claim is about recipiency rather than about a broken file.
#[test]
fn an_identity_present_and_readable_and_not_a_recipient_does_not_decrypt() {
    let fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    let stranger = fixture.scratch("stranger-identity.txt");
    let minted = std::process::Command::new("age-keygen")
        .arg("-o")
        .arg(&stranger)
        .output()
        .expect("could not run age-keygen");
    assert!(
        minted.status.success(),
        "could not mint a stranger identity"
    );
    let recipient = std::process::Command::new("age-keygen")
        .arg("-y")
        .arg(&stranger)
        .output()
        .expect("could not run age-keygen");
    assert!(recipient.status.success(), "could not derive the recipient");
    let recipient = String::from_utf8(recipient.stdout).unwrap();
    let recipient = recipient.trim();

    // The two predicates the preflight applies, in the order it applies them.
    assert!(stranger.exists(), "the stranger's identity is not present");
    assert!(
        std::fs::File::open(&stranger).is_ok(),
        "the stranger's identity is not readable"
    );

    // And it is a working identity, on a document it is a recipient of.
    let theirs = "secrets/safix/users/cy/theirs.yaml";
    fixture.encrypt_to(
        theirs,
        &[recipient],
        "theirs: \"fixture-value-for-theirs\"\n",
    );
    let opened = std::process::Command::new("sops")
        .arg("decrypt")
        .arg(theirs)
        .current_dir(&fixture.repo)
        .env("SOPS_AGE_KEY_FILE", &stranger)
        .output()
        .expect("could not run sops");
    assert!(
        opened.status.success(),
        "the stranger's identity does not open the document it is a recipient of:\n{}",
        String::from_utf8_lossy(&opened.stderr)
    );

    // Present, readable, working, and not a recipient of this one.
    let refused = fixture.run_env(
        &["get", "ana", "api-token"],
        None,
        &[("SOPS_AGE_KEY_FILE", &stranger.to_string_lossy())],
    );
    assert!(
        !refused.succeeded(),
        "a non-recipient identity read the value:\n{}",
        refused.combined()
    );
    assert!(
        refused.stdout.is_empty(),
        "a refused read put bytes on standard output"
    );
    refused.silent_about("fixture-value-for-api-token");

    // The recipient identity opens the same file, so what the refusal above
    // reports is the identity rather than the file or the placement.
    let read = fixture
        .run(&["get", "ana", "api-token"])
        .expect_success("get with the identity the file is encrypted to");
    assert_eq!(read.stdout, b"fixture-value-for-api-token");
}

/// `--version` is answered, on standard output, and exits zero.
///
/// A decision rather than an observation, and one that had a single pin. The
/// retired shell runtime reached its unknown-subcommand refusal for `--version`
/// and exited 1; this binary answers it, because that is the convention for a
/// compiled binary and a strictly wider surface rather than a different answer
/// to a question both were asked. The comparative check that recorded the
/// divergence went with the oracle, so the decision is asserted here instead of
/// nowhere.
///
/// Standard output rather than standard error, and a version-shaped string
/// rather than any string, because a caller reading `safix --version | cut -d\  -f2`
/// is the whole reason a compiled binary answers it.
#[test]
fn version_is_answered_on_standard_output_and_is_not_an_unknown_subcommand() {
    let fixture = Fixture::new();

    let answered = fixture.run(&["--version"]).expect_success("--version");

    let printed = answered.output();
    let printed = printed.trim_end_matches('\n');
    let (name, version) = printed.split_once(' ').unwrap_or_else(|| {
        panic!("--version printed {printed:?}, which is not '<name> <version>'")
    });
    assert_eq!(
        name, "safix",
        "--version named something other than the binary"
    );
    assert_eq!(
        version.split('.').count(),
        3,
        "--version printed {version:?}, which is not three dot-separated parts"
    );
    assert!(
        version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())),
        "--version printed {version:?}, which is not a version"
    );
    answered.silent_about("unknown subcommand");
}

/// One row of a rendered table, split into its cells.
fn row<'a>(table: &'a str, name: &str) -> Vec<&'a str> {
    table
        .lines()
        .find(|line| line.split_whitespace().next() == Some(name))
        .unwrap_or_else(|| panic!("no row for {name} in:\n{table}"))
        .split_whitespace()
        .collect()
}
