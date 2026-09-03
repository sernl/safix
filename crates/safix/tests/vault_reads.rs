//! `edit`, `get`, `sync clan` and `sync keepassxc` read a vault-rooted
//! ciphertext document from the vault root, not the declaration root, once
//! `flake.safix.vault` is declared (tasks 2.10/2.11).
//!
//! Each test seeds the document straight at [`Fixture::vault_root`], never at
//! the declaration root, so a read that reached for the wrong root would find
//! nothing there and fail — `EmptyValue` for `edit`, `NoValueYet` for `get`,
//! `AbsentAtSource` for `sync` — rather than succeed on a coincidence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::{ALICE_FILE, Fixture};

/// `safix edit` reads the current value from the vault root: an editor that
/// leaves the buffer untouched reports "unchanged" only if what it opened was
/// the seeded plaintext, which lives only at the vault root.
#[test]
fn edit_reads_the_vault_document_from_the_vault_root() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.encrypt_to_vault(
        ALICE_FILE,
        &[&fixture.alice.clone()],
        "api-token: from-vault\n",
    );

    let editor = fixture.scratch("noop-editor.sh");
    std::fs::write(&editor, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = std::fs::metadata(&editor).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
    std::fs::set_permissions(&editor, permissions).unwrap();

    let run = fixture.run_env(
        &["edit", "alice", "api-token"],
        None,
        &[
            ("SAFIX_VAULT_ROOT", vault.to_str().unwrap()),
            ("VISUAL", &format!("/bin/sh {}", editor.display())),
        ],
    );
    let run = run.expect_success("editing a vault-rooted entry with an untouched buffer");
    run.says("unchanged");
}

/// `safix get` decrypts from the vault root.
#[test]
fn get_reads_the_vault_document_from_the_vault_root() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.encrypt_to_vault(
        ALICE_FILE,
        &[&fixture.alice.clone()],
        "api-token: get-canary\n",
    );

    let run = fixture.run_env(
        &["get", "alice", "api-token"],
        None,
        &[("SAFIX_VAULT_ROOT", vault.to_str().unwrap())],
    );
    let run = run.expect_success("reading a vault-rooted entry");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "get-canary",
        "get did not decrypt the vault-rooted document"
    );
}

/// `sync clan`'s safix-to-clan direction reads safix's own side from the
/// vault root before pushing it to clan.
#[test]
fn sync_clan_reads_the_vault_document_from_the_vault_root() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.seed_mapping(
        "ntfy-token",
        "safix-to-clan",
        ("meridian", "ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture.encrypt_to_vault(
        ALICE_FILE,
        &[&fixture.alice.clone()],
        "api-token: from-vault-to-clan\n",
    );

    let mut environment = fixture.clan_env();
    environment.push((
        "SAFIX_VAULT_ROOT".to_owned(),
        vault.to_str().unwrap().to_owned(),
    ));
    let borrowed: Vec<(&str, &str)> = environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();

    let run = fixture.run_env(&["sync", "clan"], None, &borrowed);
    run.expect_success("pushing a vault-rooted value to clan");
    assert_eq!(
        fixture.clan_holds("meridian", "ntfy/token"),
        Some("from-vault-to-clan".to_owned()),
        "clan did not receive the value sync read from the vault root"
    );
}

/// `sync keepassxc`'s safix-to-keepassxc mode reads safix's own side from the
/// vault root before pushing it to the modelled database.
#[test]
fn sync_keepassxc_reads_the_vault_document_from_the_vault_root() {
    let mut fixture = Fixture::new();
    let vault = fixture.declare_vault();
    fixture.seed_sync_mapping(
        "push",
        "safix-to-keepassxc",
        ("alice", "api-token"),
        "alice/pushed",
        Some("alice@example.com"),
    );
    fixture.encrypt_to_vault(
        ALICE_FILE,
        &[&fixture.alice.clone()],
        "api-token: from-vault-to-keepassxc\n",
    );

    let mut environment = fixture.store_env();
    environment.push((
        "SAFIX_CARD_STUB_DB_PASSWORD".to_owned(),
        "fixture-database-password".to_owned(),
    ));
    environment.push((
        "SAFIX_VAULT_ROOT".to_owned(),
        vault.to_str().unwrap().to_owned(),
    ));
    let borrowed: Vec<(&str, &str)> = environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();

    let run = fixture.run_sync(
        &["sync", "keepassxc"],
        "fixture-database-password\n",
        &borrowed,
    );
    run.expect_success("pushing a vault-rooted value to the modelled database");
    assert_eq!(
        fixture.store_holds("safix/alice/pushed"),
        Some("from-vault-to-keepassxc".to_owned()),
        "the database did not receive the value sync read from the vault root"
    );
}
