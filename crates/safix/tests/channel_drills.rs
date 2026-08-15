//! The suite shown to fail, once per channel it observes.
//!
//! This is the retired `differential-drills` mode, and it is the reason the rest
//! of this suite is evidence rather than eighteen assertions nobody has watched
//! go red. A green suite is best at hiding exactly one thing: that it cannot
//! fail. So a deliberately damaged runtime is put in the real one's place, once
//! per channel, and each mutation has to be caught — and caught by the channel
//! it belongs to rather than incidentally by another, because a mutation caught
//! elsewhere is evidence about that other channel and none at all about this
//! one.
//!
//! The five channels are the ones the rest of the suite reads:
//!
//! - standard output, which `Run::output` and every `assert_eq!` on a value read;
//! - standard error, which `Run::says` and the refusal snapshots read;
//! - the exit status, which `expect_success` and `expect_refusal` read;
//! - the repository, which `paths_in`, `head` and `status` read;
//! - the temporary directory, which `holds_anywhere` reads.
//!
//! The exit status is the one this form has to assert deliberately. The retired
//! comparative harness got it for nothing, because two runtimes that exit
//! differently differ whether or not anybody named the channel; with one runtime
//! there is nothing to differ from unless the status is recorded and compared.
//!
//! The comparison here is not against another runtime. It is against this same
//! binary, observed on the same repository moments earlier: the drill's claim is
//! that the channels can detect a change, and a baseline taken from the subject
//! is exactly the right instrument for that. What the channels should *contain*
//! is asserted in the eighteen behavioural tests, against literals.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::path::{Path, PathBuf};

use harness::{Fixture, safix, shim};

/// The value the fixture holds, which the residue mutation leaks and which
/// therefore has to be a real one rather than a string invented by the drill.
const VALUE: &str = "CANARY-drill-value";

/// A line the runtime does not print is caught on standard output.
#[test]
fn a_line_added_to_standard_output_is_caught_on_that_channel_alone() {
    let fixture = ready();
    assert_caught(&fixture, &["list", "ana"], "stdout", "stdout");
}

/// A line the runtime does not write is caught on standard error.
///
/// Driven at a refusal, because an invocation that writes nothing to standard
/// error would let this mutation pass unnoticed and prove nothing about the
/// channel.
#[test]
fn a_line_added_to_standard_error_is_caught_on_that_channel_alone() {
    let fixture = ready();
    assert_caught(&fixture, &["list", "cy"], "stderr", "stderr");
}

/// An exit status the runtime does not return is caught on the status channel.
///
/// The channel the comparative form got for free and this one must assert. A
/// runtime that did the right thing and exited wrong would be a runtime whose
/// caller cannot tell whether it worked.
#[test]
fn a_changed_exit_status_is_caught_on_that_channel_alone() {
    let fixture = ready();
    assert_caught(&fixture, &["list", "ana"], "status", "status");
}

/// A file left in the repository is caught on the repository channel.
#[test]
fn a_file_left_in_the_repository_is_caught_on_that_channel_alone() {
    let fixture = ready();
    assert_caught(&fixture, &["list", "ana"], "effects", "effects");
}

/// A plaintext value left in the temporary directory is caught on the residue
/// channel.
///
/// The value is the one the fixture actually holds, so this is the shape of the
/// real hazard — a run that staged a secret and did not sweep it — rather than a
/// search for a string the drill made up.
#[test]
fn a_value_left_in_the_temporary_directory_is_caught_on_that_channel_alone() {
    let fixture = ready();
    assert_caught(&fixture, &["list", "ana"], "residue", "residue");
}

/// A repository with one value in it, which is what gives `list` something to
/// report and the residue mutation something real to leak.
fn ready() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .set("ana", "api-token", VALUE)
        .expect_success("the value the drills are run over");
    fixture
}

/// Run the invocation twice — once as itself, once with one channel damaged —
/// and hold the damage to showing up on that channel and on no other.
fn assert_caught(fixture: &Fixture, arguments: &[&str], mutation: &str, expected: &str) {
    let reference = observe(fixture, safix(), arguments, &[]);
    let mutated = observe(
        fixture,
        shim(),
        arguments,
        &[
            ("SAFIX_SHIM_ROLE", "mutate"),
            ("SAFIX_SHIM_TARGET", safix()),
            ("SAFIX_SHIM_MUTATION", mutation),
            ("SAFIX_SHIM_VALUE", VALUE),
        ],
    );

    let differing = differences(&reference, &mutated);
    assert!(
        differing.contains(&expected),
        "the {mutation} mutation was not caught by the {expected} channel; \
         the channels that did differ were {differing:?}"
    );
    assert_eq!(
        differing,
        vec![expected],
        "the {mutation} mutation reached further than the channel it exists to drill"
    );
}

/// What one run left on each of the five channels.
struct Channels {
    stdout: Vec<u8>,
    stderr: String,
    status: Option<i32>,
    effects: Vec<String>,
    residue: Vec<PathBuf>,
}

/// Run one invocation and read all five channels off it.
fn observe(
    fixture: &Fixture,
    program: &str,
    arguments: &[&str],
    extra: &[(&str, &str)],
) -> Channels {
    let run = fixture.run_program(program, arguments, None, extra);
    Channels {
        stdout: run.stdout,
        stderr: run.stderr,
        status: run.code,
        effects: repository(fixture),
        residue: residue(fixture),
    }
}

/// The channels on which two observations differ, in a fixed order so the
/// assertion reads the same whichever one fired.
fn differences(reference: &Channels, mutated: &Channels) -> Vec<&'static str> {
    let mut differing = Vec::new();
    if reference.stdout != mutated.stdout {
        differing.push("stdout");
    }
    if reference.stderr != mutated.stderr {
        differing.push("stderr");
    }
    if reference.status != mutated.status {
        differing.push("status");
    }
    if reference.effects != mutated.effects {
        differing.push("effects");
    }
    if reference.residue != mutated.residue {
        differing.push("residue");
    }
    differing
}

/// Everything about the repository a run could change: which files are in it,
/// what the working tree says about them, and which commit it is on.
fn repository(fixture: &Fixture) -> Vec<String> {
    let mut effects: Vec<String> = files(&fixture.repo)
        .into_iter()
        .map(|path| {
            path.strip_prefix(&fixture.repo)
                .unwrap_or(&path)
                .display()
                .to_string()
        })
        .collect();
    effects.sort();
    effects.push(format!("HEAD {}", fixture.head()));
    effects.push(format!("status {}", fixture.status()));
    effects
}

/// Every file under the run's temporary directory holding the fixture's value.
fn residue(fixture: &Fixture) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = files(&fixture.tmpdir())
        .into_iter()
        .filter(|path| std::fs::read_to_string(path).is_ok_and(|text| text.contains(VALUE)))
        .collect();
    found.sort();
    found
}

/// Every file under a directory, `.git` aside: its contents are git's business
/// and change under commands that changed nothing an operator can see.
fn files(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            found.extend(files(&path));
        } else {
            found.push(path);
        }
    }
    found
}
