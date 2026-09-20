//! The sops driver.
//!
//! sops is the cryptographic authority and stays a subprocess: every decrypt,
//! encrypt and re-wrap goes through the binary, and the file format, its MAC,
//! its initialization-vector reuse rule and its key wrapping are not
//! reimplemented here. [`document`] is the one exception in appearance only —
//! it reads two metadata fields without decrypting, and reads nothing else about
//! the format.
//!
//! A decrypted value comes back as a [`Secret`], so it is zeroed when dropped
//! and cannot be rendered, logged or serialized on the way past. The value
//! travels down a pipe: the key name reaches sops in argv, which is public, and
//! the value never does.

pub mod document;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::secret::Secret;

/// The line sops writes when `.sops.yaml` covers no path like the one asked
/// for.
///
/// Matched as a substring of sops's own standard error, which is a coupling to
/// its wording and is deliberate: the alternative is treating every creation
/// failure alike, and this one has a remedy the others do not.
const NO_CREATION_RULE: &str = "no matching creation rules found";

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
        let source = crate::ciphertext::Source {
            path: file.to_path_buf(),
            format: crate::ciphertext::Format::from_path(file)?,
            key: key.to_owned(),
        };
        let identities = crate::ciphertext::Identities::from_environment();
        if source.format != crate::ciphertext::Format::Yaml
            || key.is_empty()
            || !identities.age_ssh_key_paths.is_empty()
        {
            return Ok(Decrypted {
                value: crate::ciphertext::read(&source, &identities)?,
                status: 0,
            });
        }
        let decrypted = self.decrypt_document_json(file, None)?;
        if decrypted.status != 0 {
            return Ok(decrypted);
        }
        let mut buffer = zeroize::Zeroizing::new(Vec::new());
        decrypted
            .value
            .write_to(&mut *buffer)
            .map_err(|cause| Error::SecretRead { cause })?;
        Ok(Decrypted {
            value: crate::ciphertext::extract_json(&buffer, &source)?,
            status: 0,
        })
    }

    /// Decrypt one whole document to JSON in memory.
    ///
    /// What the installer reads: one subprocess per distinct document rather
    /// than one per entry, with the `/`-nested key lookup done here afterwards
    /// — a host resolving eight entries out of one audience's file decrypts
    /// once.
    ///
    /// `identity` names a file to export as `SOPS_AGE_KEY_FILE` for this child
    /// alone. It is a child's environment rather than this process's because
    /// `std::env::set_var` is `unsafe` under the 2024 edition and this
    /// workspace forbids unsafe code; the effect on sops is the same one an
    /// exported variable would have had. `None` leaves whatever the caller's
    /// own environment names, which is what the build-time check mode runs
    /// under.
    ///
    /// # Errors
    ///
    /// [`Error::SopsUnavailable`] when the binary cannot be run, and
    /// [`Error::SecretRead`] when its output cannot be read. A sops that runs
    /// and refuses comes back in [`Decrypted::status`], as it does for
    /// [`Sops::decrypt_key`].
    pub fn decrypt_document_json(&self, file: &Path, identity: Option<&Path>) -> Result<Decrypted> {
        let mut command = self.command();
        command
            .arg("decrypt")
            .arg("--output-type")
            .arg("json")
            .arg(file)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(identity) = identity {
            command.env("SOPS_AGE_KEY_FILE", identity);
        }

        let mut child = command.spawn().map_err(|cause| self.unavailable(cause))?;

        let value = {
            let mut stdout = child.stdout.take().ok_or(Error::SopsPipeMissing)?;
            Secret::read_from(&mut stdout)?
        };

        let status = child.wait().map_err(|cause| self.unavailable(cause))?;

        Ok(Decrypted {
            value,
            status: status.code().unwrap_or(1),
        })
    }

    /// Create a document at `destination` holding one empty key, encrypted as
    /// the creation rule for `relative` says.
    ///
    /// The bytes are produced somewhere else and `--filename-override` is what
    /// applies the rule for the path the file will occupy, so a failure — most
    /// often a rule that has not been regenerated — leaves no half-made file
    /// beside the others. The document holds a missing-value placeholder;
    /// format-aware candidate writing replaces it with the secret.
    ///
    /// `config` names a rendering to read creation rules from in place of the
    /// upward search from `root` — design V10's disposable rules for a
    /// vault-rooted document, whose vault working tree carries no committed
    /// `.sops.yaml` at all. `None` for a declaration-rooted document, which
    /// still has one to be found upward from `root`.
    ///
    /// sops's standard error is captured rather than inherited, because one line
    /// of it is a refusal this runtime intercepts and rewords. The rest is
    /// carried into [`Error::SopsCreateFailed`] verbatim, and on success it is
    /// discarded, which is what the shell runtime does with it too.
    ///
    /// # Errors
    ///
    /// [`Error::NoCreationRule`] when `.sops.yaml` covers no such path,
    /// [`Error::SopsCreateFailed`] for any other refusal from sops, and
    /// [`Error::SopsUnavailable`] when the binary cannot be run.
    pub fn create_empty_document(
        &self,
        root: &Path,
        relative: &str,
        key: &str,
        destination: &Path,
        config: Option<&Path>,
    ) -> Result<()> {
        let document = serde_json::to_vec(&BTreeMap::from([(key, "")])).map_err(|cause| {
            Error::SopsKeyIndex {
                key: key.to_owned(),
                cause: cause.to_string(),
            }
        })?;

        // Created before sops runs and left behind when it fails, which is what
        // the shell runtime's `>"$out"` redirection does; the scratch registry
        // is what removes it either way.
        let out = File::create(destination).map_err(|cause| Error::FileUnwritable {
            path: destination.display().to_string(),
            cause,
        })?;

        let mut command = self.command();
        if let Some(config) = config {
            command.arg("--config").arg(config);
        }
        let mut child = command
            .arg("encrypt")
            .arg("--filename-override")
            .arg(relative)
            .arg("--input-type")
            .arg("json")
            .arg("--output-type")
            .arg("yaml")
            .arg("/dev/stdin")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|cause| self.unavailable(cause))?;

        let written = child
            .stdin
            .take()
            .ok_or(Error::SopsPipeMissing)
            .and_then(|mut stdin| {
                stdin
                    .write_all(&document)
                    .map_err(|cause| Error::SecretRead { cause })
            });
        let finished = child
            .wait_with_output()
            .map_err(|cause| self.unavailable(cause))?;
        if finished.status.success() {
            written?;
        }
        if finished.status.success() {
            return Ok(());
        }

        let complaint = String::from_utf8_lossy(&finished.stderr);
        if complaint.contains(NO_CREATION_RULE) {
            return Err(Error::NoCreationRule {
                file: relative.to_owned(),
            });
        }
        Err(Error::SopsCreateFailed {
            file: relative.to_owned(),
            output: complaint
                .strip_suffix('\n')
                .unwrap_or(&complaint)
                .to_owned(),
        })
    }

    /// The command that re-wraps one file's data key to the recipients its
    /// creation rule now names.
    ///
    /// Handed back as a [`Command`] rather than run, because how its three
    /// streams are connected is the caller's decision and not this driver's: run
    /// one at a time and they are the operator's own terminal, which is what an
    /// interactive confirmation needs; run several at once and they are pipes,
    /// which is what ordering the output of a fan-out needs.
    ///
    /// The working directory is `root`, which is the declaration root for a
    /// declaration-rooted document and the vault root for a vault-rooted one:
    /// sops resolves a rule's `path_regex` against the path relative to the
    /// config it read, and `--filename-override` is not this command's, so
    /// `root` alone is what makes that path the target's own. `config` is
    /// `None` there, and sops discovers the committed `.sops.yaml` upward from
    /// `root` exactly as it always has; `config` is `Some` for a vault-rooted
    /// document, whose vault working tree carries no committed policy to
    /// discover — design V10's disposable rendering names the scratch rules
    /// there instead of relying on that upward search.
    ///
    /// # Errors
    ///
    /// Refuses unrecognized ciphertext extensions before starting SOPS.
    pub fn update_keys_command(
        &self,
        root: &Path,
        relative: &str,
        assume_yes: bool,
        config: Option<&Path>,
    ) -> Result<Command> {
        let mut command = self.command();
        if let Some(config) = config {
            command.arg("--config").arg(config);
        }
        command
            .arg("updatekeys")
            .arg("--input-type")
            .arg(crate::ciphertext::Format::from_path(Path::new(relative))?.as_str());
        if assume_yes {
            command.arg("--yes");
        }
        command.arg(relative).current_dir(root);
        Ok(command)
    }

    /// Construct every SOPS child with the same managed GPG home.
    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        crate::identity::configure_pinentry(&mut command);
        if std::env::var_os("SOPS_GPG_EXEC").is_none_or(|value| value.is_empty())
            && let Some(program) = std::env::var_os("SAFIX_GPG").filter(|value| !value.is_empty())
        {
            command.env("SOPS_GPG_EXEC", program);
        }
        let identities = crate::ciphertext::Identities::from_environment();
        if let Some(home) = identities.gnupg_home {
            command.env("GNUPGHOME", home);
        }
        if let Some(path) = identities.age_key_file {
            command.env("SOPS_AGE_KEY_FILE", path);
        }
        command
    }

    pub(crate) fn unavailable(&self, cause: std::io::Error) -> Error {
        Error::SopsUnavailable {
            program: self.program.display().to_string(),
            cause,
        }
    }
}
