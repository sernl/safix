//! The harness shown to work, and its one stub shown to assert.
//!
//! A fixture that silently built nothing would make every test below it green
//! for the wrong reason, so the fixture is exercised on its own: a key minted
//! here, a value written and read through the real sops, and the stubbed
//! evaluator observed to refuse an attribute the runtime does not name.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use std::process::Command;

use harness::{ALICE_FILE, Fixture};

/// The fixture mints a key, writes a value through the real sops, and reads it
/// back.
#[test]
fn the_fixture_stands_up_a_repository_with_real_backends() {
    let fixture = Fixture::new();

    assert!(
        fixture.alice.starts_with("age1"),
        "alice's recipient is minted"
    );
    assert!(fixture.bob.starts_with("age1"), "bob's recipient is minted");
    assert_ne!(
        fixture.alice, fixture.bob,
        "the two are distinct identities"
    );

    fixture.make_sops_file(ALICE_FILE, &["api-token"]);
    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "fixture-value-for-api-token",
        "the fixture value round-trips through sops"
    );
    assert!(
        fixture.read(ALICE_FILE).contains("ENC[AES256_GCM"),
        "the file holds sops ciphertext"
    );
    assert_eq!(
        fixture.subject("HEAD"),
        format!("fixture: {ALICE_FILE}"),
        "the fixture committed the file it wrote"
    );
    assert_eq!(fixture.status(), "", "the fixture leaves a clean tree");
}

/// The stub answers the attributes the runtime declares.
#[test]
fn the_evaluator_stub_answers_what_the_runtime_reads() {
    let fixture = Fixture::new();
    let listing = fixture.run(&["list", "alice"]).expect_success("list");
    listing.says("api-token");
    listing.says(ALICE_FILE);
}

/// A rename of an attribute fails in the suite rather than at an operator's
/// terminal.
///
/// The drill for the whole harness: the stub is asked for an attribute the
/// runtime does not name, and must refuse rather than answer.
#[test]
fn the_evaluator_stub_refuses_an_attribute_the_runtime_does_not_name() {
    let fixture = Fixture::new();
    let command = fixture.command(&["list", "alice"]);
    let stub = command.get_program().to_owned();
    drop(command);

    let mut probe = Command::new(env!("CARGO_BIN_EXE_safix-nix-stub"));
    probe
        .arg("eval")
        .arg("--json")
        .arg(format!("{}#safix.lib.placementz", fixture.repo.display()))
        .env("SAFIX_REPO_ROOT", &fixture.repo);
    let refused = probe.output().unwrap();
    assert!(
        !refused.status.success(),
        "a renamed attribute was answered"
    );
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("unexpected attribute"),
        "the stub does not say which attribute it refused"
    );

    let mut mode = Command::new(env!("CARGO_BIN_EXE_safix-nix-stub"));
    mode.arg("eval")
        .arg("--raw")
        .arg(format!("{}#safix.lib.placements", fixture.repo.display()))
        .env("SAFIX_REPO_ROOT", &fixture.repo);
    let refused = mode.output().unwrap();
    assert!(
        !refused.status.success(),
        "an attribute read in the wrong mode was answered"
    );

    assert!(
        stub.to_string_lossy().ends_with("safix"),
        "the fixture drives the built binary"
    );
}
