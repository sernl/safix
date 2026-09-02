//! The store's own command, driven for real against a database made for it.
//!
//! `sync_path.rs` drives the modelled store, and `tests/support/card-stubs.rs`
//! states what that can and cannot establish. What it cannot establish is that
//! the argument vectors mean to `keepassxc-cli` what this runtime thinks they
//! mean: the model answers the vectors safix sends because it was written to, and
//! would go on answering them after the command changed its options, its output
//! convention or its wording. This target is the other half of that sentence.
//!
//! # The database is created here and nowhere else
//!
//! Every claim below is made against a database this test creates with
//! `keepassxc-cli db-create` inside its own temporary directory, holds for the
//! length of one test, and removes. That is a safety property rather than
//! tidiness: the machines this suite is developed on have the operator's own
//! database, which is the fleet's root of trust, and a run that reached it would
//! edit entries nobody asked it to.
//!
//! [`Scratch`] is the structural guard. It is the only way this file names a
//! database, its path is always under a directory this process made, and every
//! invocation goes through it — so a database of the operator's cannot be named
//! by forgetting something. `harness::refuse_a_real_database` is the same
//! discipline one level up, for the runs that drive the command through safix.
//!
//! # What this covers
//!
//! Exactly the behaviours `crates/safix-core/src/store.rs` says it measured, each
//! against the real command at the version the machine has:
//!
//! - `db-create -p` takes the new password twice on standard input.
//! - A group must exist before an entry can be added under it, and `mkdir`
//!   creates one level.
//! - `add -p` takes the database password and then the entry's, one line each,
//!   and `edit -p` replaces the value of an entry that is there.
//! - `show -s -a Password` prints the value with a newline of its own appended,
//!   which is the byte the runtime removes.
//! - `ls -R -f` lists groups with a trailing slash and entries without one, which
//!   is what absence is answered from.
//! - The value round-trips byte for byte, spaces and non-ASCII included.
//! - A wrong database password and an absent entry both exit non-zero, which is
//!   why absence is read from the listing rather than from a status.
//! - An entry password carrying a newline does not survive the command, which is
//!   the measurement the runtime's refusal of such a value exists for.
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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The password the scratch database is created with.
const UNLOCK: &str = "scratch-database-password";

/// A database this process created, in a directory this process made.
///
/// The one way this file names a database. `new` returns `None` where the command
/// is not installed, so the absence is a statement rather than a failure, and
/// `Drop` removes the directory however a test ends.
struct Scratch {
    directory: PathBuf,
}

impl Scratch {
    /// A fresh database, or nothing when the store's own command is not here.
    fn new(name: &str) -> Option<Self> {
        if !installed() {
            return None;
        }
        let directory =
            scratch_root().join(format!("safix-store-cli-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the scratch directory does not open");
        let scratch = Self { directory };

        // Created rather than opened, which is the whole safety property: the path
        // did not exist a moment ago and nothing but this test writes to it.
        let created = scratch.run(
            &["db-create", "--quiet", "--set-password", "DATABASE"],
            &format!("{UNLOCK}\n{UNLOCK}\n"),
        );
        assert!(
            created.status,
            "keepassxc-cli would not create a database: {}",
            created.stderr
        );
        assert!(scratch.database().is_file(), "no database was created");
        Some(scratch)
    }

    /// The database every invocation names.
    fn database(&self) -> PathBuf {
        self.directory.join("scratch.kdbx")
    }

    /// One invocation of the command, with the database's own path substituted for
    /// the placeholder.
    ///
    /// The substitution is what makes the guard structural: a caller writes
    /// `DATABASE` and cannot write a path of its own, so no invocation in this
    /// file can name a database this process did not create.
    fn run(&self, arguments: &[&str], fed: &str) -> Finished {
        let database = self.database().display().to_string();
        let arguments: Vec<String> = arguments
            .iter()
            .map(|word| {
                if *word == "DATABASE" {
                    database.clone()
                } else {
                    (*word).to_owned()
                }
            })
            .collect();
        let names_another_database = |word: &String| {
            word.rsplit_once('.')
                .is_some_and(|(_, tail)| tail == "kdbx")
                && *word != database
        };
        assert!(
            !arguments.iter().any(names_another_database),
            "an invocation named a database this test did not create: {arguments:?}"
        );

        let mut child = Command::new("keepassxc-cli")
            .args(&arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not run keepassxc-cli");
        child
            .stdin
            .take()
            .expect("the command was started without a pipe")
            .write_all(fed.as_bytes())
            .expect("could not feed the command");
        let finished = child
            .wait_with_output()
            .expect("the command did not finish");
        Finished {
            status: finished.status.success(),
            stdout: finished.stdout,
            stderr: String::from_utf8_lossy(&finished.stderr).into_owned(),
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct Finished {
    status: bool,
    stdout: Vec<u8>,
    stderr: String,
}

/// Where a scratch database goes.
///
/// A memory-backed filesystem when the machine has the conventional one, and the
/// ordinary temporary directory otherwise. The database here holds fixture
/// strings rather than anybody's secrets, and the preference is still worth
/// stating: the rest of this suite stages plaintext on tmpfs, and a file removed
/// on drop is not the same as a file that never reached a disk block.
fn scratch_root() -> PathBuf {
    let shared = PathBuf::from("/dev/shm");
    if shared.is_dir() {
        return shared;
    }
    std::env::temp_dir()
}

/// Whether the store's own command is on this machine.
fn installed() -> bool {
    Command::new("keepassxc-cli")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The argument vectors the runtime builds, as this test drives them.
///
/// Taken from `safix_core::store` rather than written out, so a change to what
/// the runtime sends is a change to what this asserts. That is the whole point of
/// the target: the model answers what it was told to answer, and the real command
/// answers what it understands.
fn vectors(entry: &str) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let database = Path::new("DATABASE");
    (
        safix_core::store::listing_arguments(database, None, None),
        safix_core::store::group_arguments(database, "safix", None, None),
        safix_core::store::write_arguments(
            database,
            entry,
            Some("alice@example.com"),
            false,
            None,
            None,
        ),
        safix_core::store::read_arguments(database, entry, None, None),
    )
}

fn words(arguments: &[String]) -> Vec<&str> {
    arguments.iter().map(String::as_str).collect()
}

/// The vectors the runtime sends are the ones the real command understands, and a
/// value comes back exactly as it went in.
#[test]
fn the_runtimes_own_vectors_round_trip_a_value_through_a_real_database() {
    let Some(scratch) = Scratch::new("round-trip") else {
        eprintln!(
            "keepassxc-cli is not installed here, so nothing was established about the \
             real command. The delegation itself is held everywhere by `safix-sync` \
             against the model."
        );
        return;
    };
    let entry = "safix/alice/grafana";
    let (listing, group, write, read) = vectors(entry);

    // A database holding no entry lists one line, and it is a marker rather than
    // an entry. The runtime skips it, and this is the measurement that says it
    // has to: without the skip a fresh database reads as holding an entry called
    // `[empty]`.
    let empty = scratch.run(&words(&listing), &format!("{UNLOCK}\n"));
    assert!(empty.status, "the listing refused: {}", empty.stderr);
    assert_eq!(
        String::from_utf8_lossy(&empty.stdout).trim(),
        "[empty]",
        "the listing of a database holding nothing is not the marker the runtime skips"
    );

    // A group must exist before an entry can be added under it, and `mkdir`
    // creates one level: the nested group is refused until its parent is there.
    let nested =
        safix_core::store::group_arguments(Path::new("DATABASE"), "safix/alice", None, None);
    let too_deep = scratch.run(&words(&nested), &format!("{UNLOCK}\n"));
    assert!(
        !too_deep.status,
        "mkdir created a group whose parent does not exist"
    );
    assert!(scratch.run(&words(&group), &format!("{UNLOCK}\n")).status);
    assert!(scratch.run(&words(&nested), &format!("{UNLOCK}\n")).status);
    // An existing group is refused too, which is why the runtime reads the groups
    // it has rather than creating them blindly.
    assert!(!scratch.run(&words(&group), &format!("{UNLOCK}\n")).status);

    // The value, with a leading space, a trailing space and a non-ASCII byte, so
    // that "byte for byte" is a claim rather than a word.
    let value = " a value with spaces and \u{fc} ";
    let added = scratch.run(&words(&write), &format!("{UNLOCK}\n{value}"));
    assert!(added.status, "add refused: {}", added.stderr);

    // The listing now carries the two groups with a trailing slash and the entry
    // without one, which is exactly what `Database::open` reads.
    let listed = scratch.run(&words(&listing), &format!("{UNLOCK}\n"));
    let lines: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    for wanted in ["safix/", "safix/alice/", entry] {
        assert!(
            lines.iter().any(|line| line == wanted),
            "the listing has no {wanted} in {lines:?}"
        );
    }

    // The read prints the value followed by a newline of its own. Removing exactly
    // one is what the runtime does, and this is the measurement it rests on.
    let shown = scratch.run(&words(&read), &format!("{UNLOCK}\n"));
    assert!(shown.status, "show refused: {}", shown.stderr);
    assert_eq!(
        shown.stdout,
        format!("{value}\n").into_bytes(),
        "the value did not come back with exactly one newline appended"
    );

    // The username reached the entry, which is the one field beyond the value a
    // mapping may set.
    let summary = scratch.run(
        &["show", "--quiet", "--show-protected", "DATABASE", entry],
        &format!("{UNLOCK}\n"),
    );
    assert!(
        String::from_utf8_lossy(&summary.stdout).contains("UserName: alice@example.com"),
        "the username did not reach the entry: {}",
        String::from_utf8_lossy(&summary.stdout)
    );

    // An edit replaces the value of an entry that is there, and an add over one
    // refuses — which is why the runtime chooses between them by the listing.
    let again = scratch.run(&words(&write), &format!("{UNLOCK}\nsomething-else"));
    assert!(!again.status, "add created an entry that already exists");
    let edit =
        safix_core::store::write_arguments(Path::new("DATABASE"), entry, None, true, None, None);
    assert!(
        scratch
            .run(&words(&edit), &format!("{UNLOCK}\nthe-second-value"))
            .status
    );
    let second = scratch.run(&words(&read), &format!("{UNLOCK}\n"));
    assert_eq!(second.stdout, b"the-second-value\n".to_vec());
}

/// The two failures that look alike, and the one a value cannot survive.
#[test]
fn a_wrong_password_and_an_absent_entry_are_both_refusals_and_a_newline_is_lost() {
    let Some(scratch) = Scratch::new("refusals") else {
        eprintln!(
            "keepassxc-cli is not installed here, so nothing was established about the \
             real command's refusals."
        );
        return;
    };
    let entry = "safix/alice/grafana";
    let (_, group, write, read) = vectors(entry);
    let nested =
        safix_core::store::group_arguments(Path::new("DATABASE"), "safix/alice", None, None);
    assert!(scratch.run(&words(&group), &format!("{UNLOCK}\n")).status);
    assert!(scratch.run(&words(&nested), &format!("{UNLOCK}\n")).status);

    // An absent entry and a wrong password both exit non-zero, which is the
    // measurement behind reading absence out of the listing instead.
    let absent = scratch.run(&words(&read), &format!("{UNLOCK}\n"));
    assert!(!absent.status, "show found an entry that is not there");

    assert!(
        scratch
            .run(&words(&write), &format!("{UNLOCK}\nvalue"))
            .status
    );
    let wrong = scratch.run(&words(&read), "a-different-password\n");
    assert!(
        !wrong.status,
        "show opened the database with a wrong password"
    );

    // A value carrying a newline does not survive: what lands is the bytes before
    // it. This is the measurement the runtime's refusal exists for — the refusal
    // is asserted in `sync_path.rs`, and what is asserted here is that the
    // constraint it defends is real.
    let edit =
        safix_core::store::write_arguments(Path::new("DATABASE"), entry, None, true, None, None);
    assert!(
        scratch
            .run(&words(&edit), &format!("{UNLOCK}\nfirst\nsecond"))
            .status
    );
    let shown = scratch.run(&words(&read), &format!("{UNLOCK}\n"));
    assert_eq!(
        shown.stdout,
        b"first\n".to_vec(),
        "a value carrying a newline survived the command, so the refusal defends nothing"
    );
}

/// A fixed password, for the one test here that opens the database through
/// [`safix_core::store::Database`] rather than by driving the command's own
/// argument vectors directly.
struct FixedPassword;

impl safix_core::enroll::custody::DatabasePassword for FixedPassword {
    fn database_password(&mut self, _database: &Path) -> safix_core::Result<safix_core::Secret> {
        safix_core::Secret::read_from(&mut format!("{UNLOCK}\n").as_bytes())
    }
}

/// `audit`'s keepassxc target opens the database and reads each mapping's
/// entry, and never writes: `Database::open` and `Database::read` are its own
/// primitives, exercised here against the real command rather than the model
/// `sync_path.rs` drives, reading an entry that holds a value and one that
/// does not.
#[test]
fn database_open_and_read_answer_a_compare_only_pass_against_a_real_database() {
    let Some(scratch) = Scratch::new("audit-read-only") else {
        eprintln!(
            "keepassxc-cli is not installed here, so nothing was established about \
             audit's read-only pass against the real command."
        );
        return;
    };
    let entry = "safix/alice/grafana";
    let (_, group, write, _) = vectors(entry);
    let nested = safix_core::store::group_arguments(Path::new("DATABASE"), "safix/alice");
    assert!(scratch.run(&words(&group), &format!("{UNLOCK}\n")).status);
    assert!(scratch.run(&words(&nested), &format!("{UNLOCK}\n")).status);
    let value = "CANARY-real-database-value";
    assert!(
        scratch
            .run(&words(&write), &format!("{UNLOCK}\n{value}"))
            .status
    );

    let database = safix_core::store::Database::open(scratch.database(), &mut FixedPassword)
        .expect("Database::open refused a database this test just created");

    let held = database
        .read(entry)
        .expect("Database::read refused an entry this test just wrote");
    let expected = safix_core::Secret::read_from(&mut value.as_bytes()).expect("a fixture value");
    assert!(
        held.is_some_and(|secret| secret.equals(&expected)),
        "the value read back through Database::read is not what audit would compare against"
    );

    let absent = database
        .read("safix/alice/nowhere")
        .expect("Database::read refused an entry the listing says is not there");
    assert!(
        absent.is_none(),
        "an entry the listing does not carry read back as present"
    );
}
