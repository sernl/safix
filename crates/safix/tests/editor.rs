//! `safix edit`: which editor is chosen, and what each of the four outcomes
//! writes.
//!
//! The editor is a shell script this file writes, which is what makes the four
//! outcomes fixtures rather than a person's keystrokes. Each is held to three
//! things: what the entry holds afterwards, whether anything was committed, and
//! that no staging root survived the run — because the buffer is plaintext in a
//! directory, and a run that refused is exactly the one most likely to leave it.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::path::Path;

use harness::{ANA_FILE, Fixture};

/// A shell script standing in for the operator's editor.
///
/// `EDITOR` is split on whitespace and executed directly rather than through a
/// shell, so this arrives as the program `sh` with the script as its first
/// argument and the staged path appended as its second — which is also the
/// assertion that the splitting works, since a runtime that handed the whole
/// string to a shell would reach the same script by a different route and a
/// runtime that did not split at all would fail to find a program named
/// `sh <script>`.
fn editor(fixture: &Fixture, name: &str, body: &str) -> String {
    let path = fixture.scratch(name);
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    format!("/bin/sh {}", path.display())
}

/// No staging root outlived the run, whichever way it ended.
fn no_buffer_left(fixture: &Fixture, before: &[std::path::PathBuf], what: &str) {
    assert_eq!(
        fixture.staging_roots(),
        before,
        "{what} left the editor's buffer behind"
    );
}

#[test]
fn the_four_outcomes_of_an_edit_write_what_each_is_supposed_to() {
    let fixture = Fixture::new();
    fixture
        .set("ana", "api-token", "the-original")
        .expect_success("the value being edited");
    let settled = fixture.head();

    // 1. The editor refuses. Nothing is written, nothing is committed, and the
    //    refusal names the status rather than reporting an empty value — the two
    //    have different causes and different fixes.
    let roots = fixture.staging_roots();
    let refused = fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[("EDITOR", &editor(&fixture, "refuse", "exit 3"))],
        )
        .expect_refusal("an editor that exited non-zero");
    refused.says("the editor exited 3");
    assert_eq!(fixture.value(ANA_FILE, "api-token"), "the-original");
    assert_eq!(fixture.head(), settled, "a failed editor committed");
    no_buffer_left(&fixture, &roots, "a failed editor");

    // 2. The buffer comes back byte-identical. sops re-encrypts with a fresh
    //    nonce, so a runtime that wrote anyway would produce a commit whose
    //    ciphertext differs and whose value does not — which is why this is
    //    decided before the write rather than left to git.
    let roots = fixture.staging_roots();
    fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[("EDITOR", &editor(&fixture, "leave", "true"))],
        )
        .expect_success("an editor that changed nothing")
        .says("unchanged");
    assert_eq!(fixture.value(ANA_FILE, "api-token"), "the-original");
    assert_eq!(fixture.head(), settled, "an unchanged buffer committed");
    no_buffer_left(&fixture, &roots, "an unchanged edit");

    // 3. The buffer comes back empty, which is the state a truncated write
    //    leaves behind, so it takes the same refusal an empty value takes
    //    anywhere else.
    let roots = fixture.staging_roots();
    let emptied = fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[("EDITOR", &editor(&fixture, "empty", ": > \"$1\""))],
        )
        .expect_refusal("an editor that emptied the buffer");
    emptied.says("empty");
    assert_eq!(fixture.value(ANA_FILE, "api-token"), "the-original");
    assert_eq!(fixture.head(), settled, "an emptied buffer committed");
    no_buffer_left(&fixture, &roots, "an emptied edit");

    // 4. Changed and non-empty, which is the one that writes. It goes through
    //    `set`'s own path, so the commit names the file and not the value.
    let roots = fixture.staging_roots();
    fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[(
                "EDITOR",
                &editor(&fixture, "change", "printf 'the-edited-value' > \"$1\""),
            )],
        )
        .expect_success("an editor that changed the value");
    assert_eq!(fixture.value(ANA_FILE, "api-token"), "the-edited-value");
    assert_ne!(
        fixture.head(),
        settled,
        "a changed buffer committed nothing"
    );
    assert!(
        !fixture
            .git(&["log", "--format=%s%n%b"])
            .contains("the-edited-value"),
        "a commit message carries the value"
    );
    no_buffer_left(&fixture, &roots, "a completed edit");
    assert!(
        fixture.scratch_files().is_empty(),
        "an edit left a candidate document beside the target"
    );
}

/// The editor safix will not choose for you.
#[test]
fn the_editor_is_the_one_the_operator_named_or_the_run_refuses() {
    let fixture = Fixture::new();

    // Neither variable set: refused before anything is decrypted or staged, and
    // the refusal names both. safix adds no fallback program, because a program
    // the operator did not pick, holding their plaintext, can be left or saved
    // by accident and nothing here can tell which happened.
    let roots = fixture.staging_roots();
    let refused = fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[("EDITOR", ""), ("VISUAL", "")],
        )
        .expect_refusal("no editor named");
    refused.says("$VISUAL");
    refused.says("$EDITOR");
    no_buffer_left(&fixture, &roots, "a run with no editor");

    // The preferred variable wins over the other.
    let marker = fixture.scratch("which-editor");
    // Records which of the two was run, and writes a value, so the run reaches
    // the write rather than the empty-value refusal.
    let record = |name: &str| {
        editor(
            &fixture,
            name,
            &format!(
                "printf '{name}' > {}\nprintf 'chosen-by-{name}' > \"$1\"",
                marker.display()
            ),
        )
    };
    fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[
                ("EDITOR", &record("fallback")),
                ("VISUAL", &record("visual")),
            ],
        )
        .expect_success("both variables set");
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap_or_default(),
        "visual",
        "the editor variable was preferred over the visual one"
    );

    // An entry that holds no value yet opens on an empty buffer, so `edit` is an
    // authoring verb as well as an amending one.
    let authored = editor(&fixture, "author", "printf 'authored-here' > \"$1\"");
    fixture
        .run_env(&["edit", "ana", "wifi-psk"], None, &[("EDITOR", &authored)])
        .expect_success("authoring an entry that held nothing");
    assert_eq!(
        fixture.value(harness::SHARED_FILE, "wifi-psk"),
        "authored-here"
    );
}

/// The path reaches the editor's argument vector, and the value does not.
///
/// The staged path is an argument on purpose and is stated as one; what must
/// never be there is the value. Read out of the editor's own `/proc` entry
/// rather than out of a process listing, so the observation needs no privilege
/// and no timing.
#[test]
fn the_editor_receives_the_path_and_never_the_value() {
    let fixture = Fixture::new();
    fixture
        .set("ana", "api-token", "CANARY-edited-value")
        .expect_success("the value being edited");

    let spool = fixture.scratch("argv");
    let observer = editor(
        &fixture,
        "observe",
        &format!(
            "tr '\\0' '\\n' < /proc/$$/cmdline > {}\nprintf 'CANARY-replacement' > \"$1\"",
            spool.display()
        ),
    );

    fixture
        .run_env(
            &["edit", "ana", "api-token"],
            None,
            &[("EDITOR", &observer)],
        )
        .expect_success("the observed edit");

    let argv = std::fs::read_to_string(&spool).unwrap_or_default();
    assert!(
        !argv.is_empty(),
        "the observing editor recorded nothing, so the assertion is vacuous"
    );
    assert!(
        argv.lines().any(|line| Path::new(line.trim())
            .file_name()
            .is_some_and(|name| name == "api-token")),
        "the staged path was not in the editor's argv, so nothing was observed:\n{argv}"
    );
    assert!(
        !argv.contains("CANARY-edited-value"),
        "the value was in the editor's argv, where a process listing reads it:\n{argv}"
    );
}
