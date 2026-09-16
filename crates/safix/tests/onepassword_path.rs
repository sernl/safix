//! `safix sync 1password` and `safix audit 1password`, driven against a
//! stand-in command and the real everything else.
//!
//! sops, age and git are real here as everywhere in this suite; the 1Password
//! command is the stand-in `tests/support/op-stub.rs` models. Every other stub in
//! this suite could in principle be replaced by a check that runs the real tool
//! in a sandbox — `store_cli.rs` is exactly that for `keepassxc-cli`. This one
//! cannot, permanently: `_1password-cli` at this flake's pin is unfree, so naming
//! it from a check, a package or a devshell makes evaluation fail for every
//! consumer who has not allowed unfree packages; there is no self-hostable
//! 1Password server to point a sandboxed node at; and every authentication path
//! needs the network, which no `nix build` and no hermetic VM node has. Each
//! ground alone is sufficient and none of them expires.
//!
//! `harness::refuse_a_real_onepassword` is the structural guard, and it is why
//! every run below goes through [`Fixture::onepassword_env`]: a run whose
//! override does not name the stand-in, whose service-account token is not the
//! fixture's own fake, or which names an account through `OP_ACCOUNT`, fails
//! before a process is spawned. A machine that develops this suite plausibly
//! holds a real, signed-in `op` and the operator's own vaults, and there is no
//! undo for a value overwritten there.
//!
//! # What each fixture is
//!
//! One mapping per mode and one state per mapping, the shape `sync_path.rs`
//! establishes; plus the three this target's own decisions need: an item
//! carrying members no declaration names, a value carrying newlines, and a run
//! in which exactly one of three writes refuses.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use harness::{ALICE_FILE, Fixture, real_sops, shim};
use serde_json::json;

/// The vault every mapping in this suite names.
///
/// Synthetic, and named so in one place: no vault of anybody's appears in this
/// repository.
const VAULT: &str = "fixture-vault";

/// A second vault, for the claim that the item's address is the vault and the
/// title together.
const OTHER_VAULT: &str = "other-vault";

/// A fixture with one mapping of each mode, and alice holding a value for each.
fn declared() -> Fixture {
    let mut fixture = Fixture::new();
    for name in ["push-me", "pull-me", "both-ways", "back-me-up"] {
        fixture.seed_output(name, ALICE_FILE);
    }

    fixture.seed_onepassword_mapping(
        "push",
        "safix-to-1password",
        ("alice", "push-me"),
        VAULT,
        "pushed",
        json!({
            "username": "alice@example.com",
            "url": "https://grafana.example.invalid",
            "notes": "minted by safix",
            "tags": ["work", "fleet"],
        }),
    );
    fixture.seed_onepassword_mapping(
        "pull",
        "1password-to-safix",
        ("alice", "pull-me"),
        VAULT,
        "pulled",
        json!({}),
    );
    fixture.seed_onepassword_mapping(
        "both",
        "two-way",
        ("alice", "both-ways"),
        VAULT,
        "both",
        json!({}),
    );
    fixture.seed_onepassword_mapping(
        "copy",
        "backup",
        ("alice", "back-me-up"),
        VAULT,
        "copied",
        json!({}),
    );
    fixture
}

/// One item as the documented JSON prints it, holding one value.
fn item(title: &str, value: &str) -> serde_json::Value {
    json!({
        "id": title,
        "title": title,
        "category": "LOGIN",
        "fields": [{
            "id": "password",
            "type": "CONCEALED",
            "purpose": "PASSWORD",
            "label": "password",
            "value": value,
        }],
    })
}

fn borrowed(extra: &[(String, String)]) -> Vec<(&str, &str)> {
    extra
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// What one field of a stored item holds, or nothing where the item carries no
/// such field.
fn stored_field(fixture: &Fixture, vault: &str, title: &str, id: &str) -> Option<String> {
    let held = fixture.onepassword_holds(vault, title)?;
    let document: serde_json::Value = serde_json::from_str(&held).unwrap();
    document["fields"]
        .as_array()?
        .iter()
        .find(|field| field["id"] == json!(id))
        .and_then(|field| field["value"].as_str())
        .map(str::to_owned)
}

/// One stored item, parsed.
fn stored_item(fixture: &Fixture, vault: &str, title: &str) -> serde_json::Value {
    let held = fixture
        .onepassword_holds(vault, title)
        .unwrap_or_else(|| panic!("{vault}/{title} is not in the stand-in's store"));
    serde_json::from_str(&held).unwrap()
}

/// Each mode converges exactly as its name says, over one run.
#[test]
fn each_mode_converges_exactly_as_its_name_says() {
    let fixture = declared();
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    for name in ["push-me", "both-ways", "back-me-up"] {
        fixture
            .run_with(&["set", "alice", name], &format!("safix-{name}"))
            .expect_success("seeding the safix side");
    }
    fixture
        .run_with(&["set", "alice", "pull-me"], "safix-before-the-pull")
        .expect_success("seeding the entry a pull overwrites");
    fixture.onepassword_seed_item(VAULT, "pulled", &item("pulled", "op-pulled"));

    let run = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("one mapping of each mode, each with something to do");

    run.says("push  alice.push-me -> fixture-vault/pushed  safix-to-1password  updated");
    run.says("pull  fixture-vault/pulled -> alice.pull-me  1password-to-safix  pulled");
    run.says("both  alice.both-ways -> fixture-vault/both  two-way  updated");
    run.says("copy  alice.back-me-up -> fixture-vault/copied  backup  updated");

    // What each side holds afterwards.
    assert_eq!(
        stored_field(&fixture, VAULT, "pushed", "password").as_deref(),
        Some("safix-push-me"),
        "the item did not converge to safix's value"
    );
    assert_eq!(
        fixture.value(ALICE_FILE, "pull-me"),
        "op-pulled",
        "safix did not converge to the item's value"
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "both", "password").as_deref(),
        Some("safix-both-ways"),
        "a two-way mapping with an absent far side did not bootstrap"
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "copied", "password").as_deref(),
        Some("safix-back-me-up"),
        "a backup mapping did not write into absence"
    );

    // Every declared field reached its documented home.
    let pushed = stored_item(&fixture, VAULT, "pushed");
    assert_eq!(
        stored_field(&fixture, VAULT, "pushed", "username").as_deref(),
        Some("alice@example.com")
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "pushed", "notesPlain").as_deref(),
        Some("minted by safix"),
        "the declared note is not the item's own note"
    );
    // The autofill website rather than a `url`-typed custom field, which is
    // where 1Password's documentation says a browser looks.
    assert_eq!(
        pushed["urls"][0]["href"], "https://grafana.example.invalid",
        "the declared url is not the item's autofill website"
    );
    assert_eq!(pushed["tags"], json!(["work", "fleet"]));
    assert!(
        stored_item(&fixture, VAULT, "both")["fields"]
            .as_array()
            .unwrap()
            .iter()
            .all(|field| field["purpose"] != json!("USERNAME")),
        "a mapping declaring no field wrote one"
    );

    // The two-way mapping recorded its agreement, in a concealed field of the
    // item itself rather than beside it and rather than in the repository.
    let memory = stored_field(&fixture, VAULT, "both", "safix-sync-state")
        .expect("a two-way mapping recorded no agreement");
    assert!(
        memory.starts_with("safix-sync-v1 "),
        "the recorded agreement carries no format tag: {memory}"
    );
    assert_eq!(
        stored_item(&fixture, VAULT, "both")["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["id"] == json!("safix-sync-state"))
            .unwrap()["type"],
        json!("CONCEALED"),
        "the recorded agreement is not concealed"
    );
    assert_no_oracle(&fixture, &["safix-both-ways", "op-pulled"]);

    // Nothing a protected read produced reached standard output.
    assert_eq!(run.output(), "", "a value reached standard output");
    for value in ["safix-push-me", "op-pulled", "safix-both-ways"] {
        run.silent_about(value);
    }
}

/// No value and no field ever travels an argument vector or an environment.
#[test]
fn no_value_and_no_field_ever_travels_an_argument_vector() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_output("grafana-note", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "push",
        "safix-to-1password",
        ("alice", "push-me"),
        VAULT,
        "pushed",
        json!({
            "username": "alice@example.com",
            "url": "https://grafana.example.invalid",
            // The one channel-sensitive shape: a field read out of another
            // entry of the mapping's own person, which is a secret. It is
            // admissible on every field of this target, and that is the
            // asymmetry against keepassxc the capability table exists for.
            "notes": {"entry": "grafana-note"},
            "tags": ["SAFIX-TAG-WORK"],
        }),
    );
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "grafana-note"], "SOURCED-NOTE-VALUE")
        .expect_success("seeding the entry a field is sourced from");
    fixture
        .run_with(&["set", "alice", "push-me"], "SECRET-VALUE")
        .expect_success("seeding the safix side");

    fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("a mapping declaring all four fields");

    let argv = fixture.onepassword_recorded("argv").join("\n");
    let environment = fixture.onepassword_recorded("env").join("\n");
    let stdin = fixture.onepassword_recorded("stdin").join("\n");

    for carried in [
        "SECRET-VALUE",
        "SOURCED-NOTE-VALUE",
        "alice@example.com",
        "https://grafana.example.invalid",
        "SAFIX-TAG-WORK",
    ] {
        assert!(
            stdin.contains(carried),
            "{carried} did not cross standard input:\n{stdin}"
        );
        assert!(
            !argv.contains(carried),
            "{carried} reached an argument vector:\n{argv}"
        );
        assert!(
            !environment.contains(carried),
            "{carried} reached the child's environment:\n{environment}"
        );
    }

    // The instrument's own rule, asserted against what it recorded: it exits
    // non-zero on any word carrying an assignment, so a run that reached this
    // far cannot have spelled one.
    assert!(
        !argv.split_whitespace().any(|word| word.contains('=')),
        "an argument vector carried an assignment:\n{argv}"
    );
    assert!(
        !fixture
            .onepassword_recorded("exit")
            .iter()
            .any(|path| path == "assignment-in-argv"),
        "the stand-in refused an assignment, so one was spelled"
    );
}

/// A value carrying newlines crosses whole, with no refusal anywhere.
#[test]
fn a_multi_line_value_crosses_whole() {
    let mut fixture = Fixture::new();
    fixture.seed_output("multi", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "multi",
        "two-way",
        ("alice", "multi"),
        VAULT,
        "multi",
        json!({}),
    );
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    let value = "first line\nsecond line\nthird";
    fixture
        .run_with(&["set", "alice", "multi"], value)
        .expect_success("seeding a multi-line value");

    let run = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("a multi-line value on a transport that carries one");
    run.says("multi  alice.multi -> fixture-vault/multi  two-way  updated");
    run.silent_about("spans");

    assert_eq!(
        stored_field(&fixture, VAULT, "multi", "password").as_deref(),
        Some(value),
        "the multi-line value did not cross byte-identically"
    );

    // And it reads back the same, which is the other half of the claim: a
    // second run over the same two sides is unchanged.
    let second = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("a second run over a multi-line value");
    second.says("0 updated, 0 pulled, 1 unchanged");
}

/// An edit preserves what the declaration does not name.
#[test]
fn an_edit_preserves_what_the_declaration_does_not_name() {
    let mut fixture = Fixture::new();
    fixture.seed_output("rotated", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "rotate",
        "safix-to-1password",
        ("alice", "rotated"),
        VAULT,
        "rich",
        json!({"username": "alice@example.com"}),
    );
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    // Three members no declaration names: a passkey, a one-time-password
    // field, and a section a person made.
    fixture.onepassword_seed_item(
        VAULT,
        "rich",
        &json!({
            "id": "rich",
            "title": "rich",
            "category": "LOGIN",
            "passkey": {"credentialId": "kept-credential", "rpId": "example.invalid"},
            "sections": [{"id": "custom", "label": "notes from a person"}],
            "fields": [
                {
                    "id": "password",
                    "type": "CONCEALED",
                    "purpose": "PASSWORD",
                    "label": "password",
                    "value": "the-old-value",
                },
                {
                    "id": "one-time",
                    "type": "OTP",
                    "label": "one-time password",
                    "value": "otpauth://totp/rich?secret=KEPT",
                },
                {
                    "id": "personal",
                    "type": "STRING",
                    "label": "a person's own field",
                    "section": {"id": "custom"},
                    "value": "kept by a person",
                },
            ],
        }),
    );
    fixture
        .run_with(&["set", "alice", "rotated"], "the-new-value")
        .expect_success("seeding the safix side");

    fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("a push onto an item carrying a passkey");

    let after = stored_item(&fixture, VAULT, "rich");
    assert_eq!(
        after["passkey"],
        json!({"credentialId": "kept-credential", "rpId": "example.invalid"}),
        "the passkey did not survive the edit"
    );
    assert_eq!(
        after["sections"],
        json!([{"id": "custom", "label": "notes from a person"}]),
        "the section no declaration names did not survive the edit"
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "rich", "one-time").as_deref(),
        Some("otpauth://totp/rich?secret=KEPT"),
        "the one-time-password field did not survive the edit"
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "rich", "personal").as_deref(),
        Some("kept by a person"),
        "a person's own field did not survive the edit"
    );

    // And only the value and the declared field moved.
    assert_eq!(
        stored_field(&fixture, VAULT, "rich", "password").as_deref(),
        Some("the-new-value")
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "rich", "username").as_deref(),
        Some("alice@example.com")
    );

    // The write went through `item edit` rather than `item create`, which is
    // what makes the round trip the mechanism rather than a coincidence of the
    // stand-in's tolerance.
    let argv = fixture.onepassword_recorded("argv").join("\n");
    assert!(argv.contains("item edit rich"), "the write was not an edit");
    assert!(
        !argv.contains("item create"),
        "an existing item was created rather than edited:\n{argv}"
    );
}

/// A signed-out run refuses before reading any side.
#[test]
fn a_signed_out_run_refuses_before_reading_any_side() {
    let fixture = declared();
    let spool = fixture.work.join("sops-spy");
    let sops = real_sops();
    let mut extra = fixture.onepassword_env();
    extra.push(("SAFIX_OP_STUB_SIGNED_OUT".to_owned(), "1".to_owned()));
    // The recording sops, so "no side was decrypted" is read off what sops was
    // handed rather than off the absence of a report line.
    extra.push(("SAFIX_SOPS".to_owned(), shim().to_owned()));
    extra.push(("SAFIX_SHIM_ROLE".to_owned(), "spy".to_owned()));
    extra.push(("SAFIX_SHIM_SOPS".to_owned(), sops));
    extra.push((
        "SAFIX_SHIM_SPY".to_owned(),
        spool.to_string_lossy().into_owned(),
    ));
    let extra = borrowed(&extra);

    let refused = fixture
        .run_graphical_env(&["sync", "1password"], &extra)
        .expect_refusal("a run with no session");
    assert_eq!(refused.refusal_code(), "onepassword_signed_out");

    // Exactly one invocation, and it was the preflight.
    let argv = fixture.onepassword_recorded("argv");
    assert_eq!(
        argv.len(),
        1,
        "the run reached the service more than once:\n{argv:?}"
    );
    assert!(
        argv[0].contains("whoami"),
        "the one invocation was not the preflight: {}",
        argv[0]
    );

    // And safix's own side of no mapping was decrypted, which is the whole
    // reason the preflight precedes the first read.
    let handed = std::fs::read_to_string(spool.join("argv")).unwrap_or_default();
    assert!(
        !handed.contains("decrypt"),
        "the run decrypted a safix side before the preflight refused:\n{handed}"
    );
}

/// Each refusal has its own sentence and leaves both sides alone.
#[test]
fn the_refusals_each_have_their_own_code_and_leave_both_sides_alone() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_output("holds-nothing", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "push",
        "safix-to-1password",
        ("alice", "push-me"),
        VAULT,
        "pushed",
        json!({}),
    );
    fixture.seed_onepassword_mapping(
        "empty",
        "safix-to-1password",
        ("alice", "holds-nothing"),
        VAULT,
        "empty",
        json!({}),
    );
    fixture.seed_onepassword_mapping(
        "needs-it",
        "1password-to-safix",
        ("alice", "push-me"),
        VAULT,
        "absent",
        json!({}),
    );
    let base = fixture.onepassword_env();
    let extra = borrowed(&base);

    fixture
        .run_with(&["set", "alice", "push-me"], "safix-push-me")
        .expect_success("seeding the one side that holds a value");

    let _head_before_the_refusals = fixture.head();

    // A mapping name no declaration carries stops the run, before anything.
    let unknown = fixture
        .run_graphical_env(&["sync", "1password", "typo"], &extra)
        .expect_refusal("a mapping name nobody declared");
    assert_eq!(unknown.refusal_code(), "unknown_sync_mapping");

    // A safix side holding nothing, and a far side holding no item under a
    // pulling mode, are each their own mapping's refusal.
    let refused = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_refusal("two mappings that cannot converge");
    refused.says("empty  ");
    refused.says("no value yet");
    refused.says("needs-it  ");
    refused.says("the vault 'fixture-vault' holds no item 'absent'");
    refused.says("mode = \"safix-to-1password\" makes the item follow safix");
    // The one that could converge still did, which is the per-mapping rule.
    refused.says("push  alice.push-me -> fixture-vault/pushed");

    // A vault the session cannot see is a distinct fault with a distinct
    // remedy.
    let mut unreachable = base.clone();
    unreachable.push(("SAFIX_OP_STUB_UNKNOWN_VAULT".to_owned(), VAULT.to_owned()));
    let unreachable = borrowed(&unreachable);
    let refused = fixture
        .run_env(&["sync", "1password"], None, &unreachable)
        .expect_refusal("a vault this session cannot see");
    refused.says("names the vault 'fixture-vault', and this session cannot see");
    refused.says("A service account reaches exactly the vaults");

    // The program refusing over one item carries its own words and the vector
    // it was run with.
    // Rotated first, so the push has something to write: the earlier run in
    // this test already converged that mapping, and an agreeing mapping issues
    // no write for the switch to refuse.
    fixture
        .run_with(&["set", "alice", "push-me"], "rotated-again")
        .expect_success("rotating the value the refused write would carry");
    // The rotation above is a commit of its own, so the "a refused run
    // committed" claim below is made against the history as it stands here.
    let head = fixture.head();
    let mut refusing = base.clone();
    refusing.push(("SAFIX_OP_STUB_REFUSES".to_owned(), "pushed".to_owned()));
    let refusing = borrowed(&refusing);
    let refused = fixture
        .run_env(&["sync", "1password"], None, &refusing)
        .expect_refusal("the program refusing over one item");
    refused.says("the 1Password command refused over the item 'fixture-vault/pushed'");
    refused.says("No value and no field is in that line");

    // The program absent at all is the run's own refusal.
    let mut absent = base.clone();
    absent.retain(|(name, _)| name != "SAFIX_OP");
    absent.push((
        "SAFIX_OP".to_owned(),
        fixture
            .work
            .join("there-is-no-op-here")
            .to_string_lossy()
            .into_owned(),
    ));
    let absent = borrowed(&absent);
    let refused = fixture
        .run_graphical_env(&["sync", "1password"], &absent)
        .expect_refusal("the program absent");
    assert_eq!(refused.refusal_code(), "onepassword_unavailable");

    // None of them left a commit, a dirty tree, or a partial write.
    assert_eq!(fixture.head(), head, "a refused run committed");
    assert_eq!(fixture.status(), "", "a refused run left the tree dirty");
    assert!(
        fixture.onepassword_holds(VAULT, "absent").is_none(),
        "a refused pulling mapping created the item it could not read"
    );
    assert!(
        fixture.onepassword_holds(VAULT, "empty").is_none(),
        "a mapping whose safix side holds nothing wrote an item"
    );
}

/// A failure on one mapping does not end the run.
#[test]
fn a_failure_on_one_mapping_does_not_end_the_run() {
    let mut fixture = Fixture::new();
    for name in ["first", "second", "third"] {
        fixture.seed_output(name, ALICE_FILE);
        fixture.seed_onepassword_mapping(
            name,
            "safix-to-1password",
            ("alice", name),
            VAULT,
            name,
            json!({}),
        );
    }
    let mut extra = fixture.onepassword_env();
    extra.push(("SAFIX_OP_STUB_REFUSES".to_owned(), "second".to_owned()));
    let extra = borrowed(&extra);

    for name in ["first", "second", "third"] {
        fixture
            .run_with(&["set", "alice", name], &format!("safix-{name}"))
            .expect_success("seeding the safix side");
    }

    let run = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_refusal("one of three writes refusing");

    // All three are in the report.
    for name in ["first", "second", "third"] {
        run.says(&format!("{name}  "));
    }
    run.says("first  alice.first -> fixture-vault/first  safix-to-1password  updated");
    run.says("second  alice.second <-> fixture-vault/second  safix-to-1password  refused");
    run.says("third  alice.third -> fixture-vault/third  safix-to-1password  updated");
    run.says("2 updated, 0 pulled, 0 unchanged, 0 conflict, 1 refused");

    assert_eq!(
        stored_field(&fixture, VAULT, "first", "password").as_deref(),
        Some("safix-first")
    );
    assert_eq!(
        stored_field(&fixture, VAULT, "third", "password").as_deref(),
        Some("safix-third")
    );
    assert!(
        fixture.onepassword_holds(VAULT, "second").is_none(),
        "the refused write landed anyway"
    );
}

/// Two-way converges toward the side that moved, and will not guess when both
/// did.
#[test]
fn two_way_converges_toward_the_side_that_moved_and_will_not_guess_when_both_did() {
    let mut fixture = Fixture::new();
    fixture.seed_output("agreed", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "both",
        "two-way",
        ("alice", "agreed"),
        VAULT,
        "both",
        json!({}),
    );
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "agreed"], "the-agreed-value")
        .expect_success("seeding the safix side");
    fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("the bootstrap that records the agreement");

    // Exactly one side moves: the item's. safix converges to it.
    let mut moved = stored_item(&fixture, VAULT, "both");
    for field in moved["fields"].as_array_mut().unwrap() {
        if field["id"] == json!("password") {
            field["value"] = json!("the-item-moved");
        }
    }
    fixture.onepassword_seed_item(VAULT, "both", &moved);
    let pulled = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("one side moved");
    pulled.says("both  fixture-vault/both -> alice.agreed  two-way  pulled");
    assert_eq!(fixture.value(ALICE_FILE, "agreed"), "the-item-moved");

    // Both sides move: nothing is written and the finding names both remedies.
    let mut moved = stored_item(&fixture, VAULT, "both");
    for field in moved["fields"].as_array_mut().unwrap() {
        if field["id"] == json!("password") {
            field["value"] = json!("the-item-moved-again");
        }
    }
    fixture.onepassword_seed_item(VAULT, "both", &moved);
    fixture
        .run_with(&["set", "alice", "agreed"], "safix-moved-too")
        .expect_success("moving safix's side as well");
    let conflict = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_refusal("both sides moved");
    conflict.says("two-way  conflict");
    conflict.says("mode = \"safix-to-1password\";");
    conflict.says("mode = \"1password-to-safix\";");
    assert_eq!(
        stored_field(&fixture, VAULT, "both", "password").as_deref(),
        Some("the-item-moved-again"),
        "a conflict wrote the item"
    );
    assert_eq!(fixture.value(ALICE_FILE, "agreed"), "safix-moved-too");

    // A memory a person corrupted is treated as absent, which bootstraps
    // rather than refusing: the two sides here differ, so it is the conflict
    // above rather than a refusal about the memory itself.
    let mut corrupted = stored_item(&fixture, VAULT, "both");
    for field in corrupted["fields"].as_array_mut().unwrap() {
        if field["id"] == json!("safix-sync-state") {
            field["value"] = json!("safix-sync-v99 whatever-a-person-typed");
        }
    }
    fixture.onepassword_seed_item(VAULT, "both", &corrupted);
    let after = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_refusal("a memory under a tag this version does not know");
    after.says("two-way  conflict");
    after.silent_about("safix-sync-v99");

    // And with one side empty, an unreadable memory bootstraps rather than
    // refusing.
    fixture.seed_output("fresh", ALICE_FILE);
    let mut second = Fixture::new();
    second.seed_output("fresh", ALICE_FILE);
    second.seed_onepassword_mapping(
        "fresh",
        "two-way",
        ("alice", "fresh"),
        VAULT,
        "fresh",
        json!({}),
    );
    let extra = second.onepassword_env();
    let extra = borrowed(&extra);
    second.onepassword_seed_item(
        VAULT,
        "fresh",
        &json!({
            "id": "fresh",
            "title": "fresh",
            "category": "LOGIN",
            "fields": [{
                "id": "safix-sync-state",
                "type": "CONCEALED",
                "label": "safix-sync-state",
                "value": "not a memory at all",
            }],
        }),
    );
    second
        .run_with(&["set", "alice", "fresh"], "only-safix-holds-one")
        .expect_success("seeding the one side that holds a value");
    let bootstrapped = second
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("an unreadable memory with one side empty");
    bootstrapped.says("fresh  alice.fresh -> fixture-vault/fresh  two-way  updated");
}

/// An item no mapping declares is reported and never removed.
#[test]
fn an_item_no_mapping_declares_is_reported_and_never_removed() {
    let mut fixture = Fixture::new();
    fixture.seed_output("push-me", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "push",
        "safix-to-1password",
        ("alice", "push-me"),
        VAULT,
        "pushed",
        json!({}),
    );
    // A second mapping naming the same item title in another vault, which is a
    // second item: the address is the vault and the title together.
    fixture.seed_output("elsewhere", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "elsewhere",
        "safix-to-1password",
        ("alice", "elsewhere"),
        OTHER_VAULT,
        "pushed",
        json!({}),
    );
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    for name in ["push-me", "elsewhere"] {
        fixture
            .run_with(&["set", "alice", name], &format!("safix-{name}"))
            .expect_success("seeding the safix side");
    }
    fixture.onepassword_seed_item(
        VAULT,
        "left-behind",
        &item("left-behind", "somebody-else's"),
    );

    let run = fixture
        .run_env(&["sync", "1password"], None, &extra)
        .expect_success("a vault holding an item no mapping declares");
    run.says("fixture-vault/left-behind is in a declared vault and no mapping declares it.");
    run.says("Nothing here will remove it; a person does that.");
    assert_eq!(
        stored_field(&fixture, VAULT, "left-behind", "password").as_deref(),
        Some("somebody-else's"),
        "the item no mapping declares was touched"
    );

    // Two mappings naming one title in two vaults are two items, and neither
    // is reported as lingering.
    run.silent_about("other-vault/pushed is in a declared vault");
    run.silent_about("fixture-vault/pushed is in a declared vault");

    // The read is scoped to the mapped item rather than to the vault: every
    // `item get` names the title the declaration named.
    let argv = fixture.onepassword_recorded("argv");
    for line in argv.iter().filter(|line| line.contains("item get")) {
        assert!(
            line.contains("item get pushed"),
            "a read asked for something other than the mapped item: {line}"
        );
    }
    assert!(
        !argv
            .iter()
            .any(|line| line.contains("item get") && line.contains("left-behind")),
        "a field of an unmapped item was requested:\n{argv:?}"
    );

    // And the run's exit status was not moved by it.
    assert!(
        fixture
            .run_env(&["audit", "1password"], None, &extra)
            .succeeded(),
        "an item no mapping declares moved audit's exit status"
    );
}

/// audit compares without writing, and names a field without printing it.
#[test]
fn audit_compares_without_writing_and_names_a_field_without_printing_it() {
    let mut fixture = Fixture::new();
    fixture.seed_output("agrees", ALICE_FILE);
    fixture.seed_output("diverges", ALICE_FILE);
    fixture.seed_onepassword_mapping(
        "fields",
        "safix-to-1password",
        ("alice", "agrees"),
        VAULT,
        "fields",
        json!({"notes": "THE-DECLARED-NOTE"}),
    );
    fixture.seed_onepassword_mapping(
        "value",
        "safix-to-1password",
        ("alice", "diverges"),
        VAULT,
        "value",
        json!({"notes": "ALSO-DECLARED"}),
    );
    let extra = fixture.onepassword_env();
    let extra = borrowed(&extra);

    fixture
        .run_with(&["set", "alice", "agrees"], "the-same-value")
        .expect_success("seeding the agreeing mapping");
    fixture
        .run_with(&["set", "alice", "diverges"], "safix-has-this")
        .expect_success("seeding the diverging mapping");

    // The value-agreeing, field-diverging item, and the value-diverging one.
    fixture.onepassword_seed_item(VAULT, "fields", &item("fields", "the-same-value"));
    fixture.onepassword_seed_item(VAULT, "value", &{
        let mut held = item("value", "the-item-has-that");
        held["fields"].as_array_mut().unwrap().push(json!({
            "id": "notesPlain",
            "type": "STRING",
            "purpose": "NOTES",
            "label": "notesPlain",
            "value": "THE-ITEM-NOTE",
        }));
        held
    });

    let run = fixture
        .run_env(&["audit", "1password"], None, &extra)
        .expect_refusal("one field divergence and one value divergence");

    run.says("fields  alice.agrees <-> fixture-vault/fields  safix-to-1password  fields diverged");
    run.says("these declared fields differ: notes.");
    // A value divergence is reported as itself, never as the field one it also
    // has.
    run.says("value  alice.diverges <-> fixture-vault/value  safix-to-1password  diverged");
    run.says("safix sync 1password value");

    // Neither content reached any output path.
    run.silent_about("THE-DECLARED-NOTE");
    run.silent_about("THE-ITEM-NOTE");
    run.silent_about("safix-has-this");
    run.silent_about("the-item-has-that");

    // And nothing was written: no create, no edit.
    let argv = fixture.onepassword_recorded("argv").join("\n");
    assert!(
        !argv.contains("item create"),
        "audit created an item:\n{argv}"
    );
    assert!(!argv.contains("item edit"), "audit edited an item:\n{argv}");
    assert_eq!(
        stored_field(&fixture, VAULT, "fields", "password").as_deref(),
        Some("the-same-value"),
        "audit wrote the item"
    );
}

/// Neither a value nor a derivative of one reaches the repository.
fn assert_no_oracle(fixture: &Fixture, values: &[&str]) {
    for value in values {
        assert!(
            fixture.holds_anywhere(value).is_none(),
            "the value {value} reached the tree"
        );
        assert!(
            fixture
                .holds_anywhere(&sha256_hex(fixture, value.as_bytes()))
                .is_none(),
            "a digest of {value} reached the tree"
        );
    }
    assert!(
        fixture.holds_anywhere("safix-sync-v1").is_none(),
        "the recorded agreement's own tag reached the tree, so the memory is in the \
         repository rather than in the item"
    );
}

/// SHA-256 as hex, computed by `sha256sum` rather than by the runtime.
///
/// The reason `sync_path.rs` gives for its own copy: a test that asked the
/// runtime for the digest it would have written and then searched the tree for
/// that would pass over a runtime that computed nothing at all.
fn sha256_hex(fixture: &Fixture, bytes: &[u8]) -> String {
    let path = fixture.work.join("oracle-input");
    std::fs::write(&path, bytes).expect("could not write the oracle's input");
    let finished = std::process::Command::new("sha256sum")
        .arg(&path)
        .output()
        .expect("could not run sha256sum");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&finished.stdout)
        .split_whitespace()
        .next()
        .expect("sha256sum printed nothing")
        .to_owned()
}
