//! `safix group`, which edits a declaration and never a value.
//!
//! The claims are the ones a hand edit would have made the operator responsible
//! for. That an addition is one inserted line and every name that was there
//! survives it. That the recipient policy is regenerated from the declarations the
//! edit implies and committed beside it, in that order — an evaluation reads the
//! files git tracks, so a run that regenerated first would write the policy of the
//! membership as it stood before the edit. That a removal says what it does not
//! undo and names the report that will carry the shrink. And that a group an
//! organization's silo declarations cover is that organization's managers' to edit
//! and nobody else's, while a group no silo set names is anybody's, exactly as
//! before.
//!
//! # What is fixtured and what is asserted
//!
//! The evaluation is stubbed, so what follows an edit — the audience the group's
//! membership now implies — is a document the harness writes. That the real
//! resolver follows a membership change is `modules/flake/checks/subjects.nix`'s
//! claim, made against literals over the real algebra; what is asserted here is
//! that the verb's own disclosure and `check`'s report name the same thing about
//! the same tree.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{Fixture, SHARED_FILE};

/// The declaration a fleet writes for a group of two, as a formatter leaves it.
const ONCALL: &str = "\
{
  # who carries the pager
  flake.safix.groups.oncall.members = [
    \"alice\"
    \"bob\"
  ];
}
";

/// A group covered by no silo set, so nobody's to manage.
const STANDBY: &str = "{\n  flake.safix.groups.standby.members = [ \"alice\" ];\n}\n";

/// acme manages bob and covers `oncall` through its silo declarations; alice
/// manages for acme, mallory is declared and manages nothing, and `standby` is
/// covered by nobody.
fn managed_fleet() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.seed_holder_of_nothing("mallory");
    fixture.seed_holder_of_nothing("carol");
    fixture.delegate("acme", &["alice"], &["bob"]);
    fixture.declare_group("oncall", &["alice", "bob"], &["acme"]);
    fixture.declare_group("standby", &["alice"], &[]);
    fixture.write_group_declaration("oncall", ONCALL);
    fixture.write_group_declaration("standby", STANDBY);
    fixture
}

/// The addition: one line, the policy re-derived, both committed.
#[test]
fn an_addition_is_one_line_and_the_policy_is_re_derived_beside_it() {
    let fixture = managed_fleet();
    fixture.commit_as("alice", "alice@example.com");
    // The committed policy grants alice alone while the declarations imply a rule
    // for both, so a run that regenerates it changes the file — which is what
    // makes "the policy re-derives" an observation rather than a claim about two
    // identical documents.
    fixture.write_policy(&["alice"]);
    fixture.git(&[
        "commit",
        "-q",
        "-am",
        "fixture: a policy behind the declarations",
    ]);

    let run = fixture
        .run(&["group", "add", "oncall", "carol"])
        .expect_success("adding carol to oncall");
    run.says("carol is a member of oncall");
    run.says("Membership growth is a re-wrap");
    run.says("safix fix");

    let declaration = fixture.read("safix/groups/oncall.nix");
    assert_eq!(
        declaration.lines().count(),
        ONCALL.lines().count() + 1,
        "the addition was not one inserted line: {declaration}"
    );
    assert!(declaration.contains("    \"carol\""));
    for kept in ["# who carries the pager", "\"alice\"", "\"bob\""] {
        assert!(
            declaration.contains(kept),
            "the edit lost {kept}: {declaration}"
        );
    }

    // The policy was regenerated from the declarations the edit implies, and the
    // two were committed together.
    assert!(
        fixture.read(".sops.yaml").contains("&bob"),
        "the policy was not regenerated: {}",
        fixture.read(".sops.yaml")
    );
    assert_eq!(
        fixture.paths_in("HEAD"),
        vec![
            ".sops.yaml".to_owned(),
            "safix/groups/oncall.nix".to_owned()
        ],
        "the commit is not the declaration and the policy that saw it"
    );
    assert_eq!(
        fixture.status(),
        "",
        "the run left the tree dirty: {}",
        fixture.status()
    );

    // The delegation is recorded in the commit, in the words the run announced.
    let message = fixture.message("HEAD");
    assert!(
        message.contains("add carol to the oncall group"),
        "the commit does not name the act: {message}"
    );
    assert!(
        message.contains(
            "Scaffolded by alice for acme, whose silo declarations cover \
             flake.safix.groups.oncall."
        ),
        "the commit does not record the organization the edit was performed for: {message}"
    );

    // A second run writes nothing and commits nothing.
    let head = fixture.head();
    let again = fixture
        .run(&["group", "add", "oncall", "carol"])
        .expect_success("adding carol twice");
    again.says("already a member of oncall; nothing was written");
    assert_eq!(fixture.head(), head, "an idempotent re-run committed");
}

/// The removal: one line gone, the disclosure made, the report named.
#[test]
fn a_removal_says_what_it_does_not_undo_and_the_next_check_reports_the_shrink() {
    let mut fixture = managed_fleet();
    fixture.commit_as("alice", "alice@example.com");
    // A file the group's audience covers, holding a value bob has been able to
    // read. This is the state a shrink is discovered in.
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);

    let run = fixture
        .run(&["group", "remove", "oncall", "bob"])
        .expect_success("removing bob from oncall");
    run.says("bob is no longer a member of oncall");
    run.says("takes nothing back");
    run.says("no re-wrap unreads it");
    run.says("safix check reports the shrink as the revocation it is");
    run.says("explicitly not that remedy");

    let declaration = fixture.read("safix/groups/oncall.nix");
    assert_eq!(
        declaration.lines().count(),
        ONCALL.lines().count() - 1,
        "the removal was not one removed line: {declaration}"
    );
    assert!(!declaration.contains("\"bob\""));
    assert!(declaration.contains("\"alice\""), "a bystander was lost");

    // What an evaluation would derive from the edited membership, written here
    // because the evaluation is stubbed: the file's audience no longer names bob
    // and its ciphertext is exactly where it was. `check` then reports the shrink
    // as the revocation the verb said it was.
    let alice = fixture.alice.clone();
    fixture.set_audience(SHARED_FILE, &["alice"], &[&alice]);
    fixture.write_policy(&["alice"]);

    let report = fixture.run(&["check"]);
    assert_eq!(report.code, Some(1), "check did not report the shrink");
    report.says(&format!(
        "{SHARED_FILE} is not encrypted to the audience declared for it"
    ));
    report.says("so this is a revocation");
    report.says("Only a new value revokes.");

    // Removing somebody the group does not hold writes nothing and commits
    // nothing.
    let head = fixture.head();
    let again = fixture
        .run(&["group", "remove", "oncall", "bob"])
        .expect_success("removing bob twice");
    again.says("is not a member of oncall; nothing was removed");
    assert_eq!(fixture.head(), head, "an idempotent re-run committed");
}

/// The delegation over groups: silo coverage decides it, and an uncovered group is
/// anybody's.
#[test]
fn a_covered_group_is_its_organizations_and_an_uncovered_one_is_anybodys() {
    let fixture = managed_fleet();
    let head = fixture.head();
    let declaration = fixture.read("safix/groups/oncall.nix");

    // mallory is declared and manages nothing, so the covered group is refused —
    // with the coverage named, not a consent.
    fixture.commit_as("mallory", "mallory@example.com");
    let refused = fixture
        .run_graphical(&["group", "add", "oncall", "carol"])
        .expect_refusal("an out-of-scope group edit");
    assert_eq!(refused.refusal_code(), "scaffold_out_of_scope");
    let refused = fixture
        .run(&["group", "add", "oncall", "carol"])
        .expect_refusal("an out-of-scope group edit");
    refused.says("flake.safix.groups.oncall is delegated to flake.safix.organizations.acme");
    refused.says("by the flake.safix.silos sets that hold it");
    refused.says("mallory is not among the managers named there");
    refused.says("flake.safix.organizations.acme.managers");
    refused.says("are not authorization");
    assert_eq!(
        fixture.read("safix/groups/oncall.nix"),
        declaration,
        "a refused edit wrote to the declaration"
    );
    assert_eq!(fixture.head(), head, "a refused edit committed");

    // The same identity, over a group no silo set covers: nothing is consulted,
    // nothing is mentioned, and the edit goes through. mallory is not even
    // declared as a manager anywhere, which is the point.
    let permitted = fixture
        .run(&["group", "add", "standby", "bob"])
        .expect_success("editing a group nobody manages");
    permitted.silent_about("manager");
    permitted.silent_about("acme");
    assert!(
        fixture.read("safix/groups/standby.nix").contains("\"bob\""),
        "the uncovered group was not edited"
    );
    assert!(
        !fixture.message("HEAD").contains("Scaffolded by"),
        "an unmanaged edit recorded a delegation: {}",
        fixture.message("HEAD")
    );

    // And a manager of the covering organization is within scope for the covered
    // group.
    fixture.commit_as("alice", "alice@example.com");
    fixture
        .run(&["group", "add", "oncall", "carol"])
        .expect_success("a manager editing the group acme covers");
}

/// Everything the verb refuses before it edits anything.
#[test]
fn refusals_each_have_their_own_code_and_leave_the_declaration_alone() {
    let mut fixture = managed_fleet();
    fixture.commit_as("alice", "alice@example.com");

    // The two declarations the last two refusals need, written before HEAD is
    // read: the fixture's own writes commit, so a HEAD read before them would make
    // the assertion below about the fixture rather than about a refusal.
    fixture.declare_group("infra", &["alice"], &[]);
    fixture.declare_group("computed", &["alice"], &[]);
    fixture.write_group_declaration(
        "computed",
        "{\n  flake.safix.groups.computed.members = lib.mkAfter [ \"alice\" ];\n}\n",
    );

    let head = fixture.head();
    let declaration = fixture.read("safix/groups/oncall.nix");
    let mut codes = Vec::new();

    // A group the declarations do not name.
    let refused = fixture
        .run(&["group", "add", "oncal", "bob"])
        .expect_refusal("a group nobody declared");
    refused.says("'oncal' is not a declared group of flake.safix.groups");
    refused.says("- oncall");
    codes.push(
        fixture
            .run_graphical(&["group", "add", "oncal", "bob"])
            .refusal_code(),
    );

    // A subject the declarations do not name. Refused rather than written,
    // because a membership naming one is refused at the next evaluation.
    let refused = fixture
        .run(&["group", "add", "oncall", "zed"])
        .expect_refusal("a subject nobody declared");
    refused.says("'zed' is not a declared subject");
    refused.says("refused at the next evaluation");
    codes.push(
        fixture
            .run_graphical(&["group", "add", "oncall", "zed"])
            .refusal_code(),
    );

    // A declaration this verb has no file for. The group is declared and the
    // membership lives somewhere else, which is supported and is not editable
    // here.
    let refused = fixture
        .run(&["group", "add", "infra", "bob"])
        .expect_refusal("a group declared somewhere else");
    refused.says("safix/groups/infra.nix is not a group declaration this can edit");
    refused.says("declarations merge");
    codes.push(
        fixture
            .run_graphical(&["group", "add", "infra", "bob"])
            .refusal_code(),
    );

    // A `members` value the editor cannot read, which the option itself would
    // take. Refused rather than compounded.
    let refused = fixture
        .run(&["group", "add", "computed", "bob"])
        .expect_refusal("a membership computed elsewhere");
    refused.says("is not a group declaration this can edit");

    // The usage line, for an act that is neither.
    fixture
        .run(&["group", "invite", "oncall", "bob"])
        .expect_refusal("a third act")
        .says("usage: safix group add|remove <group> <subject>");
    fixture
        .run(&["group", "add", "oncall"])
        .expect_refusal("a missing subject")
        .says("usage: safix group add|remove <group> <subject>");

    assert_eq!(
        codes,
        vec!["unknown_group", "unknown_subject", "no_group_declaration"],
        "two refusals about a group share one code"
    );
    assert_eq!(fixture.head(), head, "a refusal committed something");
    assert_eq!(
        fixture.read("safix/groups/oncall.nix"),
        declaration,
        "a refusal edited the declaration"
    );
}
