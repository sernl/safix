//! The password database, and the whole of how `sync` reaches it.
//!
//! One transport: the store's own command, with the database's password read
//! once per run and held for the length of it. Every value travels standard
//! input in and a pipe out; what travels an argument vector is the database
//! path, the entry path and a username, all of which are public strings the
//! command line takes.
//!
//! # Why the session's secret service is not a second transport here
//!
//! The Secret Service collection `KeePassXC` publishes *is* its exposed group. An
//! item found through the service is an entry in whatever group the operator's
//! exposure setting names, and an item created through it lands there — so the
//! service cannot address `<group>/<path>`, which is the thing a mapping
//! declares. Two transports addressing different entries would make a mapping's
//! convergence depend on which one ran; worse, a service read of an entry in a
//! group the operator has not exposed returns nothing, which is indistinguishable
//! from "the database holds no value here", and a `backup` mapping meeting that
//! answer would write a secret into a group no declaration named.
//!
//! [`crate::enroll::custody`] keeps both transports, and the asymmetry is not an
//! inconsistency: its entry is safix's own, addressed by an attribute rather than
//! by a path, and the exposed group is the right home for it. This module
//! addresses a group and a path the consumer chose.
//!
//! `openspec/changes/add-keepassxc-sync/design.md` records the decision as an
//! amendment made during apply, with the measurement behind it.
//!
//! # What the tool's own behaviour requires, measured rather than assumed
//!
//! Read out of `keepassxc-cli` 2.7.12 against scratch databases:
//!
//! - Every database-opening command takes its password on standard input, and
//!   `-q` silences the prompt without changing the read.
//! - A group must exist before an entry can be added under it, and `mkdir`
//!   creates one level: `mkdir <db> a/b` on a database with no `a` refuses.
//!   Creating an existing group refuses too, so the groups that exist are read
//!   rather than guessed at.
//! - `ls -R -f` lists every group with a trailing `/` and every entry without
//!   one, which is what [`Database::open`] reads: opening the database once and
//!   then answering "is this entry there" out of that listing is what keeps
//!   absence apart from a database that would not open, since both exit non-zero
//!   and neither status distinguishes them.
//! - `show -s -a Password` prints the value followed by a newline of its own,
//!   which [`Database::read`] removes. Removing exactly one is exact whatever the
//!   entry holds, including a multi-line value some other tool wrote: the added
//!   byte is the last one either way.
//! - `add --password-prompt` and `edit --password-prompt` read the entry's value
//!   as one line, so a value carrying a newline cannot be written through them —
//!   what would land is the bytes before the first newline. The caller refuses
//!   such a value rather than writing part of it; see [`Database::write`].

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::enroll::custody::{DatabasePassword, keepassxc_cli};
use crate::error::{Error, Result};
use crate::model::{Keepassxc, Yubikey};
use crate::secret::Secret;

/// The suffix safix reserves for the entry a two-way mapping records its last
/// agreement in.
///
/// The same string `modules/flake/safix/keepassxc.nix` reserves, and the
/// evaluation refusal there is what keeps a declaration from naming one: the
/// companion of a declared path is that path plus this suffix, and a declared
/// path carrying the suffix is refused, so the two name spaces cannot overlap.
pub const STATE_SUFFIX: &str = ".safix-sync-state";

/// What the store's own listing prints instead of nothing when it has nothing to
/// list.
///
/// Measured rather than assumed: `ls -R -f` over a database holding no entry
/// prints this one line. Without skipping it a fresh database would be read as
/// holding one entry with that name, and while nothing would then be written to
/// it, "the database holds this" would be false — which is the sort of thing the
/// rest of this module is built not to guess about.
const EMPTY_LISTING: &str = "[empty]";

/// The entry a mapping's last agreement is recorded in, beside the entry itself.
#[must_use]
pub fn companion_of(entry: &str) -> String {
    format!("{entry}{STATE_SUFFIX}")
}

/// Whether this entry path is one safix reserves rather than one a mapping may
/// name.
#[must_use]
pub fn is_companion(entry: &str) -> bool {
    entry.ends_with(STATE_SUFFIX)
}

/// One database, open for the length of one run.
///
/// No `Debug`: the password is held here, and deriving one would print it
/// through the field.
pub struct Database {
    program: PathBuf,
    path: PathBuf,
    key: Secret,
    /// A `YubiKey` challenge-response slot the database requires to open, or
    /// none when the database opens on its password alone.
    yubikey: Option<Yubikey>,
    /// A key file the database requires to open, or none when the database
    /// opens on its password alone.
    key_file: Option<String>,
    /// Every entry the database holds, as `ls -R -f` listed them, updated by
    /// every write this run makes.
    entries: BTreeSet<String>,
    /// Every group it holds, without the trailing slash the listing carries.
    groups: BTreeSet<String>,
}

impl Database {
    /// Open one database, having asked for its password once.
    ///
    /// The password is read before anything else happens and the listing that
    /// follows is what establishes that it opened, so a run whose password is
    /// wrong refuses here rather than reporting every mapping as unjudgeable.
    ///
    /// The composite-key factors, if any, come from the `Keepassxc` the caller
    /// already has: the same declaration that names this database also names
    /// what else it takes to open.
    ///
    /// # Errors
    ///
    /// [`Error::StoreUnavailable`] when the command cannot be run,
    /// [`Error::DatabaseUnreadable`] when it runs and will not open the
    /// database, and whatever reading the password failed with.
    pub fn open(
        path: PathBuf,
        mirror: &Keepassxc,
        password: &mut dyn DatabasePassword,
    ) -> Result<Self> {
        let key = password.database_password(&path)?;
        let mut database = Self {
            program: keepassxc_cli(),
            path,
            key,
            yubikey: mirror.yubikey.clone(),
            key_file: mirror.key_file.clone(),
            entries: BTreeSet::new(),
            groups: BTreeSet::new(),
        };
        let listing = database.listing()?;
        for line in listing.lines() {
            if line.is_empty() || line == EMPTY_LISTING {
                continue;
            }
            match line.strip_suffix('/') {
                Some(group) => database.groups.insert(group.to_owned()),
                None => database.entries.insert(line.to_owned()),
            };
        }
        Ok(database)
    }

    /// The database this run is converging against, for a report that names it.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the database holds this entry.
    ///
    /// Answered from the listing taken when the database was opened rather than
    /// by asking about the entry, because `show` exits non-zero both for an entry
    /// that is not there and for a database that would not open, and reading the
    /// difference out of the tool's wording is a coupling this does not need.
    #[must_use]
    pub fn holds(&self, entry: &str) -> bool {
        self.entries.contains(entry)
    }

    /// Every entry under one group, in path order.
    ///
    /// What the report's information line about entries no mapping declares is
    /// computed from, companion entries included: a companion whose own entry is
    /// gone is exactly as much of a leftover as the entry would have been.
    pub fn under(&self, group: &str) -> impl Iterator<Item = &str> {
        let prefix = format!("{group}/");
        self.entries
            .iter()
            .filter(move |entry| entry.starts_with(&prefix))
            .map(String::as_str)
    }

    /// What the database holds for one entry, or nothing when it holds no such
    /// entry.
    ///
    /// # Errors
    ///
    /// [`Error::StoreUnavailable`] when the command cannot be run,
    /// [`Error::StorePipeMissing`] when it was started with a pipe that was not
    /// there, and [`Error::StoreCommandFailed`] carrying its own message when it
    /// refuses over an entry the listing says is there.
    pub fn read(&self, entry: &str) -> Result<Option<Secret>> {
        if !self.holds(entry) {
            return Ok(None);
        }
        let arguments = read_arguments(
            &self.path,
            entry,
            self.yubikey.as_ref(),
            self.key_file.as_deref(),
        );
        let mut child = self.spawn(&arguments)?;

        // The password first and the pipe closed with it, then the value on the
        // way back. The command reads one line before it prints anything, and the
        // password is short, so writing it before draining cannot fill a buffer
        // either side is waiting on.
        {
            let mut stdin = child.stdin.take().ok_or(Error::StorePipeMissing)?;
            self.key
                .write_to(&mut stdin)
                .and_then(|()| stdin.write_all(b"\n"))
                .and_then(|()| stdin.flush())
                .map_err(|cause| Error::SecretRead { cause })?;
        }

        let printed = {
            let mut stdout = child.stdout.take().ok_or(Error::StorePipeMissing)?;
            Secret::read_from(&mut stdout)?
        };

        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::StoreUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if !finished.status.success() {
            return Err(Error::StoreCommandFailed {
                entry: entry.to_owned(),
                arguments: arguments.join(" "),
                output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
            });
        }
        Ok(Some(printed.without_one_trailing_newline()))
    }

    /// Put one value in the database, creating the entry and its groups when
    /// they are absent.
    ///
    /// # Errors
    ///
    /// [`Error::ValueSpansLines`] when the value carries a newline the store's
    /// command cannot carry, [`Error::StoreUnavailable`] when the command cannot
    /// be run, [`Error::StorePipeMissing`] when its standard input was not there
    /// to write, and [`Error::StoreCommandFailed`] carrying its own message when
    /// it refuses.
    pub fn write(&mut self, entry: &str, value: &Secret, username: Option<&str>) -> Result<()> {
        if value.spans_lines() {
            return Err(Error::ValueSpansLines {
                entry: entry.to_owned(),
            });
        }
        let existing = self.holds(entry);
        if !existing {
            self.create_groups(entry)?;
        }
        let arguments = write_arguments(
            &self.path,
            entry,
            username,
            existing,
            self.yubikey.as_ref(),
            self.key_file.as_deref(),
        );
        self.feed(entry, &arguments, value)?;
        self.entries.insert(entry.to_owned());
        Ok(())
    }

    /// Every group a new entry's path needs, outermost first.
    ///
    /// One `mkdir` per level, because the command creates one level and refuses
    /// when the parent is absent, and only for the levels the listing does not
    /// already carry, because it refuses over a group that exists as well.
    fn create_groups(&mut self, entry: &str) -> Result<()> {
        let mut segments: Vec<&str> = entry.split('/').collect();
        segments.pop();

        let mut path = String::new();
        for segment in segments {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(segment);
            if self.groups.contains(&path) {
                continue;
            }
            let arguments = group_arguments(
                &self.path,
                &path,
                self.yubikey.as_ref(),
                self.key_file.as_deref(),
            );
            self.feed(&path, &arguments, &Secret::empty())?;
            self.groups.insert(path.clone());
        }
        Ok(())
    }

    /// The entries and groups the database holds, as text.
    fn listing(&self) -> Result<String> {
        let arguments =
            listing_arguments(&self.path, self.yubikey.as_ref(), self.key_file.as_deref());
        let mut child = self.spawn(&arguments)?;
        {
            let mut stdin = child.stdin.take().ok_or(Error::StorePipeMissing)?;
            self.key
                .write_to(&mut stdin)
                .and_then(|()| stdin.write_all(b"\n"))
                .and_then(|()| stdin.flush())
                .map_err(|cause| Error::SecretRead { cause })?;
        }
        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::StoreUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if !finished.status.success() {
            return Err(Error::DatabaseUnreadable {
                database: self.path.display().to_string(),
                output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
            });
        }
        Ok(String::from_utf8_lossy(&finished.stdout).into_owned())
    }

    /// One command, with the database's password and then the value on standard
    /// input.
    ///
    /// The newline separates the two answers and does not follow the second,
    /// because the second is a value rather than an answer and end of input is
    /// what ends it — the rule [`crate::enroll::custody`] states for the same
    /// tool. A command taking no value gets the password alone, which is what an
    /// empty value means here: `mkdir` reads one line and stops.
    fn feed(&self, subject: &str, arguments: &[String], value: &Secret) -> Result<()> {
        let mut child = self.spawn(arguments)?;
        {
            let mut stdin = child.stdin.take().ok_or(Error::StorePipeMissing)?;
            self.key
                .write_to(&mut stdin)
                .and_then(|()| stdin.write_all(b"\n"))
                .map_err(|cause| Error::SecretRead { cause })?;
            if !value.is_empty() {
                value
                    .write_to(&mut stdin)
                    .map_err(|cause| Error::SecretRead { cause })?;
            }
            stdin.flush().map_err(|cause| Error::SecretRead { cause })?;
        }
        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::StoreUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if finished.status.success() {
            return Ok(());
        }
        Err(Error::StoreCommandFailed {
            entry: subject.to_owned(),
            arguments: arguments.join(" "),
            output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
        })
    }

    /// One invocation, with all three streams captured.
    ///
    /// Standard error is captured rather than inherited because the command
    /// writes its password prompt there and a refusal's own words are carried
    /// into [`Error::StoreCommandFailed`]; standard output is captured because
    /// for `show` it is the value.
    fn spawn(&self, arguments: &[String]) -> Result<std::process::Child> {
        Command::new(&self.program)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|cause| Error::StoreUnavailable {
                program: self.program.display().to_string(),
                cause,
            })
    }
}

/// The argument-vector fragment that carries a database's composite-key
/// factors, spliced into every constructor below right after `--quiet`.
///
/// `slot[:serial]` is keepassxc-cli's own punctuation for `-y`, joined here
/// rather than asked of the declaration: a bare slot when no serial was
/// declared, `slot:serial` when one was. `pub(crate)` because
/// [`crate::enroll::custody`] opens the same database by a different route
/// and needs the identical splice — one definition beside the four
/// constructors that already know keepassxc-cli's exact flag spelling, rather
/// than a second copy free to drift from this one.
#[must_use]
pub(crate) fn composite_key_arguments(
    yubikey: Option<&Yubikey>,
    key_file: Option<&str>,
) -> Vec<String> {
    let mut arguments = Vec::new();
    if let Some(yubikey) = yubikey {
        let spec = match &yubikey.serial {
            Some(serial) => format!("{}:{serial}", yubikey.slot),
            None => yubikey.slot.clone(),
        };
        arguments.push("-y".to_owned());
        arguments.push(spec);
    }
    if let Some(key_file) = key_file {
        arguments.push("-k".to_owned());
        arguments.push(key_file.to_owned());
    }
    arguments
}

/// The argument vector one entry's value is read through.
///
/// `--show-protected` because the value is one, and `--attributes Password`
/// because the whole of what a comparison needs is the value: a summary would
/// print the title and the username beside it, which the report has no use for
/// and which would then be on this pipe.
#[must_use]
pub fn read_arguments(
    database: &Path,
    entry: &str,
    yubikey: Option<&Yubikey>,
    key_file: Option<&str>,
) -> Vec<String> {
    let mut arguments = vec!["show".to_owned(), "--quiet".to_owned()];
    arguments.extend(composite_key_arguments(yubikey, key_file));
    arguments.extend([
        "--show-protected".to_owned(),
        "--attributes".to_owned(),
        "Password".to_owned(),
        database.display().to_string(),
        entry.to_owned(),
    ]);
    arguments
}

/// The argument vector one entry's value is written through.
///
/// `--password-prompt` is what makes the value arrive on standard input instead
/// of in argv, which is the reason this transport is shaped the way it is.
/// `edit` for an entry that is there and `add` for one that is not, because each
/// refuses the other's case.
#[must_use]
pub fn write_arguments(
    database: &Path,
    entry: &str,
    username: Option<&str>,
    existing: bool,
    yubikey: Option<&Yubikey>,
    key_file: Option<&str>,
) -> Vec<String> {
    let mut arguments = vec![
        if existing { "edit" } else { "add" }.to_owned(),
        "--quiet".to_owned(),
    ];
    arguments.extend(composite_key_arguments(yubikey, key_file));
    arguments.push("--password-prompt".to_owned());
    if let Some(username) = username {
        arguments.push("--username".to_owned());
        arguments.push(username.to_owned());
    }
    arguments.push(database.display().to_string());
    arguments.push(entry.to_owned());
    arguments
}

/// The argument vector one group is created through.
#[must_use]
pub fn group_arguments(
    database: &Path,
    group: &str,
    yubikey: Option<&Yubikey>,
    key_file: Option<&str>,
) -> Vec<String> {
    let mut arguments = vec!["mkdir".to_owned(), "--quiet".to_owned()];
    arguments.extend(composite_key_arguments(yubikey, key_file));
    arguments.push(database.display().to_string());
    arguments.push(group.to_owned());
    arguments
}

/// The argument vector the database's own contents are listed through.
#[must_use]
pub fn listing_arguments(
    database: &Path,
    yubikey: Option<&Yubikey>,
    key_file: Option<&str>,
) -> Vec<String> {
    let mut arguments = vec!["ls".to_owned(), "--quiet".to_owned()];
    arguments.extend(composite_key_arguments(yubikey, key_file));
    arguments.extend([
        "--recursive".to_owned(),
        "--flatten".to_owned(),
        database.display().to_string(),
    ]);
    arguments
}

fn trimmed(complaint: &str) -> String {
    complaint.trim_end_matches('\n').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_companion_is_the_entry_plus_the_reserved_suffix_and_is_recognised_as_one() {
        assert_eq!(
            companion_of("safix/alice/grafana"),
            "safix/alice/grafana.safix-sync-state"
        );
        assert!(is_companion(&companion_of("safix/alice/grafana")));
        assert!(!is_companion("safix/alice/grafana"));
    }

    /// The suffix is the one the nix half reserves.
    ///
    /// Asserted against the literal rather than against the nix file, which this
    /// crate cannot read. `modules/flake/checks/keepassxc.nix` asserts the same
    /// literal on the other side, so a change to one without the other fails a
    /// check on whichever side moved.
    #[test]
    fn the_reserved_suffix_is_the_one_the_declaration_refuses() {
        assert_eq!(STATE_SUFFIX, ".safix-sync-state");
    }

    #[test]
    fn no_argument_vector_can_carry_a_value() {
        let database = Path::new("/keys/master.kdbx");
        let yubikey = Yubikey {
            slot: "1".to_owned(),
            serial: Some("12345678".to_owned()),
        };
        let key_file = "/keys/master.key";
        let vectors = [
            read_arguments(database, "safix/alice/grafana", None, None),
            read_arguments(
                database,
                "safix/alice/grafana",
                Some(&yubikey),
                Some(key_file),
            ),
            write_arguments(
                database,
                "safix/alice/grafana",
                Some("alice"),
                false,
                None,
                None,
            ),
            write_arguments(
                database,
                "safix/alice/grafana",
                None,
                true,
                Some(&yubikey),
                Some(key_file),
            ),
            group_arguments(database, "safix/alice", None, None),
            group_arguments(database, "safix/alice", Some(&yubikey), Some(key_file)),
            listing_arguments(database, None, None),
            listing_arguments(database, Some(&yubikey), Some(key_file)),
        ];
        for vector in &vectors {
            let shown = vector.join(" ");
            assert!(
                shown.contains("/keys/master.kdbx"),
                "the database is public: {shown}"
            );
            for forbidden in ["--password=", "--value", "--generate"] {
                assert!(
                    !shown.contains(forbidden),
                    "{forbidden} reached argv: {shown}"
                );
            }
        }
        // The key file's own contents never appear here because there is
        // nothing to assert of them in an argv test: `key_file` is
        // `Option<&str>`, a path, never a `Secret`, so the type signature
        // already forbids the file's bytes from reaching this function at all.
    }

    /// The fourth constructor `no_argument_vector_can_carry_a_value` exercises
    /// but does not pin a literal for: `read_arguments`, `write_arguments` and
    /// `listing_arguments` each have their own literal-pinning test above, and
    /// this one gives `group_arguments` the same independent coverage.
    #[test]
    fn a_group_is_created_with_the_declared_factors_alongside_it() {
        let database = Path::new("/keys/master.kdbx");
        let bare = group_arguments(database, "safix/alice", None, None).join(" ");
        assert_eq!(bare, "mkdir --quiet /keys/master.kdbx safix/alice");

        let yubikey = Yubikey {
            slot: "1".to_owned(),
            serial: Some("12345678".to_owned()),
        };
        let factored = group_arguments(
            database,
            "safix/alice",
            Some(&yubikey),
            Some("/keys/master.key"),
        )
        .join(" ");
        assert_eq!(
            factored,
            "mkdir --quiet -y 1:12345678 -k /keys/master.key /keys/master.kdbx safix/alice"
        );
    }

    #[test]
    fn a_write_adds_what_is_absent_and_edits_what_is_there() {
        let database = Path::new("/keys/master.kdbx");
        let added =
            write_arguments(database, "safix/alice/grafana", None, false, None, None).join(" ");
        let edited =
            write_arguments(database, "safix/alice/grafana", None, true, None, None).join(" ");
        assert!(added.starts_with("add --quiet --password-prompt"));
        assert!(edited.starts_with("edit --quiet --password-prompt"));

        let yubikey = Yubikey {
            slot: "1".to_owned(),
            serial: None,
        };
        let factored = write_arguments(
            database,
            "safix/alice/grafana",
            None,
            false,
            Some(&yubikey),
            Some("/keys/master.key"),
        )
        .join(" ");
        assert!(factored.starts_with("add --quiet -y 1 -k /keys/master.key --password-prompt"));
    }

    #[test]
    fn a_username_reaches_argv_and_its_absence_leaves_the_field_alone() {
        let database = Path::new("/keys/master.kdbx");
        let named = write_arguments(
            database,
            "safix/alice/grafana",
            Some("alice@example"),
            true,
            None,
            None,
        );
        let unnamed = write_arguments(database, "safix/alice/grafana", None, true, None, None);
        assert!(named.contains(&"--username".to_owned()));
        assert!(named.contains(&"alice@example".to_owned()));
        assert!(!unnamed.contains(&"--username".to_owned()));

        let yubikey = Yubikey {
            slot: "2".to_owned(),
            serial: None,
        };
        let factored = write_arguments(
            database,
            "safix/alice/grafana",
            Some("alice@example"),
            true,
            Some(&yubikey),
            None,
        );
        assert!(factored.contains(&"--username".to_owned()));
        assert!(factored.contains(&"alice@example".to_owned()));
        assert!(factored.contains(&"-y".to_owned()));
    }

    #[test]
    fn the_read_asks_for_the_protected_password_attribute_and_nothing_else() {
        let arguments = read_arguments(
            Path::new("/keys/master.kdbx"),
            "safix/alice/grafana",
            None,
            None,
        );
        assert_eq!(
            arguments.join(" "),
            "show --quiet --show-protected --attributes Password /keys/master.kdbx safix/alice/grafana"
        );

        let yubikey = Yubikey {
            slot: "1".to_owned(),
            serial: Some("12345678".to_owned()),
        };
        let factored = read_arguments(
            Path::new("/keys/master.kdbx"),
            "safix/alice/grafana",
            Some(&yubikey),
            Some("/keys/master.key"),
        );
        assert_eq!(
            factored.join(" "),
            "show --quiet -y 1:12345678 -k /keys/master.key --show-protected --attributes Password /keys/master.kdbx safix/alice/grafana"
        );
    }

    /// The listing is what absence is answered from, so its shape is a contract.
    #[test]
    fn the_listing_is_recursive_and_flat() {
        let arguments = listing_arguments(Path::new("/keys/master.kdbx"), None, None).join(" ");
        assert_eq!(
            arguments,
            "ls --quiet --recursive --flatten /keys/master.kdbx"
        );

        let yubikey = Yubikey {
            slot: "1".to_owned(),
            serial: None,
        };
        let factored =
            listing_arguments(Path::new("/keys/master.kdbx"), Some(&yubikey), None).join(" ");
        assert_eq!(
            factored,
            "ls --quiet -y 1 --recursive --flatten /keys/master.kdbx"
        );
    }

    /// A database nothing opened holds nothing, and asking is not a refusal.
    #[test]
    fn a_read_of_an_entry_the_listing_does_not_carry_is_absence_rather_than_a_refusal() {
        let database = Database {
            program: PathBuf::from("safix-no-such-store-command"),
            path: PathBuf::from("/keys/master.kdbx"),
            key: Secret::empty(),
            yubikey: None,
            key_file: None,
            entries: BTreeSet::new(),
            groups: BTreeSet::new(),
        };
        // No process is spawned, which is what makes this assertion about the
        // listing rather than about the command: the program named does not exist.
        assert!(
            database
                .read("safix/alice/grafana")
                .expect("absence")
                .is_none()
        );
    }

    #[test]
    fn a_value_carrying_a_newline_is_refused_before_the_command_is_run() {
        let mut database = Database {
            program: PathBuf::from("safix-no-such-store-command"),
            path: PathBuf::from("/keys/master.kdbx"),
            key: Secret::empty(),
            yubikey: None,
            key_file: None,
            entries: BTreeSet::new(),
            groups: BTreeSet::new(),
        };
        let value = Secret::read_from(&mut b"two\nlines".as_slice()).expect("a fixture value");
        let refusal = database.write("safix/alice/grafana", &value, None);
        assert!(matches!(refusal, Err(Error::ValueSpansLines { .. })));
    }

    #[test]
    fn entries_under_a_group_exclude_the_ones_outside_it() {
        let database = Database {
            program: PathBuf::from("safix-no-such-store-command"),
            path: PathBuf::from("/keys/master.kdbx"),
            key: Secret::empty(),
            yubikey: None,
            key_file: None,
            entries: BTreeSet::from([
                "safix/alice/grafana".to_owned(),
                "safix/alice/grafana.safix-sync-state".to_owned(),
                "elsewhere/router".to_owned(),
            ]),
            groups: BTreeSet::new(),
        };
        assert_eq!(
            database.under("safix").collect::<Vec<_>>(),
            [
                "safix/alice/grafana",
                "safix/alice/grafana.safix-sync-state"
            ]
        );
    }

    #[test]
    fn an_absent_command_is_refused_by_name() {
        let database = Database {
            program: PathBuf::from("safix-no-such-store-command"),
            path: PathBuf::from("/keys/master.kdbx"),
            key: Secret::empty(),
            yubikey: None,
            key_file: None,
            entries: BTreeSet::from(["safix/alice/grafana".to_owned()]),
            groups: BTreeSet::new(),
        };
        let refusal = database.read("safix/alice/grafana");
        assert!(matches!(refusal, Err(Error::StoreUnavailable { .. })));
    }
}
