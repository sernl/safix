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
//! Each window is held to five things: the exit status the signal implies, a
//! repository identical to the one the run found, no candidate document left
//! beside the target, no definition record for a value that was never committed,
//! and the typed value present in no file under the repository or under the run's
//! temporary directory.

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

/// The five things every interrupted run is held to.
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

    // A record asserts that a value was minted under a declaration. An
    // interrupted run minted nothing it committed, so a record left behind would
    // be an assertion about a mint this repository never made — and `check` would
    // then report drift against a value that is not there.
    assert!(
        !fixture.exists("state/safix/definitions"),
        "the interrupted run left a definition record for a value it did not commit"
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

/// A signal while a generator's script is running leaves no staging root.
///
/// The window the 0.2 contract opened, and the one it is worth having a drill
/// for: from the moment the staging root is created until the run returns, a
/// directory on tmpfs holds every input and every output in the clear. `Drop`
/// covers a return, an error and a panic; a signal is not any of those, and the
/// only thing that covers it is the registration made *before* the directory
/// exists, swept from the handler.
///
/// The script sleeps so the signal has somewhere to land, and writes its output
/// first so the root is populated when it does — a sweep that skipped a
/// non-empty directory would pass over exactly the case this exists for.
///
/// Both signals, because they take different paths out: `SIGINT` exits 130 and
/// `SIGTERM` exits 143, and the sweep is what they share.
#[test]
fn a_signal_during_a_generator_leaves_no_staging_root() {
    for (signal, status) in [("INT", 130), ("TERM", 143)] {
        let mut fixture = Fixture::new();
        let before = fixture.head();

        fixture.seed_generator(
            "slow",
            ANA_FILE,
            &[],
            &serde_json::json!({
                "dependencies": [], "description": null,
                "files": {}, "prompts": {}, "share": false,
                "runtimeInputs": [],
                "script": "printf 'CANARY-mid-generation' > \"$out/slow\"; sleep 30",
                "validation": null,
            }),
        );

        let run = fixture.interrupt_after("2", signal, &["generate", "ana", "slow"], "", &[]);

        assert_eq!(
            run.code,
            Some(status),
            "a generator interrupted by SIG{signal} exited {:?}\n{}",
            run.code,
            run.combined()
        );
        assert_eq!(
            fixture.head(),
            before,
            "a generator interrupted by SIG{signal} committed"
        );
        assert!(
            fixture.staging_roots().is_empty(),
            "a generator interrupted by SIG{signal} left a staging root"
        );
        assert_pristine(&fixture, &before, "CANARY-mid-generation");
    }
}

/// A signal while a generator's script is running is not the script failing.
///
/// The window this exists for is narrow and the failure in it was quiet. A
/// script the operator interrupted is a child that died on a signal, so the
/// status the runtime waits on carries no exit code; read as an ordinary result
/// that is a failure, and the run ended with 1 and a sentence blaming the
/// generator for the operator's Ctrl-C.
///
/// Making that observable takes two things the earlier drill did not have.
///
/// The signal goes to the command alone rather than to its process group. Under
/// `timeout` the script dies at the same instant the runtime is signalled, so
/// there is no window in which a child of the runtime is still running — which
/// is exactly the window under test.
///
/// And the assertion is inside the window rather than after it. Both the fixed
/// and the broken runtime exit 130 here, because the handler thread exits with
/// that status whatever the run's own thread concludes; the exit code alone can
/// therefore never separate them. What separates them is *when* the sweep runs.
/// The fix holds the quiescence lock across the script, so the handler cannot
/// sweep until the script has been waited on; without it the handler sweeps
/// immediately and takes the staging root out from under a script that is still
/// writing into it. So the script asks, after the signal has landed, whether its
/// own output directory is still there, and records the answer where the test
/// can read it.
///
/// Observed red before the fix: the witness reads `swept`.
#[test]
fn a_signal_during_a_generator_does_not_sweep_the_root_the_script_is_using() {
    let mut fixture = Fixture::new();
    let witness = fixture.scratch("out-after-the-signal");
    let before = fixture.head();

    fixture.seed_generator(
        "slow",
        ANA_FILE,
        &[],
        &serde_json::json!({
            "dependencies": [], "description": null,
            "files": {}, "prompts": {}, "share": false,
            "runtimeInputs": [],
            // Writes its output, waits long enough for the signal to be
            // delivered and acted on, then reports whether `$out` survived. The
            // non-zero exit is what the run must *not* report: a runtime that
            // read this status before reading the interrupt would call it
            // "generator failed (exit 3)".
            "script": "printf 'CANARY-mid-generation' > \"$out/slow\"\n\
                       sleep 3\n\
                       if [ -d \"$out\" ]; then\n\
                         printf present > \"$SAFIX_TEST_WITNESS\"\n\
                       else\n\
                         printf swept > \"$SAFIX_TEST_WITNESS\"\n\
                       fi\n\
                       exit 3",
            "validation": null,
        }),
    );

    let run = fixture.interrupt_command_after(
        std::time::Duration::from_millis(700),
        rustix::process::Signal::INT,
        &["generate", "ana", "slow"],
        &[("SAFIX_TEST_WITNESS", &witness.to_string_lossy())],
    );

    assert_eq!(
        std::fs::read_to_string(&witness).unwrap_or_default(),
        "present",
        "the staging root was swept while the generator was still writing into it\n{}",
        run.combined()
    );
    assert_eq!(
        run.code,
        Some(130),
        "an interrupted generator exited {:?}\n{}",
        run.code,
        run.combined()
    );
    run.silent_about("generator 'slow' failed");
    assert_eq!(fixture.head(), before, "an interrupted generator committed");
    assert!(
        fixture.staging_roots().is_empty(),
        "an interrupted generator left a staging root"
    );
    assert_pristine(&fixture, &before, "CANARY-mid-generation");
}

/// A generator interrupted while sops holds its candidate open leaves no
/// definition record.
///
/// The window task 1.4 of `settle-clan-vars-parity` names, and the only one in
/// which the claim is not vacuous. Every window above it stops before the values
/// exist, so no record was ever due; here the script has run, the values are in
/// memory, the candidate document is staged, and the run is one rename away from
/// committing. A record that landed in this window would assert a mint whose value
/// the interrupted run did not commit, and the next `check` would report drift
/// against a value that is not there.
///
/// The signal comes from inside sops for the reason the `set` window above gives:
/// it is a fixture rather than a race. `generate` drives the same `sops set`, so
/// the same hold applies.
#[test]
fn a_generator_interrupted_during_encryption_leaves_no_definition_record() {
    let mut fixture = Fixture::new();
    let before = fixture.head();

    fixture.seed_generator(
        "recorded",
        ANA_FILE,
        &[],
        &serde_json::json!({
            "dependencies": [], "description": null,
            "files": {}, "prompts": {}, "share": false,
            "runtimeInputs": [],
            "script": "printf 'CANARY-minted-not-committed' > \"$out/recorded\"",
            "validation": null,
        }),
    );

    let sops = real_sops();
    let run = fixture.run_env(
        &["generate", "ana", "recorded"],
        None,
        &[
            ("SAFIX_SOPS", shim()),
            ("SAFIX_SHIM_ROLE", "interrupt"),
            ("SAFIX_SHIM_SOPS", &sops),
            ("SAFIX_SHIM_HOLD", "set"),
        ],
    );

    assert_eq!(run.code, Some(130), "an interrupted mint exits 130");
    assert!(
        !fixture.exists("state/safix/definitions/ana/recorded"),
        "the interrupted mint recorded a definition for a value it did not commit"
    );
    assert_pristine(&fixture, &before, "CANARY-minted-not-committed");
}

/// A signal while a validation fragment is judging a candidate is not a
/// rejection.
///
/// The second window the same reading covers, and the one whose wrong answer is
/// the more misleading of the two: a run that reported the candidate rejected
/// would be telling the operator their value was judged and found wanting, when
/// nothing judged it.
///
/// This one carries coverage rather than severity. By the time a validation
/// runs, the staging root is already gone — it belongs to the mint that produced
/// the candidate — so there is no in-window observable of the kind the test
/// above uses, and both readings exit 130. What it holds is that the run says
/// nothing about a rejection, over the window that the quiescence lock the test
/// above proves is what keeps the run's own thread the one that answers.
#[test]
fn a_signal_during_a_validation_is_not_reported_as_a_rejection() {
    let mut fixture = Fixture::new();
    let before = fixture.head();

    fixture.seed_generator(
        "judged",
        ANA_FILE,
        &[],
        &serde_json::json!({
            "dependencies": [], "description": null,
            "files": {}, "prompts": {}, "share": false,
            "runtimeInputs": [],
            "script": "printf 'CANARY-judged' > \"$out/judged\"",
            "validation": "sleep 3; exit 1",
        }),
    );

    let run = fixture.interrupt_command_after(
        std::time::Duration::from_millis(700),
        rustix::process::Signal::TERM,
        &["generate", "ana", "judged"],
        &[],
    );

    assert_eq!(
        run.code,
        Some(143),
        "a terminated validation exited {:?}\n{}",
        run.code,
        run.combined()
    );
    run.silent_about("rejected");
    assert_eq!(fixture.head(), before, "a terminated validation committed");
    assert_pristine(&fixture, &before, "CANARY-judged");
}
