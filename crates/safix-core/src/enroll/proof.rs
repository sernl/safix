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
//! A governed file the person's audience covers, in full, straight into the null
//! sink. No plaintext is retained by this process. A canary encrypted for the
//! occasion would prove that a fresh file made from a fresh rule opens, which
//! is not the question; the question is whether the store the
//! person already has opens.
//!
//! # What a failure means
//!
//! That the enrollment is incomplete, and not that it is wrong. The identity
//! block, the recipient and the re-wrap are additive and correct on their own, so
//! nothing is undone: the run reports what is outstanding and exits non-zero.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::workspace::Workspace;

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
    /// The cryptographic tool refused, and this is what it exited with.
    Refused {
        /// The file it was asked for, repository-relative.
        file: String,
        /// The cryptographic tool's exit status.
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
/// Refuses unsupported ciphertext formats or unavailable cryptographic tools.
/// A tool that runs and refuses is [`Outcome::Refused`], rather than a successful
/// proof backed by some other identity.
pub fn decrypt_with(
    workspace: &Workspace,
    identity_file: &Path,
    relative: &str,
) -> Result<Outcome> {
    let source = crate::ciphertext::Source {
        path: workspace.vault_absolute(relative),
        format: crate::ciphertext::Format::from_path(Path::new(relative))?,
        key: String::new(),
    };
    let status = crate::ciphertext::prove_with_age_identity(&source, identity_file)?;

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
