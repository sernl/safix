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
        .filter(|line| matches!(*line, "decrypt" | "--decrypt"))
        .count()
}

/// Every frame the picker drew, in the order it drew them.
///
/// A frame begins by homing the cursor and clearing the screen, which is what
/// separates one from the next. The last of them is what was on screen when the
/// run ended, and it is the only one a claim about what an operator sees can be
/// made against: every earlier frame is a keystroke ago.
fn frames(drawn: &str) -> Vec<String> {
    drawn
        .split("\x1b[H\x1b[2J")
        .skip(1)
        .map(str::to_owned)
        .collect()
}

/// The last frame's lines, with the decoration taken off.
///
/// Every escape sequence goes; a sequence that positions the cursor becomes a
/// line break, because that is what it does on the terminal — the query and the
/// key help are written at absolute line numbers rather than after a newline.
fn lines(drawn: &str) -> Vec<String> {
    let frame = frames(drawn).pop().unwrap_or_default();
    let mut plain = String::new();
    let mut rest = frame.as_str();
    while let Some(at) = rest.find('\x1b') {
        plain.push_str(&rest[..at]);
        let tail = &rest[at.saturating_add(1)..];
        let (end, final_byte) = tail
            .char_indices()
            .find(|(_, character)| character.is_ascii_alphabetic())
            .map_or((tail.len(), ' '), |(index, character)| {
                (index.saturating_add(character.len_utf8()), character)
            });
        if final_byte == 'H' {
            plain.push('\n');
        }
        rest = &tail[end..];
    }
    plain.push_str(rest);
    plain
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The line of the last frame that begins with this name, or a failure naming
/// what was drawn instead.
fn row(drawn: &str, name: &str) -> String {
    let lines = lines(drawn);
    lines
        .iter()
        .find(|line| line.split_whitespace().next() == Some(name))
        .map_or_else(
            || panic!("no row for {name} was drawn\n{lines:#?}"),
            String::clone,
        )
}

/// Whether the cursor is on the row for this name.
///
/// The cursor is reverse video drawn over whatever colour the row already has,
/// so the name is one optional colour sequence behind the reverse-video one.
fn cursor_on(frame: &str, name: &str) -> bool {
    frame.split("\x1b[7m").skip(1).any(|after| {
        let body = after
            .strip_prefix("\x1b[33m")
            .or_else(|| after.strip_prefix("\x1b[36m"))
            .unwrap_or(after);
        body.starts_with(name)
    })
}

/// Where the picker's settings file is.
fn settings_path(fixture: &Fixture) -> std::path::PathBuf {
    fixture.picker_state().join("safix").join("picker.json")
}

/// What the picker's settings file holds, or an empty string when there is
/// none.
fn settings(fixture: &Fixture) -> String {
    std::fs::read_to_string(settings_path(fixture)).unwrap_or_default()
}

/// The permission bits the settings file carries.
fn settings_mode(fixture: &Fixture) -> u32 {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::metadata(settings_path(fixture))
        .expect("the settings file was never written")
        .permissions()
        .mode()
        & 0o777
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
/// test would then hold over a runtime with no quiet period at all.
#[test]
fn moving_through_entries_without_pausing_decrypts_only_where_the_cursor_rests() {
    let fixture = Fixture::new();
    three_values(&fixture);
    let spool = fixture.scratch("movement-spool");
    let sops = real_sops();

    // A rest before anything is typed, so the picker is reading by the time the
    // movements arrive and each of them is a read of its own; then the two
    // movements, one write each and no rest between them; then the group's own
    // rest, which is the cursor coming to rest on `mail-password`; then
    // leaving. Up, because the cursor starts on the bottom row — the
    // alphabetically first entry — and up is towards the top of the screen,
    // which is later in the alphabet.
    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x1b[A", "\x1b[A"], &["\x1b"]],
        extra: &observed(&spool, &sops),
        ..Pick::default()
    });
    assert_eq!(picked.run.code, Some(1), "leaving the picker exited oddly");
    // alice's entries alphabetically: the cursor starts on `aliased-secret`,
    // whose key is `custom-key`, moves through `api-token` without resting, and
    // comes to rest on `mail-password`.
    assert_eq!(
        decrypts(&spool),
        2,
        "moving through an entry triggered an extra decryption\n{}",
        picked.drawn
    );
    assert!(picked.drawn.contains("VALUE-FOR-THE-ALIAS"));
    assert!(picked.drawn.contains("VALUE-FOR-THE-MAIL-PASSWORD"));
    assert!(!picked.drawn.contains("VALUE-FOR-THE-API-TOKEN"));
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
/// the bytes an operator sees against the bytes `safix list` prints.
///
/// Tab first, because `list` prints all eight columns and the picker keeps the
/// eighth behind tab: the two tables are the same row only with it showing.
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
        keystrokes: &[&["\t"], &["\x1b"]],
        ..Pick::default()
    });

    for name in ["aliased-secret", "api-token", "mail-password", "wifi-psk"] {
        let printed = listed
            .output()
            .lines()
            .find(|line| line.split_whitespace().next() == Some(name))
            .map_or_else(
                || panic!("`list` printed no line for {name}"),
                str::to_owned,
            );
        assert_eq!(
            row(&picked.drawn, name),
            printed.trim_end(),
            "the picker's row for {name} is not the line `list` prints"
        );
    }
}

/// The table is drawn bottom-up, and the cursor starts on its bottom row.
///
/// Which is the alphabetically first entry: the list grows upwards from the
/// query being typed, so the row a person is looking at when the picker opens
/// is the one nearest their hands.
#[test]
fn the_bottom_row_is_the_first_entry_and_carries_the_cursor() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        ..Pick::default()
    });
    let drawn = lines(&picked.drawn);
    let at = |name: &str| {
        drawn
            .iter()
            .position(|line| line.split_whitespace().next() == Some(name))
            .unwrap_or_else(|| panic!("no row for {name}\n{drawn:#?}"))
    };
    assert!(
        at("NAME") < at("wifi-psk"),
        "the header is not above the rows\n{drawn:#?}"
    );
    assert!(
        at("wifi-psk") < at("mail-password")
            && at("mail-password") < at("api-token")
            && at("api-token") < at("aliased-secret"),
        "the rows are not in reverse alphabetical order\n{drawn:#?}"
    );
    assert!(
        at("aliased-secret") < at(">"),
        "the bottom row is not against the query line\n{drawn:#?}"
    );
    // The cursor is reverse video, and it is on that bottom row.
    let frame = frames(&picked.drawn).pop().unwrap_or_default();
    assert!(
        cursor_on(&frame, "aliased-secret"),
        "the cursor is not on the alphabetically first entry\n{frame:?}"
    );
    assert!(
        !cursor_on(&frame, "mail-password"),
        "a second row carries the cursor\n{frame:?}"
    );
}

/// The last line is the key help, and it names every key that does something.
#[test]
fn the_last_line_is_the_key_help() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        ..Pick::default()
    });
    let drawn = lines(&picked.drawn);
    let last = drawn.last().map_or_else(String::new, String::clone);
    for key in [
        "Enter choose",
        "Esc/^C cancel",
        "move",
        "scroll",
        "Tab columns",
        "^P preview",
    ] {
        assert!(
            last.contains(key),
            "the key help does not name {key}\n{last:?}"
        );
    }
}

/// The horizontal arrows scroll the columns and never leave.
///
/// Both directions, past both ends: a picker that read either of them as
/// leaving would take a keystroke someone reached for by accident as a decision
/// to abandon the choice.
#[test]
fn scrolling_the_columns_never_leaves_the_picker() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[
            // Left at the first column, then right past the last, then back.
            &["\x1b[D", "\x1b[D", "\x1b[C", "\x1b[C", "\x1b[C", "\x1b[C"],
            &["\x1b[D", "\x1b[D", "\x1b[D", "\x1b[D", "\x1b[D", "\x1b[D"],
            &["mail\r"],
        ],
        ..Pick::default()
    });
    assert!(
        picked.run.succeeded(),
        "the picker did not survive the arrows, exit {:?}\n{}",
        picked.run.code,
        picked.run.stderr
    );
    assert!(
        picked.drawn.contains("VALUE-FOR-THE-MAIL-PASSWORD"),
        "the entry chosen after scrolling was not the one read\n{}",
        picked.drawn
    );
    // Scrolling right and back leaves the name column where it was, so the
    // frame is the one the picker opened with.
    assert_eq!(
        row(&picked.drawn, "mail-password")
            .split_whitespace()
            .next(),
        Some("mail-password"),
        "scrolling back did not restore the first column"
    );
}

/// One column of scroll drops the name column and keeps the rest.
#[test]
fn scrolling_right_moves_the_table_by_one_column() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b[C"], &["\x1b"]],
        ..Pick::default()
    });
    let drawn = lines(&picked.drawn);
    assert!(
        drawn.iter().any(|line| line.starts_with("ORIGIN")),
        "the header does not start at the second column\n{drawn:#?}"
    );
    assert!(
        !drawn.iter().any(|line| line.starts_with("NAME")),
        "the first column is still drawn\n{drawn:#?}"
    );
}

/// A key the picker does not bind does nothing at all.
///
/// Neither leaving, nor a byte in the query: an escape sequence is parsed to
/// its end, so the `P` of `\x1bOP` and the `H` of `\x1b[H` are part of a key
/// rather than characters someone typed.
#[test]
fn an_unbound_key_leaves_the_query_unchanged() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[
            // F1, Home, End, Delete, Insert, Page Up, Page Down and ^A.
            &[
                "\x1bOP", "\x1b[H", "\x1b[F", "\x1b[3~", "\x1b[2~", "\x1b[5~", "\x1b[6~", "\x01",
            ],
            &["mail\r"],
        ],
        ..Pick::default()
    });
    assert!(
        picked.run.succeeded(),
        "an unbound key ended the picker, exit {:?}\n{}",
        picked.run.code,
        picked.run.stderr
    );
    assert!(
        picked.drawn.contains("VALUE-FOR-THE-MAIL-PASSWORD"),
        "the query was not `mail` by the time enter arrived\n{}",
        picked.drawn
    );
    // The query line never carried a byte of a sequence, in any frame.
    for frame in frames(&picked.drawn) {
        for line in lines(&frame) {
            if let Some(query) = line.strip_prefix("> ") {
                assert!(
                    "mail".starts_with(query),
                    "a key this picker does not bind reached the query: {query:?}"
                );
            }
        }
    }
}

/// Leaving prints the one line naming the outcome, and no prose.
#[test]
fn leaving_prints_one_refusal_and_no_paragraph() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        graphical: true,
        ..Pick::default()
    });
    assert_eq!(picked.run.code, Some(1), "leaving the picker exited oddly");
    for line in picked.run.stderr.lines() {
        let bare = line.trim().trim_start_matches(['\u{d7}', ' ', '\u{2502}']);
        assert!(
            bare.is_empty() || bare == "safix::selection_cancelled",
            "leaving printed something other than the outcome: {line:?}"
        );
    }
    assert!(
        picked.run.stderr.contains("safix::selection_cancelled"),
        "leaving printed nothing at all"
    );
}

/// ^C leaves the way escape does, as a keystroke rather than as a signal.
///
/// The empty first group is load-bearing: raw mode is what clears `ISIG`, so a
/// ^C that arrived before the picker had put the terminal in it would be a
/// SIGINT to the process group and would exit 130 through the signal handler —
/// a different claim, made by
/// `an_interrupt_mid_picker_restores_the_terminal_and_exits_130`.
#[test]
fn control_c_leaves_without_choosing() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&[], &["\x03"]],
        graphical: true,
        ..Pick::default()
    });
    assert_eq!(
        picked.run.code,
        Some(1),
        "^C did not exit as a refusal does"
    );
    assert_eq!(picked.run.refusal_code(), "selection_cancelled");
    assert!(
        picked.restored,
        "^C left the terminal in the mode the picker put it in"
    );
}

/// Tab reveals the file column, and the next run opens with it revealed.
#[test]
fn tab_reveals_the_file_column_and_the_next_run_remembers() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let closed = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        ..Pick::default()
    });
    assert!(
        !lines(&closed.drawn)
            .iter()
            .any(|line| line.contains("FILE")),
        "the file column was shown before tab was pressed\n{:#?}",
        lines(&closed.drawn)
    );

    let opened = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\t"], &["\x1b"]],
        ..Pick::default()
    });
    assert!(
        lines(&opened.drawn)
            .iter()
            .any(|line| line.contains("FILE")),
        "tab did not reveal the file column\n{:#?}",
        lines(&opened.drawn)
    );
    assert!(
        settings(&fixture).contains("\"extraColumns\":true"),
        "the toggle was not written to the settings file: {:?}",
        settings(&fixture)
    );
    // The file names an entry, which is not a value and is still nobody else's
    // business.
    assert_eq!(
        settings_mode(&fixture),
        0o600,
        "the settings file is readable by somebody else"
    );

    let again = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        ..Pick::default()
    });
    assert!(
        lines(&again.drawn).iter().any(|line| line.contains("FILE")),
        "the next run forgot that the file column was showing\n{:#?}",
        lines(&again.drawn)
    );
}

/// ^P hides the pane, decrypts nothing further, and the next run opens hidden.
#[test]
fn the_preview_toggle_survives_into_the_next_run() {
    let fixture = Fixture::new();
    three_values(&fixture);
    let sops = real_sops();
    let first = fixture.scratch("toggle-spool");

    // A rest with the preview on, so something is decrypted and the pane is
    // drawn; then ^P; then a rest with it off; then leaving.
    let off = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x10"], &["\x1b"]],
        extra: &observed(&first, &sops),
        ..Pick::default()
    });
    assert!(
        decrypts(&first) >= 1,
        "nothing was decrypted before ^P, so this test would hold over a \
         runtime that previews nothing"
    );
    assert!(
        frames(&off.drawn)
            .first()
            .is_some_and(|frame| frame.contains("Decrypted Value")),
        "the pane was not drawn before ^P\n{}",
        off.drawn
    );
    assert!(
        !frames(&off.drawn)
            .last()
            .is_some_and(|frame| frame.contains("Decrypted Value")),
        "^P did not hide the pane\n{}",
        off.drawn
    );

    let second = fixture.scratch("remembered-spool");
    let remembered = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x1b"]],
        extra: &observed(&second, &sops),
        ..Pick::default()
    });
    assert_eq!(
        decrypts(&second),
        0,
        "the second run decrypted something, so it opened with the preview on\n{}",
        remembered.drawn
    );
    assert!(
        !remembered.drawn.contains("Decrypted Value"),
        "the second run drew the pane\n{}",
        remembered.drawn
    );
}

/// A field term narrows the list to that column.
#[test]
fn a_field_query_narrows_the_list_to_that_column() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["name:mail"], &["\x1b"]],
        ..Pick::default()
    });
    let drawn = lines(&picked.drawn);
    assert!(
        drawn
            .iter()
            .any(|line| line.split_whitespace().next() == Some("mail-password")),
        "the entry the query names is not offered\n{drawn:#?}"
    );
    for excluded in ["api-token", "aliased-secret", "wifi-psk"] {
        assert!(
            !drawn
                .iter()
                .any(|line| line.split_whitespace().next() == Some(excluded)),
            "{excluded} is still offered under `name:mail`\n{drawn:#?}"
        );
    }
    // The same word against a column it is not in offers nothing at all, which
    // is what makes the narrowing above a statement about the column.
    let empty = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["origin:mail"], &["\x1b"]],
        ..Pick::default()
    });
    for name in ["mail-password", "api-token", "aliased-secret", "wifi-psk"] {
        assert!(
            !lines(&empty.drawn)
                .iter()
                .any(|line| line.split_whitespace().next() == Some(name)),
            "{name} was offered for a term its origin does not match\n{:#?}",
            lines(&empty.drawn)
        );
    }
}

/// The stamp columns show the record's dates, and the two coloured rows are the
/// last choice and the newest entry.
///
/// The records are written rather than taken from the clock: a date the test
/// chose is a date it can assert, and `TZ` fixes the offset the run renders at,
/// so this states what an operator in that zone sees rather than what the
/// machine happens to be set to.
#[test]
fn the_stamp_columns_and_the_two_colours_read_off_the_records() {
    let fixture = Fixture::new();
    three_values(&fixture);
    // 2023-11-14T22:13:20Z, 2021-09-13T12:26:40Z and 2024-07-17T01:00:00Z.
    fixture.write(
        "state/safix/definitions/alice/api-token.stamps",
        "v1 created=1700000000 updated=1700000000\n",
    );
    fixture.write(
        "state/safix/definitions/alice/aliased-secret.stamps",
        "v1 created=1631535400 updated=1631535400\n",
    );
    fixture.write(
        "state/safix/definitions/alice/mail-password.stamps",
        "v1 created=1721178000 updated=1721181600\n",
    );

    // First run: choose `api-token`, which is what makes it the last choice.
    let chosen = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["api-\r"]],
        extra: &[("TZ", "UTC")],
        ..Pick::default()
    });
    assert!(
        chosen.run.succeeded(),
        "choosing failed, exit {:?}\n{}",
        chosen.run.code,
        chosen.run.stderr
    );

    let picked = fixture.pick(&Pick {
        arguments: &["view", "--no-preview"],
        keystrokes: &[&["\x1b"]],
        extra: &[("TZ", "UTC")],
        ..Pick::default()
    });
    assert!(
        row(&picked.drawn, "api-token").contains("14/11/2023 10:13pm"),
        "the created stamp is not the record's date: {:?}",
        row(&picked.drawn, "api-token")
    );
    assert!(
        row(&picked.drawn, "mail-password").contains("17/07/2024 01:00am")
            && row(&picked.drawn, "mail-password").contains("17/07/2024 02:00am"),
        "the two stamps are not both the record's: {:?}",
        row(&picked.drawn, "mail-password")
    );
    // An entry with no record at all claims no date, and both of its stamp
    // cells are the empty one rather than one of them.
    assert_eq!(
        row(&picked.drawn, "wifi-psk")
            .split_whitespace()
            .collect::<Vec<&str>>(),
        vec!["wifi-psk", "shared", "-", "-", "wifi-psk", "-", "-"],
        "an entry with no stamp record was given a date"
    );

    let frame = frames(&picked.drawn).pop().unwrap_or_default();
    assert!(
        frame.contains("\x1b[33mapi-token"),
        "the last chosen row is not yellow\n{frame:?}"
    );
    assert!(
        frame.contains("\x1b[36mmail-password"),
        "the newest row is not cyan\n{frame:?}"
    );
    assert!(
        !frame.contains("\x1b[33mmail-password") && !frame.contains("\x1b[36mapi-token"),
        "the two colours landed on each other's rows\n{frame:?}"
    );
}

/// The decrypted value is drawn green, and only the value is.
#[test]
fn the_decrypted_value_is_green() {
    let fixture = Fixture::new();
    three_values(&fixture);

    let picked = fixture.pick(&Pick {
        arguments: &["view"],
        keystrokes: &[&[], &["\x1b"]],
        ..Pick::default()
    });
    assert!(
        picked.drawn.contains("\x1b[32mVALUE-FOR-THE-ALIAS\x1b[0m"),
        "the previewed value was not drawn green and closed\n{}",
        picked.drawn
    );
}
