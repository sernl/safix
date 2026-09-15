//! `safix view` and the nameless `safix edit`: what is offered, what is
//! decrypted, and what the terminal looks like afterwards.
//!
//! Every test here drives a real pseudoterminal through
//! [`Fixture::pick`](harness::Fixture::pick), because a picker has no seam: the
//! non-interactive path is `view <name>` itself, and a selection override would
//! be a second code path shipping to operators whose only caller is a test.
//!
//! Which check runs which test, from `modules/flake/checks/cli.nix`:
//!
//! - `safix-view-no-terminal` — `a_run_with_no_terminal_is_refused_naming_both_remedies`
//! - `safix-view-nothing-to-pick` — `a_user_holding_nothing_is_refused_rather_than_offered_an_empty_list`
//! - `safix-view-selection` — `a_typed_query_and_enter_prints_the_chosen_value`
//! - `safix-view-cancelled` — `cancelling_writes_nothing_and_restores_the_terminal`
//! - `safix-view-preview-streams` — `the_preview_never_reaches_stdout_or_stderr`
//! - `safix-edit-nameless` — `edit_with_no_name_reaches_the_editor_for_the_chosen_entry`
//!
//! and `safix-picker` in `modules/flake/checks/single-runtime.nix` runs the
//! whole target, which is what carries the interrupt and staging claims.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::path::Path;

use harness::{ALICE_FILE, Fixture, Pick, real_sops, shim};

/// A shell script standing in for the operator's editor.
///
/// `crates/safix/tests/editor.rs` writes the same script for the named form's
/// own tests; it is repeated rather than shared because the two targets compile
/// separately, and the four outcomes stay that file's claim — see its module
/// documentation for the division.
fn editor(fixture: &Fixture, name: &str, body: &str) -> String {
    let path = fixture.scratch(name);
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    format!("/bin/sh {}", path.display())
}

/// The environment that puts a recording sops in the runtime's way.
fn observed<'a>(spool: &'a Path, sops: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("SAFIX_SOPS", shim()),
        ("SAFIX_SHIM_ROLE", "spy"),
        ("SAFIX_SHIM_SOPS", sops),
        ("SAFIX_SHIM_SPY", spool.to_str().unwrap()),
    ]
}

/// How many times the recorded sops was asked to decrypt something.
fn decrypts(spool: &Path) -> usize {
    std::fs::read_to_string(spool.join("argv"))
        .unwrap_or_default()
        .lines()
        .filter(|line| *line == "decrypt")
        .count()
}

/// The keys the recorded sops was asked to decrypt, in the order it was asked.
///
/// A sequence rather than a count, because the entry a highlight moved through
/// and the entry it rested on are the same count and a different claim. Each
/// invocation is recorded as its whole argument vector, one argument per line,
/// so a `decrypt --extract ["key"]` is three consecutive lines.
fn decrypted_keys(spool: &Path) -> Vec<String> {
    let recorded = std::fs::read_to_string(spool.join("argv")).unwrap_or_default();
    let lines: Vec<&str> = recorded.lines().collect();
    lines
        .windows(3)
        .filter(|window| window[0] == "decrypt" && window[1] == "--extract")
        .filter_map(|window| {
            window[2]
                .strip_prefix("[\"")
                .and_then(|index| index.strip_suffix("\"]"))
                .map(str::to_owned)
        })
        .collect()
}

/// The lines a picker drew, with the terminal decoration taken off.
///
/// The frame is one string of lines: a status line, the query line, then the
/// aligned table, whose highlighted line is wrapped in the reverse-video pair.
/// Only that wrapping and the screen-clearing sequence the frame opens with are
/// removed, so what a caller compares is the picker's own bytes.
fn drawn_lines(drawn: &str) -> Vec<String> {
    drawn
        .lines()
        .map(|line| {
            line.trim_start_matches("\x1b[?1049h")
                .trim_start_matches("\x1b[H\x1b[2J")
                .trim_start_matches("\x1b[7m")
                .trim_end_matches("\x1b[0m")
                .to_owned()
        })
        .collect()
}

/// Three values distinct enough that a rendering can be told from its
/// neighbours.
fn three_values(fixture: &Fixture) {
    fixture
        .set("alice", "api-token", "VALUE-FOR-THE-API-TOKEN")
        .expect_success("seeding the api token");
    fixture
        .set("alice", "mail-password", "VALUE-FOR-THE-MAIL-PASSWORD")
        .expect_success("seeding the mail password");
    fixture
        .set("alice", "aliased-secret", "VALUE-FOR-THE-ALIAS")
        .expect_success("seeding the aliased secret");
}

/// No terminal at all is a refusal, and it names both ways round it.
#[test]
fn a_run_with_no_terminal_is_refused_naming_both_remedies() {
    let fixture = Fixture::new();
    // Pipes on all three streams and no controlling terminal, which is also
    // exactly what a check's sandbox provides.
    let run = fixture
        .run_graphical_with(&["view"], "")
        .expect_refusal("a picker with no terminal");
    assert_eq!(run.refusal_code(), "picker_needs_terminal");
    run.says("safix view");
    run.says("safix list");
}

/// Nothing to choose from is its own refusal, naming the user.
#[test]
fn a_user_holding_nothing_is_refused_rather_than_offered_an_empty_list() {
    let mut fixture = Fixture::new();
    fixture.seed_holder_of_nothing("carol");
    let picked = fixture.pick(&Pick {
        arguments: &["view", "carol"],
        graphical: true,
        ..Pick::default()
    });
    let run = picked.run.expect_refusal("a user who holds nothing");
    assert_eq!(run.refusal_code(), "nothing_to_pick");
    run.says("carol");
}

/// A query narrowing to one entry, and enter, reads that one.
#[test]
fn a_typed_query_and_enter_prints_the_chosen_value() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&["mail\r"]],
        ..Pick::default()
    });
    assert!(
        picked.run.succeeded(),
        "choosing an entry was refused, exit {:?}\n{}",
        picked.run.code,
        picked.drawn
    );
    assert!(
        picked.drawn.contains("VALUE-FOR-THE-MAIL-PASSWORD"),
        "the chosen entry's value never reached the terminal\n{}",
        picked.drawn
    );
    for other in ["VALUE-FOR-THE-API-TOKEN", "VALUE-FOR-THE-ALIAS"] {
        assert!(
            !picked.drawn.contains(other),
            "an entry that was not chosen was decrypted anyway: {other}"
        );
    }
}

/// Leaving writes nothing, keeps nothing, and puts the terminal back.
#[test]
fn cancelling_writes_nothing_and_restores_the_terminal() {
    let fixture = Fixture::new();
    three_values(&fixture);
    let settled = fixture.head();
    let roots = fixture.staging_roots();

    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&["\x1b"]],
        graphical: true,
        ..Pick::default()
    });
    assert_eq!(picked.run.code, Some(1), "leaving the picker exited oddly");
    assert_eq!(picked.run.refusal_code(), "selection_cancelled");
    assert!(
        picked.restored,
        "the terminal was left in the mode the picker put it in"
    );
    assert!(
        fixture.scratch_files().is_empty(),
        "leaving the picker left a candidate document behind"
    );
    assert_eq!(fixture.head(), settled, "leaving the picker committed");
    assert_eq!(
        fixture.staging_roots(),
        roots,
        "leaving the picker left a staging root behind"
    );
}

/// An interrupted picker exits 130 with the terminal back.
///
/// The signal path is the one no `Drop` covers: the handler ends the process
/// with `std::process::exit`, so the restore has to be in the handler itself.
#[test]
fn an_interrupt_mid_picker_restores_the_terminal_and_exits_130() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        interrupt: Some(("1", "INT")),
        ..Pick::default()
    });
    assert_eq!(
        picked.run.code,
        Some(130),
        "an interrupted picker did not exit 130\n{}",
        picked.run.stderr
    );
    assert!(
        picked.restored,
        "an interrupted picker left the terminal in raw mode"
    );
}

/// The preview is drawn on the terminal and on nothing else.
#[test]
fn the_preview_never_reaches_stdout_or_stderr() {
    let fixture = Fixture::new();
    three_values(&fixture);

    // The terminal is standard input alone, so both other streams are pipes
    // this test can read.
    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x1b"]],
        ..Pick::default()
    });
    // The first entry alice holds, which is the one the highlight rested on.
    let previewed = "VALUE-FOR-THE-ALIAS";
    assert!(
        picked.drawn.contains(previewed),
        "nothing was previewed, so this test would hold over a runtime that \
         previews nothing\n{}",
        picked.drawn
    );
    assert!(
        !picked.run.output().contains(previewed),
        "the preview reached standard output"
    );
    assert!(
        !picked.run.stderr.contains(previewed),
        "the preview reached standard error"
    );
}

/// `--no-preview` decrypts nothing at all until a choice is made.
#[test]
fn no_preview_decrypts_nothing_until_a_choice_is_made() {
    let fixture = Fixture::new();
    three_values(&fixture);
    let spool = fixture.scratch("no-preview-spool");
    let sops = real_sops();

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&[], &["\x1b"]],
        extra: &observed(&spool, &sops),
        ..Pick::default()
    });
    assert_eq!(picked.run.code, Some(1), "leaving the picker exited oddly");
    assert_eq!(
        decrypts(&spool),
        0,
        "a suppressed preview decrypted something anyway"
    );
}

/// Moving decrypts nothing; resting decrypts one thing.
///
/// The movements arrive as separate reads less than the quiet period apart,
/// which is what leaves the quiet period as the only thing holding the
/// moved-through entry back. Two movements in one read would be held apart by
/// the read boundary instead — the draw loop settles once per read — and this
/// test would then hold over a runtime with no quiet period at all; that is
/// 7.18 in this change's tasks.
#[test]
fn moving_through_entries_without_pausing_decrypts_only_where_the_highlight_rests() {
    let fixture = Fixture::new();
    three_values(&fixture);
    let spool = fixture.scratch("movement-spool");
    let sops = real_sops();

    // A rest before anything is typed, so the picker is reading by the time the
    // movements arrive and each of them is a read of its own; then the two
    // movements, one write each and no rest between them; then the group's own
    // rest, which is the highlight coming to rest on `mail-password`; then
    // leaving.
    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x1b[B", "\x1b[B"], &["\x1b"]],
        extra: &observed(&spool, &sops),
        ..Pick::default()
    });
    assert_eq!(picked.run.code, Some(1), "leaving the picker exited oddly");
    // alice's entries in the order the placements carry them: the highlight
    // starts on `aliased-secret`, whose key is `custom-key`, moves through
    // `api-token` without resting, and comes to rest on `mail-password`.
    assert_eq!(
        decrypted_keys(&spool),
        vec!["custom-key".to_owned(), "mail-password".to_owned()],
        "an entry was decrypted somewhere other than where the highlight \
         rested\n{}",
        picked.drawn
    );
}

/// The nameless `edit` reaches the editor for the entry that was chosen.
#[test]
fn edit_with_no_name_reaches_the_editor_for_the_chosen_entry() {
    let fixture = Fixture::new();
    fixture
        .set("alice", "api-token", "the-original")
        .expect_success("the value being edited");
    let seen = fixture.scratch("edited-path");
    let script = editor(
        &fixture,
        "choose",
        &format!(
            "printf '%s' \"$1\" > {}\nprintf 'the-chosen-edit' > \"$1\"",
            seen.display()
        ),
    );

    let picked = fixture.pick(&Pick {
        arguments: &["edit"],
        keystrokes: &[&["api\r"]],
        extra: &[("EDITOR", &script)],
        ..Pick::default()
    });
    assert!(
        picked.run.succeeded(),
        "editing the chosen entry was refused, exit {:?}\n{}{}",
        picked.run.code,
        picked.run.stderr,
        picked.drawn
    );
    let staged = std::fs::read_to_string(&seen).expect("the editor recorded no path");
    assert!(
        staged.contains("api-token"),
        "the editor was handed {staged}, which is not the chosen entry's buffer"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "the-chosen-edit",
        "the chosen entry's edited value did not land"
    );
}

/// Neither editor variable set is refused before anything is offered.
#[test]
fn edit_with_no_editor_refuses_before_offering_anything() {
    let fixture = Fixture::new();
    three_values(&fixture);

    // The fixture removes both variables itself, so this run has neither.
    let picked = fixture.pick(&Pick {
        arguments: &["edit"],
        keystrokes: &[&["\x1b"]],
        graphical: true,
        ..Pick::default()
    });
    let code = picked.run.refusal_code();
    assert_eq!(code, "no_editor", "the missing editor was not what refused");
    for offered in ["api-token", "mail-password", "NAME"] {
        assert!(
            !picked.drawn.contains(offered),
            "a list was drawn before the editor was settled: {offered}"
        );
    }

    // The ordering, read off the refusal a user who holds nothing gets: with
    // the editor settled first this is still the missing editor, and with the
    // selection first it would be `safix::nothing_to_pick`. The code is the
    // evidence of the order.
    let mut holder = Fixture::new();
    holder.seed_holder_of_nothing("carol");
    let empty = holder.pick(&Pick {
        arguments: &["edit", "carol"],
        keystrokes: &[&["\x1b"]],
        graphical: true,
        ..Pick::default()
    });
    assert_eq!(
        empty.run.refusal_code(),
        "no_editor",
        "the selection was offered before the editor was settled"
    );
}

/// Editing is never offered a public output, and reading still is.
#[test]
fn edit_never_offers_a_public_output() {
    let mut fixture = Fixture::new();
    fixture.seed_public_output("published-token", "public/published-token");
    fixture
        .set("alice", "api-token", "the-editable-one")
        .expect_success("an ordinary entry beside the public one");

    let editing = fixture.pick(&Pick {
        arguments: &["edit", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        extra: &[("EDITOR", &editor(&fixture, "never-run", "true"))],
        ..Pick::default()
    });
    assert!(
        !editing.drawn.contains("published-token"),
        "editing offered a public output\n{}",
        editing.drawn
    );
    assert!(
        editing.drawn.contains("api-token"),
        "editing offered nothing at all, so the absence above says nothing\n{}",
        editing.drawn
    );

    let reading = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        ..Pick::default()
    });
    assert!(
        reading.drawn.contains("published-token"),
        "reading stopped offering the public output too\n{}",
        reading.drawn
    );
}

/// A preview stages nothing, because there is no path to hand anybody.
#[test]
fn nothing_is_staged_by_a_preview() {
    let fixture = Fixture::new();
    three_values(&fixture);
    let before = fixture.staging_roots();

    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x1b"]],
        ..Pick::default()
    });
    assert!(
        picked.drawn.contains("VALUE-FOR-THE-ALIAS"),
        "nothing was previewed, so this test would hold over a runtime that \
         previews nothing\n{}",
        picked.drawn
    );
    assert_eq!(
        fixture.staging_roots(),
        before,
        "a preview left a staging root behind"
    );
    assert!(
        fixture.scratch_files().is_empty(),
        "a preview left a file beside a target"
    );
}

/// The row the picker draws for an entry is the line `list` prints for it.
///
/// Read off the terminal rather than out of `render`, because that is where the
/// claim lives: the picker's unit test holds `render::listing` against
/// `render::listing_row`, which are both `render`'s own, so a row built inside
/// the picker instead of by `list`'s builder stays green there. This compares
/// the bytes an operator sees against the bytes `safix list` prints, which is
/// 3.15 in this change's tasks.
#[test]
fn the_row_the_picker_draws_is_the_line_list_prints() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let listed = fixture
        .run(&["list", "alice"])
        .expect_success("listing what alice holds");
    // No preview, so the run decrypts nothing to draw one frame; the picker
    // offers every entry `list` prints, so both tables are aligned over the
    // same rows and a column width is not a difference between them.
    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        ..Pick::default()
    });
    let drawn = drawn_lines(&picked.drawn);

    for name in ["aliased-secret", "api-token", "mail-password", "wifi-psk"] {
        let printed = listed
            .output()
            .lines()
            .find(|line| line.split_whitespace().next() == Some(name))
            .map_or_else(
                || panic!("`list` printed no line for {name}"),
                str::to_owned,
            );
        assert!(
            drawn.contains(&printed),
            "the picker's row for {name} is not the line `list` prints\n\
             list:   {printed:?}\npicker: {:?}",
            drawn
                .iter()
                .filter(|line| line.contains(name))
                .collect::<Vec<&String>>()
        );
    }
}
