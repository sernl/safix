//! Moving a declared value across the clan boundary, both ways.
//!
//! Every clan here is the stub — `tests/support/clan-stub.rs` states why that is
//! permitted where stubbing sops is not, and what it can and cannot establish.
//! What it can establish is the delegation, which is what these tests are about:
//! that the read runs clan's command and takes the raw bytes off the pipe, that
//! the write runs clan's command and puts the value on standard input and
//! nowhere else, that clan's own refusals reach the operator as clan's words,
//! that a run converges rather than churning, and that nothing on this side ever
//! goes looking for a file clan placed.
//!
//! What it cannot establish is that those arguments mean to a real clan what
//! this runtime thinks they mean. That was established separately, by driving
//! the real clan CLI over a miniature clan built with it; the findings are
//! recorded in `openspec/changes/clan-bridge/design.md`, and the stub's
//! behaviour is written against them rather than against a reading of the source
//! alone.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{ALICE_FILE, Fixture, Run};

/// The var the fixture mappings name on clan's side.
const VAR: &str = "ntfy/token";

/// The machine they name.
const MACHINE: &str = "meridian";

/// One run of a bridge verb, with the stubbed clan in place.
fn bridge(fixture: &Fixture, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
    let mut environment = fixture.clan_env();
    environment.extend(
        extra
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned())),
    );
    let borrowed: Vec<(&str, &str)> = environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    fixture.run_env(arguments, None, &borrowed)
}

/// A fixture carrying one mapping of the given direction.
fn with_mapping(direction: &str) -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_mapping(
        "ntfy-token",
        direction,
        (MACHINE, "ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture
}

// ── clan to safix ──────────────────────────────────────────────────────────

/// A value clan holds reaches safix, lands in the declared file, and commits.
///
/// The end-to-end claim for the import direction, asserted against literals on
/// both sides: the bytes clan was holding are the bytes the key decrypts to
/// afterwards, and the commit names the mapping rather than the value.
#[test]
fn import_moves_a_clan_value_into_the_declared_entry_and_commits_it() {
    let fixture = with_mapping("clan-to-safix");
    fixture.clan_seed(MACHINE, VAR, "CANARY-from-clan");
    let before = fixture.head();

    let run = bridge(&fixture, &["import"], &[]).expect_success("importing a declared mapping");
    run.says("ntfy-token");
    run.says("updated");
    run.silent_about("CANARY-from-clan");

    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-from-clan",
        "the imported value is not what clan was holding"
    );
    assert_ne!(fixture.head(), before, "the import committed nothing");
    let subject = fixture.subject("HEAD");
    assert_eq!(
        subject, "chore(safix): import ntfy-token for alice",
        "the commit does not name the mapping and the direction"
    );
    assert!(
        !subject.contains("CANARY-from-clan"),
        "the commit message names the value"
    );
}

/// A second import immediately after a first writes nothing and commits
/// nothing.
///
/// Convergence, asserted rather than claimed. The comparison happens before the
/// write in both directions; here it saves a commit, and the assertion is that
/// the history did not move.
#[test]
fn a_second_import_writes_nothing_and_commits_nothing() {
    let fixture = with_mapping("clan-to-safix");
    fixture.clan_seed(MACHINE, VAR, "CANARY-from-clan");

    bridge(&fixture, &["import"], &[]).expect_success("the first import");
    let settled = fixture.head();
    let document = fixture.read(ALICE_FILE);

    let again = bridge(&fixture, &["import"], &[]).expect_success("the second import");
    again.says("unchanged");
    // The tally rather than the word: the closing line names every outcome, so
    // "the report does not contain 'updated'" would be false for a report that
    // says zero of them.
    again.says("0 updated, 1 unchanged");
    assert_eq!(fixture.head(), settled, "the second import committed");
    assert_eq!(
        fixture.read(ALICE_FILE),
        document,
        "the second import rewrote the file, which re-encrypts it for no reason"
    );
}

/// A clan var that has not been generated yet is a state, not a failure.
#[test]
fn an_ungenerated_clan_var_is_reported_and_the_run_continues() {
    let fixture = with_mapping("clan-to-safix");
    let before = fixture.head();

    let run = bridge(&fixture, &["import"], &[])
        .expect_success("a mapping whose source holds nothing yet");
    run.says("absent at source");
    assert_eq!(fixture.head(), before, "an absent source produced a commit");
}

/// The safix-side write is the hand-set write, drift refusal included.
///
/// Driven through a drifted fixture rather than by inspecting the call, which is
/// the only way to establish that the imported value takes the refusal rather
/// than that the code appears to route through something that would.
#[test]
fn an_import_into_a_drifted_file_is_refused_before_anything_lands() {
    let fixture = with_mapping("clan-to-safix");
    let stranger = fixture.new_recipient();
    fixture.clan_seed(MACHINE, VAR, "CANARY-into-drift");

    fixture.encrypt_to(
        ALICE_FILE,
        &[&fixture.alice, &stranger],
        "api-token: \"fixture-value-for-api-token\"\n",
    );
    fixture.git(&["add", "--", ALICE_FILE]);
    fixture.git(&["commit", "-q", "-m", "fixture: recipients drifted"]);

    let before = fixture.head();
    let document = fixture.read(ALICE_FILE);

    let run = bridge(&fixture, &["import"], &[]).expect_refusal("importing into a drifted file");
    run.says(&stranger);
    run.silent_about("CANARY-into-drift");
    assert_eq!(fixture.head(), before, "the refused import committed");
    assert_eq!(
        fixture.read(ALICE_FILE),
        document,
        "the refused import wrote the file"
    );
}

// ── safix to clan ──────────────────────────────────────────────────────────

/// A value safix holds reaches clan, through clan's own command.
#[test]
fn export_moves_a_safix_value_into_clan_through_clans_own_command() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-from-safix")
        .expect_success("seeding the source");
    let before = fixture.head();

    let run = bridge(&fixture, &["export"], &[]).expect_success("exporting a declared mapping");
    run.says("ntfy-token");
    run.says("updated");
    run.silent_about("CANARY-from-safix");

    assert_eq!(
        fixture.clan_holds(MACHINE, VAR).as_deref(),
        Some("CANARY-from-safix"),
        "clan does not hold what safix was holding"
    );
    assert_eq!(
        fixture.clan_writes(),
        1,
        "clan was asked to write more than once"
    );
    assert_eq!(
        fixture.head(),
        before,
        "the export committed in this repository, where nothing changed"
    );
}

/// A second export writes nothing, which is the whole reason the comparison
/// precedes the write.
///
/// clan's write is unconditional and commits what it wrote, and its `age`
/// backend re-encrypts an unchanged value into fresh ciphertext. Without the
/// read-first comparison this count would rise by one per run, forever, each
/// increment a commit in the clan repository whose diff decrypts to what it
/// decrypted to before.
#[test]
fn a_second_export_does_not_ask_clan_to_write_again() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-from-safix")
        .expect_success("seeding the source");

    bridge(&fixture, &["export"], &[]).expect_success("the first export");
    assert_eq!(
        fixture.clan_writes(),
        1,
        "the first export did not write once"
    );

    let again = bridge(&fixture, &["export"], &[]).expect_success("the second export");
    again.says("unchanged");
    assert_eq!(
        fixture.clan_writes(),
        1,
        "the second export asked clan to write again, so every run would commit in clan's repository"
    );
}

/// An export whose source holds no value is refused, naming both remedies.
///
/// The runtime sibling of a refusal evaluation cannot make: an entry declares
/// where a value lives rather than that one is there, so this is answerable only
/// when something reads the file.
#[test]
fn an_export_whose_source_holds_no_value_is_refused() {
    let fixture = with_mapping("safix-to-clan");

    let run = bridge(&fixture, &["export"], &[]).expect_refusal("exporting an entry with no value");
    run.says("holds no value yet");
    run.says("safix set alice api-token");
    run.says("safix generate alice api-token");
    assert_eq!(
        fixture.clan_writes(),
        0,
        "a refused export still asked clan to write"
    );
}

/// An export into a generator clan already considers stale is refused.
///
/// Confirmed against the real clan before it was written here: changing a
/// generator's definition makes `clan vars check` report an outdated
/// invalidation hash while `clan vars get` keeps returning the old value, which
/// is exactly the silent replacement this refusal prevents.
#[test]
fn an_export_into_a_stale_generator_is_refused_and_names_both_remedies() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-would-be-lost")
        .expect_success("seeding the source");

    let run = bridge(&fixture, &["export"], &[("SAFIX_CLAN_STUB_STALE", "ntfy")])
        .expect_refusal("exporting into a stale generator");

    run.says("outdated");
    run.says("clan vars generate meridian");
    run.says("clan-to-safix");
    run.says("no option that exports anyway");
    run.silent_about("CANARY-would-be-lost");
    assert_eq!(
        fixture.clan_writes(),
        0,
        "the stale-generator refusal still wrote into clan"
    );
}

// ── the boundary itself ────────────────────────────────────────────────────

/// The read took the raw bytes rather than a rendering meant for a terminal.
///
/// clan's read command branches on whether its standard output is a terminal and
/// substitutes a printable form when it is, so this is not incidental: a `get`
/// that inherited a terminal would hand back a rendering of the value in place
/// of the value, and nothing downstream could tell. The stub records what it saw
/// on each read, so the claim is made from the far side rather than assumed from
/// the near one.
#[test]
fn the_clan_read_happened_on_a_pipe_and_not_a_terminal() {
    let fixture = with_mapping("clan-to-safix");
    fixture.clan_seed(MACHINE, VAR, "CANARY-raw-bytes");

    bridge(&fixture, &["import"], &[]).expect_success("importing a declared mapping");

    let seen = fixture.clan_recorded("isatty");
    assert!(!seen.is_empty(), "clan's read was never reached");
    assert!(
        seen.lines().all(|line| line == "pipe"),
        "a clan read inherited a terminal, so what came back was a rendering: {seen:?}"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-raw-bytes",
        "the raw bytes are not what landed"
    );
}

/// No value ever reached clan's argument vector, in either direction.
///
/// The machine, the generator and the file travel in argv because that is what
/// clan's own command line takes and all three are public. The value travels a
/// pipe, and this is the assertion that it travelled nothing else.
#[test]
fn no_value_reaches_clans_argument_vector_in_either_direction() {
    let down = with_mapping("clan-to-safix");
    down.clan_seed(MACHINE, VAR, "CANARY-argv-down");
    bridge(&down, &["import"], &[]).expect_success("the import");

    let up = with_mapping("safix-to-clan");
    up.set("alice", "api-token", "CANARY-argv-up")
        .expect_success("seeding the source");
    bridge(&up, &["export"], &[]).expect_success("the export");

    for (fixture, value) in [(&down, "CANARY-argv-down"), (&up, "CANARY-argv-up")] {
        let argv = fixture.clan_recorded("argv");
        assert!(!argv.is_empty(), "clan was never invoked");
        assert!(
            !argv.contains(value),
            "a value reached clan's argument vector:\n{argv}"
        );
        assert!(
            argv.contains(MACHINE) && argv.contains(VAR),
            "the endpoints did not reach clan's argument vector:\n{argv}"
        );
    }
}

// ── refusals ───────────────────────────────────────────────────────────────

/// No clan on `PATH` refuses the whole run before any mapping is touched.
///
/// Deliberately not a soft failure that skips clan-side mappings: a run that
/// quietly did half its mappings would report "unchanged" for the half it never
/// looked at, which is worse than a run that does none.
#[test]
fn an_absent_clan_refuses_both_verbs_before_anything_is_transferred() {
    for (direction, verb) in [("clan-to-safix", "import"), ("safix-to-clan", "export")] {
        let fixture = with_mapping(direction);
        let before = fixture.head();

        let run = bridge(
            &fixture,
            &[verb],
            &[("SAFIX_CLAN", "safix-no-such-clan-command")],
        )
        .expect_refusal("a transfer with no clan installed");

        run.says("clan is the authority on its own store");
        assert_eq!(
            fixture.head(),
            before,
            "the run with no clan committed something"
        );
        // No report at all, rather than a report of zeroes. The refusal happens
        // before the first mapping is touched, so there is nothing to report an
        // outcome for — and a run that printed one would be printing an outcome
        // for a mapping it never looked at.
        run.silent_about("mapping(s):");
        run.silent_about("ntfy-token  ");
    }
}

/// A mapping whose clan side does not resolve is refused, naming the triple.
///
/// The one thing evaluation could not check, because the clan half of a mapping
/// lives in another flake. clan's own words carry the refusal.
#[test]
fn a_clan_side_that_does_not_resolve_is_refused_naming_all_three_names() {
    let fixture = with_mapping("clan-to-safix");

    let run = bridge(&fixture, &["import"], &[("SAFIX_CLAN_STUB_UNKNOWN", VAR)])
        .expect_refusal("a mapping clan has no var for");
    run.says("meridian");
    run.says("ntfy");
    run.says("token");
    run.says("clan vars list meridian");
}

/// clan's own refusal reaches the operator as clan's words, with the mapping
/// attached.
#[test]
fn clans_own_refusal_is_carried_rather_than_reworded() {
    let fixture = with_mapping("clan-to-safix");

    let run = bridge(&fixture, &["import"], &[("SAFIX_CLAN_STUB_REFUSES", VAR)])
        .expect_refusal("a clan that refused for a reason of its own");

    run.says("for a reason of its own");
    run.says("ntfy-token");
}

/// A mapping name nothing declares is refused, naming what is declared.
#[test]
fn an_undeclared_mapping_name_is_refused_naming_the_declared_ones() {
    let fixture = with_mapping("clan-to-safix");

    let run = bridge(&fixture, &["import", "ntfy-tokne"], &[])
        .expect_refusal("a mapping name nothing declares");
    run.says("'ntfy-tokne' is not a declared mapping");
    run.says("ntfy-token");
}

/// A mapping named to the verb that does not act on it is refused as that,
/// rather than as an unknown name.
///
/// The operator has spelled the mapping correctly and asked the wrong verb, and
/// a message saying "not a declared mapping" about a mapping three lines above
/// in their own file would send them looking for a typo that is not there.
#[test]
fn a_mapping_named_to_the_wrong_verb_is_told_which_verb_acts_on_it() {
    let fixture = with_mapping("safix-to-clan");

    let run = bridge(&fixture, &["import", "ntfy-token"], &[])
        .expect_refusal("an export mapping named to import");
    run.says("declared safix-to-clan");
    run.says("safix export ntfy-token");
}

/// A verb acts on its own direction and not the other's.
#[test]
fn each_verb_acts_on_its_own_direction_alone() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-not-imported")
        .expect_success("seeding the source");

    let run = bridge(&fixture, &["import"], &[]).expect_success("importing with no import mapping");
    run.says("no mapping is declared for this direction");
    assert_eq!(
        fixture.clan_writes(),
        0,
        "the import verb wrote through an export mapping"
    );
}

/// A consumer who has never heard of clan is not refused for having no bridge.
#[test]
fn an_empty_bridge_is_silent_rather_than_refused() {
    let fixture = Fixture::new();
    for verb in ["import", "export"] {
        let run = bridge(&fixture, &[verb], &[]).expect_success("a verb over an empty bridge");
        run.says("no mapping is declared for this direction");
    }
}

/// Both verbs appear in the command's help, with their directions stated.
#[test]
fn both_verbs_appear_in_the_help_with_their_directions() {
    let fixture = Fixture::new();
    let scaffold = fixture.run(&["--help"]).expect_success("the general help");
    scaffold.says("safix import");
    scaffold.says("safix export");
    scaffold.says("clan-to-safix");
    scaffold.says("safix-to-clan");

    fixture
        .run(&["import", "-h"])
        .expect_success("the import help")
        .says("clan-to-safix");
    fixture
        .run(&["export", "-h"])
        .expect_success("the export help")
        .says("safix-to-clan");
}
