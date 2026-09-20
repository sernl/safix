//! `safix rotate` and `safix rotation`, which act on a value's age.
//!
//! The deadline is arithmetic over two numbers the fixture writes: a declared
//! interval and a stamp record. So every claim here is made by dating a value
//! rather than by waiting, and what is asserted is which verb does what with
//! the date — the report `check` prints, the values `rotate` re-mints, the
//! ones it refuses to, and the declaration `rotation` edits.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::{ALICE_FILE, Fixture};
use serde_json::json;

/// An hour, which is the shortest interval the declarations admit.
const HOUR: u64 = 3_600;

/// A generator with no inputs, writing a fresh value into `name`.
fn mints(name: &str) -> serde_json::Value {
    json!({
        "script": format!("openssl rand -hex 8 > \"$out/{name}\""),
        "network": false,
        "runtimeInputs": ["openssl"],
        "prompts": {}, "dependencies": [], "files": {},
        "share": false, "validation": null, "description": null,
    })
}

/// The declaration `safix rotation` edits, in the shape `adduser` scaffolds
/// and a formatter leaves.
const ALICE: &str = "\
{
  flake.safix.users.alice = {
    recipient = \"age1example\";

    private = {
      api-token = {
        sopsKey = \"api-token\";
      };
    };
  };
}
";

/// A fleet with one policy, one generated entry under it and one typed entry
/// under it, both stamped an hour and a minute ago — so both are due by the
/// same minute, and the only difference between them is the remedy.
fn overdue_fleet() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ALICE_FILE, &["api-token", "mail-password"]);
    fixture.seed_generator("api-token", ALICE_FILE, &[], &mints("api-token"));
    fixture.declare_rotation("hourly", HOUR);
    fixture.govern("alice", "api-token", "hourly", HOUR);
    fixture.govern("alice", "mail-password", "hourly", HOUR);
    let an_hour_ago = safix_core::stamps::now()
        .saturating_sub(HOUR)
        .saturating_sub(60);
    fixture.stamp("alice", "api-token", an_hour_ago);
    fixture.stamp("alice", "mail-password", an_hour_ago);
    fixture
}

/// The two remedies, named by what can mint the value and nothing else.
#[test]
fn check_names_rotate_for_a_generated_value_and_set_for_a_typed_one() {
    let fixture = overdue_fleet();

    let run = fixture.run(&["check"]);
    assert!(!run.succeeded(), "an overdue value is a finding");
    run.says("which the 'hourly' rotation policy says may not be this old")
        .says("safix rotate alice api-token")
        .says("safix set alice mail-password");
}

/// A value inside its interval is not reported, and neither is one with no
/// policy at all.
#[test]
fn check_is_silent_about_a_value_inside_its_interval() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ALICE_FILE, &["api-token", "mail-password"]);
    fixture.declare_rotation("hourly", HOUR);
    fixture.govern("alice", "api-token", "hourly", HOUR);
    fixture.stamp("alice", "api-token", safix_core::stamps::now());

    fixture
        .run(&["check"])
        .silent_about("rotation policy")
        .silent_about("safix rotate");
}

/// A policy applied to a value nothing has stamped is due at once, so it
/// cannot wait out an interval that never started.
#[test]
fn a_governed_value_with_no_stamp_record_is_due() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ALICE_FILE, &["mail-password"]);
    fixture.declare_rotation("hourly", HOUR);
    fixture.govern("alice", "mail-password", "hourly", HOUR);

    fixture
        .run(&["check"])
        .says("safix set alice mail-password");
}

/// The bulk form mints what it can, lists what it cannot, exits zero, and the
/// countdown starts again for the value it minted.
#[test]
fn rotate_due_mints_the_generated_value_lists_the_typed_one_and_restarts_the_countdown() {
    let fixture = overdue_fleet();

    let before = fixture.run(&["list", "alice"]);
    before.says("ROTATES");
    assert!(
        before.output().contains("due"),
        "an overdue entry reads `due` before the run:\n{}",
        before.output()
    );

    fixture
        .run(&["rotate", "--due", "--yes"])
        .expect_success("rotating everything due")
        .says("safix set alice mail-password");

    let after = fixture.run(&["list", "alice"]).expect_success("listing");
    let rotated = after
        .output()
        .lines()
        .find(|line| line.starts_with("api-token "))
        .expect("the rotated entry has a row")
        .to_owned();
    assert!(
        rotated.ends_with("0d 01:00:00") || rotated.ends_with("0d 00:59:59"),
        "the rotated value's countdown restarts at its full hour:\n{rotated}"
    );
    fixture
        .run(&["check"])
        .says("safix set alice mail-password")
        .silent_about("safix rotate alice api-token");
}

/// Nothing due is a report rather than a failure.
#[test]
fn rotate_due_with_nothing_overdue_says_so_and_exits_zero() {
    let fixture = Fixture::new();

    fixture
        .run(&["rotate", "--due"])
        .expect_success("a run with nothing to do")
        .says("Nothing is past its rotation deadline.");
}

/// The named form on a value nothing mints refuses, and names the verb that
/// does write one.
#[test]
fn rotate_refuses_an_entry_with_no_generator_and_names_set() {
    let fixture = overdue_fleet();

    fixture
        .run(&["rotate", "alice", "mail-password"])
        .expect_refusal("rotating a typed value")
        .says("safix set alice mail-password");
}

/// The named form re-mints one entry and moves its stamp.
#[test]
fn rotate_one_entry_replaces_its_value() {
    let fixture = overdue_fleet();

    fixture
        .run(&["generate", "alice", "api-token"])
        .expect_success("minting the first value");
    let first = fixture
        .run(&["get", "alice", "api-token"])
        .expect_success("reading the first value")
        .output();

    fixture
        .run(&["rotate", "--yes", "alice", "api-token"])
        .expect_success("rotating one entry");
    let second = fixture
        .run(&["get", "alice", "api-token"])
        .expect_success("reading the rotated value")
        .output();

    assert_ne!(first, second, "a rotation mints a new value");
}

/// The scaffold writes the policy into the entry's own declaration, replaces
/// it rather than adding a second, and takes it away again.
#[test]
fn rotation_set_assigns_replaces_and_unsets_the_declaration() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.declare_rotation("hourly", HOUR);
    fixture.declare_rotation("quarterly", 90 * 86_400);
    fixture.write_declaration_text("alice", ALICE);

    fixture
        .run(&["rotation", "set", "alice", "api-token", "hourly"])
        .expect_success("assigning a policy");
    assert!(
        fixture
            .declaration_text("alice")
            .contains("rotation = \"hourly\";"),
        "the entry's declaration carries the policy:\n{}",
        fixture.declaration_text("alice")
    );

    fixture
        .run(&["rotation", "set", "alice", "api-token", "quarterly"])
        .expect_success("replacing the policy");
    let replaced = fixture.declaration_text("alice");
    assert!(replaced.contains("rotation = \"quarterly\";"));
    assert!(
        !replaced.contains("rotation = \"hourly\";"),
        "a second policy replaces the first rather than joining it:\n{replaced}"
    );

    fixture
        .run(&["rotation", "unset", "alice", "api-token"])
        .expect_success("removing the policy");
    assert!(
        !fixture.declaration_text("alice").contains("rotation ="),
        "unset removes the line rather than nulling it:\n{}",
        fixture.declaration_text("alice")
    );
}

/// A policy the declarations do not define is refused before the file is
/// touched, and the refusal names the ones they do.
#[test]
fn rotation_set_refuses_an_undeclared_policy_and_edits_nothing() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.declare_rotation("quarterly", 90 * 86_400);
    fixture.write_declaration_text("alice", ALICE);

    fixture
        .run(&["rotation", "set", "alice", "api-token", "monthly"])
        .expect_refusal("an undeclared policy")
        .says("quarterly");
    assert_eq!(
        fixture.declaration_text("alice"),
        ALICE,
        "a refused assignment writes nothing"
    );
}

/// An entry the declarations do not place is refused the same way.
#[test]
fn rotation_set_refuses_an_unknown_entry() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.declare_rotation("quarterly", 90 * 86_400);
    fixture.write_declaration_text("alice", ALICE);

    fixture
        .run(&["rotation", "set", "alice", "nothing-here", "quarterly"])
        .expect_refusal("an unknown entry")
        .says("nothing-here");
}
