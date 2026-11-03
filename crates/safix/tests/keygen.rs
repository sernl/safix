//! `keygen --kind age --hybrid`, over the real `age-keygen`.
//!
//! What is asserted is what an operator cannot check by reading the output: that
//! the identity file still holds the first identity byte for byte, that exactly
//! one hybrid identity followed it, that the recipient printed is the one that
//! identity derives, and that no private line reached either stream.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use harness::{Fixture, Run};

/// An identity file in a mode-700 directory of its own, not yet written.
fn identity_file(fixture: &Fixture) -> PathBuf {
    let directory = fixture.scratch_dir("keygen-identities");
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    directory.join("keys.txt")
}

fn keygen(fixture: &Fixture, file: &Path, arguments: &[&str]) -> Run {
    let mut full = vec!["keygen"];
    full.extend_from_slice(arguments);
    fixture.run_env(&full, None, &[("SAFIX_AGE_KEY_FILE", file.to_str().unwrap())])
}

/// What `age-keygen -y` derives from one private line.
fn recipient_of_line(line: &str) -> String {
    let mut child = Command::new("age-keygen")
        .arg("-y")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("age-keygen does not run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{line}\n").as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "age-keygen -y refused the line");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn keygen_hybrid_appends_beside_an_existing_identity_and_prints_only_the_recipient() {
    let fixture = Fixture::new();
    let file = identity_file(&fixture);

    let plain = keygen(&fixture, &file, &["--kind", "age"]).expect_success("plain keygen");
    let first = std::fs::read(&file).unwrap();
    let hybrid =
        keygen(&fixture, &file, &["--kind", "age", "--hybrid"]).expect_success("hybrid keygen");
    let after = std::fs::read(&file).unwrap();

    assert!(
        after.starts_with(&first),
        "the hybrid mint rewrote the identity already there"
    );
    let added = String::from_utf8(after[first.len()..].to_vec()).unwrap();
    let private: Vec<&str> = added
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("AGE-SECRET-KEY"))
        .collect();
    assert_eq!(private.len(), 1, "not exactly one identity was appended");
    assert!(
        private[0].starts_with("AGE-SECRET-KEY-PQ-1"),
        "the appended identity is not a hybrid one"
    );

    let recipient = recipient_of_line(private[0]);
    assert!(recipient.starts_with("age1pq1"), "not a hybrid recipient");
    hybrid.says(&format!("Public recipient: {recipient}\n"));
    for run in [&plain, &hybrid] {
        run.silent_about("AGE-SECRET-KEY");
    }

    let shown = keygen(&fixture, &file, &["--show"]).expect_success("keygen --show");
    assert_eq!(
        shown.output(),
        format!("{recipient}\n"),
        "keygen --show does not name the hybrid recipient"
    );
}

#[test]
fn keygen_hybrid_outside_the_age_profile_is_a_usage_error() {
    let fixture = Fixture::new();
    let file = identity_file(&fixture);
    keygen(&fixture, &file, &["--kind", "age"]).expect_success("plain keygen");
    let before = std::fs::read(&file).unwrap();

    for arguments in [&["--kind", "pgp", "--hybrid"][..], &["--hybrid"][..]] {
        keygen(&fixture, &file, arguments)
            .expect_refusal("--hybrid outside --kind age")
            .says("keygen --kind age [--hybrid]");
        assert_eq!(
            std::fs::read(&file).unwrap(),
            before,
            "a refused --hybrid changed the identity file"
        );
    }
}

#[test]
fn keygen_help_states_the_hybrid_cost_and_reader_floor() {
    let fixture = Fixture::new();
    let help = fixture.run(&["keygen", "-h"]).expect_success("keygen -h");
    help.says("--hybrid");
    help.says("3.12.0");
    help.says("1.3.0");
}
