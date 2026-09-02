//! Converging a two-way mapping toward whichever side changed, over the
//! stubbed clan.
//!
//! `tests/bridge.rs`'s own header states why stubbing clan is permitted here
//! and what it can and cannot establish; this file exercises the same
//! boundary for `bridge_sync::converge` rather than for the one-way
//! transfers. What is real here is safix's own side of the convergence: sops,
//! age and git write and read for real, including the companion entry's own
//! write, so "the value lands before the agreement, as two separate commits"
//! is asserted against the repository's own history rather than against a
//! reading of the code.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod harness;

use harness::{ALICE_FILE, Fixture, Run};

/// The var a fixture's two-way mapping names on clan's side.
const VAR: &str = "ntfy/token";

/// The machine a per-machine two-way mapping names.
const MACHINE: &str = "meridian";

/// One run of `sync clan`, with the stubbed clan in place.
fn converge(fixture: &Fixture, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
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

/// A fixture carrying one per-machine two-way mapping, its companion
/// placement minted beside it.
fn with_two_way_mapping() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_two_way_mapping(
        "bothways",
        (MACHINE, "ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture
}

/// The companion entry's own key, as `bridge.nix`'s `stateSuffix` names it.
const COMPANION_KEY: &str = "api-token-safix-bridge-sync-state";

// ── the four outcome classes ────────────────────────────────────────────────

/// Neither side holding anything writes nothing and reports unchanged.
#[test]
fn neither_side_holding_anything_is_unchanged_and_writes_nothing() {
    let fixture = with_two_way_mapping();
    let before = fixture.head();

    let run = converge(&fixture, &["sync", "clan", "bothways"], &[])
        .expect_success("converging with neither side holding a value");
    run.says("bothways");
    run.says("unchanged");

    assert_eq!(fixture.head(), before, "an unchanged run committed");
    assert_eq!(fixture.clan_writes(), 0, "an unchanged run wrote into clan");
}

/// safix holding a value and clan holding none bootstraps toward clan, and
/// the value lands before the agreement, as two separate commits.
#[test]
fn safix_only_bootstraps_toward_clan_and_records_the_agreement_as_a_second_commit() {
    let fixture = with_two_way_mapping();
    fixture
        .set("alice", "api-token", "CANARY-safix-bootstrap")
        .expect_success("seeding the safix side");
    let after_seeding = fixture.head();

    let run = converge(&fixture, &["sync", "clan", "bothways"], &[])
        .expect_success("bootstrapping toward clan");
    run.says("converged bothways");
    run.silent_about("CANARY-safix-bootstrap");

    assert_eq!(
        fixture.clan_holds(MACHINE, VAR).as_deref(),
        Some("CANARY-safix-bootstrap"),
        "clan does not hold what safix was holding"
    );
    assert_eq!(
        fixture.clan_writes(),
        1,
        "clan was asked to write more than once"
    );

    // A push writes clan's side through clan's own command, external to this
    // repository, and the companion afterward as this repository's own,
    // single new commit \u{2014} there is no "value's own commit" here the way
    // there is on the pull direction, because the value never lands in this
    // repository at all.
    assert_ne!(fixture.head(), after_seeding, "no commit landed");
    assert_eq!(
        fixture.subject("HEAD"),
        "chore(safix): remember the agreement for bothways for alice",
        "the agreement's commit does not name the mapping"
    );
    assert_eq!(
        fixture.subject("HEAD~1"),
        fixture.subject(&after_seeding),
        "more than one commit landed in this repository for a push"
    );

    // The companion holds a digest, distinct from the value itself, and reads
    // back as something other than the plaintext.
    let companion = fixture.value(ALICE_FILE, COMPANION_KEY);
    assert!(
        companion.starts_with("safix-bridge-sync-v1 "),
        "the companion does not carry the expected format tag"
    );
    assert!(
        !companion.contains("CANARY-safix-bootstrap"),
        "the companion holds the value rather than a digest of it"
    );
}

/// clan holding a value and safix holding none bootstraps toward safix, and
/// the value lands before the agreement, as two separate commits.
#[test]
fn clan_only_bootstraps_toward_safix_and_records_the_agreement_as_a_second_commit() {
    let fixture = with_two_way_mapping();
    fixture.clan_seed(MACHINE, VAR, "CANARY-clan-bootstrap");

    let run = converge(&fixture, &["sync", "clan", "bothways"], &[])
        .expect_success("bootstrapping toward safix");
    run.says("converged bothways");
    run.silent_about("CANARY-clan-bootstrap");

    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-clan-bootstrap",
        "safix does not hold what clan was holding"
    );
    assert_eq!(
        fixture.subject("HEAD~1"),
        "chore(safix): converge bothways for alice",
        "the value's own commit does not name the mapping"
    );
    assert_eq!(
        fixture.subject("HEAD"),
        "chore(safix): remember the agreement for bothways for alice",
        "the agreement's commit does not name the mapping"
    );
}

/// Both sides holding different values with no agreement recorded yet is a
/// conflict: nothing is written on either side, and the report names the
/// remedy.
#[test]
fn both_sides_holding_different_values_with_no_agreement_is_a_conflict() {
    let fixture = with_two_way_mapping();
    fixture
        .set("alice", "api-token", "CANARY-safix-side")
        .expect_success("seeding the safix side");
    fixture.clan_seed(MACHINE, VAR, "CANARY-clan-side");
    let before = fixture.head();

    let run = converge(&fixture, &["sync", "clan", "bothways"], &[])
        .expect_refusal("a conflict is what makes the run's exit code non-zero");
    run.says("bothways");
    run.says("conflict");
    run.says("direction = \"safix-to-clan\"");
    run.says("direction = \"clan-to-safix\"");
    run.says("safix sync clan bothways");
    run.silent_about("CANARY-safix-side");
    run.silent_about("CANARY-clan-side");

    assert_eq!(fixture.head(), before, "a conflict committed");
    assert_eq!(fixture.clan_writes(), 0, "a conflict wrote into clan");
    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-safix-side",
        "a conflict overwrote safix's side"
    );
}

/// A divergence after a bootstrap converges using the recorded agreement,
/// proving the companion's own write is read back by a later run rather than
/// only ever written.
#[test]
fn a_later_divergence_converges_using_the_recorded_agreement() {
    let fixture = with_two_way_mapping();
    fixture
        .set("alice", "api-token", "CANARY-first")
        .expect_success("seeding the safix side");

    converge(&fixture, &["sync", "clan", "bothways"], &[]).expect_success("the bootstrap run");
    assert_eq!(fixture.clan_writes(), 1, "the bootstrap did not write once");

    // safix moves again; clan stays exactly where the bootstrap left it, which
    // is what the recorded agreement should now read as unmoved.
    fixture
        .set("alice", "api-token", "CANARY-second")
        .expect_success("moving the safix side again");

    let run = converge(&fixture, &["sync", "clan", "bothways"], &[])
        .expect_success("converging the second divergence");
    run.says("converged bothways");

    assert_eq!(
        fixture.clan_holds(MACHINE, VAR).as_deref(),
        Some("CANARY-second"),
        "clan did not converge to safix's second value"
    );
    assert_eq!(
        fixture.clan_writes(),
        2,
        "the second divergence did not write into clan exactly once more"
    );
}

// ── the safix-to-clan discipline ────────────────────────────────────────────

/// A two-way push toward clan is refused by the identical stale-generator
/// refusal a safix-to-clan write carries, under the identical condition.
#[test]
fn a_stale_generator_refuses_a_two_way_push_toward_clan() {
    let fixture = with_two_way_mapping();
    fixture
        .set("alice", "api-token", "CANARY-would-be-lost")
        .expect_success("seeding the safix side");

    let run = converge(
        &fixture,
        &["sync", "clan", "bothways"],
        &[("SAFIX_CLAN_STUB_STALE", "ntfy")],
    )
    .expect_refusal("a refused mapping is what makes the run's exit code non-zero");
    run.says("bothways");
    run.says("outdated");
    run.says("clan vars generate meridian");
    run.silent_about("CANARY-would-be-lost");

    assert_eq!(
        fixture.clan_writes(),
        0,
        "the stale-generator refusal still wrote into clan"
    );
}

// ── addressing a shared placement ───────────────────────────────────────────

/// A shared-placement two-way mapping's clan side is reached by a machine
/// discovered from clan's own `machines list`, never a declared one.
///
/// The stub cannot express "a real candidate machine that does not declare
/// this generator at all" \u{2014} its `vars get` only distinguishes a globally
/// unknown var id from a known one that has or has not been generated \u{2014} so
/// this covers the discovery path with a single-candidate answer; the search
/// stopping at the first success and the exhaustion refusal over more than
/// one candidate are unit tested directly against `bridge::Addressing`
/// (`bridge.rs::tests`), where the stub is a bash script this suite's
/// own controls.
#[test]
fn a_shared_placements_machine_is_discovered_from_clan() {
    let mut fixture = Fixture::new();
    fixture.seed_two_way_mapping_shared(
        "bothways-shared",
        ("ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture.clan_seed("meridian", VAR, "CANARY-shared-clan-value");

    let run = converge(
        &fixture,
        &["sync", "clan", "bothways-shared"],
        &[("SAFIX_CLAN_STUB_MACHINES", "meridian")],
    )
    .expect_success("a shared mapping's machine is discovered rather than declared");
    run.says("converged bothways-shared");

    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-shared-clan-value",
        "the shared mapping did not converge through the discovered machine"
    );
}
