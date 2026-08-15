//! The value reaches sops down a pipe, and reaches it no other way.
//!
//! This is the retired `differential-pipes` mode, which was also never a
//! comparison: it observed the sops process itself and held the value to
//! travelling down a pipe. The two channels it rules out are the two a bystander
//! on the same machine can read without any privilege at all — a process listing
//! shows argv, and `/proc/<pid>/environ` shows the environment — so the claim is
//! about disclosure to somebody who is merely present, not about an attacker.
//!
//! What stands in sops's place records the argument vector and environment it
//! was handed and then becomes the real sops. It reads them from itself rather
//! than out of `/proc`, so the observation is made the same way on every
//! platform and needs no privilege of its own.
//!
//! The run has to succeed and the value has to come back out again, or the
//! assertion would hold just as well over a runtime that sent sops nothing at
//! all.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::path::Path;

use harness::{ANA_FILE, Fixture, real_sops, shim};

/// A `set` observed at the sops process: the value is stored, and it was in
/// neither of the two channels a bystander can read.
#[test]
fn the_value_reaches_sops_in_neither_argv_nor_the_environment() {
    let fixture = Fixture::new();
    let spool = fixture.scratch("spy");
    let sops = real_sops();

    fixture
        .run_env(
            &["set", "ana", "api-token"],
            Some("CANARY-piped-value\nCANARY-piped-value\n"),
            &observed(&spool, &sops),
        )
        .expect_success("the set through the observed sops");

    // Stored, and readable again. Without this the two silences below would be
    // satisfied by a runtime that never sent sops the value in the first place.
    assert_eq!(
        fixture.value(ANA_FILE, "api-token"),
        "CANARY-piped-value",
        "the value did not round-trip, so how it travelled proves nothing"
    );

    let argv = read(&spool.join("argv"));
    let environ = read(&spool.join("environ"));

    assert!(
        argv.contains("set"),
        "the observed sops was never invoked, so nothing was observed:\n{argv}"
    );
    assert!(
        !argv.contains("CANARY-piped-value"),
        "the value was in sops' argv, where a process listing reads it"
    );
    assert!(
        !environ.contains("CANARY-piped-value"),
        "the value was in sops' environment, where /proc/<pid>/environ reads it"
    );

    // The key name is public and travels in argv on purpose. Asserting it is
    // there is what keeps the two silences above from being satisfied by a spool
    // that recorded nothing legible.
    assert!(
        argv.contains("api-token") || argv.contains("api_token"),
        "the key name was not in argv either, so the spool holds nothing to be silent about:\n{argv}"
    );
}

/// The same claim over `generate`, whose value the operator never typed.
///
/// A generator's output is minted by a script and handed to sops by the same
/// path, and a value nobody typed is exactly the one a runtime is most likely to
/// pass around carelessly.
#[test]
fn a_generated_value_reaches_sops_the_same_way() {
    let mut fixture = Fixture::new();
    let spool = fixture.scratch("spy");
    let sops = real_sops();

    fixture.seed_generator(
        "api-token",
        ANA_FILE,
        &[],
        &serde_json::json!({
            "dependencies": [], "description": "a value minted from nothing",
            "files": {}, "prompts": {}, "share": false,
            "runtimeInputs": [],
            "script": "printf 'CANARY-minted-value' > \"$out/api-token\"",
            "validation": null,
        }),
    );

    fixture
        .run_env(&["generate", "ana"], None, &observed(&spool, &sops))
        .expect_success("the generate through the observed sops");

    assert_eq!(
        fixture.value(ANA_FILE, "api-token"),
        "CANARY-minted-value",
        "the minted value did not round-trip, so how it travelled proves nothing"
    );

    let argv = read(&spool.join("argv"));
    let environ = read(&spool.join("environ"));
    assert!(
        argv.contains("api-token"),
        "the observed sops was never invoked with the key, so nothing was observed:\n{argv}"
    );
    assert!(
        !argv.contains("CANARY-minted-value"),
        "the minted value was in sops' argv, where a process listing reads it"
    );
    assert!(
        !environ.contains("CANARY-minted-value"),
        "the minted value was in sops' environment, where /proc/<pid>/environ reads it"
    );
}

/// The reading shown to find what it is looking for.
///
/// The two silences above are assertions that a string is absent, and an absence
/// is what a spool that was never written, a path that was misspelled or a
/// search that reads nothing all produce. So the same reading is pointed at a
/// spool that does hold a value, and has to find it. Without this the check
/// would stay green over a harness that observed nothing whatsoever.
#[test]
fn the_reading_that_found_no_value_finds_one_that_is_there() {
    let fixture = Fixture::new();
    let planted = fixture.scratch("planted");
    std::fs::create_dir_all(&planted).unwrap();
    std::fs::write(
        planted.join("argv"),
        "sops\nset\n--value\nCANARY-planted-value\n",
    )
    .unwrap();

    assert!(
        read(&planted.join("argv")).contains("CANARY-planted-value"),
        "the reading the pipe assertions are made of cannot see a value that is there"
    );
}

/// The environment that puts the recording sops in the runtime's way.
fn observed<'a>(spool: &'a Path, sops: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("SAFIX_SOPS", shim()),
        ("SAFIX_SHIM_ROLE", "spy"),
        ("SAFIX_SHIM_SOPS", sops),
        ("SAFIX_SHIM_SPY", spool.to_str().unwrap()),
    ]
}

/// A spool file, or an empty string when the shim never wrote one.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
