//! Whose act a scaffold is, over the verb that edits a person's custody record.
//!
//! Three fixtures, which are the three states the feature has. A manager
//! scaffolding for somebody who consented to that: the run proceeds and its commit
//! says whose act it was. Somebody outside the delegation attempting the same
//! thing: refused before the card is selected and before any file is written, with
//! the organization and its managers named. And a person no delegation covers:
//! nothing is consulted, nothing is mentioned, and a run by an identity the
//! declarations do not name at all goes through — which is the sharpest form the
//! compatibility promise can be asserted in, because a verb that consulted
//! delegation there would refuse.
//!
//! # Why the identity is set in the repository
//!
//! The acting identity is the one a commit made here would carry, so a test that
//! wanted to act as somebody sets `git config user.name` in the fixture
//! repository. There is no flag to pass instead, deliberately — see
//! `safix_core::delegation` — and the harness removes `GIT_AUTHOR_NAME` and its
//! neighbours from every run's environment so that whoever is running the suite
//! cannot be the answer.
//!
//! # Why the card is stubbed
//!
//! For the reason the head of `tests/support/card-stubs.rs` gives, which is a
//! safety property rather than a convenience. The permitted run below is the whole
//! ceremony against that surface, and its proof does not pass because no card is
//! there — the same observation `enrollment.rs` makes, and not what these tests
//! are about.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{Fixture, SHARED_FILE};

/// The serial the fixture's one card answers with.
const SERIAL: &str = "12345678";

/// The recipient the stubbed generator prints for it.
///
/// Synthetic, and nothing here encrypts to it: the `age1yubikey1` prefix is the
/// load-bearing part, because it is what makes the captured block a card's.
const CARD: &str = "age1yubikey1qfixture000000000000000000000000000000000000000000000000";

/// The environment one enrollment run needs, with the card's own switches.
///
/// The same shape `enrollment.rs` builds; duplicated rather than shared because
/// each test target compiles on its own and a helper module for two lines of
/// pushes would be a module.
fn card_env(fixture: &Fixture, serials: &str, recipient: &str) -> Vec<(String, String)> {
    let mut environment = fixture.card_env();
    environment.push(("SAFIX_CARD_STUB_SERIALS".to_owned(), serials.to_owned()));
    environment.push(("SAFIX_CARD_STUB_RECIPIENT".to_owned(), recipient.to_owned()));
    environment
}

fn as_pairs(environment: &[(String, String)]) -> Vec<(&str, &str)> {
    environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// A fixture where acme manages bob, alice manages for acme, and mallory is a
/// declared person who manages nothing.
fn managed_fleet() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    // A file bob's audience covers, so the ceremony's proof has something to open
    // and the re-wrap has something to re-wrap.
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);
    fixture.seed_holder_of_nothing("mallory");
    fixture.delegate("acme", &["alice"], &["bob"]);
    fixture
}

/// The permitted scaffold, and the record it leaves of whose act it was.
#[test]
fn a_manager_scaffolds_for_a_managed_person_and_the_commit_records_the_organization() {
    let fixture = managed_fleet();
    fixture.commit_as("alice", "alice@example.com");

    let environment = card_env(&fixture, SERIAL, CARD);
    let run = fixture.run_on_terminal(
        &["enroll", "bob", "--no-store-pin"],
        "",
        &as_pairs(&environment),
    );

    // The delegation is stated before anything is edited, in the words the commit
    // will carry: who is acting, for whom, and which declaration says so.
    run.says("alice is a declared manager of acme");
    run.says("which flake.safix.users.bob.managedBy names");

    // The scaffold went through: the card is in bob's record.
    let declaration = fixture.read("safix/users/bob.nix");
    assert!(
        declaration.contains(CARD),
        "the card did not reach bob's record: {declaration}"
    );

    // And the commit says whose act it was, in its body rather than its subject:
    // the subject names the ceremony, and this is the other half — what it was
    // performed as.
    let message = fixture.message("HEAD");
    assert!(
        message.contains("enroll 12345678 as a recovery recipient for bob"),
        "the ceremony's commit is not the one that names it: {message}"
    );
    assert!(
        message
            .contains("Scaffolded by alice for acme, which flake.safix.users.bob.managedBy names."),
        "the commit does not record the organization the scaffold was performed for: {message}"
    );
}

/// The refusals, both of them, before the card and before any edit.
#[test]
fn an_out_of_scope_actor_is_refused_before_the_card_and_before_any_file() {
    let fixture = managed_fleet();
    let head = fixture.head();
    let declaration = fixture.read("safix/users/bob.nix");
    let environment = card_env(&fixture, SERIAL, CARD);

    // mallory is declared and manages nothing, which is the refusal the spec's
    // scenario names: the organization, and where its managers are declared.
    fixture.commit_as("mallory", "mallory@example.com");
    let refused = fixture
        .run_on_terminal(
            &["enroll", "bob", "--no-store-pin"],
            "",
            &as_pairs(&environment),
        )
        .expect_refusal("an out-of-scope enrollment");
    refused.says("flake.safix.users.bob is delegated to flake.safix.organizations.acme");
    refused.says("mallory is not among the managers named there");
    refused.says("flake.safix.organizations.acme.managers");
    refused.says("- alice");
    refused.says("are not authorization");

    // An identity no declaration corresponds to is its own refusal, because its
    // remedy is a different one: no edit to a managers list would help.
    fixture.commit_as("Somebody Else", "somebody@example.com");
    let unmatched = fixture
        .run_on_terminal(
            &["enroll", "bob", "--no-store-pin"],
            "",
            &as_pairs(&environment),
        )
        .expect_refusal("an enrollment by an undeclarable identity");
    unmatched.says("would be authored by 'Somebody Else <somebody@example.com>'");
    unmatched.says("flake.safix.users declares nobody of that name");
    unmatched.says("git config user.name");

    // Neither refusal reached the card, and neither touched the tree. The card
    // matters as much as the tree here: provisioning replaces a PIN and a PUK, and
    // a refusal that had got that far would have changed something irreversible
    // while refusing to change anything reversible.
    assert_eq!(
        fixture.card_recorded("argv"),
        "",
        "a refused run reached the card"
    );
    assert_eq!(fixture.head(), head, "a refused run committed something");
    assert_eq!(
        fixture.read("safix/users/bob.nix"),
        declaration,
        "a refused run edited the record it was refused over"
    );
    assert_eq!(
        fixture.status(),
        "",
        "a refused run left the tree dirty: {}",
        fixture.status()
    );

    // Both codes, read under the graphical reporter, which is where a code is
    // rendered. Two refusals rather than one, because the two have different
    // remedies.
    for (identity, expected) in [
        ("mallory", "scaffold_out_of_scope"),
        ("Somebody Else", "actor_undeclared"),
    ] {
        fixture.commit_as(identity, "someone@example.com");
        let mut graphical = as_pairs(&environment);
        graphical.push(("SAFIX_ERROR_FORMAT", ""));
        assert_eq!(
            fixture
                .run_on_terminal(&["enroll", "bob", "--no-store-pin"], "", &graphical)
                .refusal_code(),
            expected,
            "the refusal for {identity} carries the wrong code"
        );
    }
}

/// A person no delegation covers, scaffolded by an identity nobody declares.
#[test]
fn an_unmanaged_person_never_consults_delegation() {
    let fixture = managed_fleet();
    // Nobody the declarations name, which is the identity the managed target
    // above refuses outright. alice declares no `managedBy`, so this run must not
    // reach the check at all.
    fixture.commit_as("Somebody Else", "somebody@example.com");

    // The card surface answers with no card connected, so the run refuses there —
    // which is the observation: it got past the delegation gate, which sits before
    // the card is selected.
    let environment = card_env(&fixture, "", CARD);
    let refused = fixture
        .run_on_terminal(
            &["enroll", "alice", "--no-store-pin"],
            "",
            &as_pairs(&environment),
        )
        .expect_refusal("an enrollment with no card");
    refused.says("no card is connected");

    // And it said nothing about delegation, because there was none to consult.
    refused.silent_about("manager");
    refused.silent_about("acme");
    refused.silent_about("managedBy");
}
