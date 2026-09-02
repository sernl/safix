//! The bridge and mirror audits: which declared mappings' two sides no longer
//! agree.
//!
//! The transfer suite next door establishes that a value crosses the clan
//! boundary correctly. This one establishes what is true of a bridge between
//! transfers, which is the state the clan-target audit exists to name: a
//! mapping exported once and since replaced on clan's side reads as agreeing
//! to anything that does not compare the two sides, and clan replacing it is
//! silent by construction — `design.md` records that hazard and this is the
//! report over it. The keepassxc target reports the same kind of question over
//! the password database, without ever converging what it finds — that is
//! `sync`'s job.
//!
//! Every clan here is the stub, for the reasons `tests/support/clan-stub.rs`
//! gives, and every database here is the modelled one `tests/support/
//! card-stubs.rs` drives, for the reason `sync_path.rs`'s own header gives.
//! What the audit asks of either is one read per mapping and nothing else, so
//! the write counters are what show that a report is a report: an audit that
//! wrote would move them.
//!
//! Two claims run through every test below. The report names the mapping and
//! never a value, asserted with canaries distinct on each side so that a report
//! leaking either one fails on the side it leaked. And a mapping the operator
//! cannot decrypt, or a database entry the store's own command refuses over, is
//! reported rather than skipped, because a report that dropped it would be a
//! report about who ran it.
//!
//! # The drills, and what each one broke
//!
//! Both were observed red, and each was caught by the test whose claim it
//! breaks rather than incidentally by another.
//!
//! Making two present values agree — the comparison in `audit::compare`
//! answering `None` for the differing pair — fails
//! `a_diverged_mapping_is_reported_naming_the_mapping_and_no_value` on the exit
//! status, and `a_transfer_resolves_what_the_audit_reported` on there being
//! nothing to resolve. That second one is what shows the first is not passing
//! by calling everything diverged.
//!
//! Skipping the mapping whose safix side does not decrypt — the shape the
//! decision rejected — fails
//! `a_mapping_the_operator_cannot_decrypt_is_reported_rather_than_skipped`
//! alone, and leaves every other test in this target green. That is the point
//! of it: the report reads clean while being a report about who ran it, and
//! nothing else here notices.

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

/// Both directions, so that every claim below is made of a mapping whichever
/// way its value moves.
const DIRECTIONS: [&str; 2] = ["clan-to-safix", "safix-to-clan"];

/// The password the modelled database is opened with, fed to the one prompt.
const UNLOCK: &str = "fixture-database-password\n";

/// One run of `audit clan`, with the stubbed clan in place.
///
/// The shape the transfer suite drives its verb through, because the audit
/// reaches clan the same way it does and a second shape here would be a second
/// thing to keep true.
fn audit(fixture: &Fixture, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
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

/// A fixture carrying one bridge mapping of the given direction.
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

/// The environment a keepassxc-target audit needs: the store stub, and the
/// password it expects.
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

// ── the clan target: the two sides compared ────────────────────────────────

/// Two sides holding different values are reported, naming the mapping and
/// neither value.
///
/// The claim task 4.4 asks for, and the state D5 names: an exported value clan
/// has since replaced is invisible to anything that does not compare the two
/// sides, so this is what makes it visible. Asserted in both directions,
/// because a divergence is a divergence whichever way the mapping runs.
#[test]
fn a_diverged_mapping_is_reported_naming_the_mapping_and_no_value() {
    for direction in DIRECTIONS {
        let fixture = with_mapping(direction);
        fixture
            .set("alice", "api-token", "CANARY-held-by-safix")
            .expect_success("seeding the safix side");
        fixture.clan_seed(MACHINE, VAR, "CANARY-held-by-clan");
        let before = fixture.head();

        let report = audit(&fixture, &["audit", "clan"], &[]);
        assert_eq!(
            report.code,
            Some(1),
            "the audit did not report the divergence in the {direction} direction\n{}",
            report.combined()
        );

        report.says("ntfy-token");
        report.says("different values");
        report.says("1 finding(s) over 1 mapping(s)");
        report.silent_about("CANARY-held-by-safix");
        report.silent_about("CANARY-held-by-clan");

        // The report is a report. Nothing was written on either side of the
        // boundary, and the remedy is named rather than taken.
        report.says("safix sync clan ntfy-token");
        assert_eq!(fixture.head(), before, "the audit committed something");
        assert_eq!(fixture.clan_writes(), 0, "the audit wrote into clan");
    }
}

/// Two sides holding the same value produce no finding.
///
/// The other half of task 4.4, and the half that makes a clean report mean
/// something: without it every assertion above would hold over a report that
/// called every mapping diverged.
#[test]
fn a_mapping_whose_two_sides_agree_produces_no_finding() {
    for direction in DIRECTIONS {
        let fixture = with_mapping(direction);
        fixture
            .set("alice", "api-token", "CANARY-in-step")
            .expect_success("seeding the safix side");
        fixture.clan_seed(MACHINE, VAR, "CANARY-in-step");

        let report = audit(&fixture, &["audit", "clan"], &[])
            .expect_success("auditing a mapping whose two sides agree");
        report.says("no disagreement");
        report.says("All 1 declared mapping(s) agree");
        report.silent_about("CANARY-in-step");
        report.silent_about("finding(s)");
    }
}

/// A `sync clan` run converges a diverged mapping, and the audit then reports
/// nothing.
///
/// The two verbs related to each other rather than each asserted alone: the
/// audit's whole claim is that it reports what a sync would resolve, and a
/// report that stayed red after the sync would be reporting something else.
#[test]
fn a_sync_resolves_what_the_audit_reported() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-to-be-exported")
        .expect_success("seeding the safix side");
    fixture.clan_seed(MACHINE, VAR, "CANARY-clan-still-holds");

    let before = audit(&fixture, &["audit", "clan"], &[]);
    assert_eq!(before.code, Some(1), "the audit found nothing to report");

    audit(&fixture, &["sync", "clan"], &[]).expect_success("converging the diverged mapping");

    let after = audit(&fixture, &["audit", "clan"], &[])
        .expect_success("auditing after the sync converged it");
    after.says("no disagreement");
}

/// One side holding a value the other does not is a finding, and which side it
/// is decides what the report names.
///
/// A destination holding nothing is a mapping nothing has transferred yet, and
/// the verb resolves it. A source holding nothing is a mapping with nothing to
/// send, which that verb would refuse — so the report names minting the source
/// instead, and the two are asserted apart because a report that named the
/// remedy in both cases would be pointing at a refusal.
#[test]
fn one_side_holding_a_value_alone_is_reported_by_which_side_it_is() {
    let source = with_mapping("safix-to-clan");
    source
        .set("alice", "api-token", "CANARY-never-exported")
        .expect_success("seeding the source");

    let report = audit(&source, &["audit", "clan"], &[]);
    assert_eq!(report.code, Some(1), "an untransferred mapping is in step");
    report.says("holds a value that");
    report.says("safix sync clan ntfy-token");
    report.silent_about("CANARY-never-exported");

    let destination = with_mapping("clan-to-safix");
    destination
        .set("alice", "api-token", "CANARY-from-nowhere")
        .expect_success("seeding the destination");

    let backwards = audit(&destination, &["audit", "clan"], &[]);
    assert_eq!(
        backwards.code,
        Some(1),
        "a mapping whose source holds nothing is in step"
    );
    backwards.says("holds no value while");
    backwards.says(&format!("mint the source at {MACHINE} {VAR}"));
    backwards.silent_about("CANARY-from-nowhere");
}

/// Neither side holding a value is a bridge nobody has bootstrapped, not a
/// disagreement.
///
/// Reporting it would report every mapping of a fresh consumer's bridge under a
/// remedy this report cannot name, and would mean a first run is red for having
/// declared the bridge correctly.
#[test]
fn a_mapping_neither_side_holds_a_value_for_is_not_a_finding() {
    for direction in DIRECTIONS {
        audit(&with_mapping(direction), &["audit", "clan"], &[])
            .expect_success("auditing a mapping nothing has minted yet")
            .says("no disagreement");
    }
}

// ── what the clan-target report cannot quietly leave out ──────────────────

/// A mapping this operator cannot decrypt is reported as unjudged rather than
/// skipped.
///
/// The severity evidence for the decision recorded in `design.md`: the shape
/// that was rejected is bridge rows for the mappings the caller can decrypt and
/// silence for the rest, which makes the report a function of who ran it and
/// leaves a clean one meaning something narrower than it reads as. This drives
/// exactly that case — a file encrypted to somebody else — and requires the run
/// to be red and to say so.
#[test]
fn a_mapping_the_operator_cannot_decrypt_is_reported_rather_than_skipped() {
    let fixture = with_mapping("safix-to-clan");
    let stranger = fixture.new_recipient();
    fixture.clan_seed(MACHINE, VAR, "CANARY-held-by-clan");

    fixture.encrypt_to(
        ALICE_FILE,
        &[&stranger],
        "api-token: \"CANARY-alice-cannot-read\"\n",
    );
    fixture.git(&["add", "--", ALICE_FILE]);
    fixture.git(&["commit", "-q", "-m", "fixture: encrypted to someone else"]);

    let report = audit(&fixture, &["audit", "clan"], &[]);
    assert_eq!(
        report.code,
        Some(1),
        "a mapping that could not be judged passed silently\n{}",
        report.combined()
    );
    report.says("ntfy-token");
    report.says("could not be judged");
    report.says("rather than left out");
    report.silent_about("CANARY-held-by-clan");
    report.silent_about("CANARY-alice-cannot-read");
    report.silent_about("no disagreement");
}

/// No clan refuses the whole run before any mapping is compared.
///
/// The audit needs clan for the same reason `sync clan` does and refuses on
/// the same terms: a report that quietly compared the mappings it could reach
/// would say "agrees" about a side it never looked at.
#[test]
fn an_absent_clan_refuses_the_clan_target_before_anything_is_compared() {
    let fixture = with_mapping("safix-to-clan");
    fixture
        .set("alice", "api-token", "CANARY-never-compared")
        .expect_success("seeding the safix side");

    let refused = audit(
        &fixture,
        &["audit", "clan"],
        &[("SAFIX_CLAN", "safix-no-such-clan-command")],
    )
    .expect_refusal("an audit with no clan installed");

    refused.says("clan is the authority on its own store");
    refused.silent_about("no disagreement");
    refused.silent_about("finding(s) over");
    refused.silent_about("CANARY-never-compared");
}

/// A consumer who has never heard of clan is not refused for having no bridge.
#[test]
fn an_empty_bridge_is_silent_rather_than_refused() {
    audit(&Fixture::new(), &["audit", "clan"], &[])
        .expect_success("an audit over an empty bridge")
        .says("no mapping is declared");
}

// ── naming one mapping ─────────────────────────────────────────────────────

/// A named mapping narrows the run to it, in either direction.
///
/// There is no wrong verb here: comparing a mapping is the same act whichever
/// way its value moves, and this is the assertion of that — the same name a
/// `--direction` filter on `sync` would refuse is accepted here regardless.
#[test]
fn a_named_mapping_is_audited_whichever_direction_it_is_declared() {
    for direction in DIRECTIONS {
        let fixture = with_mapping(direction);
        fixture
            .set("alice", "api-token", "CANARY-named")
            .expect_success("seeding the safix side");
        fixture.clan_seed(MACHINE, VAR, "CANARY-named");

        audit(&fixture, &["audit", "clan", "ntfy-token"], &[])
            .expect_success("auditing one named mapping")
            .says("All 1 declared mapping(s) agree");
    }
}

/// A mapping name nothing declares is refused, naming what is declared.
#[test]
fn an_undeclared_mapping_name_is_refused_naming_the_declared_ones() {
    let fixture = with_mapping("safix-to-clan");

    let refused = audit(&fixture, &["audit", "clan", "ntfy-tokne"], &[])
        .expect_refusal("a mapping name nothing declares");
    refused.says("'ntfy-tokne' is not a declared mapping");
    refused.says("ntfy-token");
}

// ── the keepassxc target: compare-only, lingering, no writes ──────────────

/// `audit keepassxc` compares both sides of a diverged mapping and writes
/// nothing to either.
#[test]
fn audit_keepassxc_compares_a_diverged_mapping_without_writing() {
    let mut fixture = Fixture::new();
    fixture.seed_output("password", ALICE_FILE);
    fixture.seed_sync_mapping(
        "grafana",
        "safix-to-keepassxc",
        ("alice", "password"),
        "alice/grafana",
        None,
    );
    fixture
        .run_with(&["set", "alice", "password"], "CANARY-safix-side")
        .expect_success("seeding the safix side");
    fixture.store_seed("safix/alice/grafana", "CANARY-database-side");

    let extra = store_env(&fixture);
    let extra = borrowed(&extra);
    let report = fixture.run_sync(&["audit", "keepassxc"], UNLOCK, &extra);
    assert_eq!(
        report.code,
        Some(1),
        "a diverged keepassxc mapping did not fail the run\n{}",
        report.combined()
    );
    report.says("diverged");
    report.says("safix sync keepassxc grafana");
    report.silent_about("CANARY-safix-side");
    report.silent_about("CANARY-database-side");

    assert_eq!(
        fixture.value(ALICE_FILE, "password"),
        "CANARY-safix-side",
        "the audit wrote the safix side"
    );
    assert_eq!(
        fixture.store_holds("safix/alice/grafana").as_deref(),
        Some("CANARY-database-side"),
        "the audit wrote the database side"
    );
}

/// `audit keepassxc` reports agreement when both sides hold the same value.
#[test]
fn audit_keepassxc_reports_agreement_when_both_sides_match() {
    let mut fixture = Fixture::new();
    fixture.seed_output("password", ALICE_FILE);
    fixture.seed_sync_mapping(
        "grafana",
        "safix-to-keepassxc",
        ("alice", "password"),
        "alice/grafana",
        None,
    );
    fixture
        .run_with(&["set", "alice", "password"], "CANARY-in-step")
        .expect_success("seeding the safix side");
    fixture.store_seed("safix/alice/grafana", "CANARY-in-step");

    let extra = store_env(&fixture);
    let extra = borrowed(&extra);
    fixture
        .run_sync(&["audit", "keepassxc"], UNLOCK, &extra)
        .expect_success("a mapping whose two sides agree")
        .says("agreeing");
}

/// Entries under the declared group that no mapping declares are reported as
/// information, and are never removed by running the audit.
#[test]
fn audit_keepassxc_reports_lingering_entries_without_removing_them() {
    let mut fixture = Fixture::new();
    fixture.seed_output("password", ALICE_FILE);
    fixture.seed_sync_mapping(
        "grafana",
        "safix-to-keepassxc",
        ("alice", "password"),
        "alice/grafana",
        None,
    );
    fixture
        .run_with(&["set", "alice", "password"], "CANARY-in-step")
        .expect_success("seeding the safix side");
    fixture.store_seed("safix/alice/grafana", "CANARY-in-step");
    fixture.store_seed("safix/alice/orphan", "CANARY-leftover");

    let extra = store_env(&fixture);
    let extra = borrowed(&extra);
    let report = fixture
        .run_sync(&["audit", "keepassxc"], UNLOCK, &extra)
        .expect_success("a lingering entry does not fail the run on its own");
    report.says("safix/alice/orphan");
    report.says("no mapping declares it");
    assert_eq!(
        fixture.store_holds("safix/alice/orphan").as_deref(),
        Some("CANARY-leftover"),
        "the audit removed the lingering entry"
    );
}

// ── bare audit: both targets ───────────────────────────────────────────────

/// Bare `audit`, with no target named, compares both the clan target and the
/// keepassxc target in one run.
#[test]
fn bare_audit_covers_both_targets() {
    let mut fixture = with_mapping("clan-to-safix");
    fixture.seed_output("password", ALICE_FILE);
    fixture.seed_sync_mapping(
        "grafana",
        "safix-to-keepassxc",
        ("alice", "password"),
        "alice/grafana",
        None,
    );
    fixture
        .run_with(&["set", "alice", "password"], "CANARY-in-step")
        .expect_success("seeding the keepassxc mapping's safix side");
    fixture.store_seed("safix/alice/grafana", "CANARY-in-step");
    // The clan mapping is left unbootstrapped, which is agreement rather than a
    // finding — the clan-target claim `a_mapping_neither_side_holds_a_value_for_is_not_a_finding`
    // already establishes.

    let mut environment = fixture.clan_env();
    environment.extend(store_env(&fixture));
    let extra: Vec<(&str, &str)> = environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();

    let report = fixture
        .run_sync(&["audit"], UNLOCK, &extra)
        .expect_success("a bare audit over both targets, both clean");
    report.says("All 1 declared mapping(s) agree");
    report.says("agreeing");
}

// ── the page an operator is shown ──────────────────────────────────────────

/// The verb is on the one page an operator is shown, and explains itself.
///
/// The scaffold listing is held to the verb table by a unit test; what this
/// adds is that the built command prints both, so a verb that compiled into the
/// table and out of the help would fail here.
#[test]
fn the_audit_appears_in_the_help_and_says_why_it_is_not_check() {
    let fixture = Fixture::new();

    fixture
        .run(&["--help"])
        .expect_success("the general help")
        .says("safix audit");

    let help = fixture
        .run(&["audit", "-h"])
        .expect_success("the audit's own help");
    help.says("safix audit [clan|keepassxc]");
    help.says("decrypts nothing");
    help.says("rather than skipped");
}
