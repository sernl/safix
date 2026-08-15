//! What an interrupted write leaves behind, which is nothing.
//!
//! This is the retired `differential-abort` mode re-expressed as what it always
//! was. It never compared two runtimes — the shell runtime acted on no signal in
//! any of these windows — so it loses nothing by driving one, and gains the
//! ability to say what the runtime must do rather than that both did the same.
//!
//! A write has four windows a signal can arrive in: waiting for the value,
//! waiting for the confirmation, waiting for either under a signal that is not
//! `SIGINT`, and waiting for sops while it holds the candidate document open.
//! The last is the one the whole scratch discipline exists for, and it is a
//! fixture rather than a race here: sops is a shim that signals the runtime
//! itself, from inside the window, and then finishes normally.
//!
//! Each window is held to four things: the exit status the signal implies, a
//! repository identical to the one the run found, no candidate document left
//! beside the target, and the typed value present in no file under the
//! repository or under the run's temporary directory.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{ANA_FILE, Fixture, SHARED_FILE, real_sops, shim};

/// The directory the shared file would need, which no run before this one has
/// made.
const SHARED_DIR: &str = "secrets/safix/shared/ana,bo";

/// `SIGINT` while the value is being waited for.
///
/// The candidate document already exists at this point — `set` creates it before
/// it asks, so that a file it cannot create is refused before the operator
/// types — which is what makes "no candidate is left behind" a claim about a
/// sweep rather than about a file that was never made.
#[test]
fn a_signal_at_the_value_prompt_leaves_the_repository_as_it_found_it() {
    let fixture = Fixture::new();
    let before = fixture.head();

    let run = fixture.interrupt_after("2", "INT", &["set", "ana", "wifi-psk"], "", &[]);

    assert_eq!(run.code, Some(130), "an interrupted run exits 130");
    assert_pristine(&fixture, &before, "CANARY-never-typed");
    assert!(
        !fixture.exists(SHARED_DIR),
        "the directory the run made for the file it did not write survived it"
    );
}

/// `SIGINT` while the confirmation is being waited for.
///
/// The value has been read by this point and is in the runtime's memory, which
/// is the difference between this window and the one above: what must not
/// survive is something the run actually holds.
#[test]
fn a_signal_at_the_confirmation_leaves_no_trace_of_the_value_already_typed() {
    let fixture = Fixture::new();
    let before = fixture.head();

    let run = fixture.interrupt_after(
        "2",
        "INT",
        &["set", "ana", "wifi-psk"],
        "CANARY-typed-once\n",
        &[],
    );

    assert_eq!(run.code, Some(130), "an interrupted run exits 130");
    run.silent_about("CANARY-typed-once");
    assert_pristine(&fixture, &before, "CANARY-typed-once");
}

/// `SIGTERM`, which is the signal a supervisor sends and which must be answered
/// the same way with the status that names it.
#[test]
fn a_termination_is_answered_the_same_way_and_exits_143() {
    let fixture = Fixture::new();
    let before = fixture.head();

    let run = fixture.interrupt_after("2", "TERM", &["set", "ana", "wifi-psk"], "", &[]);

    assert_eq!(run.code, Some(143), "a terminated run exits 143");
    assert_pristine(&fixture, &before, "CANARY-never-typed");
}

/// `SIGINT` while sops holds the candidate document open.
///
/// The window the scratch discipline exists for. The run has to wait for its
/// child before it can sweep, and has to stop before the rename: a candidate
/// that was fully written and then renamed would be a value committed by a run
/// the operator interrupted.
///
/// The target already holds a value, so the claim is not merely that nothing was
/// created — it is that the file's bytes are the ones the run found and the key
/// still reads back the value it held.
///
/// The signal comes from inside sops rather than from a timer, which is what
/// makes this a fixture rather than a race: sops settles into being waited on,
/// signals the runtime alone, stays running for a moment, and then does the real
/// work and exits normally. So the status under test is the run's own decision
/// about having been interrupted, not a report of a child that died.
///
/// Both answers are on standard input before the run starts, so a run that did
/// not stop here would commit the second value; the exit status and the
/// unchanged value are what rule that out. Neutralizing the signal was observed
/// to fail here, with the run exiting 0.
#[test]
fn a_signal_during_encryption_stops_before_the_rename() {
    let fixture = Fixture::new();
    fixture
        .set("ana", "api-token", "CANARY-first-value")
        .expect_success("the value the interrupted run must not replace");

    let before = fixture.head();
    let untouched = fixture.read(ANA_FILE);

    let sops = real_sops();
    let run = fixture.run_env(
        &["set", "ana", "api-token"],
        Some("CANARY-second-value\nCANARY-second-value\n"),
        &[
            ("SAFIX_SOPS", shim()),
            ("SAFIX_SHIM_ROLE", "interrupt"),
            ("SAFIX_SHIM_SOPS", &sops),
            // Only in front of `sops set`, which is the invocation that holds
            // the candidate open. Sent in front of every invocation, the signal
            // would land in whichever window the run reached first.
            ("SAFIX_SHIM_HOLD", "set"),
        ],
    );

    assert_eq!(run.code, Some(130), "an interrupted run exits 130");
    assert_eq!(
        fixture.read(ANA_FILE),
        untouched,
        "the interrupted run changed the file it was writing into"
    );
    assert_eq!(
        fixture.value(ANA_FILE, "api-token"),
        "CANARY-first-value",
        "the interrupted run replaced the value that was there"
    );
    assert_pristine(&fixture, &before, "CANARY-second-value");
}

/// The four things every interrupted run is held to.
fn assert_pristine(fixture: &Fixture, before: &str, value: &str) {
    assert_eq!(
        fixture.head(),
        before,
        "the interrupted run committed something"
    );
    assert_eq!(
        fixture.status(),
        "",
        "the interrupted run left the working tree dirty"
    );
    assert!(
        !fixture.exists(SHARED_FILE),
        "the interrupted run wrote the file"
    );

    let candidates = fixture.scratch_files();
    assert!(
        candidates.is_empty(),
        "the interrupted run left a candidate document behind: {candidates:?}"
    );

    if let Some(path) = fixture.holds_anywhere(value) {
        panic!("the interrupted run left the value in {}", path.display());
    }
}
