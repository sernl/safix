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

/// One run of `sync clan`, with the stubbed clan in place.
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

/// A fixture carrying two mappings, one of each direction, so a run over both
/// is a run over more than one mapping's worth of state.
fn with_both_directions() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_mapping(
        "down",
        "clan-to-safix",
        (MACHINE, "ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture.seed_mapping(
        "up",
        "safix-to-clan",
        (MACHINE, "mail", "password"),
        ("alice", "mail-password"),
    );
    fixture
}

// ── clan to safix ──────────────────────────────────────────────────────────

/// A value clan holds reaches safix, lands in the declared file, and commits.
///
/// The end-to-end claim for the clan-to-safix direction, asserted against
/// literals on both sides: the bytes clan was holding are the bytes the key
/// decrypts to afterwards, and the commit names the mapping rather than the
/// value.
#[test]
fn a_clan_to_safix_mapping_moves_a_clan_value_into_the_declared_entry_and_commits_it() {
    let fixture = with_mapping("clan-to-safix");
    fixture.clan_seed(MACHINE, VAR, "CANARY-from-clan");
    let before = fixture.head();

    let run =
        bridge(&fixture, &["sync", "clan"], &[]).expect_success("converging a declared mapping");
    run.says("ntfy-token");
    run.says("pulled ntfy-token");
    run.silent_about("CANARY-from-clan");

    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-from-clan",
        "the converged value is not what clan was holding"
    );
    assert_ne!(fixture.head(), before, "the run committed nothing");
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

/// A second run immediately after a first writes nothing and commits nothing.
///
/// Convergence, asserted rather than claimed. The comparison happens before the
/// write in both directions; here it saves a commit, and the assertion is that
/// the history did not move.
#[test]
fn a_second_run_over_the_same_mapping_writes_nothing_and_commits_nothing() {
    let fixture = with_mapping("clan-to-safix");
    fixture.clan_seed(MACHINE, VAR, "CANARY-from-clan");

    bridge(&fixture, &["sync", "clan"], &[]).expect_success("the first run");
    let settled = fixture.head();
    let document = fixture.read(ALICE_FILE);

    let again = bridge(&fixture, &["sync", "clan"], &[]).expect_success("the second run");
    again.says("unchanged");
    // The tally rather than the word: the closing line names every outcome, so
    // "the report does not contain 'updated'" would be false for a report that
    // says zero of them.
    again.says("0 updated, 1 unchanged");
    assert_eq!(fixture.head(), settled, "the second run committed");
    assert_eq!(
        fixture.read(ALICE_FILE),
        document,
        "the second run rewrote the file, which re-encrypts it for no reason"
    );
}

/// A clan var that has not been generated yet is a state, not a failure.
#[test]
fn an_ungenerated_clan_var_is_reported_and_the_run_continues() {
    let fixture = with_mapping("clan-to-safix");
    let before = fixture.head();

    let run = bridge(&fixture, &["sync", "clan"], &[])
        .expect_success("a mapping whose source holds nothing yet");
    run.says("absent at source");
    assert_eq!(fixture.head(), before, "an absent source produced a commit");
}

/// The safix-side write is the hand-set write, drift refusal included.
///
/// Driven through a drifted fixture rather than by inspecting the call, which is
/// the only way to establish that the converged value takes the refusal rather
/// than that the code appears to route through something that would.
#[test]
fn converging_into_a_drifted_file_is_refused_before_anything_lands() {
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

    let run =
        bridge(&fixture, &["sync", "clan"], &[]).expect_refusal("converging into a drifted file");
    run.says(&stranger);
    run.silent_about("CANARY-into-drift");
    assert_eq!(fixture.head(), before, "the refused run committed");
    assert_eq!(
        fixture.read(ALICE_FILE),
        document,
        "the refused run wrote the file"
    );
}

// ── safix to clan ──────────────────────────────────────────────────────────

/// A value safix holds reaches clan, through clan's own command.
#[test]
fn a_safix_to_clan_mapping_moves_a_safix_value_into_clan_through_clans_own_command() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-from-safix")
        .expect_success("seeding the source");
    let before = fixture.head();

    let run =
        bridge(&fixture, &["sync", "clan"], &[]).expect_success("converging a declared mapping");
    run.says("ntfy-token");
    run.says("pushed ntfy-token");
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
        "the run committed in this repository, where nothing changed"
    );
}

/// A second run over the same mapping writes nothing, which is the whole reason
/// the comparison precedes the write.
///
/// clan's write is unconditional and commits what it wrote, and its `age`
/// backend re-encrypts an unchanged value into fresh ciphertext. Without the
/// read-first comparison this count would rise by one per run, forever, each
/// increment a commit in the clan repository whose diff decrypts to what it
/// decrypted to before.
#[test]
fn a_second_run_does_not_ask_clan_to_write_again() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-from-safix")
        .expect_success("seeding the source");

    bridge(&fixture, &["sync", "clan"], &[]).expect_success("the first run");
    assert_eq!(fixture.clan_writes(), 1, "the first run did not write once");

    let again = bridge(&fixture, &["sync", "clan"], &[]).expect_success("the second run");
    again.says("unchanged");
    assert_eq!(
        fixture.clan_writes(),
        1,
        "the second run asked clan to write again, so every run would commit in clan's repository"
    );
}

/// A source with no value is refused, naming both remedies.
///
/// The runtime sibling of a refusal evaluation cannot make: an entry declares
/// where a value lives rather than that one is there, so this is answerable only
/// when something reads the file.
#[test]
fn a_source_with_no_value_is_refused() {
    let fixture = with_mapping("safix-to-clan");

    let run = bridge(&fixture, &["sync", "clan"], &[])
        .expect_refusal("converging an entry with no value");
    run.says("holds no value yet");
    run.says("safix set alice api-token");
    run.says("safix generate alice api-token");
    assert_eq!(
        fixture.clan_writes(),
        0,
        "a refused mapping still asked clan to write"
    );
}

/// A mapping into a generator clan already considers stale is refused.
///
/// Confirmed against the real clan before it was written here: changing a
/// generator's definition makes `clan vars check` report an outdated
/// invalidation hash while `clan vars get` keeps returning the old value, which
/// is exactly the silent replacement this refusal prevents.
#[test]
fn a_mapping_into_a_stale_generator_is_refused_and_names_both_remedies() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-would-be-lost")
        .expect_success("seeding the source");

    let run = bridge(
        &fixture,
        &["sync", "clan"],
        &[("SAFIX_CLAN_STUB_STALE", "ntfy")],
    )
    .expect_refusal("converging into a stale generator");

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

// ── one run, both directions ──────────────────────────────────────────────

/// A `sync clan` run with no target and no `--direction` converges every
/// declared mapping in its own declared direction, in one invocation.
///
/// The headline claim of the target-scoped grammar: what used to take two
/// commands, `import` and `export`, now happens in one.
#[test]
fn sync_clan_converges_both_directions_in_one_run() {
    let fixture = with_both_directions();
    fixture.clan_seed(MACHINE, "ntfy/token", "CANARY-down");
    fixture
        .set("alice", "mail-password", "CANARY-up")
        .expect_success("seeding the safix-to-clan source");

    let run = bridge(&fixture, &["sync", "clan"], &[])
        .expect_success("converging both directions in one run");
    run.says("pulled down");
    run.says("pushed up");
    run.says("2 mapping(s): 2 updated");

    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-down",
        "the clan-to-safix mapping did not converge"
    );
    assert_eq!(
        fixture.clan_holds(MACHINE, "mail/password").as_deref(),
        Some("CANARY-up"),
        "the safix-to-clan mapping did not converge"
    );
}

/// `--direction` narrows the run to mappings declared with that value, leaving
/// the other direction's mappings untouched rather than refusing them.
#[test]
fn direction_narrows_the_run_to_mappings_declared_with_that_value() {
    let fixture = with_both_directions();
    fixture.clan_seed(MACHINE, "ntfy/token", "CANARY-down");
    fixture
        .set("alice", "mail-password", "CANARY-up")
        .expect_success("seeding the safix-to-clan source");

    let run = bridge(
        &fixture,
        &["sync", "clan", "--direction", "clan-to-safix"],
        &[],
    )
    .expect_success("narrowing to clan-to-safix");
    run.says("pulled down");
    run.says("1 mapping(s): 1 updated");
    assert_eq!(
        fixture.clan_writes(),
        0,
        "the safix-to-clan mapping was written despite the --direction filter"
    );
}

/// A named mapping outside the `--direction` filter is told its actual
/// direction, distinct from an unknown-mapping refusal.
#[test]
fn a_named_mapping_outside_the_direction_filter_is_told_its_actual_direction() {
    let fixture = with_both_directions();

    let run = bridge(
        &fixture,
        &["sync", "clan", "up", "--direction", "clan-to-safix"],
        &[],
    )
    .expect_refusal("naming a safix-to-clan mapping under a clan-to-safix filter");
    run.says("'up' is declared safix-to-clan, not clan-to-safix");
    run.says("--direction safix-to-clan");
}

/// Naming more than one mapping converges exactly those, in one run.
#[test]
fn multiple_named_mappings_converge_in_one_run() {
    let fixture = with_both_directions();
    fixture.clan_seed(MACHINE, "ntfy/token", "CANARY-down");
    fixture
        .set("alice", "mail-password", "CANARY-up")
        .expect_success("seeding the safix-to-clan source");

    let run = bridge(&fixture, &["sync", "clan", "down", "up"], &[])
        .expect_success("naming both mappings");
    run.says("2 mapping(s): 2 updated");
}

/// A named mapping's actual direction is told apart from a wrong
/// `--direction` filter in both directions across the two-way boundary: a
/// two-way mapping named under a one-way filter, and a one-way mapping named
/// under `--direction two-way`. The same generic refusal
/// `rename-transfer-verbs` already gives one-way mismatches, unmodified.
#[test]
fn a_two_way_mapping_and_a_one_way_filter_are_told_apart_in_both_directions() {
    let mut fixture = Fixture::new();
    fixture.seed_mapping(
        "bothways",
        "two-way",
        (MACHINE, "ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture.seed_mapping(
        "down",
        "clan-to-safix",
        (MACHINE, "wg", "private"),
        ("alice", "wg-key"),
    );

    let run = bridge(
        &fixture,
        &["sync", "clan", "bothways", "--direction", "clan-to-safix"],
        &[],
    )
    .expect_refusal("naming a two-way mapping under a clan-to-safix filter");
    run.says("'bothways' is declared two-way, not clan-to-safix");

    let run = bridge(
        &fixture,
        &["sync", "clan", "down", "--direction", "two-way"],
        &[],
    )
    .expect_refusal("naming a clan-to-safix mapping under a two-way filter");
    run.says("'down' is declared clan-to-safix, not two-way");
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

    bridge(&fixture, &["sync", "clan"], &[]).expect_success("converging a declared mapping");

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
    bridge(&down, &["sync", "clan"], &[]).expect_success("the clan-to-safix run");

    let up = with_mapping("safix-to-clan");
    up.set("alice", "api-token", "CANARY-argv-up")
        .expect_success("seeding the source");
    bridge(&up, &["sync", "clan"], &[]).expect_success("the safix-to-clan run");

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
fn an_absent_clan_refuses_the_run_before_anything_is_transferred() {
    for direction in ["clan-to-safix", "safix-to-clan"] {
        let fixture = with_mapping(direction);
        let before = fixture.head();

        let run = bridge(
            &fixture,
            &["sync", "clan"],
            &[("SAFIX_CLAN", "safix-no-such-clan-command")],
        )
        .expect_refusal("a run with no clan installed");

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

    let run = bridge(
        &fixture,
        &["sync", "clan"],
        &[("SAFIX_CLAN_STUB_UNKNOWN", VAR)],
    )
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

    let run = bridge(
        &fixture,
        &["sync", "clan"],
        &[("SAFIX_CLAN_STUB_REFUSES", VAR)],
    )
    .expect_refusal("a clan that refused for a reason of its own");

    run.says("for a reason of its own");
    run.says("ntfy-token");
}

/// A mapping name nothing declares is refused, naming what is declared.
#[test]
fn an_undeclared_mapping_name_is_refused_naming_the_declared_ones() {
    let fixture = with_mapping("clan-to-safix");

    let run = bridge(&fixture, &["sync", "clan", "ntfy-tokne"], &[])
        .expect_refusal("a mapping name nothing declares");
    run.says("'ntfy-tokne' is not a declared mapping");
    run.says("ntfy-token");
}

/// A consumer who has never heard of clan is not refused for having no bridge.
#[test]
fn an_empty_bridge_is_silent_rather_than_refused() {
    let fixture = Fixture::new();
    let run = bridge(&fixture, &["sync", "clan"], &[]).expect_success("a run over an empty bridge");
    run.says("no mapping is declared");
}

/// `sync` appears in the command's help, and its own help names all three
/// directions and the clan target's grammar.
#[test]
fn sync_appears_in_the_help_with_all_three_directions() {
    let fixture = Fixture::new();
    let scaffold = fixture.run(&["--help"]).expect_success("the general help");
    scaffold.says("safix sync");

    let help = fixture.run(&["sync", "-h"]).expect_success("the sync help");
    help.says("clan-to-safix");
    help.says("safix-to-clan");
    help.says("two-way");
    help.says("--direction");
}
