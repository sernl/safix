//! One run's view of one repository.
//!
//! Every subcommand needs some of the same four evaluations, and needs each of
//! them at most once: the shell runtime caches them in temporary files for the
//! length of a run, and this is the same cache with the temporary files gone.
//! Holding them here rather than passing them around is also what keeps the two
//! sides of a drift judgement from being read twice and disagreeing.
//!
//! The evaluations are separate `nix eval` calls rather than one call fetching
//! everything, because the hermetic checks' stubbed `nix` dispatches on the
//! attribute each call names: a rename of one fails a check rather than an
//! operator's terminal, and the flake's evaluation cache makes the later calls
//! cheap.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::error::{Error, Result};
use crate::git::Git;
use crate::model::{Audiences, GovernedFiles, Placement, Placements, Recipients};
use crate::nix::{Attribute, Nix};
use crate::sops::Sops;

/// The repository a run is operating on, and what has been evaluated about it.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    nix: Nix,
    git: Git,
    sops: Sops,
    placements: OnceLock<Placements>,
    audiences: OnceLock<Audiences>,
    governed_files: OnceLock<GovernedFiles>,
    recipients: OnceLock<Recipients>,
}

impl Workspace {
    /// The repository this process is inside, with every driver taken from the
    /// environment.
    ///
    /// # Errors
    ///
    /// [`Error::NotInsideRepository`] when git reports none.
    pub fn discover() -> Result<Self> {
        let git = Git::from_environment();
        let root = git.repository_root()?;
        Ok(Self::at(
            root,
            git,
            Nix::from_environment(),
            Sops::from_environment(),
        ))
    }

    /// A workspace at a named root, with drivers given rather than discovered.
    #[must_use]
    pub fn at(root: PathBuf, git: Git, nix: Nix, sops: Sops) -> Self {
        Self {
            root,
            nix,
            git,
            sops,
            placements: OnceLock::new(),
            audiences: OnceLock::new(),
            governed_files: OnceLock::new(),
            recipients: OnceLock::new(),
        }
    }

    /// The repository root every path here is relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The git driver.
    #[must_use]
    pub fn git(&self) -> &Git {
        &self.git
    }

    /// The sops driver.
    #[must_use]
    pub fn sops(&self) -> &Sops {
        &self.sops
    }

    /// The nix driver.
    #[must_use]
    pub fn nix(&self) -> &Nix {
        &self.nix
    }

    /// The absolute path of a repository-relative one.
    #[must_use]
    pub fn absolute(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// `user -> name -> placement`.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn placements(&self) -> Result<&Placements> {
        cached(&self.placements, || {
            self.nix.eval_json(&self.root, Attribute::Placements)
        })
    }

    /// `file -> who can open it`.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn audiences(&self) -> Result<&Audiences> {
        cached(&self.audiences, || {
            self.nix.eval_json(&self.root, Attribute::Audiences)
        })
    }

    /// Which files the recipient policy governs.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn governed_files(&self) -> Result<&GovernedFiles> {
        cached(&self.governed_files, || {
            self.nix.eval_json(&self.root, Attribute::GovernedFiles)
        })
    }

    /// `user -> every age key that user can open a file with`.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn recipients(&self) -> Result<&Recipients> {
        cached(&self.recipients, || {
            self.nix.eval_json(&self.root, Attribute::Recipients)
        })
    }

    /// The recipient policy the declarations imply, as text.
    ///
    /// Uncached, because the one caller compares it against the committed file
    /// once and a second reader of it would be a second answer to the same
    /// question.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn policy_text(&self) -> Result<String> {
        self.nix.eval_raw(&self.root, Attribute::PolicyText)
    }

    /// The user a bare invocation means.
    ///
    /// The login name when the declarations name it, which is the case this
    /// exists for: the operator on their own workstation. A machine whose login
    /// name is not a declared user falls through to the sole declared holder
    /// when there is exactly one, and is otherwise told to name the user,
    /// because guessing between two people's custody is the one guess with a
    /// disclosure at the end of it.
    ///
    /// # Errors
    ///
    /// [`Error::NoDefaultUser`] when neither rule picks one.
    pub fn default_user(&self) -> Result<String> {
        let placements = self.placements()?;
        let login = login_name();
        if placements.declares(&login) {
            return Ok(login);
        }
        let holders: Vec<&str> = placements.holders().collect();
        if let [only] = holders.as_slice() {
            return Ok((*only).to_owned());
        }
        Err(Error::NoDefaultUser {
            login,
            holders: holders.len(),
        })
    }

    /// Where one user's one name lives, refusing anything the declarations do
    /// not place.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownUser`], [`Error::UnknownName`], [`Error::NoFileForName`]
    /// or [`Error::NotAYamlPath`].
    pub fn resolve(&self, user: &str, name: &str) -> Result<&Placement> {
        let placements = self.placements()?;
        let held = placements.held_by(user).ok_or_else(|| Error::UnknownUser {
            user: user.to_owned(),
            declared: placements.users().map(str::to_owned).collect(),
        })?;
        let placement = held.get(name).ok_or_else(|| Error::UnknownName {
            user: user.to_owned(),
            name: name.to_owned(),
            held: held.keys().cloned().collect(),
        })?;

        if placement.file.is_empty() {
            return Err(Error::NoFileForName {
                name: name.to_owned(),
            });
        }
        // Byte-wise and case-sensitive, because the suffix being matched is the
        // literal `\.yaml$` every generated `path_regex` ends in: a `.YAML`
        // path is one no creation rule covers, so treating it as equivalent
        // would admit exactly the file that fails closed at encryption time.
        if !placement.file.as_bytes().ends_with(b".yaml") {
            return Err(Error::NotAYamlPath {
                name: name.to_owned(),
                file: placement.file.clone(),
            });
        }
        Ok(placement)
    }

    /// Refuse a user the declarations do not name, without resolving a name.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownUser`].
    pub fn require_user(&self, user: &str) -> Result<()> {
        let placements = self.placements()?;
        if placements.declares(user) {
            return Ok(());
        }
        Err(Error::UnknownUser {
            user: user.to_owned(),
            declared: placements.users().map(str::to_owned).collect(),
        })
    }

    /// The text of a repository-relative file, or nothing when it does not
    /// exist.
    ///
    /// # Errors
    ///
    /// [`Error::FileUnreadable`] when it exists and cannot be read.
    pub fn read_relative(&self, relative: &str) -> Result<Option<String>> {
        let path = self.absolute(relative);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(cause) => Err(Error::FileUnreadable {
                path: path.display().to_string(),
                cause,
            }),
        }
    }
}

/// `$USER`, or what `id -un` says when it is unset or empty.
///
/// A subprocess rather than a library call because that is what the shell
/// runtime does, and the two have to answer the same on a machine where the
/// environment and the password database disagree.
fn login_name() -> String {
    if let Ok(user) = std::env::var("USER")
        && !user.is_empty()
    {
        return user;
    }
    Command::new("id")
        .arg("-un")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(String::new, |name| name.trim_end_matches('\n').to_owned())
}

/// `OnceLock::get_or_init` with a fallible initializer.
///
/// A racing second caller computes a value that is then dropped rather than
/// stored, which costs one duplicate evaluation and never two answers.
fn cached<T>(cell: &OnceLock<T>, load: impl FnOnce() -> Result<T>) -> Result<&T> {
    if let Some(value) = cell.get() {
        return Ok(value);
    }
    let value = load()?;
    Ok(cell.get_or_init(|| value))
}
