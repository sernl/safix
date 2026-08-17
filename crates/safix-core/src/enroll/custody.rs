//! Where the generated PIN and PUK go once the card has them.
//!
//! Two homes, and they answer different questions. The safix secret is the
//! default: the credentials land in the person's own custody, encrypted to the
//! recipients they already hold, named for the serial, written through the same
//! path `set` writes through. The password store is the optional second home,
//! because the store is the fleet's root of trust and a credential that exists
//! only inside the thing it unlocks is a credential with a cycle in it.
//!
//! # The caveat, stated beside the default rather than under it
//!
//! A PIN encrypted to the person's software identity adds protection only once
//! that identity is retired or absent: while it exists, whoever holds it holds
//! both. The default is there to make starting easy — the operator asked that
//! safix hold credentials for itself first — and `--no-store-pin` turns it off.
//! It is not there because it buys a property; the argument against it is real
//! and this is where it is written down.
//!
//! # Which channels a credential travels
//!
//! Pipes and D-Bus. `secret-tool` takes the secret on standard input and the
//! label and attributes in argv, which are the serial and a role — public
//! strings. `keepassxc-cli` takes the database password and then the entry
//! password on standard input, which is why the database password is prompted
//! for here and piped rather than left to the tool's own terminal: one stream
//! cannot be both a pipe and a keyboard. Nothing in either transport puts a
//! credential in an argument vector or an environment variable.
//!
//! The one exception in this change is `ykman`'s own flags, and it is
//! [`card`](super::card)'s to explain, not this module's: nothing here writes
//! one.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::secret::Secret;

/// The environment variable that replaces the secret-service tool, for checks.
pub const SECRET_TOOL_OVERRIDE: &str = "SAFIX_SECRET_TOOL";

/// The environment variable that replaces the password-store tool, for checks.
pub const KEEPASSXC_OVERRIDE: &str = "SAFIX_KEEPASSXC_CLI";

/// The attribute every entry safix writes to the secret service carries.
pub const SERVICE_ATTRIBUTE: &str = "safix-card";

/// The attribute value used to ask the service a question it will answer no to.
///
/// The reachability probe: a `secret-tool lookup` that finds nothing prints
/// nothing and exits non-zero, and one that cannot reach the service says so on
/// standard error. That is the difference the probe reads, and it reads it
/// without matching any wording.
const PROBE_VALUE: &str = "safix-reachability-probe";

/// Where a mirrored copy of the credentials goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    /// The session's secret service, which prompts for nothing when the
    /// database behind it is unlocked.
    SecretService,
    /// The store's own command, with one password prompt for the database.
    PasswordStore {
        /// The database file the entry is added to.
        database: PathBuf,
    },
    /// Nowhere. The mirror is optional and this is what it not happening looks
    /// like, with the reason the report carries.
    Skipped {
        /// Why, in the words the report prints.
        reason: &'static str,
    },
}

/// What a run asked for by way of a mirror.
#[derive(Debug, Clone, Default)]
pub struct Wish {
    /// Whether a mirror was asked for at all.
    pub mirror: bool,
    /// The database the store's own command is pointed at, when one was named.
    pub database: Option<PathBuf>,
}

/// Which transport a wish resolves to.
///
/// A function of the wish and one probe, with no side effects, so every branch is
/// a test rather than a run against somebody's real password store.
#[must_use]
pub fn choose(wish: &Wish, service_reachable: bool) -> Transport {
    if !wish.mirror {
        return Transport::Skipped {
            reason: "no mirror was asked for",
        };
    }
    if service_reachable {
        return Transport::SecretService;
    }
    match &wish.database {
        Some(database) => Transport::PasswordStore {
            database: database.clone(),
        },
        None => Transport::Skipped {
            reason: "the session's secret service did not answer and no database was named",
        },
    }
}

/// Where the operator's database password comes from.
///
/// The command's to implement, for the reason every other prompt is: a terminal
/// is not this crate's, and a library embedder that has the password already
/// should not be made to pretend it has a terminal to type it on.
pub trait DatabasePassword {
    /// The password that unlocks this database.
    ///
    /// # Errors
    ///
    /// Whatever reading it failed with.
    fn database_password(&mut self, database: &Path) -> Result<Secret>;
}

/// Whether the session's secret service answers.
///
/// A lookup of an attribute nothing holds. Found is impossible and would still
/// mean the service answered; not found with nothing on standard error means the
/// same; anything else — the tool absent, the bus absent, the collection
/// unreachable — means it did not.
#[must_use]
pub fn service_reachable() -> bool {
    let finished = Command::new(secret_tool())
        .arg("lookup")
        .arg(SERVICE_ATTRIBUTE)
        .arg(PROBE_VALUE)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match finished {
        Ok(finished) => finished.stderr.is_empty(),
        Err(_) => false,
    }
}

/// The label an entry carries, which is what a person reading the store sees.
#[must_use]
pub fn label(user: &str, serial: &str) -> String {
    format!("safix: PIV access for {user}'s YubiKey {serial}")
}

/// The entry name the credentials are filed under, in either store.
#[must_use]
pub fn entry_name(serial: &str) -> String {
    format!("safix-card-{serial}-piv-access")
}

/// The name the credentials are declared and set under, as a safix secret.
///
/// Named for the serial rather than for the person, because a person may hold
/// several cards and each one's access is its own credential.
#[must_use]
pub fn secret_name(serial: &str) -> String {
    format!("card-{serial}-piv-access")
}

/// The argument vector the secret service is written through.
///
/// Public strings only: a label naming the person and the serial, and one
/// attribute pair. The credentials are not here and cannot be — they arrive on
/// standard input.
#[must_use]
pub fn secret_tool_arguments(user: &str, serial: &str) -> Vec<String> {
    vec![
        "store".to_owned(),
        format!("--label={}", label(user, serial)),
        SERVICE_ATTRIBUTE.to_owned(),
        entry_name(serial),
    ]
}

/// The argument vector the store's own command is written through.
///
/// `--password-prompt` is what makes the entry's password arrive on standard
/// input instead of in argv, which is the whole reason this transport is shaped
/// the way it is.
#[must_use]
pub fn keepassxc_arguments(database: &Path, serial: &str) -> Vec<String> {
    vec![
        "add".to_owned(),
        "--password-prompt".to_owned(),
        database.display().to_string(),
        entry_name(serial),
    ]
}

/// Write the credentials through the chosen transport.
///
/// # Errors
///
/// [`Error::StoreUnavailable`] when the transport's command cannot be run,
/// [`Error::StoreMirrorFailed`] when it runs and refuses, and whatever reading
/// the database password failed with.
pub fn write(
    transport: &Transport,
    user: &str,
    serial: &str,
    credentials: &Secret,
    password: &mut dyn DatabasePassword,
) -> Result<()> {
    match transport {
        Transport::Skipped { .. } => Ok(()),
        Transport::SecretService => {
            let arguments = secret_tool_arguments(user, serial);
            feed(
                &secret_tool(),
                &arguments,
                "the session's secret service",
                &[credentials],
            )
        }
        Transport::PasswordStore { database } => {
            let unlock = password.database_password(database)?;
            let arguments = keepassxc_arguments(database, serial);
            feed(
                &keepassxc_cli(),
                &arguments,
                "the password store's own command",
                &[&unlock, credentials],
            )
        }
    }
}

/// Run one transport, writing each value on its own line of standard input.
///
/// One line per value, in order, because both tools read line-delimited answers
/// from standard input when it is not a terminal: the database password, then the
/// entry's. The newline separates them and does not follow the last, because the
/// last value is a value rather than an answer and end of input is what ends it —
/// a trailing newline would be a byte of the credential that nobody put there.
fn feed(
    program: &Path,
    arguments: &[String],
    transport: &'static str,
    values: &[&Secret],
) -> Result<()> {
    use std::io::Write as _;

    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|cause| Error::StoreUnavailable {
            program: program.display().to_string(),
            cause,
        })?;

    {
        let mut stdin = child.stdin.take().ok_or(Error::SopsPipeMissing)?;
        for (position, value) in values.iter().enumerate() {
            if position > 0 {
                stdin
                    .write_all(b"\n")
                    .map_err(|cause| Error::SecretRead { cause })?;
            }
            value
                .write_to(&mut stdin)
                .map_err(|cause| Error::SecretRead { cause })?;
        }
        stdin.flush().map_err(|cause| Error::SecretRead { cause })?;
    }

    let finished = child
        .wait_with_output()
        .map_err(|cause| Error::StoreUnavailable {
            program: program.display().to_string(),
            cause,
        })?;
    if finished.status.success() {
        return Ok(());
    }
    Err(Error::StoreMirrorFailed {
        transport,
        status: finished.status.code().unwrap_or(1),
        output: String::from_utf8_lossy(&finished.stderr)
            .trim_end_matches('\n')
            .to_owned(),
    })
}

/// Read one entry back out of the secret service, as bytes.
///
/// What makes "it was stored" a claim rather than an assumption: the round trip
/// goes out through the transport and back in through the same one.
///
/// # Errors
///
/// [`Error::StoreUnavailable`] when the command cannot be run.
pub fn read_back(serial: &str) -> Result<Option<Secret>> {
    let finished = Command::new(secret_tool())
        .arg("lookup")
        .arg(SERVICE_ATTRIBUTE)
        .arg(entry_name(serial))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|cause| Error::StoreUnavailable {
            program: secret_tool().display().to_string(),
            cause,
        })?;
    if !finished.status.success() {
        return Ok(None);
    }
    Ok(Some(Secret::read_from(&mut finished.stdout.as_slice())?))
}

/// The secret-service tool [`SECRET_TOOL_OVERRIDE`] names, or `secret-tool`.
#[must_use]
pub fn secret_tool() -> PathBuf {
    named(SECRET_TOOL_OVERRIDE, "secret-tool")
}

/// The store's own command [`KEEPASSXC_OVERRIDE`] names, or `keepassxc-cli`.
#[must_use]
pub fn keepassxc_cli() -> PathBuf {
    named(KEEPASSXC_OVERRIDE, "keepassxc-cli")
}

fn named(variable: &str, default: &str) -> PathBuf {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map_or_else(|| PathBuf::from(default), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_wish_means_no_mirror_and_says_so() {
        let chosen = choose(&Wish::default(), true);
        assert!(matches!(chosen, Transport::Skipped { .. }));
    }

    #[test]
    fn an_answering_service_wins_over_a_named_database() {
        let wish = Wish {
            mirror: true,
            database: Some(PathBuf::from("/keys/master.kdbx")),
        };
        assert_eq!(choose(&wish, true), Transport::SecretService);
    }

    #[test]
    fn a_silent_service_falls_back_to_the_database_that_was_named() {
        let wish = Wish {
            mirror: true,
            database: Some(PathBuf::from("/keys/master.kdbx")),
        };
        assert_eq!(
            choose(&wish, false),
            Transport::PasswordStore {
                database: PathBuf::from("/keys/master.kdbx")
            }
        );
    }

    #[test]
    fn a_silent_service_with_no_database_skips_rather_than_refusing() {
        let wish = Wish {
            mirror: true,
            database: None,
        };
        let chosen = choose(&wish, false);
        match chosen {
            Transport::Skipped { reason } => {
                assert!(reason.contains("secret service"));
                assert!(reason.contains("no database was named"));
            }
            other => unreachable!("a mirror with nowhere to go became {other:?}"),
        }
    }

    #[test]
    fn neither_transports_argument_vector_can_carry_a_credential() {
        let service = secret_tool_arguments("alice", "12345678").join(" ");
        let store = keepassxc_arguments(Path::new("/keys/master.kdbx"), "12345678").join(" ");
        for shown in [&service, &store] {
            assert!(shown.contains("12345678"), "the serial is public: {shown}");
            assert!(
                !shown.contains("--password="),
                "a password reached argv: {shown}"
            );
        }
        assert!(
            service.starts_with("store --label=safix: PIV access for alice's YubiKey 12345678")
        );
        assert!(store.contains("add --password-prompt /keys/master.kdbx"));
    }

    #[test]
    fn both_stores_and_the_safix_secret_are_named_for_the_serial() {
        assert_eq!(entry_name("12345678"), "safix-card-12345678-piv-access");
        assert_eq!(secret_name("12345678"), "card-12345678-piv-access");
    }

    #[test]
    fn an_absent_transport_is_refused_by_name_rather_than_silently_skipped() {
        struct NoPassword;
        impl DatabasePassword for NoPassword {
            fn database_password(&mut self, _database: &Path) -> Result<Secret> {
                Secret::read_from(&mut b"unused".as_slice())
            }
        }

        // Not through the environment: this asserts the refusal a missing
        // transport produces, and setting a process-wide variable from a test
        // would reach every other test in the binary.
        let refusal = feed(
            Path::new("safix-no-such-store-command"),
            &[String::from("store")],
            "the session's secret service",
            &[&Secret::empty()],
        );
        assert!(matches!(refusal, Err(Error::StoreUnavailable { .. })));
        let _ = NoPassword;
    }
}
