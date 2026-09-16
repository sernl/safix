//! A 1Password command whose behaviour is asserted rather than assumed.
//!
//! # Why this one is stubbed, and why it can never be anything else
//!
//! The reason `clan-stub.rs` gives applies here: the service is a boundary safix
//! delegates across, so what is under test is the delegation — that the value and
//! every field travel standard input as one item payload and nowhere else, that
//! no argument vector carries an assignment, that every item command names its
//! vault, that a refusal reaches the operator in the program's own words, and
//! that an edit preserves the members of an item that no declaration names.
//!
//! And one more reason `clan-stub.rs` does not have: no check of this repository
//! may drive a real `op`, permanently. `_1password-cli` at this flake's pin is
//! unfree, so naming it from a check, a package or a devshell fails evaluation
//! for every consumer who has not allowed unfree packages; there is no
//! self-hostable 1Password server to point a sandboxed node at; and every
//! authentication path needs the network, which no `nix build` and no hermetic VM
//! node has. Each ground alone is sufficient and none of them expires — so this
//! file is not a convenience standing in until a real check exists. It is the
//! only `op` this suite will ever have.
//!
//! What a stub cannot establish is that the arguments and the payload mean to
//! `op` what safix thinks they mean. `crates/safix-core/src/onepassword.rs`'s
//! header records which facts are documented rather than measured, and names
//! what a contributor holding a licensed program should re-measure.
//!
//! # The contract this stands in for
//!
//! Read out of 1Password's published command-line reference:
//!
//! - `op whoami --format json` answers for the session; it exits non-zero, saying
//!   so on standard error, when there is none.
//! - `op item get <item> --vault <vault> --format json --reveal` prints the item
//!   as `{ id, title, category, tags, urls: [{ label, primary, href }], fields:
//!   [{ id, type, purpose, label, value }] }`, with `--reveal` turning a
//!   concealed field's placeholder into its value.
//! - `op item create --vault <v> --category login --title <t> -` and
//!   `op item edit <t> --vault <v> -` take the whole item as one JSON object on
//!   standard input.
//! - `op item list --vault <vault> --format json` prints one object per item.
//! - `--account <shorthand>` selects the account and precedes the verb.
//! - an item that is not there reports `"<title>" isn't an item`; a vault this
//!   session cannot see reports `"<vault>" isn't a vault in this account`.
//! - a service account reaches only the vaults it was granted and cannot reach a
//!   built-in vault at all, which is why `--vault` is required rather than
//!   optional on every item command.
//!
//! # What makes it an instrument rather than a convenience
//!
//! It exits non-zero on any argument word containing `=`, naming the word. So
//! "no value and no field is ever an argument" is checked by the thing safix
//! actually runs rather than by review. And it requires `--vault` on every item
//! command, so a transport that omitted one fails here rather than at an
//! operator's terminal against a personal vault.
//!
//! # What it records
//!
//! One line per invocation in each of four spool files: `argv` (the whole
//! argument vector), `env` (every variable the child was given), `stdin` (the
//! payload, which is one line because JSON escapes a newline inside a string),
//! and `exit` (which path the invocation took). A test reads them to assert what
//! crossed which channel.

use std::io::Read as _;

/// Where the store, the spool and the failure switches live.
const SPOOL: &str = "SAFIX_OP_STUB_SPOOL";

/// The preflight refuses, the way a run with no session meets it.
const SIGNED_OUT: &str = "SAFIX_OP_STUB_SIGNED_OUT";

/// `item get` reports the item as not found, whatever the store holds.
const UNKNOWN_ITEM: &str = "SAFIX_OP_STUB_UNKNOWN_ITEM";

/// Every item command refuses, naming the vault.
const UNKNOWN_VAULT: &str = "SAFIX_OP_STUB_UNKNOWN_VAULT";

/// A comma-separated list of item titles whose write refuses.
///
/// Titles rather than a bare on switch, the way `clan-stub.rs`'s own `REFUSES`
/// names a var id: the claim that one mapping's failure does not end a run needs
/// a run in which exactly one of three writes refuses, and an unconditional
/// switch cannot express that.
const REFUSES: &str = "SAFIX_OP_STUB_REFUSES";

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    record("argv", &arguments.join(" "));
    record_environment();

    // Before the dispatch, so a vector carrying an assignment is refused
    // whatever it was going to ask for.
    for word in &arguments {
        if word.contains('=') {
            record("exit", "assignment-in-argv");
            refuse(&format!(
                "[ERROR] 2026/09/16 12:00:00 the argument '{word}' assigns a value. An \
                 assignment statement is recorded in shell history and can be visible to \
                 other processes; pass the item on standard input instead"
            ));
        }
    }

    // `--account <shorthand>` precedes the verb, and selects which account
    // answers rather than carrying anything that proves it.
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let rest: &[&str] = match words.as_slice() {
        ["--account", _account, rest @ ..] => rest,
        rest => rest,
    };

    match rest {
        ["whoami", ..] => whoami(),
        ["vault", "list", ..] => vault_list(rest),
        ["item", "get", title, ..] => item_get(rest, title),
        ["item", "create", ..] => item_create(rest),
        ["item", "edit", title, ..] => item_edit(rest, title),
        ["item", "list", ..] => item_list(rest),
        other => {
            record("exit", "unrecognised");
            refuse(&format!(
                "[ERROR] 2026/09/16 12:00:00 unknown command \"{}\" for \"op\"",
                other.join(" ")
            ))
        }
    }
}

/// The session preflight.
fn whoami() -> ! {
    if named(SIGNED_OUT).is_some() {
        record("exit", "signed-out");
        refuse(
            "[ERROR] 2026/09/16 12:00:00 you are not currently signed in. Please run \
             `op signin --help` for instructions",
        );
    }
    record("exit", "whoami");
    println!(
        "{{\"url\":\"https://fixture.example.com\",\"user_uuid\":\"fixture\",\
         \"account_uuid\":\"fixture\"}}"
    );
    std::process::exit(0);
}

/// Every vault the store holds, which is every vault something has been written
/// into.
fn vault_list(rest: &[&str]) -> ! {
    if let Some(vault) = named(UNKNOWN_VAULT) {
        record("exit", "unknown-vault");
        refuse(&not_a_vault(&vault));
    }
    let _ = rest;
    let mut vaults: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(spool().join("store")) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                vaults.push(decoded_name(name).unwrap_or_else(|| name.to_owned()));
            }
        }
    }
    vaults.sort();
    record("exit", "vault-list");
    println!(
        "[{}]",
        vaults
            .iter()
            .map(|vault| format!("{{\"id\":\"{vault}\",\"name\":\"{vault}\"}}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    std::process::exit(0);
}

/// One item, printed as the documented JSON.
fn item_get(rest: &[&str], title: &str) -> ! {
    let vault = vault_of(rest);
    if let Some(unknown) = named(UNKNOWN_VAULT) {
        record("exit", "unknown-vault");
        refuse(&not_a_vault(&unknown));
    }
    if named(UNKNOWN_ITEM).is_some() {
        record("exit", "unknown-item");
        refuse(&not_an_item(title));
    }
    let Some(held) = stored(&vault, title) else {
        record("exit", "absent");
        refuse(&not_an_item(title));
    };
    record("exit", "item-get");
    println!("{held}");
    std::process::exit(0);
}

/// One item, created from the payload on standard input.
fn item_create(rest: &[&str]) -> ! {
    let vault = vault_of(rest);
    let title = flag_of(rest, "--title").unwrap_or_else(|| {
        record("exit", "no-title");
        refuse("[ERROR] 2026/09/16 12:00:00 a created item needs --title")
    });
    write_item(&vault, &title, "item-create")
}

/// One item, edited from the payload on standard input.
fn item_edit(rest: &[&str], title: &str) -> ! {
    let vault = vault_of(rest);
    write_item(&vault, title, "item-edit")
}

/// The one write path, which is where the payload is read and stored.
fn write_item(vault: &str, title: &str, path: &str) -> ! {
    let mut payload = String::new();
    let _ = std::io::stdin().read_to_string(&mut payload);
    // Recorded whatever happens next, because the claim the recording exists for
    // — that the value crossed here and nowhere else — is about the runs that
    // refused too.
    record("stdin", payload.trim_end_matches('\n'));

    if let Some(unknown) = named(UNKNOWN_VAULT) {
        record("exit", "unknown-vault");
        refuse(&not_a_vault(&unknown));
    }
    if named(REFUSES)
        .unwrap_or_default()
        .split(',')
        .any(|named| !named.is_empty() && named == title)
    {
        record("exit", "refused");
        refuse(&format!(
            "[ERROR] 2026/09/16 12:00:00 could not save the item \"{title}\": the service \
             is unavailable"
        ));
    }
    if payload.trim().is_empty() {
        record("exit", "empty-payload");
        refuse("[ERROR] 2026/09/16 12:00:00 the item payload on standard input was empty");
    }
    if serde_json::from_str::<serde_json::Value>(&payload).is_err() {
        record("exit", "unparseable-payload");
        refuse(
            "[ERROR] 2026/09/16 12:00:00 the item payload on standard input is not a JSON \
             object",
        );
    }

    store(vault, title, payload.trim());
    record("exit", path);
    println!("{}", payload.trim());
    std::process::exit(0);
}

/// Every item one vault holds, as the documented listing prints them.
fn item_list(rest: &[&str]) -> ! {
    let vault = vault_of(rest);
    if let Some(unknown) = named(UNKNOWN_VAULT) {
        record("exit", "unknown-vault");
        refuse(&not_a_vault(&unknown));
    }
    let mut titles: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(vault_dir(&vault)) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str().and_then(|name| name.strip_suffix(".item")) else {
                continue;
            };
            if let Some(title) = decoded_name(name) {
                titles.push(title);
            }
        }
    }
    titles.sort();
    record("exit", "item-list");
    println!(
        "[{}]",
        titles
            .iter()
            .map(|title| format!(
                "{{\"id\":\"{title}\",\"title\":\"{title}\",\"category\":\"LOGIN\"}}"
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    std::process::exit(0);
}

/// The vault every item command has to name.
///
/// Service-account semantics, and the reason this is a refusal rather than a
/// default: a command that omitted the vault would resolve a title against
/// whatever vault the session happened to reach, which for an operator running
/// this by hand is their own personal one.
fn vault_of(rest: &[&str]) -> String {
    flag_of(rest, "--vault").unwrap_or_else(|| {
        record("exit", "no-vault");
        refuse(
            "[ERROR] 2026/09/16 12:00:00 this command needs --vault: a service account \
             cannot resolve an item without one",
        )
    })
}

/// One flag's value, where the vector carries it.
fn flag_of(rest: &[&str], flag: &str) -> Option<String> {
    rest.windows(2)
        .find(|pair| pair.first() == Some(&flag))
        .and_then(|pair| pair.get(1))
        .map(|value| (*value).to_owned())
}

/// The sentence a vault this session cannot see is reported with.
fn not_a_vault(vault: &str) -> String {
    format!(
        "[ERROR] 2026/09/16 12:00:00 \"{vault}\" isn't a vault in this account. Specify \
         the vault with its ID or name"
    )
}

/// The sentence an absent item is reported with.
fn not_an_item(title: &str) -> String {
    format!(
        "[ERROR] 2026/09/16 12:00:00 \"{title}\" isn't an item. Specify the item with its \
         UUID, name, or domain"
    )
}

/// Where one item is stored, in the stub's own layout.
///
/// Deliberately unlike anything the service has — it has no filesystem at all —
/// and hex-encoded both in the path and in the content. Nothing in the runtime
/// may read the tool's own storage instead of asking it, so a transport that
/// tried would find a directory of hex names holding hex bytes. The discipline
/// `clan-stub.rs` records, for the same reason: a stub that laid its store out
/// the way the real thing does would make a runtime that cheated pass.
fn vault_dir(vault: &str) -> std::path::PathBuf {
    spool().join("store").join(encoded_name(vault))
}

/// The file one item's JSON is held in.
fn item_path(vault: &str, title: &str) -> std::path::PathBuf {
    vault_dir(vault).join(format!("{}.item", encoded_name(title)))
}

/// What the store holds for one item, or nothing.
fn stored(vault: &str, title: &str) -> Option<String> {
    let held = std::fs::read_to_string(item_path(vault, title)).ok()?;
    decoded(&held)
}

/// Put one item's JSON in the store.
fn store(vault: &str, title: &str, payload: &str) {
    let path = item_path(vault, title);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, encoded(payload.as_bytes()));
}

/// A name as a path component, hex-encoded.
fn encoded_name(name: &str) -> String {
    encoded(name.as_bytes())
}

/// The name a hex path component stands for.
fn decoded_name(name: &str) -> Option<String> {
    String::from_utf8(decoded(name).map(String::into_bytes)?).ok()
}

/// The store holds an encoding of the item rather than the item.
///
/// Hex, which is not encryption and is not pretending to be. What it reproduces
/// is the one property of the service's own storage that matters here: a real
/// vault is remote and encrypted, so a mapped value's plaintext bytes never
/// appear in a write to a regular file on this machine. A stub that wrote them
/// would make the syscall reading fail on the stub's own behaviour rather than
/// on safix's.
fn encoded(value: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(value.len().saturating_mul(2));
    for byte in value {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// The bytes a hex string stands for, as text.
fn decoded(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !trimmed.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    let raw: Vec<char> = trimmed.chars().collect();
    for pair in raw.chunks(2) {
        let held: String = pair.iter().collect();
        bytes.push(u8::from_str_radix(&held, 16).ok()?);
    }
    String::from_utf8(bytes).ok()
}

fn spool() -> std::path::PathBuf {
    std::path::PathBuf::from(
        named(SPOOL).unwrap_or_else(|| refuse(&format!("{SPOOL} names no directory"))),
    )
}

fn named(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .filter(|value| !value.is_empty())
}

/// Every variable the child was given, one `<name>=<value>` line per
/// invocation's record.
///
/// Recorded so a test can assert the negative the design turns on: no value and
/// no field is here either, the environment carries only a location, a switch
/// and whatever session material the operator's own shell already held.
fn record_environment() {
    let mut held: Vec<String> = std::env::vars()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    held.sort();
    record("env", &held.join(" "));
}

fn record(name: &str, line: &str) {
    let spool = spool();
    let _ = std::fs::create_dir_all(&spool);
    let path = spool.join(name);
    let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
    existing.push_str(line);
    existing.push('\n');
    let _ = std::fs::write(path, existing);
}

fn refuse(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
