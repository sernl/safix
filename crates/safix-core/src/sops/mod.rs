//! The sops driver.
//!
//! sops is the cryptographic authority and stays a subprocess: every decrypt,
//! encrypt and re-wrap goes through the binary, and the file format, its MAC,
//! its initialization-vector reuse rule and its key wrapping are not
//! reimplemented here. [`document`] is the one exception in appearance only —
//! it reads the two metadata fields the python readers read, without
//! decrypting, and reads nothing else about the format.
//!
//! A decrypted value comes back as a [`Secret`], so it is zeroed when dropped
//! and cannot be rendered, logged or serialized on the way past. The value
//! travels down a pipe: the key name reaches sops in argv, which is public, and
//! the value never does.

pub mod document;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::secret::Secret;

/// The sops binary, and how it is reached.
///
/// The program name is taken from `SAFIX_SOPS` when set, so that a hermetic
/// check can drive the runtime against a fixture without a sops on `PATH`
/// meaning something different from the one the package pins.
#[derive(Debug, Clone)]
pub struct Sops {
    program: PathBuf,
}

impl Default for Sops {
    fn default() -> Self {
        Self::from_environment()
    }
}

/// What a sops subprocess produced, and what it exited with.
///
/// The bytes come back beside the status rather than only on success, because
/// the shell runtime lets sops write straight to its own standard output: a
/// failure that has already emitted bytes has emitted them there too, and a
/// driver that withheld them would differ from the runtime it is replacing on
/// exactly the path where the difference is hardest to notice.
/// No `Debug`, because it holds one: deriving it here would print a decrypted
/// value through the field, which is the disclosure [`Secret`]'s own absent
/// traits exist to prevent.
pub struct Decrypted {
    /// What sops wrote to standard output.
    pub value: Secret,
    /// The status sops exited with; zero on success.
    pub status: i32,
}

impl Sops {
    /// The binary `SAFIX_SOPS` names, or `sops`.
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            program: std::env::var_os("SAFIX_SOPS")
                .map_or_else(|| PathBuf::from("sops"), PathBuf::from),
        }
    }

    /// Decrypt one key of one file to a value in memory.
    ///
    /// Standard error is inherited, so sops's own diagnosis of a missing
    /// identity or a corrupt file reaches the operator as sops wrote it rather
    /// than wrapped in a refusal of ours that would say less.
    ///
    /// # Errors
    ///
    /// [`Error::SopsUnavailable`] when the binary cannot be run at all, and
    /// [`Error::SecretRead`] when its output cannot be read. A sops that runs
    /// and refuses is not an error here: its status comes back in
    /// [`Decrypted::status`] for the caller to exit with.
    pub fn decrypt_key(&self, file: &Path, key: &str) -> Result<Decrypted> {
        let index = serde_json::to_string(&[key]).map_err(|cause| Error::SopsKeyIndex {
            key: key.to_owned(),
            cause: cause.to_string(),
        })?;

        let mut child = Command::new(&self.program)
            .arg("decrypt")
            .arg("--extract")
            .arg(&index)
            .arg(file)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|cause| Error::SopsUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;

        let value = {
            let mut stdout = child.stdout.take().ok_or(Error::SopsPipeMissing)?;
            Secret::read_from(&mut stdout)?
        };

        let status = child.wait().map_err(|cause| Error::SopsUnavailable {
            program: self.program.display().to_string(),
            cause,
        })?;

        Ok(Decrypted {
            value,
            status: status.code().unwrap_or(1),
        })
    }
}
