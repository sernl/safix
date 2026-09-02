//! `safix upload`, which seeds a machine's own host identity before its
//! first activation.
//!
//! The claims are the ones a hand-run `scp` would have made the operator
//! responsible for. That the machine name is checked against the declared
//! fleet before anything else runs. That `--directory` writes exactly the
//! two files the paths and modes name and touches no network. That remote
//! mode probes before it writes and takes exactly one of three actions, with
//! `--force` inert on a match. That the transport wipes and extracts at the
//! fixed destination, over a tarball whose own mode bits are read back
//! rather than trusted. And that the command's own success output states
//! what a rebuild is for.
//!
//! # What is fixtured and what is asserted
//!
//! The evaluation is stubbed, as everywhere else in this suite. `ssh-keygen`,
//! `ssh-to-age`, `ssh-keyscan` and `ssh` are stubbed too — see
//! `tests/support/transport-stub.rs` for why that is permitted here where
//! stubbing sops is not — over a synthetic identity convention: an
//! "identity" is a plain string, its derived public key is
//! `ssh-ed25519 <string>`, and its derived age recipient is `age1<string>`.
//! `tar` is the one real tool this suite drives for `upload`, because
//! reading its own archive back is what task 4.7's claim is about.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

use harness::Fixture;

/// deck's identity, and the recipient it derives to under this stub's
/// convention — see the module doc.
const DECK_IDENTITY: &str = "CANARY-deck-key";
const DECK_RECIPIENT: &str = "age1CANARY-deck-key";

/// A fleet declaring one machine with a recipient, one without, and the two
/// people every fixture already carries.
fn fleet() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.declare_machine("deck", Some(DECK_RECIPIENT));
    fixture.declare_machine("keyless", None);
    fixture
}

/// Write an identity file under the fixture's own scratch directory.
fn identity_file(fixture: &Fixture, name: &str, content: &str) -> PathBuf {
    let path = fixture.work.join(name);
    std::fs::write(&path, content).expect("the identity fixture could not be written");
    path
}

/// [`Fixture::run_env`] and friends take borrowed pairs; the environment
/// helpers return owned ones.
fn borrowed(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// [`Fixture::transport_env`], with a probe answer pushed on when one is
/// given.
fn transport_env(fixture: &Fixture, presented: Option<&str>) -> Vec<(String, String)> {
    let mut environment = fixture.transport_env();
    if let Some(content) = presented {
        environment.push((
            "SAFIX_TRANSPORT_STUB_PRESENTED".to_owned(),
            content.to_owned(),
        ));
    }
    environment
}

/// Every path under `root`, relative to it, sorted.
fn paths_under(root: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
}

/// The recursive half of [`paths_under`], a free function rather than
/// nested inside it: an item declared after a statement is a lint of its
/// own.
fn walk(dir: &std::path::Path, root: &std::path::Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, root, found);
        } else {
            found.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
}

// ── 1. the verb, its parsing, and the machine-targeting refusals ──────────

#[test]
fn an_undeclared_machine_is_refused_before_anything_else_runs() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &[
                "upload",
                "ghost",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("an undeclared machine");
    run.says("'ghost' is not a declared machine of flake.safix.machines");
    run.says("Declared machines:");
    assert!(!out.exists(), "a refused run created the output directory");
    assert!(
        fixture.transport_invocations().is_empty(),
        "a refused run reached a subprocess before resolving the machine"
    );
}

#[test]
fn a_declared_machine_with_no_recipient_is_refused_distinctly() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &[
                "upload",
                "keyless",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a machine with no declared recipient");
    run.says("'keyless' is a declared machine with no recipient");
    run.says("Declare flake.safix.machines.keyless.recipient");
    run.silent_about("is not a declared machine");
    assert!(!out.exists());
    assert!(
        fixture.transport_invocations().is_empty(),
        "the null-recipient refusal fired before any identity was read"
    );
}

#[test]
fn a_persons_name_is_refused_the_same_way_as_an_undeclared_machine() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    // alice is a declared person in every fixture's placements.
    let run = fixture
        .run_env(
            &[
                "upload",
                "alice",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a person's name");
    run.says("'alice' is not a declared machine of flake.safix.machines");
    assert!(!out.exists());
}

// ── 2. --directory mode ────────────────────────────────────────────────────

#[test]
fn directory_mode_writes_the_matching_identity_at_the_declared_paths_and_modes() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("a matching directory-mode upload");
    run.says("own next rebuild is what activates it");

    let key_path = out.join("etc/ssh/ssh_host_ed25519_key");
    let pub_path = out.join("etc/ssh/ssh_host_ed25519_key.pub");
    assert_eq!(
        std::fs::read_to_string(&key_path).expect("the private key was not written"),
        DECK_IDENTITY,
    );
    assert_eq!(
        std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777,
        0o600,
        "the private key's mode is not 0600"
    );
    assert_eq!(
        std::fs::read_to_string(&pub_path).expect("the public key was not written"),
        "ssh-ed25519 CANARY-deck-key\n",
    );
    assert_eq!(
        std::fs::metadata(&pub_path).unwrap().permissions().mode() & 0o777,
        0o644,
        "the public key's mode is not 0644"
    );

    assert_eq!(
        paths_under(&out),
        vec![
            "etc/ssh/ssh_host_ed25519_key".to_owned(),
            "etc/ssh/ssh_host_ed25519_key.pub".to_owned(),
        ],
        "an extra path was created under DIR"
    );

    // 2.5: no network code path is reachable — only the two local tools ran.
    assert_eq!(
        fixture.transport_invocations(),
        vec!["ssh-keygen".to_owned(), "ssh-to-age".to_owned()],
        "directory mode reached a network-carrying tool"
    );
}

#[test]
fn directory_without_identity_is_refused_before_touching_the_filesystem() {
    let fixture = fleet();
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &["upload", "deck", "--directory", &out.to_string_lossy()],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("directory mode without --identity");
    run.says("--identity is required to write a host identity; nothing was written");
    assert!(!out.exists());
    assert!(fixture.transport_invocations().is_empty());
}

#[test]
fn a_mismatched_identity_is_refused_before_directory_is_created_naming_both_recipients() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", "CANARY-wrong-key");
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a mismatched identity");
    run.says("age1CANARY-wrong-key");
    run.says(DECK_RECIPIENT);
    run.says("deck's declared recipient");
    run.says("would not match");
    assert!(!out.exists(), "a refused write created the directory");
}

/// 2.6, first drill: one character different from the declared recipient
/// still refuses.
#[test]
fn a_recipient_one_character_different_still_refuses() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", "CANARY-deck-keX");
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a recipient one character off");
    assert!(!out.exists());
}

/// 2.6, second drill: a null-recipient machine still refuses even with a
/// key that would otherwise be humanly indistinguishable from valid, because
/// the null-recipient refusal fires first and never reads the identity.
#[test]
fn a_null_recipient_machine_refuses_before_reading_any_identity() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", "anything-at-all");
    let out = fixture.work.join("preseed");
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &[
                "upload",
                "keyless",
                "--directory",
                &out.to_string_lossy(),
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a null-recipient machine");
    run.says("with no recipient");
    assert!(!out.exists());
    assert!(
        fixture.transport_invocations().is_empty(),
        "the identity was read before the null-recipient refusal fired"
    );
}

// ── 3. remote mode: the probe and the three-way branch ────────────────────

#[test]
fn a_matching_presented_key_is_an_honest_no_op_and_opens_no_session() {
    let fixture = fleet();
    let environment = transport_env(&fixture, Some(DECK_IDENTITY));

    let run = fixture
        .run_env(
            &["upload", "deck", "--to", "10.0.0.5"],
            None,
            &borrowed(&environment),
        )
        .expect_success("a matching probe");
    run.says("deck already holds its declared identity");
    run.says("nothing was written");

    // 3.6: the severe assertion is that no write-capable session was ever
    // opened, not merely that no file changed — a bug that opened one and
    // wrote nothing by coincidence would pass a weaker check.
    assert_eq!(
        fixture.transport_invocations(),
        vec!["ssh-keyscan".to_owned(), "ssh-to-age".to_owned()],
        "a matching probe opened a session it should not have"
    );
}

#[test]
fn force_is_inert_on_a_match() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let environment = transport_env(&fixture, Some(DECK_IDENTITY));

    let run = fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--force",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("--force on a match");
    run.says("nothing was written");
    run.says("--force applies only to a mismatch");
    assert_eq!(
        fixture.transport_invocations(),
        vec!["ssh-keyscan".to_owned(), "ssh-to-age".to_owned()],
        "--force on a match opened a session"
    );
}

#[test]
fn no_key_presented_writes_given_identity() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("writing to a target with no presented key");
    run.says("own next rebuild is what activates it");
    assert_eq!(
        fixture.transport_invocations(),
        vec![
            "ssh-keyscan".to_owned(),
            "ssh-keygen".to_owned(),
            "ssh-to-age".to_owned(),
            "ssh".to_owned(),
        ],
    );

    let argv = fixture.transport_recorded("ssh-argv");
    assert!(argv.contains("root@10.0.0.5"), "argv: {argv}");
    assert!(argv.contains("BatchMode=yes"), "argv: {argv}");
    assert!(
        argv.contains("install -d -m 0700 /mnt/etc/ssh"),
        "argv: {argv}"
    );
    assert!(
        argv.contains("find /mnt/etc/ssh -mindepth 1 -delete"),
        "argv: {argv}"
    );
    assert!(argv.contains("tar -xzf - -C /mnt/etc/ssh"), "argv: {argv}");
}

#[test]
fn no_key_presented_without_identity_refuses_before_opening_a_session() {
    let fixture = fleet();
    let environment = transport_env(&fixture, None);

    let run = fixture
        .run_env(
            &["upload", "deck", "--to", "10.0.0.5"],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("no key presented and no --identity");
    run.says("--identity is required to write a host identity; nothing was written");
    assert_eq!(
        fixture.transport_invocations(),
        vec!["ssh-keyscan".to_owned()],
        "a refused run opened a session"
    );
}

#[test]
fn a_different_presented_key_is_refused_by_default() {
    let fixture = fleet();
    let environment = transport_env(&fixture, Some("CANARY-unrelated-key"));

    let run = fixture
        .run_env(
            &["upload", "deck", "--to", "10.0.0.5"],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a mismatched presented key without --force");
    run.says("deck already presents an ed25519 host key");
    run.says("age1CANARY-unrelated-key");
    run.says(DECK_RECIPIENT);
    assert_eq!(
        fixture.transport_invocations(),
        vec!["ssh-keyscan".to_owned(), "ssh-to-age".to_owned()],
        "a refused mismatch opened a session"
    );
}

#[test]
fn a_mismatched_presented_key_is_overridden_with_force_and_identity() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let environment = transport_env(&fixture, Some("CANARY-unrelated-key"));

    let run = fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--force",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("a mismatch overridden with --force");
    run.says("a changed host key was overridden rather than discovered absent");
    assert!(fixture.transport_invocations().contains(&"ssh".to_owned()));
}

/// 3.7: flipping one byte of the declared recipient turns the match branch
/// this fixture drove in the first test of this group into the mismatch
/// branch, proving the branch is driven by the comparison and not by a
/// fixture-specific shortcut.
#[test]
fn flipping_the_declared_recipient_turns_a_match_into_a_mismatch() {
    let mut fixture = fleet();
    fixture.declare_machine("deck", Some("age1CANARY-deck-keX"));
    let environment = transport_env(&fixture, Some(DECK_IDENTITY));

    let run = fixture
        .run_env(
            &["upload", "deck", "--to", "10.0.0.5"],
            None,
            &borrowed(&environment),
        )
        .expect_refusal("a declared recipient flipped by one byte");
    run.says("deck already presents an ed25519 host key");
    run.says(DECK_RECIPIENT);
    run.says("age1CANARY-deck-keX");
}

// ── 4. the transport ────────────────────────────────────────────────────

#[test]
fn the_tarball_carries_the_declared_modes_and_root_ownership() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let environment = transport_env(&fixture, None);

    fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("a write that builds a tarball");

    let tarball = fixture.transport_spool().join("ssh-stdin.tar.gz");
    assert!(tarball.exists(), "the ssh stub recorded no tarball");
    let listing = std::process::Command::new("tar")
        .arg("-tvzf")
        .arg(&tarball)
        .output()
        .expect("tar could not list the archive");
    assert!(listing.status.success(), "{listing:?}");
    let text = String::from_utf8_lossy(&listing.stdout);

    // 4.2: files at 0400, owned by root; 4.1: both staged files are present.
    for member in ["ssh_host_ed25519_key", "ssh_host_ed25519_key.pub"] {
        let line = text
            .lines()
            .find(|line| line.ends_with(member))
            .unwrap_or_else(|| panic!("the archive does not carry {member}:\n{text}"));
        assert!(
            line.starts_with("-r--------"),
            "{member} is not mode 0400 in the archive: {line}"
        );
        assert!(
            line.contains("0/0"),
            "{member} is not owned by root (0/0) in the archive: {line}"
        );
    }
}

/// 4.5: the staging root is created before the tarball is written and
/// removed after the transfer, on the success path and on a simulated
/// transport failure alike.
#[test]
fn the_staging_root_is_gone_after_a_success_and_after_a_simulated_failure() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let environment = transport_env(&fixture, None);

    fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("a successful transport");
    assert_eq!(
        fixture.staging_roots(),
        Vec::<PathBuf>::new(),
        "a staging root survived a successful transport"
    );

    let mut failing = transport_env(&fixture, None);
    failing.push((
        "SAFIX_TRANSPORT_STUB_SSH_REFUSES".to_owned(),
        "1".to_owned(),
    ));
    let identity = identity_file(&fixture, "id2", DECK_IDENTITY);
    fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&failing),
        )
        .expect_refusal("a simulated transport failure");
    assert_eq!(
        fixture.staging_roots(),
        Vec::<PathBuf>::new(),
        "a staging root survived a simulated transport failure"
    );
}

/// 4.6: the fixed destination clears the depth safety, and
/// `upload::tests::a_shallow_destination_fails_the_depth_safety` in
/// `crates/safix-core/src/upload.rs` is where the check is shown to be live
/// rather than trivially satisfied — the unit that owns the constant is
/// where a drill against it belongs.
#[test]
fn the_wipe_then_extract_sequence_names_the_fixed_destination() {
    let fixture = fleet();
    let identity = identity_file(&fixture, "id", DECK_IDENTITY);
    let environment = transport_env(&fixture, None);

    fixture
        .run_env(
            &[
                "upload",
                "deck",
                "--to",
                "10.0.0.5",
                "--identity",
                &identity.to_string_lossy(),
            ],
            None,
            &borrowed(&environment),
        )
        .expect_success("a write reaching the transport");
    let argv = fixture.transport_recorded("ssh-argv");
    assert!(argv.contains("/mnt/etc/ssh"));
}

// ── 6. help text ────────────────────────────────────────────────────────

#[test]
fn safix_help_lists_upload_in_table_order_after_group() {
    let fixture = fleet();
    let run = fixture.run(&["-h"]);
    insta::assert_snapshot!("safix_help", run.combined());
}

#[test]
fn safix_upload_help_states_the_two_modes_and_the_three_absences() {
    let fixture = fleet();
    let run = fixture.run(&["upload", "-h"]);
    run.says("A machine name is all this verb");
    run.says("A systemd-credentials delivery path for the same material.");
    run.says("next rebuild is what activates what was written here");
    insta::assert_snapshot!("safix_upload_help", run.combined());
}
