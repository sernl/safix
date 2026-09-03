//! The card alone opening a file the person's audience actually governs.
//!
//! An enrollment that appended a block and edited a list has changed
//! declarations. Whether the card can decrypt is a different claim, and nothing
//! before this point tested it: the recipient came out of the generator's own
//! output, and a recipient is a public string that a re-wrap will happily encrypt
//! to whether or not anything can open the result.
//!
//! # Why the identity source is built rather than reused
//!
//! age tries native identities before plugin identities, so an ambient
//! `keys.txt` holding the operator's software key opens every file the card also
//! opens, silently and without touching the card. A proof run against that file
//! is a proof about the software key. So the proof gets an identity source of its
//! own holding one line — the card's stub — and `SOPS_AGE_KEY_FILE` names it and
//! nothing else. Every other way sops finds identities is cleared from the
//! child's environment for the same reason.
//!
//! # What is decrypted, and where it goes
//!
//! A governed file the person's audience covers, in full, into a pipe this
//! process drains and drops. Nothing is extracted, nothing is written, and the
//! value is a [`Secret`] for the length of one statement. A canary
//! encrypted for the occasion would prove that a fresh file made from a fresh
//! rule opens, which is not the question; the question is whether the store the
//! person already has opens.
//!
//! # What a failure means
//!
//! That the enrollment is incomplete, and not that it is wrong. The identity
//! block, the recipient and the re-wrap are additive and correct on their own, so
//! nothing is undone: the run reports what is outstanding and exits non-zero.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::secret::Secret;
use crate::workspace::Workspace;

/// The variable sops reads an age identity file from.
pub const IDENTITY_FILE_VARIABLE: &str = "SOPS_AGE_KEY_FILE";

/// Every other way sops can find an age identity.
///
/// Cleared from the proof's child, all of them, because the proof's whole
/// property is that exactly one identity was reachable. An identity arriving
/// through any of these would make a passing proof mean nothing, and it would
/// pass.
pub const OTHER_IDENTITY_VARIABLES: [&str; 3] = [
    "SOPS_AGE_KEY",
    "SOPS_AGE_KEY_CMD",
    "SOPS_AGE_SSH_PRIVATE_KEY_FILE",
];

/// The name the isolated identity file is written under.
const ISOLATED_FILE: &str = "card-identity.txt";

/// What the proof found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The card opened this file, alone.
    Proven {
        /// The file it opened, repository-relative.
        file: String,
    },
    /// sops refused, and this is what it exited with.
    Refused {
        /// The file it was asked for, repository-relative.
        file: String,
        /// What sops exited with.
        status: i32,
    },
}

impl Outcome {
    /// Whether the card was shown to open a governed file.
    #[must_use]
    pub const fn proven(&self) -> bool {
        matches!(self, Self::Proven { .. })
    }
}

/// A governed file the person's audience covers and that exists on disk.
///
/// The first in the order the declarations name them, so the choice is a
/// function of the declarations rather than of a directory listing.
///
/// # Errors
///
/// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`] when the audiences
/// cannot be read, and [`Error::NoFileToProveWith`] when the person's audience
/// covers no file that exists.
pub fn file_to_prove_with(workspace: &Workspace, user: &str) -> Result<String> {
    workspace
        .audiences()?
        .0
        .iter()
        .find(|(file, record)| {
            record.audience.iter().any(|member| member == user)
                && workspace.vault_absolute(file).exists()
        })
        .map(|(file, _)| file.clone())
        .ok_or_else(|| Error::NoFileToProveWith {
            user: user.to_owned(),
        })
}

/// Write the isolated identity source, holding the card's stub and nothing else.
///
/// # Errors
///
/// [`Error::FileUnwritable`] when it cannot be written.
pub fn write_isolated_source(directory: &Path, stub: &str) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;

    let path = directory.join(ISOLATED_FILE);
    let mut text = String::from(stub.trim_end());
    text.push('\n');
    std::fs::write(&path, &text).map_err(|cause| Error::FileUnwritable {
        path: path.display().to_string(),
        cause,
    })?;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    Ok(path)
}

/// The stub line of an identity block, which is the only line the proof needs.
///
/// The metadata comments are for a person reading the file and say nothing to
/// sops, so the isolated source carries the one line that does — which also
/// means a block whose comments named a second recipient could not smuggle one
/// in.
#[must_use]
pub fn stub_of(block: &str) -> Option<String> {
    block
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
}

/// Decrypt one governed file with the isolated source alone.
///
/// # Errors
///
/// [`Error::SopsUnavailable`] when sops cannot be run, and
/// [`Error::SecretRead`] when its output cannot be read. A sops that runs and
/// refuses is [`Outcome::Refused`] rather than an error: the proof not passing is
/// an outcome the report carries, not a failure of the machinery.
pub fn decrypt_with(
    workspace: &Workspace,
    identity_file: &Path,
    relative: &str,
) -> Result<Outcome> {
    let mut command = Command::new(sops_program());
    command
        .arg("decrypt")
        .arg(workspace.vault_absolute(relative))
        .current_dir(workspace.vault_root())
        .env(IDENTITY_FILE_VARIABLE, identity_file)
        .stdin(Stdio::null())
        // Piped rather than inherited: the plaintext of a real secret is what
        // comes out, and it goes into a value that is dropped at the end of this
        // statement rather than onto the operator's terminal.
        .stdout(Stdio::piped())
        // Inherited: sops's own account of a card that is absent, a PIN that was
        // wrong or a touch that timed out is the useful half of a failed proof.
        .stderr(Stdio::inherit());
    for variable in OTHER_IDENTITY_VARIABLES {
        command.env_remove(variable);
    }

    let mut child = command
        .spawn()
        .map_err(|cause| workspace.sops().unavailable(cause))?;

    {
        let mut stdout = child.stdout.take().ok_or(Error::SopsPipeMissing)?;
        // Read into a value that cannot be printed and is zeroed when this block
        // ends. Nothing looks at it: what is being established is that sops
        // could produce it.
        let _opened = Secret::read_from(&mut stdout)?;
    }

    let status = child
        .wait()
        .map_err(|cause| workspace.sops().unavailable(cause))?
        .code()
        .unwrap_or(1);

    if status == 0 {
        return Ok(Outcome::Proven {
            file: relative.to_owned(),
        });
    }
    Ok(Outcome::Refused {
        file: relative.to_owned(),
        status,
    })
}

/// The sops binary the rest of the runtime reaches, by the same variable.
///
/// Read here rather than taken from [`Workspace`] because the driver hands back
/// commands with its own stream wiring and this one's wiring is the proof's: a
/// pipe on standard output that is drained and dropped.
fn sops_program() -> PathBuf {
    std::env::var_os("SAFIX_SOPS")
        .filter(|value| !value.is_empty())
        .map_or_else(|| PathBuf::from("sops"), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "\
#       Serial: 12345678, Slot: 1
#    Recipient: age1yubikey1qfixture
AGE-PLUGIN-YUBIKEY-1QFIXTURE000000000000000000
";

    #[test]
    fn the_isolated_source_holds_the_stub_and_not_the_comments() {
        let stub = stub_of(BLOCK).expect("the block has a stub line");
        assert_eq!(stub, "AGE-PLUGIN-YUBIKEY-1QFIXTURE000000000000000000");

        let directory = std::env::temp_dir().join(format!("safix-proof-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory can be made");
        let written = write_isolated_source(&directory, &stub).expect("it can be written");
        let text = std::fs::read_to_string(&written).expect("it can be read back");
        assert_eq!(text, format!("{stub}\n"));
        assert_eq!(text.lines().count(), 1, "the source holds exactly one line");
        std::fs::remove_dir_all(&directory).expect("it can be removed");
    }

    #[test]
    fn a_block_of_comments_alone_has_no_stub() {
        assert_eq!(stub_of("# only a comment\n"), None);
        assert_eq!(stub_of(""), None);
    }

    #[test]
    fn every_other_way_sops_finds_an_identity_is_named_so_it_can_be_cleared() {
        assert!(OTHER_IDENTITY_VARIABLES.contains(&"SOPS_AGE_KEY"));
        assert!(OTHER_IDENTITY_VARIABLES.contains(&"SOPS_AGE_KEY_CMD"));
        assert_eq!(IDENTITY_FILE_VARIABLE, "SOPS_AGE_KEY_FILE");
    }

    #[test]
    fn a_proven_outcome_is_the_only_one_that_counts_as_proven() {
        assert!(
            Outcome::Proven {
                file: String::from("a.yaml")
            }
            .proven()
        );
        assert!(
            !Outcome::Refused {
                file: String::from("a.yaml"),
                status: 1
            }
            .proven()
        );
    }
}
