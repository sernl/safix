//! What `check` and `fix` say about a raw age file's roster, read from its
//! header without decrypting it.
//!
//! A raw age header names each SSH recipient by a four-byte tag of its key and
//! every other recipient by nothing. These tests drive the real `age` and the
//! real `ssh-keygen`: the tags are the ones age writes, and the claim is that
//! `check` reads them, compares them with the declared audience, and says
//! exactly how strong the comparison is. The second half is the audience record
//! `fix` writes after a re-wrap, and the re-wrap it lets `fix` skip.
//!
//! Every expectation here is a literal written in the test.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::process::{Command, Stdio};

use harness::Fixture;

/// The value every raw age file here holds, so a report can be searched for it.
const VALUE: &[u8] = b"CANARY-raw-age-value\n";

/// The audience record `place_raw_age` names for `alice`'s `deploy-key`.
const RECORD: &str = "state/safix/definitions/alice/deploy-key.audience";

/// A fixture whose only entry is one raw age file and whose committed policy is
/// the one the declarations imply — so whatever `check` says, it says about that
/// file. Each test declares the file's audience, which is also what governs it.
fn roster() -> (Fixture, String) {
    let mut fixture = Fixture::new();
    let (alice, bob) = (fixture.alice.clone(), fixture.bob.clone());
    fixture.declare_person("alice", &alice);
    fixture.declare_person("bob", &bob);
    fixture.seed_holder_of_nothing("alice");
    fixture.seed_holder_of_nothing("bob");
    let file = fixture.place_raw_age("alice", "deploy-key");
    (fixture, file)
}

/// The bytes `age` writes for this value to these recipients.
fn encrypted(recipients: &[&str], armor: bool) -> Vec<u8> {
    let mut age = Command::new("age");
    age.arg("--encrypt");
    if armor {
        age.arg("--armor");
    }
    for recipient in recipients {
        age.args(["-r", recipient]);
    }
    let mut child = age
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), VALUE).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "age refused to encrypt");
    output.stdout
}

/// Put these bytes at the raw age file and commit them.
fn commit(fixture: &Fixture, file: &str, bytes: &[u8]) {
    let path = fixture.repo.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
    fixture.git(&["add", "-A"]);
    fixture.git(&["commit", "-qm", "fixture: raw age ciphertext"]);
}

fn bytes_of(fixture: &Fixture, relative: &str) -> Vec<u8> {
    std::fs::read(fixture.repo.join(relative)).unwrap()
}

/// What the fixture's own identity opens the file to.
fn opened(fixture: &Fixture, file: &str) -> Vec<u8> {
    let configured = fixture.command(&[]);
    let identity = configured
        .get_envs()
        .find(|(name, _)| *name == "SOPS_AGE_KEY_FILE")
        .and_then(|(_, value)| value)
        .unwrap();
    let output = Command::new("age")
        .args(["--decrypt", "-i"])
        .arg(identity)
        .arg(fixture.repo.join(file))
        .output()
        .unwrap();
    assert!(output.status.success(), "the fixture's identity opens it");
    output.stdout
}

/// The tag age wrote for an SSH key is the tag safix computes from the
/// declared key, in the binary form and the armored one; and a file whose
/// stanzas match its declared SSH recipients exactly is a note, not a pass,
/// that leaves `check` exiting zero.
#[test]
fn age_writes_the_ssh_tag_safix_computes() {
    let (mut fixture, file) = roster();
    let key = fixture.ssh_recipient();
    fixture.set_audience(&file, &["alice"], &[&key]);
    let (kind, tag) = safix_core::age_header::ssh_tag(&key).expect("the key is an SSH key");
    assert_eq!(kind, "ssh-ed25519");
    assert_eq!(tag.len(), 6);

    for armor in [false, true] {
        let bytes = encrypted(&[&key], armor);
        assert_eq!(bytes.starts_with(b"-----BEGIN AGE ENCRYPTED FILE-----"), armor);
        let stanzas = safix_core::age_header::stanzas(&bytes).expect("age's header reads");
        assert_eq!(
            stanzas,
            vec![safix_core::age_header::Stanza {
                kind: kind.clone(),
                first_argument: Some(tag.clone()),
            }]
        );
        commit(&fixture, &file, &bytes);

        let run = fixture.run(&["check"]);
        assert_eq!(run.code, Some(0), "a tag match alone exits zero\n{}", run.combined());
        run.says(&format!("Recipient roster matched by tag only: {file}"))
            .says("Each of the 1 stanza(s) matches a declared SSH key by a four-byte tag")
            .says("does not establish which keys the file is wrapped to")
            .says("Use a SOPS container when inspectable recipient metadata is required.")
            .says("safix: no drift; the note(s) above qualify that.")
            .silent_about("finding(s)")
            .silent_about("CANARY");
    }
}

/// A stanza for a key the audience does not declare is drift, named as the
/// declared subject holding that key, with the not-revocation sentence and
/// the mint that revokes.
#[test]
fn an_extra_ssh_stanza_is_drift_and_names_its_holder() {
    let (mut fixture, file) = roster();
    let declared = fixture.ssh_recipient();
    let carols = fixture.ssh_recipient();
    fixture.declare_subject("carol", &[&carols]);
    fixture.set_audience(&file, &["alice"], &[&declared]);
    commit(&fixture, &file, &encrypted(&[&declared, &carols], false));

    let run = fixture.run(&["check"]);
    assert_eq!(run.code, Some(1), "{}", run.combined());
    run.says(&format!(
        "{file} is not encrypted to the SSH recipients declared for it, by the tags age writes into its stanzas."
    ))
    .says(&format!(
        "carries a stanza whose tag no SSH recipient of its audience has:\n    - {carols}\n"
    ))
    .says("those keys are the custody of:\n    - carol\n")
    .says("Only a new value revokes.")
    .says("    safix fix --yes\n")
    .says("    safix set alice deploy-key\n")
    .says("safix: 1 finding(s).")
    .silent_about("CANARY");
}

/// A declared SSH recipient whose tag no stanza carries is missing.
#[test]
fn a_missing_ssh_recipient_is_drift() {
    let (mut fixture, file) = roster();
    let present = fixture.ssh_recipient();
    let absent = fixture.ssh_recipient();
    fixture.set_audience(&file, &["alice"], &[&present, &absent]);
    commit(&fixture, &file, &encrypted(&[&present], false));

    let run = fixture.run(&["check"]);
    assert_eq!(run.code, Some(1), "{}", run.combined());
    run.says(&format!("{file} is not encrypted to the SSH recipients declared for it"))
        .says(&format!(
            "is in its audience and has no stanza carrying its tag:\n    - {absent}\n"
        ))
        .says("    safix fix --yes\n")
        .silent_about("matched by tag only")
        .silent_about("CANARY");
}

/// A header repeating one stanza carries more stanzas with that tag than the
/// audience declares keys with it. `check` reads no MAC, so the tampered
/// header is judged rather than refused.
#[test]
fn a_duplicated_ssh_stanza_is_drift() {
    let (mut fixture, file) = roster();
    let key = fixture.ssh_recipient();
    fixture.set_audience(&file, &["alice"], &[&key]);

    let bytes = encrypted(&[&key], false);
    let mut lines: Vec<&[u8]> = bytes.split_inclusive(|&byte| byte == b'\n').collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with(b"-> ssh-ed25519 "))
        .expect("age wrote an ssh-ed25519 stanza");
    // An ssh-ed25519 stanza's body is one line of forty-three characters.
    let stanza = [lines[at], lines[at + 1]];
    lines.splice(at..at, stanza);
    commit(&fixture, &file, &lines.concat());

    let run = fixture.run(&["check"]);
    assert_eq!(run.code, Some(1), "{}", run.combined());
    run.says(&format!(
        "carries more stanzas with this tag than its audience declares keys with it:\n    - {key}\n"
    ))
    .says("    safix fix --yes\n")
    .silent_about("matched by tag only")
    .silent_about("CANARY");
}

/// An X25519 stanza names nobody, so a file holding one is unverifiable
/// however its SSH stanzas compare.
#[test]
fn an_x25519_recipient_keeps_the_file_unverifiable() {
    let (mut fixture, file) = roster();
    let key = fixture.ssh_recipient();
    let alice = fixture.alice.clone();
    fixture.set_audience(&file, &["alice"], &[&key, &alice]);
    commit(&fixture, &file, &encrypted(&[&key, &alice], false));

    let run = fixture.run(&["check"]);
    assert_eq!(run.code, Some(1), "{}", run.combined());
    run.says(&format!("Recipient roster cannot be audited: {file}"))
        .says("stanzas disclose no tag")
        .says("Use a SOPS container when inspectable recipient metadata is required.")
        .silent_about("matched by tag only")
        .silent_about("is not encrypted to the SSH recipients")
        .silent_about("CANARY");
}

/// A file that is not an age file refuses the run, naming the file and why.
#[test]
fn an_unreadable_age_header_refuses_naming_the_file() {
    let (mut fixture, file) = roster();
    let key = fixture.ssh_recipient();
    fixture.set_audience(&file, &["alice"], &[&key]);
    commit(&fixture, &file, b"not an age file\n");

    let run = fixture.run_graphical(&["check"]).expect_refusal("an unreadable header");
    assert_eq!(run.code, Some(1), "{}", run.combined());
    assert_eq!(run.refusal_code(), "recipients_unreadable");
    // The cause is rendered under the refusal and wrapped at eighty columns,
    // so its two halves are asserted apart.
    run.says(&format!("could not read the recipients of {file}"))
        .says("the age header is not readable: it does not begin with the age v1")
        .says("intro")
        .silent_about("Recipient roster");
}

/// Commit everything `fix` left, as an operator reviewing its diff would.
fn commit_all(fixture: &Fixture, message: &str) {
    fixture.git(&["add", "-A"]);
    fixture.git(&["commit", "-qm", message]);
}

/// A second `fix --yes` over an unchanged audience and unchanged bytes leaves
/// the file and its record alone, says so, and needs no `--yes`; a file
/// re-encrypted by hand to another roster is re-wrapped by the next run.
#[test]
fn fix_leaves_an_unchanged_raw_age_file_alone() {
    let (mut fixture, file) = roster();
    let alice = fixture.alice.clone();
    let bob = fixture.bob.clone();
    fixture.set_audience(&file, &["alice"], &[&alice]);
    commit(&fixture, &file, &encrypted(&[&alice], false));

    fixture
        .run(&["fix", "--yes"])
        .expect_success("the first re-wrap")
        .says(&format!("safix: re-encrypted {file} to the declared audience"));
    let wrapped = bytes_of(&fixture, &file);
    let record = fixture.read(RECORD);
    assert!(record.starts_with("safix-age-audience-v1 "), "{record}");
    commit_all(&fixture, "fixture: the first re-wrap");

    fixture
        .run(&["fix", "--yes"])
        .expect_success("the second run")
        .says(&format!(
            "safix: {file} is already wrapped to the declared audience; left alone."
        ))
        .silent_about(&format!("re-encrypted {file}"));
    assert_eq!(bytes_of(&fixture, &file), wrapped, "the file was not rewritten");
    assert_eq!(fixture.read(RECORD), record, "the record was not rewritten");

    fixture
        .run(&["fix"])
        .expect_success("a run without --yes over a file it skips")
        .says(&format!(
            "safix: {file} is already wrapped to the declared audience; left alone."
        ));
    assert_eq!(bytes_of(&fixture, &file), wrapped);

    let by_hand = encrypted(&[&alice, &bob], false);
    commit(&fixture, &file, &by_hand);
    fixture
        .run(&["fix", "--yes"])
        .expect_success("re-wrapping bytes the record does not describe")
        .says(&format!("safix: re-encrypted {file} to the declared audience"));
    let rewrapped = bytes_of(&fixture, &file);
    assert_ne!(rewrapped, by_hand, "the hand-encrypted file was re-wrapped");
    assert_ne!(fixture.read(RECORD), record, "the record names the new bytes");
    assert_eq!(
        safix_core::age_header::stanzas(&rewrapped).unwrap().len(),
        1,
        "the re-wrap is to the declared audience alone"
    );
    assert_eq!(opened(&fixture, &file), VALUE);
}

/// A changed audience is re-wrapped, and the record names the new audience and
/// the new bytes.
#[test]
fn fix_rewraps_and_records_when_the_audience_changes() {
    let (mut fixture, file) = roster();
    let alice = fixture.alice.clone();
    let bob = fixture.bob.clone();
    fixture.set_audience(&file, &["alice"], &[&alice]);
    commit(&fixture, &file, &encrypted(&[&alice], false));
    fixture.run(&["fix", "--yes"]).expect_success("the first re-wrap");
    let record = fixture.read(RECORD);
    commit_all(&fixture, "fixture: the first re-wrap");

    fixture.set_audience(&file, &["alice", "bob"], &[&alice, &bob]);
    fixture
        .run(&["fix", "--yes"])
        .expect_success("re-wrapping to the widened audience")
        .says(&format!("safix: re-encrypted {file} to the declared audience"))
        .silent_about("left alone");
    let rewrapped = bytes_of(&fixture, &file);
    assert_eq!(
        safix_core::age_header::stanzas(&rewrapped).unwrap().len(),
        2,
        "the file is wrapped to both recipients"
    );
    let now = fixture.read(RECORD);
    let audience = |line: &str| line.split(' ').nth(1).unwrap().to_owned();
    assert_ne!(audience(&now), audience(&record), "the record names the new audience");
    assert_eq!(opened(&fixture, &file), VALUE);
}

/// A record naming the file's current bytes and an audience that is no longer
/// declared is a failing finding, with the not-revocation sentence and no
/// value.
#[test]
fn a_recorded_audience_that_is_no_longer_declared_is_reported() {
    let (mut fixture, file) = roster();
    let alice = fixture.alice.clone();
    let bob = fixture.bob.clone();
    fixture.set_audience(&file, &["alice", "bob"], &[&alice, &bob]);
    commit(&fixture, &file, &encrypted(&[&alice, &bob], false));
    fixture.run(&["fix", "--yes"]).expect_success("the re-wrap that records");
    commit_all(&fixture, "fixture: the re-wrap");

    fixture.set_audience(&file, &["alice"], &[&alice]);
    let run = fixture.run(&["check"]);
    assert_eq!(run.code, Some(1), "{}", run.combined());
    run.says(&format!(
        "{file} is wrapped to an audience that is no longer declared for it."
    ))
    .says(&format!("{RECORD} records the audience"))
    .says("Only a new value revokes.")
    .says("    safix fix --yes\n")
    .silent_about("CANARY");
}
