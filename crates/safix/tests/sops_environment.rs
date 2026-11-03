//! Every sops child safix starts is denied every Vault server, unless the
//! operator named the servers it may contact.
//!
//! sops sends the operator's Vault token to whatever `vault_address` a file's
//! metadata names (GHSA-jgf3-f6rg-8x3h). `SOPS_HC_VAULT_ALLOWLIST=none` makes
//! sops refuse the connection before it reads the token. safix supports no
//! Vault recipients, so it sets that value wherever the operator left the
//! variable unset or empty, and passes an operator's own value through.
//!
//! The `set` of a name whose file does not exist yet is observed because it
//! starts sops through both of safix's builders: the one that creates the empty
//! document, and the one that writes the value into it.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::io::Write as _;
use std::path::Path;
use std::process::Stdio;

use harness::{ALICE_FILE, Fixture, real_sops, shim};

/// With the allowlist unset, both invocations carry `none`.
#[test]
fn every_sops_child_is_denied_every_vault_server() {
    let fixture = Fixture::new();
    let spool = fixture.scratch("spy");
    assert!(
        !fixture.exists(ALICE_FILE),
        "the file already exists, so the run would not create it"
    );

    set_observed(&fixture, &spool, None);

    let invocations = invocations(&spool);
    let environ = read(&spool.join("environ"));
    assert_eq!(
        environ.matches("SOPS_HC_VAULT_ALLOWLIST=none\n").count(),
        invocations,
        "a sops invocation was not denied every Vault server:\n{environ}"
    );
}

/// An allowlist the operator set reaches every invocation unchanged.
#[test]
fn an_operator_allowlist_reaches_sops_unchanged() {
    let fixture = Fixture::new();
    let spool = fixture.scratch("spy");

    set_observed(&fixture, &spool, Some("https://vault.example.invalid/"));

    let invocations = invocations(&spool);
    let environ = read(&spool.join("environ"));
    assert_eq!(
        environ
            .matches("SOPS_HC_VAULT_ALLOWLIST=https://vault.example.invalid/\n")
            .count(),
        invocations,
        "the operator's allowlist did not reach every sops invocation:\n{environ}"
    );
    assert!(
        !environ.contains("SOPS_HC_VAULT_ALLOWLIST=none"),
        "safix replaced the operator's allowlist with none:\n{environ}"
    );
}

/// `set` a value through the recording sops, with the allowlist unset or set
/// to the given value, and require it to round-trip.
fn set_observed(fixture: &Fixture, spool: &Path, allowlist: Option<&str>) {
    let sops = real_sops();
    let mut command = fixture.command(&["set", "alice", "api-token"]);
    command
        .env("SAFIX_SOPS", shim())
        .env("SAFIX_SHIM_ROLE", "spy")
        .env("SAFIX_SHIM_SOPS", &sops)
        .env("SAFIX_SHIM_SPY", spool);
    match allowlist {
        Some(value) => command.env("SOPS_HC_VAULT_ALLOWLIST", value),
        None => command.env_remove("SOPS_HC_VAULT_ALLOWLIST"),
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("could not spawn the set");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"CANARY-observed-value")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "the set through the observed sops failed\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Without the round trip the counts below would be satisfied by a run
    // that never reached sops at all.
    assert_eq!(
        fixture.value(ALICE_FILE, "api-token"),
        "CANARY-observed-value",
        "the value did not round-trip"
    );
}

/// How many times the recording sops ran, having required both the
/// invocation that creates the file and the one that writes the value.
fn invocations(spool: &Path) -> usize {
    let argv = read(&spool.join("argv"));
    let recorded: Vec<&str> = argv
        .lines()
        .filter(|line| Path::new(line).file_name() == Path::new(shim()).file_name())
        .collect();
    assert!(
        argv.lines().any(|line| line == "encrypt"),
        "no invocation created the file:\n{argv}"
    );
    assert!(
        argv.lines().any(|line| line == "set"),
        "no invocation wrote the value:\n{argv}"
    );
    recorded.len()
}

/// A spool file, or an empty string when the shim never wrote one.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
