//! Where `set` reads its value from, and what each side of that fork keeps.
//!
//! One question decides it: whether standard input is a terminal. A person typing
//! gets the hidden double prompt that has always been there; a program piping gets
//! its bytes stored as sent, with no prompt and no confirmation. The branch is
//! `clan vars set`'s own, so one piece of calling code scripts both commands.
//!
//! Both halves are driven for real. The piped half is an ordinary pipe on standard
//! input. The typed half is a pseudoterminal the harness allocates, because a pipe
//! now takes the stream source and there is no other way left to reach the prompt
//! path — a test that asserted the prompt path over a pipe would be asserting
//! against a source the command no longer selects.
//!
//! Every expectation here is a literal written in the test, and the bytes stored
//! are read back through `get` rather than through the writer that put them there.
//!
//! One claim about the piped source is not here and is not missing: that the value
//! reaches neither an argument vector nor an environment variable. `value_pipe.rs`
//! observes it at the sops process, and since a pipe now selects this source that
//! observation is this source's — asserting it a second time here would be a second
//! answer to a question already answered.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{ANA_FILE, Fixture};

/// A piped value is stored as its own bytes, and nothing was asked.
///
/// The trailing newline is the whole of the byte-exactness claim: `echo` pipes one
/// and `printf` does not, which is the doctrine the generator contract already
/// states, and a source that trimmed it would pass every line-wise comparison
/// while storing a different value. It is read back through `get`, whose standard
/// output is the value and nothing else, rather than through a reader that strips
/// it.
#[test]
fn a_piped_value_is_stored_as_its_own_bytes_and_nothing_is_asked() {
    let fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token", "mail-password"]);

    let run = fixture
        .run_with(&["set", "ana", "api-token"], "CANARY-piped\n")
        .expect_success("a piped value carrying a trailing newline");

    let read = fixture
        .run(&["get", "ana", "api-token"])
        .expect_success("reading the piped value back");
    assert_eq!(
        read.stdout, b"CANARY-piped\n",
        "the trailing newline the pipe carried was not stored"
    );

    // Nothing prompted, nothing announced itself, and nothing asked twice. The
    // second prompt exists to catch a value mistyped invisibly, and there was no
    // typist.
    run.silent_about("The value is not echoed");
    run.silent_about("value:");
    run.silent_about("again:");
    run.silent_about("no terminal");
    run.silent_about("CANARY-piped");

    // And the write is the ordinary one: the file the declarations name, the key
    // they name, one commit naming the secret and not the value.
    assert_eq!(
        fixture.subject("HEAD"),
        "chore(safix): set api-token for ana"
    );
    assert_eq!(fixture.paths_in("HEAD"), vec![ANA_FILE.to_owned()]);
    assert!(!fixture.message("HEAD").contains("CANARY-piped"));
    assert_eq!(fixture.status(), "", "the piped write left the tree dirty");

    // A value with no trailing newline is stored without one, which is the other
    // half of "exactly as sent".
    fixture
        .run_with(&["set", "ana", "mail-password"], "CANARY-no-newline")
        .expect_success("a piped value carrying no trailing newline");
    let read = fixture
        .run(&["get", "ana", "mail-password"])
        .expect_success("reading it back");
    assert_eq!(read.stdout, b"CANARY-no-newline");
}

/// An empty pipe is refused, and it is refused as an empty value.
///
/// The state a failed upstream command leaves behind — `set NAME < /dev/null`, or
/// a substitution whose producer exited non-zero — and the one mistake a script
/// makes that a person does not. Refused rather than stored, because a key holding
/// the empty string is indistinguishable from the placeholder a new file is created
/// with, which `check` reads as "declared, no value yet".
#[test]
fn an_empty_pipe_is_refused_as_an_empty_value() {
    let fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);
    let before = fixture.read(ANA_FILE);

    fixture
        .run_with(&["set", "ana", "api-token"], "")
        .expect_refusal("an empty pipe")
        .says("the value is empty");
    assert_eq!(
        fixture
            .run_graphical_with(&["set", "ana", "api-token"], "")
            .refusal_code(),
        "empty_value"
    );

    assert_eq!(
        fixture.read(ANA_FILE),
        before,
        "the refused empty pipe wrote the file"
    );
    assert_eq!(
        fixture.status(),
        "",
        "the refused empty pipe left the tree dirty"
    );
}

/// A terminal still gets the hidden double prompt, and both reads are really made.
///
/// The value is one line rather than the two the stream source would have stored,
/// which is what says the prompt path was taken. Then the same terminal is given
/// one line and nothing else: the run is refused for want of a confirmation, which
/// is what says the second read happens rather than being implied by the first
/// having succeeded.
#[test]
fn a_terminal_gets_the_hidden_double_prompt() {
    let fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    let run = fixture
        .set_on_a_terminal(&["set", "ana", "api-token"], "CANARY-typed\nCANARY-typed\n")
        .expect_success("a value typed twice at a terminal");
    run.says("setting api-token for ana");
    run.says("The value is not echoed");
    run.silent_about("CANARY-typed");

    let read = fixture
        .run(&["get", "ana", "api-token"])
        .expect_success("reading the typed value back");
    assert_eq!(
        read.stdout, b"CANARY-typed",
        "the terminal path stored something other than the one line typed"
    );

    let refused = fixture
        .set_on_a_terminal(&["set", "ana", "api-token"], "CANARY-typed\n")
        .expect_refusal("a value typed once at a terminal");
    refused.says("no confirmation read");
    assert_eq!(
        fixture.status(),
        "",
        "the unconfirmed value left the tree dirty"
    );
}
