//! `safix sync bitwarden` and `safix audit bitwarden`, driven against a
//! stand-in client and the real everything else.
//!
//! sops, age and git are real here as everywhere in this suite; the vault's
//! client is the stand-in `tests/support/bw-stub.rs` models. That stub is the
//! only `bw` any check of this repository runs, and the absence is recorded
//! rather than incidental: `bw` cannot authenticate without a network and a
//! `nix build` has none, so `modules/flake/checks/bitwarden.nix`'s header states
//! the absence where the checks live and names the measurement that would
//! unblock a VM check against `services.vaultwarden`.
//!
//! `harness::refuse_a_real_vault` is the structural guard, and it is why every
//! run below goes through [`Fixture::bitwarden_env`]: a run whose override does
//! not name the stand-in, whose client data directory is outside the fixture's
//! own scratch, or whose declaration names a server that could be somebody's
//! vault, fails before a process is spawned. A machine that develops this suite
//! plausibly holds a `bw` an operator left unlocked, and there is no undo for a
//! value written into a person's own vault.
//!
//! # Which check runs which claim
//!
//! Each test names the `modules/flake/checks/cli.nix` attribute that runs it in
//! its own doc comment. The mapping, for a reader who wants it in one place:
//!
//! | check | test |
//! | --- | --- |
//! | `safix-bitwarden-locked` | [`a_locked_client_with_no_terminal_refuses_before_any_read`] |
//! | `safix-bitwarden-unauthenticated` | [`an_unauthenticated_client_names_logging_in`] |
//! | `safix-bitwarden-server` | [`a_declared_server_that_is_not_reached_refuses_before_any_read`] |
//! | `safix-bitwarden-stale` | [`a_failed_refresh_refuses_every_mapping`] |
//! | `safix-bitwarden-ambiguous` | [`an_ambiguous_address_is_refused_rather_than_resolved`] |
//! | `safix-bitwarden-absent` | [`an_absent_item_is_created_by_a_pushing_mode_and_refused_by_a_pulling_one`] |
//! | `safix-bitwarden-argv` | [`no_payload_is_ever_a_positional_argument`] |
//! | `safix-bitwarden-session` | [`the_session_key_is_in_the_environment_and_nowhere_else`] |
//! | `safix-bitwarden-edit` | [`an_edit_preserves_every_field_the_declaration_does_not_govern`] |
//! | `safix-bitwarden-two-way` | [`two_way_records_the_agreement_in_a_hidden_field_of_the_item`] |
//! | `safix-bitwarden-conflict` | [`two_way_both_changed_is_a_conflict_and_writes_nothing`] |
//! | `safix-bitwarden-multiline` | [`a_multi_line_value_crosses_whole`] |
//! | `safix-bitwarden-tags` | [`tags_are_refused_at_evaluation`] |
//! | `safix-bitwarden-audit` | [`audit_bitwarden_writes_nothing`] |
//! | `safix-bitwarden-lingering` | [`a_lingering_item_is_information_and_does_not_move_the_exit_status`] |
//! | `safix-bitwarden-fields` | [`the_report_names_a_diverged_field_and_never_its_content`] |
//!
//! The seven tests no per-mode entry names — the narrowing pair, the refresh
//! ordering, the master-password channel, the field round trip's own twin and
//! both lock claims — are run by `safix-bitwarden-suite` in
//! `modules/flake/checks/single-runtime.nix`, which runs this target whole.
//!
//! # What each fixture is
//!
//! One mapping per mode and one state per mapping, the shape `sync_path.rs`
//! establishes; plus the four this target's own decisions need: an item carrying
//! members no declaration names, an address two items answer to, a value
//! carrying newlines, and a client that is locked rather than unlocked.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use harness::{ALICE_FILE, BW_FIXTURE_PASSWORD, Fixture, Run};
use serde_json::{Value, json};

/// The master password the locked-client runs type, as a person types it.
const UNLOCK: &str = "fixture-vault-master-password\n";

/// The folder the fixtures that name one use.
///
/// Synthetic, like every other name here: no folder of anybody's appears in this
/// repository.
const FOLDER: &str = "fleet";

/// A fixture with one mapping of each mode, every item in the vault's root.
///
/// The root rather than a folder for all four: a folder exists in the stand-in's
/// store only where an item is in it, so a mapping that pushes into an absent
/// item in an absent folder would be asserting something about the stand-in's
/// bootstrap rather than about safix. The folder-bearing address has its own
/// fixtures, [`lingering_fixture`] and
/// [`a_declared_server_that_is_not_reached_refuses_before_any_read`].
fn declared() -> Fixture {
    let mut fixture = Fixture::new();
    for name in ["push-me", "pull-me", "both-ways", "back-me-up"] {
        fixture.seed_output(name, ALICE_FILE);
    }

    fixture.seed_bitwarden_mapping(
        "push",
        "safix-to-bitwarden",
        ("alice", "push-me"),
        None,
        "pushed",
        json!({
            "username": "alice@example.com",
            "url": "https://grafana.example.invalid",
            "notes": "minted by safix",
        }),
    );
    fixture.seed_bitwarden_mapping(
        "pull",
        "bitwarden-to-safix",
        ("alice", "pull-me"),
        None,
        "pulled",
        json!({}),
    );
    fixture.seed_bitwarden_mapping(
        "both",
        "two-way",
        ("alice", "both-ways"),
        None,
        "both",
        json!({}),
    );
    fixture.seed_bitwarden_mapping(
        "copy",
        "backup",
        ("alice", "back-me-up"),
        None,
        "copied",
        json!({}),
    );
    fixture
}

/// One login item as the client prints one, holding one value.
///
/// No `id`, `name`, `folder` or `folderId`: those are
/// [`Fixture::bitwarden_seed`]'s to write, for the reason its own documentation
/// gives.
fn item(value: &str) -> Value {
    json!({
        "object": "item",
        "type": 1,
        "login": { "password": value, "username": null, "uris": [] },
        "notes": null,
        "fields": [],
    })
}

fn borrowed(extra: &[(String, String)]) -> Vec<(&str, &str)> {
    extra
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// One record's environment, without the stand-in's own switches.
///
/// A switch is the instrument's configuration rather than a channel safix used,
/// and `SAFIX_BW_STUB_FORBIDDEN` carries the forbidden words themselves — so an
/// assertion that scanned it would fail over the list it handed the stub. The
/// stand-in's own `assert_channels` draws the line in the same place.
///
/// A record's environment is one line of `NAME=value` entries separated by
/// spaces, and a value may itself carry a space, so an entry is recognised by
/// its own `NAME=` head and every token after it belongs to it until the next
/// head. Dropping tokens by shape rather than by position is what makes the
/// multi-word `SAFIX_BW_STUB_FORBIDDEN` drop whole instead of leaving its tail
/// behind.
fn environment_without_switches(record: &str) -> String {
    let line = record
        .lines()
        .find_map(|line| line.strip_prefix("env: "))
        .unwrap_or_default();
    let mut kept: Vec<&str> = Vec::new();
    let mut dropping = false;
    for token in line.split(' ') {
        let heads_an_entry = token.split_once('=').is_some_and(|(name, _)| {
            !name.is_empty()
                && name.chars().all(|letter| {
                    letter.is_ascii_uppercase() || letter.is_ascii_digit() || letter == '_'
                })
        });
        if heads_an_entry {
            dropping = token.starts_with("SAFIX_BW_STUB_");
        }
        if !dropping {
            kept.push(token);
        }
    }
    kept.join(" ")
}

/// Every invocation's subcommand, in invocation order.
///
/// The first word of the argument vector that is neither an option nor the
/// option's own value, which is how a reader of the spool tells a `sync` from a
/// `get`.
fn subcommands(fixture: &Fixture) -> Vec<String> {
    fixture
        .bitwarden_invocations()
        .iter()
        .map(|argv| {
            argv.split_whitespace()
                .find(|word| !word.starts_with("--"))
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

/// How many invocations named one subcommand.
fn issued(fixture: &Fixture, subcommand: &str) -> usize {
    subcommands(fixture)
        .iter()
        .filter(|word| *word == subcommand)
        .count()
}

/// One stored item, parsed.
fn stored(fixture: &Fixture, item: &str) -> Value {
    let held = fixture
        .bitwarden_holds(item)
        .unwrap_or_else(|| panic!("{item} is not in the stand-in's store"));
    serde_json::from_str(&held).unwrap_or_else(|_| panic!("{item} is not JSON: {held}"))
}

/// The value one stored item holds, as the client's own schema carries it.
fn password(fixture: &Fixture, item: &str) -> String {
    stored(fixture, item)["login"]["password"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

/// One named custom field of a stored item, whole.
fn custom_field(fixture: &Fixture, item: &str, name: &str) -> Option<Value> {
    stored(fixture, item)["fields"]
        .as_array()?
        .iter()
        .find(|field| field["name"] == json!(name))
        .cloned()
}

/// How many items the stand-in's store holds.
fn item_count(fixture: &Fixture) -> usize {
    std::fs::read_dir(fixture.bitwarden_spool().join("items"))
        .into_iter()
        .flatten()
        .flatten()
        .count()
}

/// Replace one stored item's value without going through safix, as a person
/// editing it on their phone would.
///
/// Through [`Fixture::bitwarden_seed`] rather than by writing the file, so the
/// member order the stand-in reads its store in is the one that function
/// establishes rather than whatever `serde_json` happened to emit.
fn edit_outside_safix(fixture: &Fixture, folder: Option<&str>, item: &str, value: &str) {
    let mut held = stored(fixture, item);
    held["login"]["password"] = json!(value);
    let object = held.as_object_mut().unwrap();
    for spliced in ["id", "name", "folder", "folderId"] {
        object.remove(spliced);
    }
    fixture.bitwarden_seed(folder, item, &held);
}

/// One run of `sync bitwarden`, with the stand-in in place and pipes on every
/// stream.
fn converge(fixture: &Fixture, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
    let mut environment = fixture.bitwarden_env();
    environment.extend(
        extra
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned())),
    );
    fixture.run_env(arguments, None, &borrowed(&environment))
}

// ── narrowing ───────────────────────────────────────────────────────────────

/// `sync bitwarden` converges this target's mappings and reports nothing about
/// the others.
///
/// Run by `safix-bitwarden-suite`. Dropping `parse_dispatch`'s `bitwarden`
/// keyword arm (task 6.8) turns this red with `MappingNameNeedsTarget`, because
/// the word would be read as a mapping name.
#[test]
fn sync_bitwarden_narrows_to_this_target() {
    let mut fixture = declared();
    fixture.seed_output("kdbx-side", ALICE_FILE);
    fixture.seed_sync_mapping(
        "stored",
        "safix-to-keepassxc",
        ("alice", "kdbx-side"),
        "alice/stored",
        json!({}),
    );
    fixture.seed_mapping(
        "bridged",
        "safix-to-clan",
        ("meridian", "ntfy", "token"),
        ("alice", "api-token"),
    );

    fixture
        .set("alice", "push-me", "safix-push-me")
        .expect_success("seeding the safix side of the pushing mapping");
    fixture
        .set("alice", "both-ways", "safix-both-ways")
        .expect_success("seeding the safix side of the two-way mapping");
    fixture.bitwarden_seed(None, "pulled", &item("vault-pulled"));

    let run = converge(&fixture, &["sync", "bitwarden"], &[])
        .expect_refusal("a pulling mapping whose item is absent refuses");

    // Every bitwarden mapping has a line.
    for mapping in ["push", "pull", "both", "copy"] {
        run.says(mapping);
    }
    // And nothing about the other two targets: no line names them, and no
    // database and no clan was reached.
    assert!(
        !run.combined().contains("alice/stored"),
        "a narrowed run reported the keepassxc target\n{}",
        run.combined()
    );
    assert!(
        !run.combined().contains("bridged"),
        "a narrowed run reported the clan target\n{}",
        run.combined()
    );
    assert_eq!(
        fixture.clan_writes(),
        0,
        "a run narrowed to bitwarden wrote into clan"
    );
}

/// A bare `sync` converges every target's mappings in one run.
///
/// Run by `safix-bitwarden-suite`. Dropping `sync_command`'s third `matches!`
/// block (task 6.9) leaves every unit test green and turns this red: the
/// narrowed run above would still work while a bare one silently stopped
/// converging this target.
#[test]
fn bare_sync_converges_bitwarden_alongside_the_others() {
    let mut fixture = Fixture::new();
    fixture.seed_output("vault-side", ALICE_FILE);
    fixture.seed_output("kdbx-side", ALICE_FILE);
    fixture.seed_bitwarden_mapping(
        "vault",
        "safix-to-bitwarden",
        ("alice", "vault-side"),
        None,
        "pushed",
        json!({}),
    );
    fixture.seed_sync_mapping(
        "stored",
        "safix-to-keepassxc",
        ("alice", "kdbx-side"),
        "alice/stored",
        json!({}),
    );
    fixture.seed_mapping(
        "bridged",
        "safix-to-clan",
        ("meridian", "ntfy", "token"),
        ("alice", "api-token"),
    );
    fixture
        .set("alice", "vault-side", "safix-vault-side")
        .expect_success("seeding the vault mapping's safix side");
    fixture
        .set("alice", "kdbx-side", "safix-kdbx-side")
        .expect_success("seeding the database mapping's safix side");
    fixture
        .set("alice", "api-token", "safix-api-token")
        .expect_success("seeding the clan mapping's safix side");

    let mut environment = fixture.bitwarden_env();
    environment.extend(fixture.store_env());
    environment.extend(fixture.clan_env());
    environment.push((
        "SAFIX_CARD_STUB_DB_PASSWORD".to_owned(),
        "fixture-database-password".to_owned(),
    ));
    let run = fixture
        .run_sync(
            &["sync"],
            "fixture-database-password\n",
            &borrowed(&environment),
        )
        .expect_success("one mapping per target, each with something to do");

    run.says("vault");
    run.says("stored");
    run.says("bridged");
    assert_eq!(
        password(&fixture, "pushed"),
        "safix-vault-side",
        "a bare run did not converge the bitwarden mapping"
    );
    assert!(
        issued(&fixture, "create") == 1,
        "a bare run issued {} creates against the vault, not one",
        issued(&fixture, "create")
    );
}

// ── the preflight, and what it refuses before any read ──────────────────────

/// A locked client with no terminal refuses before either side of any mapping
/// is read.
///
/// Run by `safix-bitwarden-locked`. The refusal is `safix::bitwarden_locked`
/// with `state = "locked"`; a plain report prints its prose rather than its
/// code, which is what the assertion reads. The spool assertion is the load
/// -bearing half: a refusal raised after the first read would still print this
/// sentence.
#[test]
fn a_locked_client_with_no_terminal_refuses_before_any_read() {
    let fixture = declared();
    let run = converge(
        &fixture,
        &["sync", "bitwarden"],
        &[("SAFIX_BW_STUB_STATE", "locked")],
    )
    .expect_refusal("a locked client with no terminal");

    run.says("the vault's client is locked and there is no terminal to ask its master");
    run.says("Nothing was read.");
    assert_eq!(
        subcommands(&fixture),
        vec![String::from("status")],
        "a locked client was asked something other than its own state"
    );
}

/// An unauthenticated client names logging in as the operator's own act, and
/// the run never tries to unlock it.
///
/// Run by `safix-bitwarden-unauthenticated`. The refusal is
/// `safix::bitwarden_locked` with `state = "unauthenticated"`. Making
/// `prose::bitwarden_locked` ignore its `state` and print one sentence for both
/// (task 3.8) turns this red on the "not logged in" assertion while the locked
/// test above stays green.
#[test]
fn an_unauthenticated_client_names_logging_in() {
    let fixture = declared();
    let run = converge(
        &fixture,
        &["sync", "bitwarden"],
        &[("SAFIX_BW_STUB_STATE", "unauthenticated")],
    )
    .expect_refusal("a client that is not logged in");

    run.says("the vault's client is not logged in");
    run.says("safix unlocks a vault; it never logs one in");
    assert_eq!(
        issued(&fixture, "unlock"),
        0,
        "a run tried to unlock a client that is not logged in"
    );
    assert_eq!(
        subcommands(&fixture),
        vec![String::from("status")],
        "an unauthenticated client was asked something other than its own state"
    );
}

/// A declared server that is not the one the client reports refuses before any
/// side is read, naming both.
///
/// Run by `safix-bitwarden-server`. The refusal is
/// `safix::bitwarden_server_mismatch`. Both URLs are asserted because a refusal
/// naming one of them leaves the operator to guess which end is wrong.
#[test]
fn a_declared_server_that_is_not_reached_refuses_before_any_read() {
    let mut fixture = declared();
    fixture.bitwarden_server_is("https://vault.example.org");

    let run = converge(
        &fixture,
        &["sync", "bitwarden"],
        &[("SAFIX_BW_STUB_SERVER", "http://127.0.0.1:8222")],
    )
    .expect_refusal("a declared server the client does not reach");

    run.says("https://vault.example.org");
    run.says("http://127.0.0.1:8222");
    run.says("Nothing was read and nothing was written.");
    assert_eq!(
        subcommands(&fixture),
        vec![String::from("status")],
        "a run refusing over the server reached past the client's own state"
    );
}

/// A failed refresh refuses every mapping, and nothing is read or written.
///
/// Run by `safix-bitwarden-stale`. The refusal is `safix::bitwarden_stale`,
/// carrying the client's own words. Making the pre-read `sync` non-fatal (task
/// 8.24) turns this red while
/// [`the_refresh_happens_once_before_the_first_read`] stays green, which is what
/// separates "the refresh happens" from "a failed refresh refuses".
#[test]
fn a_failed_refresh_refuses_every_mapping() {
    let fixture = declared();
    let run = converge(
        &fixture,
        &["sync", "bitwarden"],
        &[("SAFIX_BW_STUB_SYNC_FAILS", "1")],
    )
    .expect_refusal("a client that could not refresh its local copy");

    run.says("could not refresh its local copy");
    run.says("every bitwarden");
    run.says("connect ECONNREFUSED 127.0.0.1:8222");
    assert_eq!(
        subcommands(&fixture),
        vec![String::from("status"), String::from("sync")],
        "a run whose refresh failed reached past it"
    );
    assert_eq!(issued(&fixture, "get"), 0, "a stale run read an item");
    assert_eq!(issued(&fixture, "edit"), 0, "a stale run wrote an item");
    assert_eq!(issued(&fixture, "create"), 0, "a stale run created an item");
}

/// The refresh happens once for the whole run, before the first read.
///
/// Run by `safix-bitwarden-suite`. Syncing per mapping would make two reads of
/// one run see two vaults, so the count is the claim and the order is its other
/// half.
#[test]
fn the_refresh_happens_once_before_the_first_read() {
    let fixture = declared();
    for (name, item) in [
        ("push-me", "pushed"),
        ("pull-me", "pulled"),
        ("both-ways", "both"),
        ("back-me-up", "copied"),
    ] {
        fixture.bitwarden_seed(None, item, &self_named(name));
    }
    for name in ["push-me", "both-ways", "back-me-up"] {
        fixture
            .set("alice", name, &format!("safix-{name}"))
            .expect_success("seeding the safix side");
    }

    converge(&fixture, &["sync", "bitwarden"], &[])
        .expect_refusal("backup refuses an item that already holds another value");

    let order = subcommands(&fixture);
    assert_eq!(
        order.iter().filter(|word| *word == "sync").count(),
        1,
        "the refresh did not happen exactly once: {order:?}"
    );
    let refreshed = order.iter().position(|word| word == "sync").unwrap();
    let first_read = order
        .iter()
        .position(|word| word == "get")
        .expect("no item was read at all, so the ordering claim is vacuous");
    assert!(
        refreshed < first_read,
        "the refresh did not precede the first read: {order:?}"
    );
}

/// One item holding a value named after the entry it is mapped to.
fn self_named(name: &str) -> Value {
    item(&format!("vault-{name}"))
}

// ── addressing ──────────────────────────────────────────────────────────────

/// An address two items answer to is refused rather than resolved, and nothing
/// is written for it.
///
/// Run by `safix-bitwarden-ambiguous`. The refusal is
/// `safix::bitwarden_item_ambiguous`, and it is one mapping's rather than the
/// run's, so a plain report prints its prose under the mapping's own line.
#[test]
fn an_ambiguous_address_is_refused_rather_than_resolved() {
    let fixture = declared();
    fixture.bitwarden_seed(None, "pushed", &item("vault-pushed"));
    fixture
        .set("alice", "push-me", "safix-push-me")
        .expect_success("seeding the safix side");

    let run = converge(
        &fixture,
        &["sync", "bitwarden", "push"],
        &[("SAFIX_BW_STUB_AMBIGUOUS", "1")],
    )
    .expect_refusal("an address two items answer to");

    run.says("2 items in the vault");
    run.says("Nothing picks between them");
    assert_eq!(
        issued(&fixture, "edit"),
        0,
        "an ambiguous address was written"
    );
    assert_eq!(
        issued(&fixture, "create"),
        0,
        "an ambiguous address was created beside the two it matched"
    );
}

/// An absent item is created by a pushing mode and refused by a pulling one.
///
/// Run by `safix-bitwarden-absent`. The refusal is
/// `safix::bitwarden_item_absent`, naming the mode that makes the vault the
/// source.
///
/// Two addresses rather than the one task 8.10 names: two mappings over a single
/// address is a declaration `bitwardenLib.violationsOf`'s `twoMappingsOneItem`
/// refuses, so a fixture declaring it would be asserting runtime behaviour over
/// a tree that does not evaluate — and the pushing mapping's own create would
/// decide the pulling one's outcome by running first.
#[test]
fn an_absent_item_is_created_by_a_pushing_mode_and_refused_by_a_pulling_one() {
    let fixture = declared();
    fixture
        .set("alice", "push-me", "safix-push-me")
        .expect_success("seeding the safix side of the pushing mapping");

    let run = converge(&fixture, &["sync", "bitwarden", "push", "pull"], &[])
        .expect_refusal("a pulling mapping whose item the vault does not hold");

    run.says("push");
    run.says("updated");
    assert_eq!(
        password(&fixture, "pushed"),
        "safix-push-me",
        "a pushing mode did not create the absent item"
    );

    run.says("the mapping 'pull' is bitwarden-to-safix");
    run.says("the vault holds no item at 'pulled'");
    assert!(
        fixture.bitwarden_holds("pulled").is_none(),
        "a pulling mapping authored the item it was meant to refuse over"
    );
}

// ── the three channels ──────────────────────────────────────────────────────

/// No payload is ever a positional argument, on a create or on an edit.
///
/// Run by `safix-bitwarden-argv`. The stand-in refuses a positional that decodes
/// to JSON itself (task 7.11), so this holds the claim twice: once by the
/// instrument, and once here over every record of both runs.
#[test]
fn no_payload_is_ever_a_positional_argument() {
    let fixture = declared();
    fixture
        .set("alice", "push-me", "safix-first")
        .expect_success("seeding the safix side");

    converge(&fixture, &["sync", "bitwarden", "push"], &[])
        .expect_success("a create into an absent item");
    fixture
        .set("alice", "push-me", "safix-second")
        .expect_success("changing the safix side, so the second run edits");
    converge(&fixture, &["sync", "bitwarden", "push"], &[])
        .expect_success("an edit over the item the first run created");

    assert_eq!(issued(&fixture, "create"), 1, "the create did not happen");
    assert_eq!(issued(&fixture, "edit"), 1, "the edit did not happen");

    for record in fixture.bitwarden_records() {
        let argv = record
            .lines()
            .find_map(|line| line.strip_prefix("argv: "))
            .unwrap_or_default();
        let subcommand = argv
            .split_whitespace()
            .find(|word| !word.starts_with("--"))
            .unwrap_or_default();
        let stdin = record
            .lines()
            .find_map(|line| line.strip_prefix("stdin: "))
            .unwrap_or_default();
        if matches!(subcommand, "create" | "edit") {
            assert!(
                !stdin.trim().is_empty(),
                "a write carried no payload on standard input: {record}"
            );
        }
        for word in argv.split_whitespace() {
            assert!(
                !looks_like_base64_json(word),
                "an argument carried an encoded payload: {word}"
            );
        }
    }
}

/// Whether one word is base64 of something that starts a JSON object, which is
/// what a payload handed as the documented `<encodedJson>` positional looks
/// like.
fn looks_like_base64_json(word: &str) -> bool {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    if word.len() < 16 || !word.len().is_multiple_of(4) || word.starts_with("--") {
        return false;
    }
    if !word
        .bytes()
        .all(|byte| byte == b'=' || ALPHABET.contains(&byte))
    {
        return false;
    }
    // The first three characters of base64 carry the first two bytes, and `ey`
    // is `{"`; a payload of this transport is always an object with a member.
    word.starts_with("ey")
}

/// The session key travels one environment variable and nothing else.
///
/// Run by `safix-bitwarden-session`. This is the narrowing B3 records, asserted
/// rather than promised: `BW_SESSION` in the environment of every invocation
/// after the unlock, in no argument vector, on no output stream, and in no file
/// safix writes. Moving the key into `--session <key>` (task 4.13) turns this red
/// and is additionally refused by the stand-in itself.
#[test]
fn the_session_key_is_in_the_environment_and_nowhere_else() {
    const KEY: &str = "fixture-session-key";
    let fixture = declared();
    fixture.bitwarden_seed(None, "pulled", &item("vault-pulled"));

    let mut environment = fixture.bitwarden_env();
    environment.push(("SAFIX_BW_STUB_STATE".to_owned(), "locked".to_owned()));
    let run = fixture
        .run_sync(
            &["sync", "bitwarden", "pull"],
            UNLOCK,
            &borrowed(&environment),
        )
        .expect_success("a locked client unlocked on a terminal");
    run.says("pull");

    let records = fixture.bitwarden_records();
    let after_unlock: Vec<&String> = records
        .iter()
        .skip_while(|record| !record.contains("unlock"))
        .skip(1)
        .collect();
    assert!(
        !after_unlock.is_empty(),
        "no invocation followed the unlock, so the claim is vacuous"
    );
    for record in after_unlock {
        let argv = record
            .lines()
            .find_map(|line| line.strip_prefix("argv: "))
            .unwrap_or_default();
        let environ = record
            .lines()
            .find_map(|line| line.strip_prefix("env: "))
            .unwrap_or_default();
        assert!(
            environ.contains(&format!("BW_SESSION={KEY}")),
            "an invocation after the unlock carried no session key: {record}"
        );
        assert!(
            !argv.contains(KEY),
            "the session key reached an argument vector: {argv}"
        );
    }

    assert!(
        !String::from_utf8_lossy(&run.stdout).contains(KEY) && !run.stderr.contains(KEY),
        "the session key reached an output stream"
    );

    // Every file under the fixture's own scratch, except the stand-in's spool.
    // The spool is the instrument's record of what it was handed — the very
    // thing the assertions above read — so excluding it is not a hole: a file
    // safix wrote is what this half is about.
    let spool = fixture.bitwarden_spool();
    for path in files_under(&fixture.work) {
        if path.starts_with(&spool) {
            continue;
        }
        let held = std::fs::read(&path).unwrap_or_default();
        assert!(
            !String::from_utf8_lossy(&held).contains(KEY),
            "the session key reached {}",
            path.display()
        );
    }
}

/// Every file under one directory, recursively.
fn files_under(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = entry.path();
            if path.is_symlink() {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}

/// Neither the master password nor any mapped value travels an argument vector
/// or an environment.
///
/// Run by `safix-bitwarden-suite`. The stand-in fails any invocation carrying
/// one of the words named in `SAFIX_BW_STUB_FORBIDDEN`, so the claim is held by
/// the instrument as well as asserted here over every record — which is what
/// stops a run that stopped sending values from passing by not being looked at.
#[test]
fn no_master_password_or_value_travels_argv_or_env() {
    let fixture = declared();
    fixture.bitwarden_seed(None, "pulled", &item("vault-pulled"));
    fixture
        .set("alice", "push-me", "safix-push-me")
        .expect_success("seeding the safix side");

    let mut environment =
        fixture.bitwarden_env_forbidding(&[BW_FIXTURE_PASSWORD, "safix-push-me", "vault-pulled"]);
    environment.push(("SAFIX_BW_STUB_STATE".to_owned(), "locked".to_owned()));
    let run = fixture
        .run_sync(
            &["sync", "bitwarden", "push", "pull"],
            UNLOCK,
            &borrowed(&environment),
        )
        .expect_success("an unlock, a create and a pull");
    run.says("push");
    run.says("pull");

    let mut writes = 0usize;
    for record in fixture.bitwarden_records() {
        let argv = record
            .lines()
            .find_map(|line| line.strip_prefix("argv: "))
            .unwrap_or_default();
        let environ = environment_without_switches(&record);
        for forbidden in [BW_FIXTURE_PASSWORD, "safix-push-me", "vault-pulled"] {
            assert!(
                !argv.contains(forbidden),
                "'{forbidden}' reached an argument vector: {argv}"
            );
            assert!(
                !environ.contains(forbidden),
                "'{forbidden}' reached an environment: {record}"
            );
        }
        if argv.contains("create") || argv.contains("edit") {
            writes = writes.saturating_add(1);
        }
    }
    assert!(
        writes > 0,
        "no write happened at all, so the channel claim is vacuous"
    );
    assert_eq!(
        issued(&fixture, "unlock"),
        1,
        "the unlock did not happen exactly once"
    );
}

// ── what a write carries, and what it leaves alone ──────────────────────────

/// An edit preserves every member of the item the declaration does not govern.
///
/// Run by `safix-bitwarden-edit`. `bw edit item` replaces the whole item, so
/// this is what makes every write a read-modify-write. Constructing a fresh
/// payload instead (task 4.12) turns this red on all three preserved members.
#[test]
fn an_edit_preserves_every_field_the_declaration_does_not_govern() {
    let fixture = declared();
    fixture.bitwarden_seed(
        None,
        "pushed",
        &json!({
            "object": "item",
            "type": 1,
            "login": {
                "password": "vault-pushed",
                "totp": "otpauth://totp/fixture?secret=AAAA",
                "uris": [
                    { "match": null, "uri": "https://first.example.invalid" },
                    { "match": null, "uri": "https://second.example.invalid" },
                ],
            },
            "notes": null,
            "fields": [{ "name": "operator-note", "value": "the person's own", "type": 0 }],
        }),
    );
    fixture
        .set("alice", "push-me", "safix-push-me")
        .expect_success("seeding the safix side");

    converge(&fixture, &["sync", "bitwarden", "push"], &[])
        .expect_success("a value repair over an item carrying members safix does not govern");

    let held = stored(&fixture, "pushed");
    assert_eq!(
        held["login"]["password"],
        json!("safix-push-me"),
        "the value did not converge"
    );
    assert_eq!(
        held["login"]["totp"],
        json!("otpauth://totp/fixture?secret=AAAA"),
        "the item's own totp was deleted by a value repair"
    );
    assert_eq!(
        held["login"]["uris"][1]["uri"],
        json!("https://second.example.invalid"),
        "a second URI the declaration does not name was deleted"
    );
    assert_eq!(
        custom_field(&fixture, "pushed", "operator-note").unwrap()["value"],
        json!("the person's own"),
        "somebody's own custom field was deleted"
    );

    // And the declared fields did reach their documented homes, so the
    // preservation above is not the whole item being left alone.
    assert_eq!(
        held["login"]["username"],
        json!("alice@example.com"),
        "the declared username did not reach login.username"
    );
    assert_eq!(
        held["login"]["uris"][0]["uri"],
        json!("https://grafana.example.invalid"),
        "the declared url did not reach the first entry of login.uris"
    );
    assert_eq!(
        held["notes"],
        json!("minted by safix"),
        "the declared note did not reach the item's own notes"
    );
}

/// A two-way mapping records its agreement in a hidden custom field of the item
/// itself, in the same write as the value.
///
/// Run by `safix-bitwarden-two-way`. The count assertions are task 8.25's:
/// writing the memory in a second `edit` after the value's own would leave this
/// green on the field and add a second write to the spool, so the single-write
/// property is held rather than described.
#[test]
fn two_way_records_the_agreement_in_a_hidden_field_of_the_item() {
    let fixture = declared();
    fixture
        .set("alice", "both-ways", "safix-both-ways")
        .expect_success("seeding the safix side");

    converge(&fixture, &["sync", "bitwarden", "both"], &[])
        .expect_success("a two-way mapping bootstrapping into an absent item");

    let memory = custom_field(&fixture, "both", "safix-sync-state")
        .expect("the item carries no memory at all");
    assert_eq!(memory["type"], json!(1), "the memory is not a hidden field");
    assert!(
        memory["value"]
            .as_str()
            .is_some_and(|line| line.starts_with("safix-sync-v1 ")),
        "the memory does not carry the shared format tag: {memory}"
    );
    assert_eq!(
        item_count(&fixture),
        1,
        "a companion object was created beside the item"
    );
    assert_eq!(
        issued(&fixture, "create") + issued(&fixture, "edit"),
        1,
        "the value and the agreement did not land in one write"
    );

    // The agreement is not in the repository: no file safix committed carries
    // the value, its digest, or the memory's own line.
    let status = fixture.git(&["status", "--porcelain"]);
    assert!(
        status.trim().is_empty(),
        "a two-way run left the fixture repository dirty:\n{status}"
    );
    for path in files_under(&fixture.repo) {
        let held = String::from_utf8_lossy(&std::fs::read(&path).unwrap_or_default()).into_owned();
        assert!(
            !held.contains("safix-sync-v1 "),
            "the agreement reached {}",
            path.display()
        );
    }

    // A second run, over a changed safix side: the vault has not moved since
    // the agreement, so safix wins and the write is one edit.
    fixture
        .set("alice", "both-ways", "safix-both-ways-again")
        .expect_success("changing the safix side");
    converge(&fixture, &["sync", "bitwarden", "both"], &[])
        .expect_success("a two-way mapping whose safix side moved");
    assert_eq!(
        password(&fixture, "both"),
        "safix-both-ways-again",
        "the vault did not follow the side that changed"
    );
    assert_eq!(
        issued(&fixture, "edit"),
        1,
        "the second run's value and agreement did not land in one write"
    );
    assert_eq!(
        item_count(&fixture),
        1,
        "the second run created a second item"
    );
}

/// A two-way mapping whose two sides both changed is a conflict, and it writes
/// nothing.
///
/// Run by `safix-bitwarden-conflict`. The agreement is recorded by a first run
/// rather than seeded, because the memory carries a fingerprint of the value and
/// a hand-written one would be a fixture asserting its own arithmetic.
#[test]
fn two_way_both_changed_is_a_conflict_and_writes_nothing() {
    let fixture = declared();
    fixture
        .set("alice", "both-ways", "safix-agreed")
        .expect_success("seeding the safix side");
    converge(&fixture, &["sync", "bitwarden", "both"], &[])
        .expect_success("recording the agreement the conflict is judged against");
    let writes_before = issued(&fixture, "create") + issued(&fixture, "edit");

    // Both sides move away from the agreement.
    fixture
        .set("alice", "both-ways", "safix-changed")
        .expect_success("changing safix's side");
    edit_outside_safix(&fixture, None, "both", "vault-changed");

    let run = converge(&fixture, &["sync", "bitwarden", "both"], &[])
        .expect_refusal("both sides changed since the agreement");

    run.says("conflict");
    run.says("have both changed since the last agreement");
    run.says("mode = \"safix-to-bitwarden\";");
    run.says("mode = \"bitwarden-to-safix\";");
    assert_eq!(
        password(&fixture, "both"),
        "vault-changed",
        "a conflict wrote the vault's side"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "both-ways"),
        "safix-changed",
        "a conflict wrote safix's side"
    );
    assert_eq!(
        issued(&fixture, "create") + issued(&fixture, "edit"),
        writes_before,
        "a conflict wrote the item"
    );
}

/// A value carrying newlines crosses whole.
///
/// Run by `safix-bitwarden-multiline`. The refusal the keepassxc target makes —
/// `Error::ValueSpansLines`, because that store's entry password is one line —
/// is asserted absent here: this transport's value travels inside a JSON payload
/// on standard input, so a newline is a byte like any other.
#[test]
fn a_multi_line_value_crosses_whole() {
    const MULTILINE: &str = "-----BEGIN FIXTURE-----\nline two\nline three\n-----END-----";
    let fixture = declared();
    fixture
        .set("alice", "push-me", MULTILINE)
        .expect_success("seeding a value carrying newlines");

    let run = converge(&fixture, &["sync", "bitwarden", "push"], &[])
        .expect_success("a value carrying two newlines");
    assert!(
        !run.combined().contains("spans"),
        "a multi-line value was refused:\n{}",
        run.combined()
    );
    assert_eq!(
        password(&fixture, "pushed"),
        MULTILINE,
        "a multi-line value did not cross byte for byte"
    );
}

/// A declared `tags` is refused, and nothing about the mapping is read or
/// written.
///
/// Run by `safix-bitwarden-tags`. The refusal this repository leans on is
/// evaluation's, in `modules/flake/safix/bitwarden.nix`'s `unsupportedField`,
/// and `modules/flake/checks/bitwarden.nix` holds it against its literal
/// sentence. What a run can hold, and what this holds, is the runtime's own
/// second refusal — `Error::FieldUnsupported { target: "bitwarden", field:
/// "tags" }` — and that it fires before either side of the mapping is touched.
///
/// "No invocation at all", the form task 8.18 names, is the nix check's claim
/// and cannot be this one's: the run's preflight precedes every mapping's
/// judgement, so `status` and `sync` are issued before any declaration is looked
/// at. What is asserted here is the stronger available claim: no `get`, no
/// `create` and no `edit`.
#[test]
fn tags_are_refused_at_evaluation() {
    let mut fixture = Fixture::new();
    fixture.seed_output("tagged", ALICE_FILE);
    fixture.seed_bitwarden_mapping(
        "tags",
        "safix-to-bitwarden",
        ("alice", "tagged"),
        None,
        "tagged",
        json!({ "tags": ["work", "fleet"] }),
    );
    fixture
        .set("alice", "tagged", "safix-tagged")
        .expect_success("seeding the safix side");

    let run = converge(&fixture, &["sync", "bitwarden"], &[])
        .expect_refusal("a declaration naming a field this target cannot carry");

    run.says("tags");
    run.says("bitwarden cannot carry it");
    run.says("tags");
    assert_eq!(issued(&fixture, "get"), 0, "a refused mapping was read");
    assert_eq!(issued(&fixture, "edit"), 0, "a refused mapping was written");
    assert_eq!(
        issued(&fixture, "create"),
        0,
        "a refused mapping's item was created"
    );
    assert!(
        fixture.bitwarden_holds("tagged").is_none(),
        "a refused mapping's item exists"
    );
}

// ── audit ───────────────────────────────────────────────────────────────────

/// `audit bitwarden` writes nothing, and reports every mapping it compared.
///
/// Run by `safix-bitwarden-audit`. A mix of agreeing and diverged, because a
/// report that only listed findings would read as "the mappings agree" while
/// meaning "the ones I looked at do".
#[test]
fn audit_bitwarden_writes_nothing() {
    let fixture = declared();
    fixture
        .set("alice", "push-me", "safix-push-me")
        .expect_success("seeding the agreeing mapping's safix side");
    fixture
        .set("alice", "both-ways", "safix-both-ways")
        .expect_success("seeding the diverged mapping's safix side");
    fixture.bitwarden_seed(None, "pushed", &item("safix-push-me"));
    fixture.bitwarden_seed(None, "both", &item("vault-both"));

    let run = converge(&fixture, &["audit", "bitwarden", "push", "both"], &[])
        .expect_refusal("one mapping whose sides disagree");

    run.says("push");
    run.says("agreeing");
    run.says("both");
    run.says("diverged");
    assert_eq!(issued(&fixture, "edit"), 0, "audit edited an item");
    assert_eq!(issued(&fixture, "create"), 0, "audit created an item");
    assert_eq!(
        password(&fixture, "both"),
        "vault-both",
        "audit converged a mapping it was only meant to compare"
    );
}

/// `audit` refuses on a failed refresh with the same code `sync` produces.
///
/// Run by `safix-bitwarden-suite`. Dropping the pre-read `sync` from
/// `audit::run_bitwarden` while leaving it in the converging pass (task 5.10)
/// turns this red, which is the evidence the two paths share the contract rather
/// than resembling each other.
#[test]
fn audit_refuses_on_a_failed_refresh_exactly_as_sync_does() {
    let fixture = declared();
    let compared = converge(
        &fixture,
        &["audit", "bitwarden"],
        &[("SAFIX_BW_STUB_SYNC_FAILS", "1")],
    )
    .expect_refusal("an audit whose refresh failed");
    compared.says("could not refresh its local copy");
    compared.says("connect ECONNREFUSED 127.0.0.1:8222");

    let converged = converge(
        &fixture,
        &["sync", "bitwarden"],
        &[("SAFIX_BW_STUB_SYNC_FAILS", "1")],
    )
    .expect_refusal("a sync whose refresh failed");

    // The same sentence from both verbs, which is the shared refusal rather than
    // two refusals that resemble each other.
    let sentence = "could not refresh its local copy, so every bitwarden";
    assert!(
        compared.combined().contains(sentence) && converged.combined().contains(sentence),
        "audit and sync refused differently over one stale copy"
    );
}

// ── information, rather than findings ───────────────────────────────────────

/// A fixture whose mappings name a folder, with one item in it nobody declared.
fn lingering_fixture() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.seed_output("routed", ALICE_FILE);
    fixture.seed_bitwarden_mapping(
        "router",
        "bitwarden-to-safix",
        ("alice", "routed"),
        Some(FOLDER),
        "router",
        json!({}),
    );
    fixture.bitwarden_seed(Some(FOLDER), "router", &item("vault-router"));
    fixture.bitwarden_seed(Some(FOLDER), "stray", &item("vault-stray"));
    fixture
}

/// An item under a declared folder that no mapping names is reported, and the
/// run still exits zero.
///
/// Run by `safix-bitwarden-lingering`. No mode deletes an item, so a mapping
/// that was removed leaves its last value behind on purpose: the report says so
/// and the exit status does not move.
#[test]
fn a_lingering_item_is_information_and_does_not_move_the_exit_status() {
    let fixture = lingering_fixture();
    fixture
        .set("alice", "routed", "safix-before-the-pull")
        .expect_success("seeding the entry the pull overwrites");

    let run = converge(&fixture, &["sync", "bitwarden"], &[])
        .expect_success("one pulling mapping and one undeclared item");

    run.says("fleet/stray");
    run.says("router");
    run.says("pulled");
    assert_eq!(
        fixture.value(ALICE_FILE, "routed"),
        "vault-router",
        "the declared mapping did not converge"
    );
}

/// A diverged field is named and its content never is.
///
/// Run by `safix-bitwarden-fields`. The report carries `&'static str` field names
/// and no run-time string, which is what makes "never its content" a property of
/// the type rather than of this assertion — and this is what would catch a change
/// that started formatting the two values into the sentence.
#[test]
fn the_report_names_a_diverged_field_and_never_its_content() {
    let mut fixture = Fixture::new();
    fixture.seed_output("noted", ALICE_FILE);
    fixture.seed_bitwarden_mapping(
        "noted",
        "bitwarden-to-safix",
        ("alice", "noted"),
        None,
        "noted",
        json!({ "notes": "the declared note" }),
    );
    fixture
        .set("alice", "noted", "one-value-both-sides")
        .expect_success("seeding the safix side");
    fixture.bitwarden_seed(
        None,
        "noted",
        &json!({
            "object": "item",
            "type": 1,
            "login": { "password": "one-value-both-sides", "uris": [] },
            "notes": "the item's own note",
            "fields": [],
        }),
    );

    let run = converge(&fixture, &["sync", "bitwarden"], &[])
        .expect_refusal("a mapping whose value agrees and whose declared note does not");

    run.says("fields diverged");
    run.says("notes");
    let said = run.combined();
    assert!(
        !said.contains("the declared note"),
        "the report printed the declared field's content:\n{said}"
    );
    assert!(
        !said.contains("the item's own note"),
        "the report printed the stored field's content:\n{said}"
    );
}

// ── the session the run found, and the one it made ──────────────────────────

/// A run that found the client unlocked leaves it as it found it.
///
/// Run by `safix-bitwarden-suite`. The session was the operator's before safix
/// ran, and invalidating it would end whatever else they were doing with it.
#[test]
fn a_run_that_found_the_client_unlocked_does_not_lock_it() {
    let fixture = declared();
    fixture.bitwarden_seed(None, "pulled", &item("vault-pulled"));
    fixture
        .set("alice", "pull-me", "safix-before-the-pull")
        .expect_success("seeding the entry the pull overwrites");

    converge(
        &fixture,
        &["sync", "bitwarden", "pull"],
        &[("SAFIX_BW_STUB_STATE", "unlocked")],
    )
    .expect_success("a client that was already unlocked");

    assert_eq!(
        issued(&fixture, "unlock"),
        0,
        "a run unlocked a client that was already unlocked"
    );
    assert_eq!(
        issued(&fixture, "lock"),
        0,
        "a run locked a client it did not unlock"
    );
}

/// A run that unlocked the client locks it again, exactly once.
///
/// Run by `safix-bitwarden-suite`. The twin of the claim above: the session this
/// run created ends with the run, and the `bw lock` that ends it is what
/// invalidates the key that was in a child's environment.
#[test]
fn a_run_that_unlocked_locks_the_client_once() {
    let fixture = declared();
    fixture.bitwarden_seed(None, "pulled", &item("vault-pulled"));
    fixture
        .set("alice", "pull-me", "safix-before-the-pull")
        .expect_success("seeding the entry the pull overwrites");

    let mut environment = fixture.bitwarden_env();
    environment.push(("SAFIX_BW_STUB_STATE".to_owned(), "locked".to_owned()));
    fixture
        .run_sync(
            &["sync", "bitwarden", "pull"],
            UNLOCK,
            &borrowed(&environment),
        )
        .expect_success("a locked client unlocked on a terminal");

    assert_eq!(
        issued(&fixture, "unlock"),
        1,
        "the unlock did not happen exactly once"
    );
    assert_eq!(
        issued(&fixture, "lock"),
        1,
        "a run that unlocked did not lock exactly once"
    );
    let order = subcommands(&fixture);
    assert_eq!(
        order.last().map(String::as_str),
        Some("lock"),
        "the lock was not the last thing the run did: {order:?}"
    );
}
