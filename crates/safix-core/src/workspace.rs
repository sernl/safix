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
use crate::model::{
    Audiences, Bridge, Delegation, GeneratorPlan, GovernedFiles, Keepassxc, Placement, Placements,
    Recipients, Subjects,
};
use crate::nix::{Attribute, Nix};
use crate::sops::Sops;

/// The repository a run is operating on, and what has been evaluated about it.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    vault_root: PathBuf,
    nix: Nix,
    git: Git,
    sops: Sops,
    placements: OnceLock<Placements>,
    audiences: OnceLock<Audiences>,
    governed_files: OnceLock<GovernedFiles>,
    recipients: OnceLock<Recipients>,
    delegation: OnceLock<Delegation>,
    generator_plan: OnceLock<GeneratorPlan>,
    bridge: OnceLock<Bridge>,
    keepassxc: OnceLock<Keepassxc>,
    subjects: OnceLock<Subjects>,
    vault_creation_rules_text: OnceLock<Option<String>>,
}

/// The scratch rules file's name, chosen to read unmistakably as generated
/// and disposable rather than as a second `.sops.yaml` — design V10.
pub const VAULT_RULES_FILE: &str = ".sops-vault-rules.yaml";

impl Workspace {
    /// The repository this process is inside, with every driver taken from the
    /// environment.
    ///
    /// # Errors
    ///
    /// [`Error::NotInsideRepository`] when git reports none.
    pub fn discover() -> Result<Self> {
        Self::discover_with(Nix::from_environment())
    }

    /// The repository this process is inside, with git and sops taken from the
    /// environment and the nix driver given.
    ///
    /// [`Workspace::discover`] is the environment-only form this specializes;
    /// this exists so that the command can apply `--entry`'s precedence over
    /// `SAFIX_ENTRY` before building the `Nix` root discovery reads through —
    /// see D8 in `support-plain-nix-consumers`'s design: `--entry` changes only
    /// how declarations are evaluated, never where a run stages or commits.
    ///
    /// # Errors
    ///
    /// [`Error::NotInsideRepository`] when git reports none.
    pub fn discover_with(nix: Nix) -> Result<Self> {
        let git = Git::from_environment();
        let root = git.repository_root()?;
        let vault_root = resolve_vault_root(&git, &nix, &root)?;
        Ok(Self::at(
            root,
            vault_root,
            git,
            nix,
            Sops::from_environment(),
        ))
    }

    /// A workspace at a named root, with drivers given rather than discovered.
    #[must_use]
    pub fn at(root: PathBuf, vault_root: PathBuf, git: Git, nix: Nix, sops: Sops) -> Self {
        Self {
            root,
            vault_root,
            nix,
            git,
            sops,
            placements: OnceLock::new(),
            audiences: OnceLock::new(),
            governed_files: OnceLock::new(),
            recipients: OnceLock::new(),
            delegation: OnceLock::new(),
            generator_plan: OnceLock::new(),
            bridge: OnceLock::new(),
            keepassxc: OnceLock::new(),
            subjects: OnceLock::new(),
            vault_creation_rules_text: OnceLock::new(),
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

    /// The vault repository root, which equals [`Workspace::root`] when no
    /// vault is declared.
    #[must_use]
    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    /// The absolute path of a vault-relative one.
    #[must_use]
    pub fn vault_absolute(&self, relative: &str) -> PathBuf {
        self.vault_root.join(relative)
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

    /// Who may scaffold for whom, and over which groups.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn delegation(&self) -> Result<&Delegation> {
        cached(&self.delegation, || {
            self.nix.eval_json(&self.root, Attribute::Delegation)
        })
    }

    /// `user -> what may run, in which order, reading and writing what`.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn generator_plan(&self) -> Result<&GeneratorPlan> {
        cached(&self.generator_plan, || {
            self.nix.eval_json(&self.root, Attribute::GeneratorPlan)
        })
    }

    /// The clan this consumer bridges to, and every mapping declared for it.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn bridge(&self) -> Result<&Bridge> {
        cached(&self.bridge, || {
            self.nix.eval_json(&self.root, Attribute::Bridge)
        })
    }

    /// The password database this consumer mirrors into, and every mapping
    /// declared for it.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn keepassxc(&self) -> Result<&Keepassxc> {
        cached(&self.keepassxc, || {
            self.nix.eval_json(&self.root, Attribute::Keepassxc)
        })
    }

    /// The subject records: every declared machine, service and group.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn subjects(&self) -> Result<&Subjects> {
        cached(&self.subjects, || {
            self.nix.eval_json(&self.root, Attribute::Subjects)
        })
    }

    /// The alphabet a declared name is drawn from, unanchored.
    ///
    /// Uncached, because the one caller judges one name with it.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn name_regex(&self) -> Result<String> {
        self.nix.eval_raw(&self.root, Attribute::NameRegex)
    }

    /// The consumer's onboarding invocation, or the empty string when none is
    /// configured.
    ///
    /// A consumer who sets nothing is deliberately indistinguishable from one
    /// whose flake safix cannot see: both mean onboarding does less rather than
    /// that something is wrong.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn onboarding_hook(&self) -> Result<String> {
        let hook: Option<String> = self.nix.eval_json(&self.root, Attribute::OnboardingHook)?;
        Ok(hook.unwrap_or_default())
    }

    /// The consumer's enrollment invocation, or the empty string when none is
    /// configured.
    ///
    /// Absent and unset are the same answer, for the reason
    /// [`onboarding_hook`](Self::onboarding_hook) gives: both mean enrollment
    /// does less rather than that something is wrong.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    pub fn enroll_hook(&self) -> Result<String> {
        let hook: Option<String> = self.nix.eval_json(&self.root, Attribute::EnrollHook)?;
        Ok(hook.unwrap_or_default())
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

    /// The text of a vault-relative file, or nothing when it does not exist.
    ///
    /// # Errors
    ///
    /// [`Error::FileUnreadable`] when it exists and cannot be read.
    pub fn read_vault_relative(&self, relative: &str) -> Result<Option<String>> {
        let path = self.vault_absolute(relative);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(cause) => Err(Error::FileUnreadable {
                path: path.display().to_string(),
                cause,
            }),
        }
    }

    /// The vault's disposable creation rules, `null` exactly when
    /// [`Attribute::VaultDeclared`] is `false`.
    ///
    /// Cached like every other evaluation here, and read only by
    /// [`Workspace::stage_vault_rules`] — a caller wanting to know whether a
    /// vault is declared reads `vault_root() != root()` instead, which needs
    /// no evaluation at all.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] or [`Error::NixSchemaMismatch`].
    fn vault_creation_rules_text(&self) -> Result<Option<&str>> {
        cached(&self.vault_creation_rules_text, || {
            self.nix
                .eval_json(&self.root, Attribute::VaultCreationRulesText)
        })
        .map(Option::as_deref)
    }

    /// Render the vault's disposable creation rules to a scratch file inside
    /// the vault working tree, registered for removal before it is created —
    /// design V10's mechanism for reaching a vault-rooted document with
    /// `encrypt` or `updatekeys` when no committed `.sops.yaml` sits there.
    ///
    /// `None` when no vault is declared, which is also when the caller's
    /// `--config` argument is omitted rather than pointed at an empty file:
    /// [`Attribute::VaultCreationRulesText`] answers `null` in exactly that
    /// case, so nothing is written and nothing is registered.
    ///
    /// # Errors
    ///
    /// Whatever evaluating [`Attribute::VaultCreationRulesText`] failed with,
    /// and [`Error::FileUnwritable`] when the scratch file cannot be written.
    pub fn stage_vault_rules(&self) -> Result<Option<PathBuf>> {
        let Some(text) = self.vault_creation_rules_text()? else {
            return Ok(None);
        };
        let path = self.vault_absolute(VAULT_RULES_FILE);
        crate::scratch::register_file(&path);
        std::fs::write(&path, text).map_err(|cause| Error::FileUnwritable {
            path: path.display().to_string(),
            cause,
        })?;
        Ok(Some(path))
    }
}

/// `$USER`, or what `id -un` says when it is unset or empty.
///
/// A subprocess rather than a library call because that is what the shell
/// runtime does, and the two have to answer the same on a machine where the
/// environment and the password database disagree.
pub(crate) fn login_name() -> String {
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

/// The vault root discovery and cross-validation design V1 and V2 specify.
///
/// Resolved once, at [`Workspace::discover_with`], because both mismatch
/// refusals and the git-repository check below must run before anything is
/// evaluated or written — see V1's "refuse before evaluating or writing
/// anything".
fn resolve_vault_root(git: &Git, nix: &Nix, root: &Path) -> Result<PathBuf> {
    let named = std::env::var_os("SAFIX_VAULT_ROOT").map(PathBuf::from);
    let declared: bool = nix.eval_json(root, Attribute::VaultDeclared)?;
    let vault_root = match (named, declared) {
        (None, false) => root.to_path_buf(),
        (Some(path), true) => path,
        (None, true) => return Err(Error::VaultDeclaredWithoutRoot),
        (Some(path), false) => {
            return Err(Error::VaultRootWithoutDeclaration {
                path: path.display().to_string(),
            });
        }
    };
    verify_vault_repository(git, &vault_root)?;
    Ok(vault_root)
}

/// The vault-is-a-git-repository refusal design V2 specifies.
///
/// Applied to every resolved vault root, including the default one — where no
/// vault is declared `vault_root` equals `root`, and this is what would catch
/// a `SAFIX_REPO_ROOT` named at a non-repository, exactly as it catches a
/// misdirected `SAFIX_VAULT_ROOT`.
fn verify_vault_repository(git: &Git, vault_root: &Path) -> Result<()> {
    let found = git
        .show_toplevel(vault_root)
        .map_err(|_| Error::VaultNotARepository {
            path: vault_root.display().to_string(),
        })?;
    if canonicalized(&found) != canonicalized(vault_root) {
        return Err(Error::VaultRootNotTopLevel {
            named: vault_root.display().to_string(),
            found: found.display().to_string(),
        });
    }
    Ok(())
}

/// The canonical form of a path, or the path itself when it cannot be
/// resolved.
///
/// A path git has just reported as a repository's top level is not expected
/// to fail here; falling back to the path as given rather than propagating a
/// second failure keeps this a comparison rather than a third refusal.
fn canonicalized(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(root: &Path, vault_root: &Path) -> Workspace {
        Workspace::at(
            root.to_path_buf(),
            vault_root.to_path_buf(),
            Git::from_environment(),
            Nix::from_environment(),
            Sops::from_environment(),
        )
    }

    /// `absolute`/`read_relative` join against `root`, and
    /// `vault_absolute`/`read_vault_relative` join against `vault_root`, even
    /// when the two differ — the property [`Workspace::at`] gaining the
    /// second root exists to hold.
    #[test]
    fn the_two_roots_are_joined_independently() {
        let scratch =
            std::env::temp_dir().join(format!("safix-workspace-roots-{}", std::process::id()));
        let root = scratch.join("declaration");
        let vault_root = scratch.join("vault");
        std::fs::create_dir_all(&root).expect("a temporary directory can be made");
        std::fs::create_dir_all(&vault_root).expect("a temporary directory can be made");
        std::fs::write(root.join("marker.txt"), "declaration")
            .expect("a temporary file can be written");
        std::fs::write(vault_root.join("marker.txt"), "vault")
            .expect("a temporary file can be written");

        let space = workspace(&root, &vault_root);

        assert_eq!(space.root(), root.as_path());
        assert_eq!(space.vault_root(), vault_root.as_path());
        assert_eq!(space.absolute("marker.txt"), root.join("marker.txt"));
        assert_eq!(
            space.vault_absolute("marker.txt"),
            vault_root.join("marker.txt")
        );
        assert_eq!(
            space.read_relative("marker.txt").expect("the file exists"),
            Some("declaration".to_owned())
        );
        assert_eq!(
            space
                .read_vault_relative("marker.txt")
                .expect("the file exists"),
            Some("vault".to_owned())
        );

        // Each side is also blind to the other root's file: a stray read
        // through the wrong accessor would silently pass this test.
        assert_eq!(
            space
                .read_vault_relative("does-not-exist.txt")
                .expect("a missing file reads as none"),
            None
        );

        std::fs::remove_dir_all(&scratch).expect("the fixture can be removed");
    }
}
