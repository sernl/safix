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
    scratch::set_floor(workspace.root());
    let _guard = scratch::Guard;

    let placement = workspace.resolve(user, name)?;
    let relative = placement.file.clone();
    let key = placement.key.clone();

    refuse_bad_repository_state(workspace, &relative)?;

    let absolute = workspace.absolute(&relative);
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
        {
            let _quiet = scratch::quiet();
            workspace.sops().create_empty_document(
                workspace.root(),
                &relative,
                &key,
                &candidate,
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

    git::commit_written_files(
        workspace.git(),
        workspace.root(),
        progress,
        &format!("chore(safix): set {name} for {user}"),
        std::slice::from_ref(&relative),
    )?;
    Ok(0)
}

/// Beside the target, so the move into place is an atomic rename rather than a
/// cross-filesystem copy that can be interrupted half-written, and keeping the
/// `.yaml` suffix, because sops reads a document's format off the extension and
/// would parse a `*.tmp.1234` YAML file as JSON.
fn candidate_path(absolute: &Path) -> PathBuf {
    let mut name = absolute.as_os_str().to_owned();
    name.push(format!(".safix-tmp.{}.yaml", std::process::id()));
    PathBuf::from(name)
}

/// The states in which a commit would mean something other than "this value was
/// set".
///
/// Judged before the operator is asked for anything: a refusal that arrives
/// after they have typed a secret twice is a refusal that costs them the typing.
///
/// # Errors
///
/// [`Error::MidOperation`], [`Error::ConflictEntries`] or
/// [`Error::UncommittedChanges`].
pub fn refuse_bad_repository_state(workspace: &Workspace, relative: &str) -> Result<()> {
    let git = workspace.git();
    let root = workspace.root();

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
    if status.is_empty() {
        return Ok(());
    }
    Err(Error::UncommittedChanges {
        file: relative.to_owned(),
        status: status.to_owned(),
    })
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
        let candidate = candidate_path(Path::new("/srv/fleet/secrets/ana/secrets.yaml"));
        assert_eq!(
            candidate.parent(),
            Path::new("/srv/fleet/secrets/ana/secrets.yaml").parent()
        );
        assert_eq!(
            candidate.extension().and_then(std::ffi::OsStr::to_str),
            Some("yaml")
        );
        assert!(
            candidate
                .to_string_lossy()
                .starts_with("/srv/fleet/secrets/ana/secrets.yaml.safix-tmp.")
        );
    }
}
