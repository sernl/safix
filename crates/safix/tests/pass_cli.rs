//! The store's own command, driven for real against a store made for it.
//!
//! `pass_path.rs` drives the stubbed store, and `tests/support/pass-stub.rs`
//! states what that can and cannot establish. What it cannot establish is that
//! the argument vectors mean to `pass` what this runtime thinks they mean: the
//! stub answers the vectors safix sends because it was written to, and would go
//! on answering them after the command changed its options, its reading
//! convention or its wording. This target is the other half of that sentence,
//! and it is the one place in this repository where a real far side of a sync
//! mapping is driven at all — `pass` is free-licensed, needs no network and
//! needs no account, which is what makes it possible here and impossible for
//! the other two targets this programme adds.
//!
//! # The store is created here and nowhere else
//!
//! Every claim below is made against a store this test creates with `pass
//! init`, against a key it mints in a `GNUPGHOME` of its own, inside its own
//! temporary directory, held for the length of one test and removed. That is a
//! safety property rather than tidiness: the machines this suite is developed
//! on have the operator's own store and a gpg-agent that would answer for it,
//! and `pass insert --force` asks nothing before it replaces an entry.
//!
//! [`Scratch`] is the structural guard. It is the only way this file names a
//! store, its path is always under a directory this process made, its
//! `GNUPGHOME` is always inside that directory, and every invocation goes
//! through it — so a store of the operator's cannot be named by forgetting
//! something. `harness::refuse_a_real_store` is the same discipline one level
//! up, for the runs that drive the command through safix.
//!
//! # What this covers
//!
//! Exactly the behaviours `crates/safix-core/src/pass.rs` says it measured,
//! each against the real command at the version the machine has, one assertion
//! per behaviour:
//!
//! - `insert -m -f` reads the body to EOF and stores it.
//! - `show` prints that body back byte for byte, a trailing newline included,
//!   which is the measured answer to `design.md`'s open question and the reason
//!   no trailing-byte removal is applied to this transport.
//! - A multi-line value round-trips.
//! - A value plus a field block round-trips through the layout the runtime
//!   writes and parses.
//! - `show` on an absent entry exits non-zero, which is why absence is answered
//!   from the name listing rather than from a status.
//! - A `.gpg` file appears under the store root at the declared path, which is
//!   the listing's own coupling.
//! - `insert` on an existing entry replaces it under `--force` without a
//!   prompt.
//!
//! Every invocation is built through [`Scratch::insert`] or
//! [`Scratch::show`], which take the body on standard input, so no assertion
//! here can accidentally put a value in an argument vector.
//!
//! Absent rather than trivially green where the command is not installed: a
//! target that quietly did nothing is how a claim stops being made without
//! anybody deciding to stop making it.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// A memory-backed filesystem when the machine has the conventional one, and
/// the ordinary temporary directory otherwise.
fn scratch_root() -> PathBuf {
    let shared = PathBuf::from("/dev/shm");
    if shared.is_dir() {
        return shared;
    }
    std::env::temp_dir()
}

/// Whether the store's own command and the one it shells to are both on this
/// machine.
///
/// Both, because a store is a tree of gpg files: a machine with `pass` and no
/// `gpg` cannot make any claim below, and a test that minted a key against a
/// missing binary would fail on the machine rather than on safix.
/// `checks.safix-pass-cli` supplies both through `runOneWith`.
fn installed() -> bool {
    ["pass", "gpg"].into_iter().all(|program| {
        Command::new(program)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

/// One store, in a directory this process made, with a key of its own.
///
/// The `GNUPGHOME` is inside the same directory, so the key this test mints is
/// never in the operator's own keyring and disappears with the store it was
/// made for. `Drop` removes the whole tree, including on a panic, which is what
/// keeps a failed assertion from leaving a key behind.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    /// A store, initialised against a key minted for it.
    fn new(label: &str) -> Self {
        let root = scratch_root().join(format!("safix-pass-cli-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("store")).unwrap();
        std::fs::create_dir_all(root.join("gnupg")).unwrap();
        // gpg refuses a home directory anybody else can read.
        let scratch = Self { root };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(scratch.gnupg(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }

        let minted = Command::new("gpg")
            .args([
                "--batch",
                "--passphrase",
                "",
                "--quick-generate-key",
                "safix fixture <fixture@example.invalid>",
                "default",
                "default",
                "never",
            ])
            .env("GNUPGHOME", scratch.gnupg())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            minted.status.success(),
            "the fixture key was not minted: {}",
            String::from_utf8_lossy(&minted.stderr)
        );

        let initialised = Command::new("pass")
            .args(["init", "fixture@example.invalid"])
            .env("GNUPGHOME", scratch.gnupg())
            .env("PASSWORD_STORE_DIR", scratch.store())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            initialised.status.success(),
            "the fixture store was not initialised: {}",
            String::from_utf8_lossy(&initialised.stderr)
        );
        scratch
    }

    fn gnupg(&self) -> PathBuf {
        self.root.join("gnupg")
    }

    fn store(&self) -> PathBuf {
        self.root.join("store")
    }

    /// One invocation, with the body on standard input and never in argv.
    ///
    /// The one helper every command in this file is built through, which is
    /// what makes "no invocation this file makes carries a value in its
    /// argument vector" a property of the file's shape rather than of a
    /// reviewer's attention: `arguments` is `&[&str]` of literals, and `body`
    /// is the only path a value has.
    fn run(&self, arguments: &[&str], body: Option<&[u8]>) -> Output {
        for word in arguments {
            assert!(
                !word.contains("hunter2") && !word.contains("BEGIN KEY"),
                "the argument vector {word:?} carries a value"
            );
        }
        let mut child = Command::new("pass")
            .args(arguments)
            .env("GNUPGHOME", self.gnupg())
            .env("PASSWORD_STORE_DIR", self.store())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            if let Some(body) = body {
                stdin.write_all(body).unwrap();
            }
        }
        child.wait_with_output().unwrap()
    }

    /// The exact vector the runtime's write builds, with the body on a pipe.
    fn insert(&self, path: &str, body: &[u8]) -> Output {
        self.run(&["insert", "--multiline", "--force", path], Some(body))
    }

    /// The exact vector the runtime's read builds.
    fn show(&self, path: &str) -> Output {
        self.run(&["show", path], None)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = Command::new("gpgconf")
            .args(["--kill", "all"])
            .env("GNUPGHOME", self.gnupg())
            .status();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Print the disclaimer and say whether to go on.
fn present(what: &str) -> bool {
    if installed() {
        return true;
    }
    println!("pass is not installed; {what} asserts nothing on this machine");
    false
}

/// `insert -m -f` reads to EOF and `show` prints it back byte for byte, a
/// trailing newline included.
///
/// The measured answer to the open question `design.md` left: `--multiline`
/// preserves a body's final newline. Both shapes are asserted, because a
/// command that normalised would make exactly one of them fail.
#[test]
fn a_body_round_trips_byte_for_byte_with_and_without_a_trailing_newline() {
    if !present("the byte-exactness assertion") {
        return;
    }
    let scratch = Scratch::new("bytes");

    for (path, body) in [
        ("alice/bare", "hunter2".as_bytes()),
        ("alice/terminated", "hunter2\n".as_bytes()),
    ] {
        let written = scratch.insert(path, body);
        assert!(
            written.status.success(),
            "pass insert refused: {}",
            String::from_utf8_lossy(&written.stderr)
        );
        let read = scratch.show(path);
        assert!(
            read.status.success(),
            "pass show refused: {}",
            String::from_utf8_lossy(&read.stderr)
        );
        assert_eq!(
            read.stdout, body,
            "the store did not return {path}'s body byte for byte"
        );
    }
}

/// A value spanning lines survives the write whole.
#[test]
fn a_multi_line_value_round_trips() {
    if !present("the multi-line assertion") {
        return;
    }
    let scratch = Scratch::new("multiline");
    let body = b"-----BEGIN KEY-----\nline two\nline three\n";

    assert!(scratch.insert("alice/key", body).status.success());
    assert_eq!(
        scratch.show("alice/key").stdout,
        body,
        "a multi-line body did not survive the round trip"
    );
}

/// A value plus a field block round-trips through the layout the runtime
/// writes.
///
/// The body is built here as a literal rather than through `pass.rs`, and what
/// is asserted is that the store returns exactly those bytes — so the parse the
/// runtime performs is over a body the real tool actually stored.
#[test]
fn a_value_plus_a_field_block_round_trips() {
    if !present("the layout assertion") {
        return;
    }
    let scratch = Scratch::new("layout");
    let body = "hunter2\n\
                \n\
                login: alice@example.invalid\n\
                url: https://grafana.example.invalid\n\
                notes: minted by safix\n\
                tags: work, fleet\n";

    assert!(
        scratch
            .insert("alice/grafana", body.as_bytes())
            .status
            .success()
    );
    let read = scratch.show("alice/grafana");
    assert_eq!(
        String::from_utf8_lossy(&read.stdout),
        body,
        "the field block did not survive the round trip"
    );

    // The first line is still the value, which is what keeps `pass -c` and the
    // ecosystem's own field readers working over a record safix wrote.
    assert_eq!(
        String::from_utf8_lossy(&read.stdout).lines().next(),
        Some("hunter2")
    );
}

/// `show` on an absent entry exits non-zero, which is why absence is answered
/// from the name listing.
#[test]
fn show_on_an_absent_entry_exits_non_zero() {
    if !present("the absence assertion") {
        return;
    }
    let scratch = Scratch::new("absent");
    let read = scratch.show("alice/never-written");
    assert!(
        !read.status.success(),
        "pass show accepted an entry the store does not hold"
    );
}

/// A `.gpg` file appears under the store root at the declared path.
///
/// The listing's own coupling, measured: the runtime enumerates exactly these
/// names, and this is the assertion that the tool produces them where the
/// enumeration looks.
#[test]
fn a_gpg_file_appears_at_the_declared_path() {
    if !present("the listing assertion") {
        return;
    }
    let scratch = Scratch::new("listing");
    assert!(scratch.insert("alice/listed", b"hunter2").status.success());

    let file = scratch.store().join("alice/listed.gpg");
    assert!(
        file.is_file(),
        "no .gpg file appeared at {}",
        file.display()
    );
    assert!(
        scratch.store().join(".gpg-id").is_file(),
        "pass init wrote no .gpg-id, which is what the preflight looks for"
    );
}

/// `insert` on an existing entry replaces it under `--force`, without a prompt.
///
/// Standard input carries the new body and nothing else — no answer to a
/// question — so a command that asked one would hang or refuse rather than
/// succeed.
#[test]
fn insert_replaces_an_existing_entry_under_force_without_a_prompt() {
    if !present("the overwrite assertion") {
        return;
    }
    let scratch = Scratch::new("replace");
    assert!(scratch.insert("alice/twice", b"hunter2").status.success());

    let again = scratch.insert("alice/twice", b"hunter2-second");
    assert!(
        again.status.success(),
        "pass insert --force refused an existing entry: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        scratch.show("alice/twice").stdout,
        b"hunter2-second",
        "the second insert did not replace the entry"
    );
}
