//! `safix sync`, driven against a modelled database and the real everything else.
//!
//! sops, age and git are real here as everywhere in this suite; the store's own
//! command is the stub `tests/support/card-stubs.rs` models, for the reason that
//! file states about the other three tools it stands in for — the claims are
//! about the delegation, and a stub can be asked what it saw — plus the one that
//! matters most on this path: the real command writes into a real database, and
//! the database on the machines this is developed on is the fleet's root of
//! trust. `store_cli.rs` is the other half of that sentence, and it drives the
//! real command against a database it creates in its own temporary directory.
//!
//! `harness::refuse_a_real_database` is the structural guard, and it is why every
//! run below goes through [`Fixture::run_sync`] with [`Fixture::store_env`]: a run
//! whose override does not name the stub, or whose declared database is anywhere
//! but the fixture's own scratch directory, fails before a process is spawned.
//!
//! # What each fixture is
//!
//! One mapping per mode, and one state per mapping: two sides that agree, a
//! divergence in each direction, a two-way divergence on one side and on both,
//! and a safix side that will not decrypt. What is asserted about each is the
//! outcome the report gives it, what each side holds afterwards, and — for every
//! one of them — that no value and no derivative of a value reached the
//! repository or standard output.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use harness::{ALICE_FILE, Fixture};

/// The password the modelled database is opened with, fed to the one prompt.
const UNLOCK: &str = "fixture-database-password\n";

/// A fixture with one mapping of each mode, and alice holding a value for each.
fn declared() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_output("pull-me", ALICE_FILE);
    fixture.seed_output("both-ways", ALICE_FILE);
    fixture.seed_output("back-me-up", ALICE_FILE);

    fixture.seed_sync_mapping(
        "push",
        "safix-to-keepassxc",
        ("alice", "push-me"),
        "alice/pushed",
        Some("alice@example.com"),
    );
    fixture.seed_sync_mapping(
        "pull",
        "keepassxc-to-safix",
        ("alice", "pull-me"),
        "alice/pulled",
        None,
    );
    fixture.seed_sync_mapping(
        "both",
        "two-way",
        ("alice", "both-ways"),
        "alice/both",
        None,
    );
    fixture.seed_sync_mapping(
        "copy",
        "backup",
        ("alice", "back-me-up"),
        "alice/copied",
        None,
    );
    fixture
}

/// The environment every run needs: the store stub, and the password it expects.
fn store_env(fixture: &Fixture) -> Vec<(String, String)> {
    let mut extra = fixture.store_env();
    extra.push((
        "SAFIX_CARD_STUB_DB_PASSWORD".to_owned(),
        UNLOCK.trim_end().to_owned(),
    ));
    extra
}

fn borrowed(extra: &[(String, String)]) -> Vec<(&str, &str)> {
    extra
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// Each mode converges exactly as its name says, over one run.
#[test]
fn each_mode_converges_exactly_as_its_name_says() {
    let fixture = declared();
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    // safix holds a value for all four. The database holds one for the pull
    // mapping — the person typed it — and nothing for the other three.
    for name in ["push-me", "both-ways", "back-me-up"] {
        fixture
            .run_with(&["set", "alice", name], &format!("safix-{name}"))
            .expect_success("seeding the safix side");
    }
    fixture
        .run_with(&["set", "alice", "pull-me"], "safix-before-the-pull")
        .expect_success("seeding the entry a pull overwrites");
    fixture.store_seed("safix/alice/pulled", "kdbx-pulled");

    let run = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("one mapping of each mode, each with something to do");

    // Every declared mapping is in the report, with the outcome its mode gives it.
    run.says("push  alice.push-me -> safix/alice/pushed  safix-to-keepassxc  updated");
    run.says("pull  safix/alice/pulled -> alice.pull-me  keepassxc-to-safix  pulled");
    run.says("both  alice.both-ways -> safix/alice/both  two-way  updated");
    run.says("copy  alice.back-me-up -> safix/alice/copied  backup  updated");

    // What each side holds afterwards.
    assert_eq!(
        fixture.store_holds("safix/alice/pushed").as_deref(),
        Some("safix-push-me"),
        "the database did not converge to safix's value"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "pull-me"),
        "kdbx-pulled",
        "safix did not converge to the database's value"
    );
    assert_eq!(
        fixture.store_holds("safix/alice/both").as_deref(),
        Some("safix-both-ways"),
        "a two-way mapping with an empty database side did not bootstrap"
    );
    assert_eq!(
        fixture.store_holds("safix/alice/copied").as_deref(),
        Some("safix-back-me-up"),
        "a backup mapping did not write into absence"
    );

    // The username a mapping declares reaches the entry; one that declares none
    // leaves the field alone.
    assert_eq!(
        fixture.store_username("safix/alice/pushed"),
        "alice@example.com"
    );
    assert_eq!(fixture.store_username("safix/alice/both"), "");

    // The two-way mapping recorded its agreement, and it is beside the entry
    // rather than in the repository.
    let companion = fixture
        .store_holds("safix/alice/both.safix-sync-state")
        .expect("a two-way mapping recorded no agreement");
    assert!(
        companion.starts_with("safix-sync-v1 "),
        "the recorded agreement carries no format tag: {companion}"
    );
    assert_no_oracle(&fixture, &["safix-both-ways", "kdbx-pulled"]);

    // Nothing a protected read produced reached standard output.
    assert_eq!(run.output(), "", "a value reached standard output");
    for value in ["safix-push-me", "kdbx-pulled", "safix-both-ways"] {
        run.silent_about(value);
    }
}

/// A second run over the same tree writes nothing, anywhere.
#[test]
fn a_second_run_writes_nothing_anywhere() {
    let fixture = declared();
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    for name in ["push-me", "both-ways", "back-me-up", "pull-me"] {
        fixture
            .run_with(&["set", "alice", name], &format!("safix-{name}"))
            .expect_success("seeding the safix side");
    }
    fixture.store_seed("safix/alice/pulled", "safix-pull-me");

    fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("the first run");

    let head = fixture.head();
    let ciphertext = fixture.ciphertext_lines(ALICE_FILE);
    let stored_before: Vec<Option<String>> = [
        "safix/alice/pushed",
        "safix/alice/both",
        "safix/alice/copied",
    ]
    .into_iter()
    .map(|entry| fixture.store_holds(entry))
    .collect();
    let invocations = fixture.store_invocations().len();

    let second = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("the second run");

    for mapping in ["push", "pull", "both", "copy"] {
        second.says(&format!("{mapping}  "));
    }
    second.says("0 updated, 0 pulled, 4 unchanged");
    assert_eq!(fixture.head(), head, "the second run committed");
    assert_eq!(fixture.status(), "", "the second run left the tree dirty");
    assert_eq!(
        fixture.ciphertext_lines(ALICE_FILE),
        ciphertext,
        "the second run moved ciphertext"
    );
    let stored_after: Vec<Option<String>> = [
        "safix/alice/pushed",
        "safix/alice/both",
        "safix/alice/copied",
    ]
    .into_iter()
    .map(|entry| fixture.store_holds(entry))
    .collect();
    assert_eq!(
        stored_after, stored_before,
        "the second run wrote into the database"
    );

    // Reads happened and writes did not, which is the shape of convergence: the
    // command was invoked, and no invocation of it was `add` or `edit`.
    let further = fixture.store_invocations();
    assert!(
        further.len() > invocations,
        "the second run did not read either"
    );
    for line in further.iter().skip(invocations) {
        assert!(
            !line.starts_with("add ") && !line.starts_with("edit ") && !line.starts_with("mkdir "),
            "the second run wrote: {line}"
        );
    }
}

/// A pulled value lands as a commit shaped like a hand-set write.
#[test]
fn a_pulled_value_lands_as_a_commit_shaped_like_a_hand_set_write() {
    let mut fixture = Fixture::new();
    fixture.seed_output("pull-me", ALICE_FILE);
    fixture.seed_sync_mapping(
        "pull",
        "keepassxc-to-safix",
        ("alice", "pull-me"),
        "alice/pulled",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    // A hand-set write of another entry first, to compare the shape against.
    fixture.seed_output("by-hand", ALICE_FILE);
    fixture
        .run_with(&["set", "alice", "by-hand"], "typed-by-a-person")
        .expect_success("the hand-set write");
    let by_hand = fixture.paths_in("HEAD");

    fixture.store_seed("safix/alice/pulled", "kdbx-pulled");
    fixture
        .run_sync(&["sync", "pull"], UNLOCK, &extra)
        .expect_success("pulling one mapping");

    assert_eq!(
        fixture.paths_in("HEAD"),
        by_hand,
        "a pull committed a different set of paths from a hand-set write"
    );
    assert_eq!(
        fixture.subject("HEAD"),
        "chore(safix): sync pull for alice",
        "the commit does not name the mapping"
    );
    fixture.message("HEAD").lines().for_each(|line| {
        assert!(
            !line.contains("kdbx-pulled"),
            "the commit message carries the value: {line}"
        );
    });
    assert_eq!(fixture.value(ALICE_FILE, "pull-me"), "kdbx-pulled");
    assert_eq!(fixture.status(), "", "the pull left the tree dirty");
}

/// Two-way converges toward the side that moved, and refuses to guess when both
/// did.
#[test]
fn two_way_converges_toward_the_side_that_moved_and_will_not_guess_when_both_did() {
    let mut fixture = Fixture::new();
    fixture.seed_output("both-ways", ALICE_FILE);
    fixture.seed_sync_mapping(
        "both",
        "two-way",
        ("alice", "both-ways"),
        "alice/both",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "both-ways"], "agreed-value")
        .expect_success("seeding the safix side");
    fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("bootstrapping the two-way mapping");
    assert_eq!(
        fixture.store_holds("safix/alice/both").as_deref(),
        Some("agreed-value")
    );

    // The database side moves. safix is where the agreement left it, so safix
    // converges toward the database.
    fixture.store_seed("safix/alice/both", "changed-in-the-database");
    let pulled = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("one side changed");
    pulled.says("two-way  pulled");
    assert_eq!(
        fixture.value(ALICE_FILE, "both-ways"),
        "changed-in-the-database"
    );

    // safix's side moves. The database is where the agreement left it, so the
    // database converges toward safix.
    fixture
        .run_with(&["set", "alice", "both-ways"], "changed-in-safix")
        .expect_success("moving safix's side");
    let pushed = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("the other side changed");
    pushed.says("two-way  updated");
    assert_eq!(
        fixture.store_holds("safix/alice/both").as_deref(),
        Some("changed-in-safix")
    );

    // Both sides move. Nothing is written and the finding names both remedies.
    fixture
        .run_with(&["set", "alice", "both-ways"], "safix-moved-again")
        .expect_success("moving safix's side again");
    fixture.store_seed("safix/alice/both", "the-database-moved-too");
    let head = fixture.head();
    let conflict = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_refusal("both sides changed");
    conflict.says("two-way  conflict");
    conflict.says("have both changed since the last agreement");
    conflict.says("mode = \"safix-to-keepassxc\";");
    conflict.says("mode = \"keepassxc-to-safix\";");
    conflict.silent_about("safix-moved-again");
    conflict.silent_about("the-database-moved-too");
    assert_eq!(fixture.head(), head, "a conflict committed");
    assert_eq!(
        fixture.store_holds("safix/alice/both").as_deref(),
        Some("the-database-moved-too"),
        "a conflict wrote the database"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "both-ways"),
        "safix-moved-again",
        "a conflict wrote safix"
    );

    // A deleted memory is bootstrap semantics rather than a guess: both sides
    // hold a value and neither is chosen.
    assert_no_oracle(&fixture, &["safix-moved-again", "the-database-moved-too"]);
}

/// A backup mapping reports a divergence rather than resolving it.
#[test]
fn a_backup_mapping_never_overwrites_and_reports_the_divergence() {
    let mut fixture = Fixture::new();
    fixture.seed_output("back-me-up", ALICE_FILE);
    fixture.seed_sync_mapping(
        "copy",
        "backup",
        ("alice", "back-me-up"),
        "alice/copied",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "back-me-up"], "safix-value")
        .expect_success("seeding the safix side");
    fixture.store_seed("safix/alice/copied", "a-value-the-person-typed");

    let head = fixture.head();
    let run = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_refusal("a backup mapping over a differing value");
    run.says("backup  conflict");
    run.says("backup never overwrites one");
    run.says("mode = \"safix-to-keepassxc\";");
    run.silent_about("a-value-the-person-typed");
    run.silent_about("safix-value");

    assert_eq!(
        fixture.store_holds("safix/alice/copied").as_deref(),
        Some("a-value-the-person-typed"),
        "backup overwrote a value it holds it never overwrites"
    );
    assert_eq!(fixture.head(), head, "a reported divergence committed");
}

/// Every refusal, each for its own reason, and each leaving both sides alone.
#[test]
fn the_refusals_each_have_their_own_code_and_leave_both_sides_alone() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_output("pull-me", ALICE_FILE);
    fixture.seed_sync_mapping(
        "push",
        "safix-to-keepassxc",
        ("alice", "push-me"),
        "alice/pushed",
        None,
    );
    fixture.seed_sync_mapping(
        "pull",
        "keepassxc-to-safix",
        ("alice", "pull-me"),
        "alice/pulled",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    // A mapping nothing declares.
    let unknown = fixture
        .run_sync(&["sync", "typo"], UNLOCK, &extra)
        .expect_refusal("a mapping nothing declares");
    unknown.says("is not a declared mapping");
    unknown.says("push");

    // safix holds nothing to mirror, and the database holds no entry to pull.
    let empty = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_refusal("neither side holds anything");
    empty.says("mirrors push-me for alice into the database");
    empty.says("holds no entry at 'safix/alice/pulled'");
    empty.says("safix set alice push-me");

    // A value carrying a newline, refused rather than trimmed to fit.
    fixture
        .run_with(&["set", "alice", "push-me"], "trailing-newline\n")
        .expect_success("seeding a value with a trailing newline");
    let spans = fixture
        .run_sync(&["sync", "push"], UNLOCK, &extra)
        .expect_refusal("a value the store's command cannot carry");
    spans.says("carries a newline");
    spans.says("printf '%s'");
    spans.silent_about("trailing-newline");
    // Captured after the seeding writes above, so what follows is only about the
    // refusals leaving the tree where they found it.
    let head = fixture.head();
    assert!(
        fixture.store_holds("safix/alice/pushed").is_none(),
        "a refused value was written anyway"
    );

    // The database will not open.
    let mut wrong = fixture.store_env();
    wrong.push((
        "SAFIX_CARD_STUB_DB_PASSWORD".to_owned(),
        "a-different-password".to_owned(),
    ));
    let wrong = borrowed(&wrong);
    let locked = fixture
        .run_sync(&["sync"], UNLOCK, &wrong)
        .expect_refusal("a database that will not open");
    locked.says("did not open, so no mapping was judged");

    // No terminal to ask the password on, refused before anything is read.
    let headless = fixture
        .run_env(&["sync"], None, &extra)
        .expect_refusal("a run with no terminal");
    headless.says("needs its password and there is no terminal to ask on");
    headless.says("no terminal to ask on");

    // Mappings declared and no database to reach.
    fixture.forget_the_database();
    let nowhere = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_refusal("mappings with no database");
    nowhere.says("declares 2 mapping(s) and no database");
    nowhere.says("flake.safix.keepassxc.database");

    assert_eq!(fixture.head(), head, "a refusal committed");
    assert_eq!(fixture.status(), "", "a refusal left the tree dirty");
}

/// Each refusal's code, which is what a script branches on.
#[test]
fn the_refusal_codes_are_each_their_own() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_sync_mapping(
        "push",
        "safix-to-keepassxc",
        ("alice", "push-me"),
        "alice/pushed",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    // The one refusal reachable with no terminal, which is the reporter's own
    // path: `run_graphical_env` gives it pipes, so the run is refused for want of
    // a terminal and the code is the one under test.
    let locked = fixture.run_graphical_env(&["sync"], &extra);
    assert_eq!(locked.refusal_code(), "store_locked");

    fixture.forget_the_database();
    let nowhere = fixture.run_graphical_env(&["sync"], &extra);
    assert_eq!(nowhere.refusal_code(), "no_store_database");
}

/// A mapping whose safix side will not decrypt is reported, not skipped.
#[test]
fn a_mapping_that_cannot_be_judged_is_reported_rather_than_skipped() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_output("unreadable", "secrets/safix/shared/alice,bob/secrets.yaml");
    fixture.seed_sync_mapping(
        "push",
        "safix-to-keepassxc",
        ("alice", "push-me"),
        "alice/pushed",
        None,
    );
    fixture.seed_sync_mapping(
        "opaque",
        "safix-to-keepassxc",
        ("alice", "unreadable"),
        "alice/opaque",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "push-me"], "a-value-safix-holds")
        .expect_success("seeding the readable mapping");

    // A file encrypted to somebody else alone: it exists, its key is there, and
    // this operator's identity does not open it.
    let stranger = fixture.work.join("stranger.txt");
    harness::mint_identity(&stranger);
    fixture.encrypt_to(
        "secrets/safix/shared/alice,bob/secrets.yaml",
        &[&harness::recipient_of(&stranger)],
        "unreadable: \"a value this operator cannot read\"\n",
    );

    let run = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_refusal("a mapping that could not be judged");
    run.says("opaque");
    run.says("not judged");
    // The one that could be judged still was, which is what makes the report
    // about the mappings rather than about the run that met the first problem.
    run.says("push  alice.push-me -> safix/alice/pushed  safix-to-keepassxc  updated");
    assert_eq!(
        fixture.store_holds("safix/alice/pushed").as_deref(),
        Some("a-value-safix-holds")
    );
    assert!(
        fixture.store_holds("safix/alice/opaque").is_none(),
        "a mapping that could not be judged was written anyway"
    );
}

/// The database writes of one run are issued consecutively, with no read between
/// two of them.
///
/// The 292 MB rewrite is what this bounds: a save between two reads is a save the
/// burst discipline exists to avoid, and the invocation log is where the order is
/// visible.
#[test]
fn the_database_writes_of_a_run_are_one_burst() {
    let mut fixture = Fixture::new();
    for (id, name, path) in [
        ("one", "first", "alice/first"),
        ("two", "second", "alice/second"),
        ("three", "third", "alice/third"),
    ] {
        fixture.seed_output(name, ALICE_FILE);
        fixture.seed_sync_mapping(id, "safix-to-keepassxc", ("alice", name), path, None);
        fixture
            .run_with(&["set", "alice", name], &format!("value-of-{name}"))
            .expect_success("seeding a mapping");
    }
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("three mappings, all pushing");

    let invocations = fixture.store_invocations();
    let writes: Vec<usize> = invocations
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line.starts_with("add ") || line.starts_with("edit ") || line.starts_with("mkdir ")
        })
        .map(|(at, _)| at)
        .collect();
    // Three entries and the two group levels their paths needed. `mkdir` creates
    // one level, so `safix` and `safix/alice` are two invocations rather than one.
    assert_eq!(
        writes.len(),
        5,
        "three entries and two group levels: {invocations:?}"
    );

    let first = writes.first().copied().unwrap();
    let last = writes.last().copied().unwrap();
    for (at, line) in invocations.iter().enumerate() {
        if at > first && at < last {
            assert!(
                writes.contains(&at),
                "a read sits between two writes: {line}"
            );
        }
    }
}

/// An entry no mapping declares is reported as information, and left alone.
#[test]
fn an_entry_no_mapping_declares_is_reported_and_never_removed() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_sync_mapping(
        "push",
        "safix-to-keepassxc",
        ("alice", "push-me"),
        "alice/pushed",
        None,
    );
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "push-me"], "a-value")
        .expect_success("seeding the mapping");
    // What a removed mapping leaves: its entry, and the agreement it recorded.
    fixture.store_seed("safix/alice/withdrawn", "the-value-it-last-held");
    fixture.store_seed(
        "safix/alice/withdrawn.safix-sync-state",
        "safix-sync-v1 abcdef",
    );

    let run = fixture
        .run_sync(&["sync"], UNLOCK, &extra)
        .expect_success("a run beside two leftovers");
    run.says("safix/alice/withdrawn is in the group and no mapping declares it.");
    run.says("safix/alice/withdrawn.safix-sync-state is safix's own record");
    run.says("Nothing here will remove it");
    run.silent_about("the-value-it-last-held");

    assert_eq!(
        fixture.store_holds("safix/alice/withdrawn").as_deref(),
        Some("the-value-it-last-held"),
        "a leftover entry was removed"
    );
}

/// A declaration with no mapping is silent rather than refused.
///
/// The state a consumer who has never heard of this evaluates: no database, no
/// mapping, and a verb that says so rather than refusing. Nothing is asked for
/// either — the password prompt is behind the mapping selection, so a run with
/// nothing to do never reaches it.
#[test]
fn an_empty_mirror_is_silent() {
    let fixture = Fixture::new();
    let extra = store_env(&fixture);
    let extra = borrowed(&extra);
    let run = fixture
        .run_sync(&["sync"], "", &extra)
        .expect_success("a consumer who declares no mapping");
    run.says("no mapping is declared");
    assert!(
        fixture.store_invocations().is_empty(),
        "a run with no mapping opened the database"
    );
}

/// No digest and no derivative of any fixture value is committed anywhere.
///
/// Task 3.6's property, asserted after each fixture run rather than once: the
/// last-synced state is the one value-derived thing this verb computes, and the
/// whole security argument for putting it in the database is that a committed
/// digest of a secret confirms a guess offline. So the repository is searched for
/// the value, for its sha256, and for the recorded line's own shape.
fn assert_no_oracle(fixture: &Fixture, values: &[&str]) {
    for value in values {
        assert!(
            fixture.holds_anywhere(value).is_none(),
            "the value {value} reached the tree"
        );
        assert!(
            fixture
                .holds_anywhere(&sha256_hex(fixture, value.as_bytes()))
                .is_none(),
            "a digest of {value} reached the tree"
        );
    }
    assert!(
        fixture.holds_anywhere("safix-sync-v1").is_none(),
        "the recorded agreement's own tag reached the tree, so the memory is in the \
         repository rather than in the database"
    );
}

/// SHA-256 as hex, computed by `sha256sum` rather than by the runtime.
///
/// A test that asked the runtime for the digest it would have written and then
/// searched the tree for that would pass over a runtime that computed nothing at
/// all. `coreutils` is among the backends every check on this suite carries, so
/// the oracle is a program that has never seen this code.
fn sha256_hex(fixture: &Fixture, bytes: &[u8]) -> String {
    // A file rather than a pipe, because the two `sha256sum` implementations this
    // runs against differ: coreutils' reads standard input and the one on some of
    // these machines takes a path only. A path works under both. It goes in the
    // fixture's own scratch directory, which is on tmpfs and removed on every exit
    // path, because the bytes handed here are a fixture value.
    let path = fixture.work.join("oracle-input");
    std::fs::write(&path, bytes).expect("could not write the oracle's input");
    let finished = std::process::Command::new("sha256sum")
        .arg(&path)
        .output()
        .expect("could not run sha256sum");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&finished.stdout)
        .split_whitespace()
        .next()
        .expect("sha256sum printed nothing")
        .to_owned()
}
