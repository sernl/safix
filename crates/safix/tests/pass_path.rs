//! `safix sync pass` and `safix audit pass`, driven against a modelled store
//! and the real everything else.
//!
//! sops, age and git are real here as everywhere in this suite; the store's own
//! command is the stub `tests/support/pass-stub.rs` models, for the reason that
//! file states — the claims are about the delegation, and a stub can be asked
//! what it saw — plus the one that matters most on this path: the real command
//! replaces entries under `--force` without asking, and the store on the
//! machines this is developed on is the operator's own. `pass_cli.rs` is the
//! other half of that sentence, and it drives the real command against a store
//! it creates in its own temporary directory with its own `GNUPGHOME`.
//!
//! `harness::refuse_a_real_store` is the structural guard, and it is why every
//! run below goes through [`Fixture::run_sync`] with [`Fixture::pass_env`]: a
//! run whose `SAFIX_PASS` does not name the stub, or whose
//! `PASSWORD_STORE_DIR` is anywhere but the fixture's own scratch directory,
//! fails before a process is spawned.
//!
//! # Which check runs which test
//!
//! `modules/flake/checks/cli.nix` runs each test below as its own check
//! attribute, one filter per function: `safix-pass-push`, `safix-pass-pull`,
//! `safix-pass-backup`, `safix-pass-two-way`, `safix-pass-conflict`,
//! `safix-pass-memory`, `safix-pass-interrupted`, `safix-pass-multiline`,
//! `safix-pass-locked`, `safix-pass-no-store`, `safix-pass-pipes`,
//! `safix-pass-field-source` and `safix-pass-fields-diverged`. A test renamed
//! here without the check being renamed turns that check red rather than
//! green, because `runOne` reads the result line for a filter that named
//! nothing.
//!
//! Every expectation below is asserted against a literal written in this file,
//! never against a value re-derived through `safix_core::pass` — the
//! `behavioural-suite` rule at `openspec/specs/behavioural-suite/spec.md`.

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
use serde_json::json;

/// No prompt at all on this path: `pass` shells to gpg and the unlock belongs
/// to the operator's own agent, so there is nothing for a run to feed.
const NOTHING_TO_FEED: &str = "";

fn borrowed(extra: &[(String, String)]) -> Vec<(&str, &str)> {
    extra
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// A fixture with one mapping of each mode, and alice holding a value for each.
fn declared() -> Fixture {
    let mut fixture = Fixture::new();
    for name in ["push-me", "pull-me", "both-ways", "back-me-up"] {
        fixture.seed_output(name, ALICE_FILE);
    }

    fixture.seed_pass_mapping(
        "push",
        "safix-to-pass",
        ("alice", "push-me"),
        "alice/pushed",
        json!({
            "username": "alice@example.com",
            "url": "https://grafana.example.invalid",
            "notes": "minted by safix",
            "tags": ["work", "fleet"]
        }),
    );
    fixture.seed_pass_mapping(
        "pull",
        "pass-to-safix",
        ("alice", "pull-me"),
        "alice/pulled",
        json!({}),
    );
    fixture.seed_pass_mapping(
        "both",
        "two-way",
        ("alice", "both-ways"),
        "alice/both",
        json!({}),
    );
    fixture.seed_pass_mapping(
        "copy",
        "backup",
        ("alice", "back-me-up"),
        "alice/copied",
        json!({}),
    );
    fixture.pass_store_exists();
    fixture
}

/// A push writes the value and every declared field into one record.
#[test]
fn a_push_writes_the_value_and_the_declared_fields_in_one_record() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "push-me"], "safix-push-me")
        .expect_success("seeding the safix side");

    let run = fixture
        .run_sync(&["sync", "pass", "push"], NOTHING_TO_FEED, &extra)
        .expect_success("a push into an absent entry");
    run.says("push  alice.push-me -> alice/pushed  safix-to-pass  updated");

    // The literal the layout produces, written out here rather than derived:
    // the value's bytes, one blank line, then the four field lines in order.
    assert_eq!(
        fixture.pass_holds("alice/pushed").as_deref(),
        Some(
            "safix-push-me\n\
             \n\
             login: alice@example.com\n\
             url: https://grafana.example.invalid\n\
             notes: minted by safix\n\
             tags: work, fleet\n"
        ),
        "the record the store holds is not the one the layout writes"
    );

    // One write, not two: the whole body crosses on one pipe.
    assert_eq!(
        fixture.pass_recorded("writes"),
        vec!["1".to_owned()],
        "a push issued more than one write for one record"
    );

    run.silent_about("safix-push-me");
}

/// A pull writes the value alone into safix, and no field.
#[test]
fn a_pull_writes_only_the_value_into_safix() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "pull-me"], "safix-before-the-pull")
        .expect_success("seeding the entry a pull overwrites");
    fixture.pass_seed(
        "alice/pulled",
        "store-pulled\n\nlogin: someone@example.invalid\n",
    );

    let run = fixture
        .run_sync(&["sync", "pass", "pull"], NOTHING_TO_FEED, &extra)
        .expect_success("a pull from the store");
    run.says("pull  alice/pulled -> alice.pull-me  pass-to-safix  pulled");

    // The value alone: a safix entry is a placement with no slot for a field,
    // so the field block the record carried did not cross.
    assert_eq!(
        fixture.value(ALICE_FILE, "pull-me"),
        "store-pulled",
        "safix did not converge to the store's value, or took the field block with it"
    );
    run.silent_about("store-pulled");
    run.silent_about("someone@example.invalid");
}

/// A `backup` mapping never overwrites an entry holding a different value, and
/// writes no field either.
#[test]
fn a_backup_never_overwrites_a_differing_entry() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "back-me-up"], "safix-back-me-up")
        .expect_success("seeding the safix side");
    fixture.pass_seed("alice/copied", "the-persons-own-value");

    let run = fixture
        .run_sync(&["sync", "pass", "copy"], NOTHING_TO_FEED, &extra)
        .expect_refusal("a backup mapping meeting a differing entry");
    run.says("copy  alice.back-me-up <-> alice/copied  backup  conflict");
    run.says("backup never overwrites one");

    assert_eq!(
        fixture.pass_holds("alice/copied").as_deref(),
        Some("the-persons-own-value"),
        "a backup mapping overwrote an entry holding a different value"
    );
    assert!(
        fixture.pass_recorded("writes").is_empty(),
        "a backup mapping issued a write over an existing entry"
    );
}

/// A two-way mapping converges toward whichever side changed.
#[test]
fn a_two_way_mapping_converges_toward_the_changed_side() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "both-ways"], "agreed-value")
        .expect_success("seeding the safix side");

    // Bootstrap: the store holds nothing, so the value crosses toward it and
    // the agreement is recorded.
    fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_success("a two-way mapping with an empty store side");
    assert_eq!(
        fixture.pass_holds("alice/both").as_deref(),
        Some("agreed-value"),
        "a two-way mapping with an empty store side did not bootstrap"
    );

    // Now the store side moves, and nothing else does.
    fixture.pass_seed("alice/both", "the-store-moved");
    let run = fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_success("a two-way mapping whose store side moved");
    run.says("both  alice/both -> alice.both-ways  two-way  pulled");
    assert_eq!(
        fixture.value(ALICE_FILE, "both-ways"),
        "the-store-moved",
        "a two-way mapping did not converge toward the side that changed"
    );
}

/// Both sides changed is a conflict that writes nothing and names two
/// remedies.
#[test]
fn both_sides_changed_is_a_conflict_naming_two_remedies() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "both-ways"], "agreed-value")
        .expect_success("seeding the safix side");
    fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_success("recording an agreement to diverge from");

    // Both sides move, away from the recorded agreement and from each other.
    fixture.pass_seed("alice/both", "the-store-moved");
    fixture
        .run_with(&["set", "alice", "both-ways"], "safix-moved-too")
        .expect_success("moving safix's side as well");

    let run = fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_refusal("a two-way mapping whose two sides both moved");
    run.says("both  alice.both-ways <-> alice/both  two-way  conflict");
    run.says("mode = \"safix-to-pass\";");
    run.says("mode = \"pass-to-safix\";");

    assert_eq!(
        fixture.pass_holds("alice/both").as_deref(),
        Some("the-store-moved"),
        "a conflict wrote the store's side"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "both-ways"),
        "safix-moved-too",
        "a conflict wrote safix's side"
    );
    run.silent_about("the-store-moved");
    run.silent_about("safix-moved-too");
}

/// A mapping accounts for its own companion, so its memory is never reported as
/// an entry nothing declares.
#[test]
fn a_two_way_mappings_memory_is_not_lingering() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "both-ways"], "agreed-value")
        .expect_success("seeding the safix side");
    fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_success("recording an agreement");

    // The companion is there, beside the mapped entry and inside the store.
    let companion = fixture
        .pass_holds("alice/both.safix-sync-state")
        .expect("a two-way mapping recorded no agreement");
    assert!(
        companion.starts_with("safix-sync-v1 "),
        "the recorded agreement carries no format tag: {companion}"
    );

    let run = fixture
        .run_sync(&["audit", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_success("an audit over a mapping whose memory exists");
    run.silent_about("alice/both.safix-sync-state is in the store and no mapping declares it");
}

/// The memory is written after the value, so an interrupted run leaves the
/// older memory rather than a newer one.
#[test]
fn an_interrupted_two_way_run_leaves_the_older_memory() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "both-ways"], "agreed-value")
        .expect_success("seeding the safix side");
    fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_success("recording the first agreement");
    let first = fixture
        .pass_holds("alice/both.safix-sync-state")
        .expect("no agreement was recorded at all");

    // The interruption, modelled where it actually falls: the value landed and
    // the memory's own write did not. The store's side is the new value and the
    // memory still describes the old one.
    fixture.pass_seed("alice/both", "a-value-the-memory-does-not-describe");
    let after = fixture
        .pass_holds("alice/both.safix-sync-state")
        .expect("the memory disappeared");
    assert_eq!(
        after, first,
        "a write to the value moved the memory, which is the order this refuses"
    );

    // safix moves too, and the next run reads an older memory: a conflict that
    // writes nothing, which is the safe direction.
    fixture
        .run_with(
            &["set", "alice", "both-ways"],
            "safix-moved-after-the-crash",
        )
        .expect_success("moving safix's side after the interruption");
    let run = fixture
        .run_sync(&["sync", "pass", "both"], NOTHING_TO_FEED, &extra)
        .expect_refusal("a run reading a memory older than both sides");
    run.says("two-way  conflict");
    assert_eq!(
        fixture.pass_holds("alice/both").as_deref(),
        Some("a-value-the-memory-does-not-describe"),
        "a run over an older memory overwrote the newer value"
    );
}

/// A value spanning lines round-trips byte for byte, a trailing newline
/// included.
#[test]
fn a_multi_line_value_round_trips_byte_for_byte() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    // A value with three lines and a trailing newline, which is the shape the
    // keepassxc target refuses and this one carries.
    let value = "-----BEGIN KEY-----\nline two\nline three\n";
    fixture
        .run_with(&["set", "alice", "push-me"], value)
        .expect_success("seeding a multi-line value");

    fixture
        .run_sync(&["sync", "pass", "push"], NOTHING_TO_FEED, &extra)
        .expect_success("pushing a multi-line value");

    // The literal, written out here: the value's own bytes, then the blank
    // separator, then the field block. The value's final newline is its own and
    // is not the separator.
    assert_eq!(
        fixture.pass_holds("alice/pushed").as_deref(),
        Some(
            "-----BEGIN KEY-----\nline two\nline three\n\
             \n\
             \n\
             login: alice@example.com\n\
             url: https://grafana.example.invalid\n\
             notes: minted by safix\n\
             tags: work, fleet\n"
        ),
        "a multi-line value did not survive the write byte for byte"
    );

    // And reading it back agrees: a second run over the same tree writes
    // nothing, which is only true if the parse recovered exactly what was
    // written.
    let again = fixture
        .run_sync(&["sync", "pass", "push"], NOTHING_TO_FEED, &extra)
        .expect_success("a second run over a multi-line value");
    again.says("safix-to-pass  unchanged");
}

/// A decrypt the agent declined is its own refusal, not an absent entry.
#[test]
fn a_locked_agent_is_not_an_absent_entry() {
    let fixture = declared();
    let mut extra = fixture.pass_env();
    extra.push((
        "SAFIX_PASS_STUB_LOCKED".to_owned(),
        "alice/copied".to_owned(),
    ));
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "back-me-up"], "safix-back-me-up")
        .expect_success("seeding the safix side");
    // The entry is there — its name is in the listing — and it will not
    // decrypt. A `backup` mapping meeting that answer must not write.
    fixture.pass_seed("alice/copied", "the-persons-own-value");

    let run = fixture
        .run_sync(&["sync", "pass", "copy"], NOTHING_TO_FEED, &extra)
        .expect_refusal("a decrypt the agent declined");
    run.says("did not decrypt");
    run.says("gpg: decryption failed: No secret key");
    run.says("not the absent-entry refusal");
    run.silent_about("the store holds no entry at");

    assert_eq!(
        fixture.pass_holds("alice/copied").as_deref(),
        Some("the-persons-own-value"),
        "a backup mapping wrote over an entry it merely could not read"
    );
}

/// A declared store that is not one refuses before any mapping is read.
#[test]
fn an_absent_store_refuses_before_any_mapping_is_read() {
    let mut fixture = declared();
    // Inside the scratch directory, which the guard requires, and not a store:
    // no directory there at all, so no `.gpg-id` either.
    let absent = fixture.work.join("not-a-store");
    fixture.pass_store_is(&absent);
    let mut extra = fixture.pass_env();
    extra.push((
        "PASSWORD_STORE_DIR".to_owned(),
        absent.to_string_lossy().into_owned(),
    ));
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "push-me"], "safix-push-me")
        .expect_success("seeding the safix side");

    let run = fixture
        .run_sync(&["sync", "pass"], NOTHING_TO_FEED, &extra)
        .expect_refusal("a declared store that is not a store");
    run.says("is not a pass store");
    run.says("pass init");
    // No mapping was looked at, so none of them is reported at all — which is
    // the property the refusal's own position exists for.
    run.silent_about("safix-to-pass  unchanged");
    assert!(
        fixture.pass_recorded("argv").is_empty(),
        "a run refused for an absent store still invoked the store's command"
    );
}

/// No value and no field reaches an argument vector or the environment.
#[test]
fn no_value_and_no_field_reaches_an_argument_vector_or_the_environment() {
    let fixture = declared();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "push-me"], "safix-push-me")
        .expect_success("seeding the safix side");

    fixture
        .run_sync(&["sync", "pass", "push"], NOTHING_TO_FEED, &extra)
        .expect_success("a push whose vectors are being observed");

    // Every argument vector, as the thing safix ran recorded it.
    let vectors = fixture.pass_recorded("argv");
    assert!(
        !vectors.is_empty(),
        "the store's command was never invoked, so this asserts nothing"
    );
    for vector in &vectors {
        for forbidden in [
            "safix-push-me",
            "alice@example.com",
            "https://grafana.example.invalid",
            "minted by safix",
        ] {
            assert!(
                !vector.contains(forbidden),
                "the argument vector {vector:?} carries {forbidden:?}"
            );
        }
    }

    // And the environment: one variable beyond what the run itself set, and it
    // is a location rather than a value.
    let environment = fixture.pass_recorded("env");
    assert!(
        environment
            .iter()
            .any(|line| line.starts_with("PASSWORD_STORE_DIR=")),
        "the store's location did not reach the child's environment"
    );
    for line in &environment {
        for forbidden in ["safix-push-me", "minted by safix"] {
            assert!(
                !line.contains(forbidden),
                "the environment line {line:?} carries {forbidden:?}"
            );
        }
    }
}

/// A field sourced from another entry is resolved at run time and never
/// printed.
#[test]
fn a_field_sourced_from_another_entry_is_resolved_and_never_printed() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_output("grafana-login", ALICE_FILE);
    fixture.seed_pass_mapping(
        "sourced",
        "safix-to-pass",
        ("alice", "push-me"),
        "alice/sourced",
        json!({"username": {"entry": "grafana-login"}}),
    );
    fixture.pass_store_exists();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "push-me"], "safix-push-me")
        .expect_success("seeding the value");
    fixture
        .run_with(&["set", "alice", "grafana-login"], "a-secret-login")
        .expect_success("seeding the entry the field is sourced from");

    let run = fixture
        .run_sync(&["sync", "pass", "sourced"], NOTHING_TO_FEED, &extra)
        .expect_success("a field sourced from another entry");
    run.says("sourced  alice.push-me -> alice/sourced  safix-to-pass  updated");

    // Resolved and written into the body — this is the one target where an
    // `{ entry = …; }` source is admissible, because nothing it carries travels
    // an argument vector.
    assert_eq!(
        fixture.pass_holds("alice/sourced").as_deref(),
        Some("safix-push-me\n\nlogin: a-secret-login\n"),
        "the sourced field did not reach the record body"
    );

    run.silent_about("a-secret-login");
    for vector in fixture.pass_recorded("argv") {
        assert!(
            !vector.contains("a-secret-login"),
            "the argument vector {vector:?} carries the resolved field's value"
        );
    }
    for line in fixture.pass_recorded("env") {
        assert!(
            !line.contains("a-secret-login"),
            "the environment line {line:?} carries the resolved field's value"
        );
    }
}

/// A field-only divergence is reported as its own word, naming the field and
/// never its content.
#[test]
fn a_field_only_divergence_is_reported_as_its_own_word() {
    let mut fixture = Fixture::new();
    fixture.seed_output("pull-me", ALICE_FILE);
    fixture.seed_pass_mapping(
        "drifted",
        "pass-to-safix",
        ("alice", "pull-me"),
        "alice/drifted",
        json!({"notes": "what the declaration says"}),
    );
    fixture.pass_store_exists();
    let extra = fixture.pass_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "pull-me"], "the-agreed-value")
        .expect_success("seeding the safix side");
    // The values agree and the declared field does not, which is the only state
    // that reports `fields diverged`.
    fixture.pass_seed(
        "alice/drifted",
        "the-agreed-value\n\nnotes: what the record says\n",
    );

    let run = fixture
        .run_sync(&["sync", "pass", "drifted"], NOTHING_TO_FEED, &extra)
        .expect_refusal("a field-only divergence under a pulling mode");
    run.says("pass-to-safix  fields diverged");
    run.says("these declared fields differ: notes");
    // The field's own content is a secret on either side, so neither appears.
    run.silent_about("what the declaration says");
    run.silent_about("what the record says");

    assert_eq!(
        fixture.pass_holds("alice/drifted").as_deref(),
        Some("the-agreed-value\n\nnotes: what the record says\n"),
        "a pulling mode wrote the store's side over a field divergence"
    );
}
