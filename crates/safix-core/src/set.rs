//! Writing one value into the file the declarations place it in.
//!
//! The order below is the safety property, and it is why this sequence lives in
//! the library rather than in the command. Each step is a refusal that has to
//! happen before the step after it:
//!
//! 1. resolve the name, refusing one no declaration places;
//! 2. refuse the repository states in which a commit would mean something other
//!    than what its message says — before the operator is asked for anything,
//!    so a refusal costs them nothing they have typed;
//! 3. prepare a candidate document beside the target, never in place;
//! 4. read the value;
//! 5. write it into the candidate through `sops`;
//! 6. refuse the candidate whose recipients are not the declared audience;
//! 7. rename the candidate over the target;
//! 8. stage and commit that path alone.
//!
//! Six comes after five and before seven for the reason
//! [`Error::RecipientDrift`] gives: `sops set` takes an existing file's
//! recipients from that file's own metadata, so the only document that can be
//! judged is the one that was actually produced, and the only moment it can be
//! judged in is before it lands. A refusal there is a run that never wrote —
//! [`scratch`] removes the candidate and any directory the run created.
//!
//! Nothing here has a terminal in it. The value arrives through [`ValueSource`],
//! which is the command's to implement, and the running commentary goes to a
//! [`Progress`] rather than to standard error.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::progress::{Progress, log, note};
use crate::secret::Secret;
use crate::sops::document;
use crate::workspace::Workspace;
use crate::{git, scratch};

/// Where the value comes from.
///
/// One method rather than two, because reading a value twice and comparing the
/// entries is one act with one outcome: the caller that prompts is also the
/// caller that knows whether a terminal was involved, and splitting it would put
/// half of that knowledge here.
///
/// Four implementations, and only two of them are in this crate. The command owns
/// the prompt that reads a typed value twice and the stream that reads a piped one
/// once, because both are about a terminal; [`crate::edit`] owns the buffer and
/// [`crate::bridge`] the value clan already handed over. Everything after the read
/// is this module's, identically for all four.
pub trait ValueSource {
    /// The value for this user's secret of this name.
    ///
    /// # Errors
    ///
    /// Whatever reading it failed with; [`Error::EmptyValue`] and
    /// [`Error::EntriesDiffer`] are the two refusals an interactive
    /// implementation raises.
    fn read(&mut self, user: &str, name: &str) -> Result<Secret>;
}

/// Set one value, and commit the file holding it.
///
/// Returns zero, or the status `sops` exited with when `sops` refused — which
/// the command exits with in turn, having printed nothing of its own, because
/// sops's standard error is inherited and it has already said why.
///
/// # Errors
///
/// Every refusal on the path above: [`Error::UnknownUser`],
/// [`Error::UnknownName`], [`Error::NotAYamlPath`], [`Error::MidOperation`],
/// [`Error::ConflictEntries`], [`Error::UncommittedChanges`],
/// [`Error::NoCreationRule`], [`Error::NoAudienceForFile`] and
/// [`Error::RecipientDrift`] among them.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    source: &mut dyn ValueSource,
    user: &str,
    name: &str,
) -> Result<i32> {
    run_committing(
        workspace,
        progress,
        source,
        user,
        name,
        &format!("chore(safix): set {name} for {user}"),
    )
}

/// The same write, committed under a subject the caller chose.
///
/// The bridge is the caller that needs this: a value arriving through a mapping
/// is not "set by hand", and a commit saying it was would be the one sentence in
/// the history that is wrong about where the value came from. Everything else —
/// the ordering above, every refusal in it, the staged write and the rename — is
/// the same code, which is the point: an imported value takes the hand-set
/// path's refusals because it *is* the hand-set path.
///
/// # Errors
///
/// Every refusal [`run`] raises.
pub fn run_committing(
    workspace: &Workspace,
    progress: &dyn Progress,
    source: &mut dyn ValueSource,
    user: &str,
    name: &str,
    subject: &str,
) -> Result<i32> {
    scratch::set_floor(workspace.vault_root());
    let _guard = scratch::Guard;

    let placement = workspace.resolve(user, name)?;
    let relative = placement.file.clone();
    let key = placement.key.clone();

    refuse_bad_repository_state(workspace, &[(workspace.vault_root(), relative.as_str())])?;

    let absolute = workspace.vault_absolute(&relative);
    let candidate = candidate_path(&absolute);
    scratch::register_file(&candidate);

    log(
        progress,
        &format!(
            "safix: {name} ({origin}, owner {owner}) -> {relative} [{key}]",
            origin = placement.origin,
            owner = placement.owner,
        ),
    );

    if absolute.exists() {
        std::fs::copy(&absolute, &candidate).map_err(|cause| Error::FileUnwritable {
            path: candidate.display().to_string(),
            cause,
        })?;
    } else {
        if let Some(directory) = absolute.parent()
            && !directory.is_dir()
        {
            scratch::register_dir(directory);
            std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
                path: directory.display().to_string(),
                cause,
            })?;
        }
        note(
            progress,
            &format!(
                "{relative} does not exist yet; creating it through sops so the creation rules apply."
            ),
        );
        let config = workspace.stage_vault_rules()?;
        {
            let _quiet = scratch::quiet();
            workspace.sops().create_empty_document(
                workspace.vault_root(),
                &relative,
                &key,
                &candidate,
                config.as_deref(),
            )?;
        }
        if let Some(status) = scratch::interrupted() {
            return Ok(status);
        }
    }

    let value = source.read(user, name)?;
    if let Some(status) = scratch::interrupted() {
        return Ok(status);
    }

    let status = {
        let _quiet = scratch::quiet();
        workspace.sops().set_key(&candidate, &key, &value)?
    };
    if status != 0 {
        return Ok(status);
    }
    // Where bash's `trap ... INT` would have run: after the foreground child was
    // waited on and before the next command, which here is the rename. A run
    // that reached this having been interrupted stops with nothing renamed and
    // nothing committed, and the guard sweeps the candidate on the way out.
    if let Some(status) = scratch::interrupted() {
        return Ok(status);
    }

    refuse_recipient_drift(workspace, &relative, &candidate)?;

    std::fs::rename(&candidate, &absolute).map_err(|cause| Error::FileUnwritable {
        path: absolute.display().to_string(),
        cause,
    })?;
    scratch::keep_dirs();

    let identity = workspace.git().author_identity(workspace.root())?;
    git::commit_written_files(
        workspace.git(),
        workspace.vault_root(),
        progress,
        subject,
        std::slice::from_ref(&relative),
        Some(&identity),
    )?;
    if workspace.vault_root() != workspace.root() {
        progress.write(&workspace.disclose_lock_bump());
    }
    Ok(0)
}

/// Beside the target, so the move into place is an atomic rename rather than a
/// cross-filesystem copy that can be interrupted half-written, and keeping the
/// `.yaml` suffix, because sops reads a document's format off the extension and
/// would parse a `*.tmp.1234` YAML file as JSON.
pub(crate) fn candidate_path(absolute: &Path) -> PathBuf {
    let mut name = absolute.as_os_str().to_owned();
    name.push(format!(".safix-tmp.{}.yaml", std::process::id()));
    PathBuf::from(name)
}

/// The states in which a commit would mean something other than what its
/// message says.
///
/// Judged before the operator is asked for anything: a refusal that arrives
/// after they have typed a secret twice is a refusal that costs them the
/// typing. Takes the whole set of `(root, relative)` pairs one operation is
/// about to touch, checked in order and refusing on the first failure before
/// any of them is written — design V4's preflight, generalized from a single
/// vault-root check to cover a cross-root operation's declaration-root paths
/// too, in the same call.
///
/// # Errors
///
/// [`Error::MidOperation`], [`Error::ConflictEntries`] or
/// [`Error::UncommittedChanges`].
pub fn refuse_bad_repository_state(workspace: &Workspace, touches: &[(&Path, &str)]) -> Result<()> {
    let git = workspace.git();
    for &(root, relative) in touches {
        if let Some(operation) = git.operation_in_progress(root)? {
            return Err(Error::MidOperation {
                state: operation.state,
                marker: operation.marker.display().to_string(),
            });
        }
        if git.has_conflict_entries(root, relative)? {
            return Err(Error::ConflictEntries {
                file: relative.to_owned(),
            });
        }
        let status = git.status_of(root, relative)?;
        let status = status.trim_end_matches('\n');
        if !status.is_empty() {
            return Err(Error::UncommittedChanges {
                file: relative.to_owned(),
                status: status.to_owned(),
            });
        }
    }
    Ok(())
}

/// Refuse a candidate document whose recipients are not the audience the
/// declarations name for the path it is about to occupy.
///
/// Judged on the candidate rather than on the file in place, which is also what
/// covers the new-file case: the recipients there came from a `.sops.yaml`
/// creation rule that may itself be stale.
///
/// Both sides are the ones `check` uses — the declared side is
/// `flake.safix.lib.audiences` and the actual side is
/// [`document::recipients_of`] — so the two cannot disagree about a file this
/// wrote and that then reports as drifted.
///
/// # Errors
///
/// [`Error::NoAudienceForFile`], [`Error::CandidateRecipientsUnreadable`] or
/// [`Error::RecipientDrift`].
pub fn refuse_recipient_drift(
    workspace: &Workspace,
    relative: &str,
    candidate: &Path,
) -> Result<()> {
    let declared = workspace
        .audiences()?
        .for_file(relative)
        .ok_or_else(|| Error::NoAudienceForFile {
            file: relative.to_owned(),
        })?
        .recipients
        .clone();

    let text = std::fs::read_to_string(candidate).map_err(|cause| Error::FileUnreadable {
        path: candidate.display().to_string(),
        cause,
    })?;
    let actual =
        document::recipients_of(&text).map_err(|cause| Error::CandidateRecipientsUnreadable {
            file: relative.to_owned(),
            cause: Box::new(cause),
        })?;

    let found = document::drift(&actual, &declared);
    if found.is_empty() {
        return Ok(());
    }
    Err(Error::RecipientDrift {
        file: relative.to_owned(),
        extra: found.extra,
        missing: found.missing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_candidate_sits_beside_the_target_and_keeps_the_yaml_suffix() {
        let candidate = candidate_path(Path::new("/srv/fleet/secrets/alice/secrets.yaml"));
        assert_eq!(
            candidate.parent(),
            Path::new("/srv/fleet/secrets/alice/secrets.yaml").parent()
        );
        assert_eq!(
            candidate.extension().and_then(std::ffi::OsStr::to_str),
            Some("yaml")
        );
        assert!(
            candidate
                .to_string_lossy()
                .starts_with("/srv/fleet/secrets/alice/secrets.yaml.safix-tmp.")
        );
    }
}

/// Task 6.9's three preflight cases, at [`refuse_bad_repository_state`]'s own
/// level rather than through a full command: both roots clean, the vault
/// root dirty with the declaration root clean, and the reverse. Each dirty
/// case is an untracked file sitting at the path the touch names — the
/// simplest state [`crate::git::Git::status_of`]'s `--untracked-files=all`
/// already reports as non-empty.
///
/// Task 6.10's drill — narrowing the touch list to the declaration root
/// alone turns the vault-dirty case green when it should refuse — was
/// observed manually rather than encoded as a standing test: calling
/// `refuse_bad_repository_state` with only `[(declaration_root, "user.nix")]`
/// in [`the_vault_root_being_dirty_is_refused_naming_it`]'s own fixture
/// returns `Ok(())`, which is the evidence the vault-root check is
/// independently load-bearing rather than redundant with the declaration
/// root's.
#[cfg(test)]
mod preflight_tests {
    use std::process::Command;

    use super::*;
    use crate::git::Git;
    use crate::nix::Nix;
    use crate::sops::Sops;

    fn init_repo(root: &Path) {
        std::fs::create_dir_all(root).expect("a temporary directory can be made");
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q"])
            .status()
            .expect("git can be run");
        assert!(status.success());
    }

    fn workspace_at(root: &Path, vault_root: &Path) -> Workspace {
        Workspace::at(
            root.to_path_buf(),
            vault_root.to_path_buf(),
            Git::default(),
            Nix::from_environment(),
            Sops::from_environment(),
        )
    }

    struct TwoRoots {
        scratch: PathBuf,
        vault: PathBuf,
        declaration: PathBuf,
    }

    impl TwoRoots {
        fn new(label: &str) -> Self {
            let scratch = std::env::temp_dir()
                .join(format!("safix-preflight-{label}-{}", std::process::id()));
            let vault = scratch.join("vault");
            let declaration = scratch.join("declaration");
            init_repo(&vault);
            init_repo(&declaration);
            Self {
                scratch,
                vault,
                declaration,
            }
        }
    }

    impl Drop for TwoRoots {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.scratch);
        }
    }

    #[test]
    fn both_roots_clean_admits_the_touches() {
        let fixture = TwoRoots::new("clean");
        let workspace = workspace_at(&fixture.declaration, &fixture.vault);
        refuse_bad_repository_state(
            &workspace,
            &[
                (&fixture.vault, "secrets/opaque.yaml"),
                (&fixture.declaration, "user.nix"),
            ],
        )
        .expect("two clean roots admit the touches");
    }

    #[test]
    fn the_vault_root_being_dirty_is_refused_naming_it() {
        let fixture = TwoRoots::new("vault-dirty");
        std::fs::write(fixture.vault.join("secrets-opaque.yaml"), "stray\n")
            .expect("an untracked file can be written");
        let workspace = workspace_at(&fixture.declaration, &fixture.vault);
        let error = refuse_bad_repository_state(
            &workspace,
            &[
                (&fixture.vault, "secrets-opaque.yaml"),
                (&fixture.declaration, "user.nix"),
            ],
        )
        .expect_err("an untracked file at the touched path refuses");
        assert!(
            matches!(&error, Error::UncommittedChanges { file, .. } if file == "secrets-opaque.yaml"),
            "the refusal names the vault-root path: {error:?}"
        );
    }

    #[test]
    fn the_declaration_root_being_dirty_is_refused_naming_it() {
        let fixture = TwoRoots::new("declaration-dirty");
        std::fs::write(fixture.declaration.join("user.nix"), "{ }\n")
            .expect("an untracked file can be written");
        let workspace = workspace_at(&fixture.declaration, &fixture.vault);
        let error = refuse_bad_repository_state(
            &workspace,
            &[
                (&fixture.vault, "secrets-opaque.yaml"),
                (&fixture.declaration, "user.nix"),
            ],
        )
        .expect_err("an untracked file at the touched path refuses");
        assert!(
            matches!(&error, Error::UncommittedChanges { file, .. } if file == "user.nix"),
            "the refusal names the declaration-root path: {error:?}"
        );
    }
}
