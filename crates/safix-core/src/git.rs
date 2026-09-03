//! The git driver.
//!
//! Two jobs. Finding the repository, which every subcommand needs because every
//! path safix handles is repository-relative. And, for the writing
//! subcommands, refusing the states in which a commit would mean something
//! other than what its message says, then staging and committing exactly the
//! paths that were written and no others.
//!
//! Staging and committing are reached by [`set`](crate::set),
//! [`generate`](crate::generate) and [`adduser`](crate::adduser), each through
//! [`commit_written_files`] and each naming exactly the paths it wrote: the one
//! file `set` moved into place, the one-or-more outputs of a generator run, and
//! the scaffold together with the policy regenerated beside it. Nothing here
//! commits a path a caller did not name, which is what keeps a message naming
//! one secret from carrying somebody else's staged change.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::progress::{Progress, note};

/// The states a commit must not be made in.
///
/// Mid-rebase and mid-merge because a partial commit is rejected outright
/// during a merge and silently reorders history during a rebase. The names are
/// the paths git leaves in its own directory while each is in progress, and the
/// refusal quotes the one it found.
const IN_PROGRESS_MARKERS: [&str; 5] = [
    "rebase-merge",
    "rebase-apply",
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
];

/// The git binary, and the repository it is pointed at.
#[derive(Debug, Clone)]
pub struct Git {
    program: PathBuf,
}

impl Default for Git {
    fn default() -> Self {
        Self::from_environment()
    }
}

impl Git {
    /// The binary `SAFIX_GIT` names, or `git`.
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            program: std::env::var_os("SAFIX_GIT")
                .map_or_else(|| PathBuf::from("git"), PathBuf::from),
        }
    }

    /// The repository this process is inside, or the one `SAFIX_REPO_ROOT`
    /// names.
    ///
    /// # Errors
    ///
    /// [`Error::NotInsideRepository`] when git reports none.
    pub fn repository_root(&self) -> Result<PathBuf> {
        if let Some(root) = std::env::var_os("SAFIX_REPO_ROOT") {
            return Ok(PathBuf::from(root));
        }
        let root = self.capture(
            Path::new("."),
            &["rev-parse".into(), "--show-toplevel".into()],
        )?;
        let root = root.trim_end_matches('\n');
        if root.is_empty() {
            return Err(Error::NotInsideRepository);
        }
        Ok(PathBuf::from(root))
    }

    /// The state a commit would be made into, when it is one that forbids it.
    ///
    /// # Errors
    ///
    /// [`Error::GitUnavailable`] when git cannot be run.
    pub fn operation_in_progress(&self, root: &Path) -> Result<Option<InProgress>> {
        let git_dir = self.capture(root, &["rev-parse".into(), "--absolute-git-dir".into()])?;
        let git_dir = PathBuf::from(git_dir.trim_end_matches('\n'));
        Ok(IN_PROGRESS_MARKERS
            .into_iter()
            .map(|marker| (marker, git_dir.join(marker)))
            .find(|(_, path)| path.exists())
            .map(|(marker, path)| InProgress {
                state: marker,
                marker: path,
            }))
    }

    /// Whether one path has unmerged conflict entries in the index.
    ///
    /// # Errors
    ///
    /// [`Error::GitUnavailable`] when git cannot be run.
    pub fn has_conflict_entries(&self, root: &Path, relative: &str) -> Result<bool> {
        let listed = self.capture(
            root,
            &["ls-files".into(), "-u".into(), "--".into(), relative.into()],
        )?;
        Ok(!listed.is_empty())
    }

    /// The porcelain status of one path, empty when it is clean.
    ///
    /// Standard error is discarded rather than inherited: the porcelain output
    /// is the whole signal, and a warning on git's stderr would otherwise print
    /// in the middle of a refusal, where it reads as part of the explanation the
    /// operator is meant to act on.
    ///
    /// # Errors
    ///
    /// [`Error::GitUnavailable`] when git cannot be run.
    pub fn status_of(&self, root: &Path, relative: &str) -> Result<String> {
        let output = self
            .command(root)
            .arg("status")
            .arg("--porcelain")
            .arg("--untracked-files=all")
            .arg("--")
            .arg(relative)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|cause| self.unavailable(cause))?;
        String::from_utf8(output.stdout).map_err(|cause| Error::GitOutputNotText {
            cause: cause.to_string(),
        })
    }

    /// Stage exactly these paths.
    ///
    /// # Errors
    ///
    /// [`Error::GitCommandFailed`] when git refuses.
    pub fn stage(&self, root: &Path, paths: &[String]) -> Result<()> {
        let mut arguments: Vec<OsString> = vec!["add".into(), "--".into()];
        arguments.extend(paths.iter().map(OsString::from));
        self.run(root, &arguments)
    }

    /// Whether anything is staged for these paths.
    ///
    /// # Errors
    ///
    /// [`Error::GitUnavailable`] when git cannot be run.
    pub fn has_staged_changes(&self, root: &Path, paths: &[String]) -> Result<bool> {
        let mut command = self.command(root);
        command
            .arg("diff")
            .arg("--cached")
            .arg("--quiet")
            .arg("--")
            .args(paths)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let status = command.status().map_err(|cause| self.unavailable(cause))?;
        Ok(!status.success())
    }

    /// Commit exactly these paths under this message, leaving anything else
    /// staged where it is.
    ///
    /// # Errors
    ///
    /// [`Error::GitCommandFailed`] when git refuses.
    pub fn commit_paths(&self, root: &Path, message: &str, paths: &[String]) -> Result<()> {
        let mut arguments: Vec<OsString> = vec![
            "commit".into(),
            "-q".into(),
            "-m".into(),
            message.into(),
            "--".into(),
        ];
        arguments.extend(paths.iter().map(OsString::from));
        self.run(root, &arguments)
    }

    /// The abbreviated object name of the current commit.
    ///
    /// # Errors
    ///
    /// [`Error::GitCommandFailed`] when git refuses.
    pub fn head_short(&self, root: &Path) -> Result<String> {
        let head = self.capture(root, &["rev-parse".into(), "--short".into(), "HEAD".into()])?;
        Ok(head.trim_end_matches('\n').to_owned())
    }

    /// The top level of the git repository containing `path`.
    ///
    /// Unlike [`repository_root`](Self::repository_root), this never consults
    /// `SAFIX_REPO_ROOT`: it asks git about a path named explicitly, for a
    /// caller verifying a second root — the vault — rather than discovering
    /// the process's own.
    ///
    /// # Errors
    ///
    /// [`Error::GitUnavailable`] when git cannot be run, and
    /// [`Error::GitCommandFailed`] when it runs and refuses.
    pub fn show_toplevel(&self, path: &Path) -> Result<PathBuf> {
        let output = self.capture(path, &["rev-parse".into(), "--show-toplevel".into()])?;
        Ok(PathBuf::from(output.trim_end_matches('\n')))
    }

    /// Who a commit made here would be authored by.
    ///
    /// `git var GIT_AUTHOR_IDENT` rather than two reads of `git config`, because
    /// the question is who the commit will name and that is what this answers:
    /// one resolution of `user.name` and `user.email` through this repository's
    /// own configuration, its includes and its worktree, and through the
    /// environment where an invocation overrides them. Two `config` reads would
    /// answer a narrower question and could disagree with the commit that
    /// follows, which is the one thing a delegation check must not do.
    ///
    /// The value is git's ident line — `Name <email> <timestamp> <zone>` — and
    /// the timestamp is dropped: it is the moment this was asked rather than a
    /// property of the identity.
    ///
    /// # Errors
    ///
    /// [`Error::GitCommandFailed`] when git cannot resolve an identity at all,
    /// which is the state a commit would also be refused in, and
    /// [`Error::GitOutputNotText`] when what it printed is not text.
    pub fn author_identity(&self, root: &Path) -> Result<Identity> {
        let ident = self.capture(root, &["var".into(), "GIT_AUTHOR_IDENT".into()])?;
        Ok(parse_identity(ident.trim_end_matches('\n')))
    }

    fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(&self.program);
        command.arg("-C").arg(root);
        command
    }

    fn unavailable(&self, cause: std::io::Error) -> Error {
        Error::GitUnavailable {
            program: self.program.display().to_string(),
            cause,
        }
    }

    fn capture(&self, root: &Path, arguments: &[OsString]) -> Result<String> {
        let output = self
            .command(root)
            .args(arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|cause| self.unavailable(cause))?;
        if !output.status.success() {
            return Err(Error::GitCommandFailed {
                arguments: describe(arguments),
            });
        }
        String::from_utf8(output.stdout).map_err(|cause| Error::GitOutputNotText {
            cause: cause.to_string(),
        })
    }

    fn run(&self, root: &Path, arguments: &[OsString]) -> Result<()> {
        let status = self
            .command(root)
            .args(arguments)
            .stdin(Stdio::null())
            .status()
            .map_err(|cause| self.unavailable(cause))?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::GitCommandFailed {
                arguments: describe(arguments),
            })
        }
    }
}

/// Stage these paths and commit them alone, or say that there was nothing to
/// commit.
///
/// One decision point, and it is git's rather than a byte comparison of our own:
/// `sops set --idempotent` leaves an unchanged value's file untouched, so a
/// re-run moves a byte-identical file into place and git has nothing staged.
///
/// Scoped to the paths written on both halves. An unscoped staged-changes test
/// would read another path's staged change as this command's work and commit on
/// a run that wrote nothing, and an unscoped commit would carry that path into a
/// commit whose message names one secret; committing the paths alone leaves the
/// rest of the index staged where it was.
///
/// More than one path only ever arrives from one generator writing more than one
/// output. A keypair split across two commits is a state in which the tree holds
/// a private half and a public half that do not match, so the outputs of one run
/// go in together or not at all.
///
/// # Errors
///
/// [`Error::GitCommandFailed`] when git refuses, and [`Error::GitUnavailable`]
/// when it cannot be run.
pub fn commit_written_files(
    git: &Git,
    root: &Path,
    progress: &dyn Progress,
    message: &str,
    paths: &[String],
) -> Result<()> {
    git.stage(root, paths)?;
    if !git.has_staged_changes(root, paths)? {
        note(
            progress,
            "unchanged — the file already holds this value, so nothing was committed.",
        );
        return Ok(());
    }
    git.commit_paths(root, message, paths)?;
    note(
        progress,
        &format!(
            "committed {} — the value is not in the message.",
            git.head_short(root)?
        ),
    );
    Ok(())
}

fn describe(arguments: &[OsString]) -> String {
    arguments
        .iter()
        .map(|argument| OsStr::to_string_lossy(argument).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Who a commit would be authored by.
///
/// Both halves are carried because both reach the commit, and a refusal about an
/// identity has to print what the operator would see in `git log` rather than
/// half of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// `user.name`, as this repository resolves it.
    pub name: String,
    /// `user.email`, as this repository resolves it.
    pub email: String,
}

/// Split git's ident line into its name and its address.
///
/// The shape is `Name <email> <timestamp> <zone>`, and the address is taken from
/// between the angle brackets rather than by splitting on whitespace: a name may
/// hold spaces and an address may not hold a bracket, so the brackets are the one
/// unambiguous boundary. A line that carries no brackets leaves the address empty
/// rather than being guessed at from the words around it.
fn parse_identity(ident: &str) -> Identity {
    let (name, rest) = ident.split_once('<').unwrap_or((ident, ""));
    Identity {
        name: name.trim().to_owned(),
        email: rest
            .split_once('>')
            .map_or(String::new(), |(email, _)| email.trim().to_owned()),
    }
}

/// A git operation that is part-way through, and the marker that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InProgress {
    /// The state's name, as `mid-<state>` reads in the refusal.
    pub state: &'static str,
    /// The path whose existence is the evidence.
    pub marker: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ident_line_splits_at_the_brackets_and_keeps_a_name_with_spaces() {
        assert_eq!(
            parse_identity("Alice Example <alice@example.com> 1766000000 +0000"),
            Identity {
                name: String::from("Alice Example"),
                email: String::from("alice@example.com"),
            }
        );
    }

    #[test]
    fn an_ident_line_with_no_address_leaves_it_empty_rather_than_guessing() {
        assert_eq!(
            parse_identity("alice"),
            Identity {
                name: String::from("alice"),
                email: String::new(),
            }
        );
    }

    /// `author_identity` reads the configuration of the root it is asked
    /// about, not some other root — the property `delegation.rs`'s call site
    /// (`workspace.git().author_identity(workspace.root())`) depends on now
    /// that `Workspace` carries a second, vault root: two repositories with
    /// different configured identities read apart.
    #[test]
    fn author_identity_reads_the_named_roots_own_configuration() {
        let scratch =
            std::env::temp_dir().join(format!("safix-git-identity-{}", std::process::id()));
        let first = scratch.join("first");
        let second = scratch.join("second");
        for (root, name, email) in [
            (&first, "Alice Example", "alice@example.com"),
            (&second, "Bob Example", "bob@example.com"),
        ] {
            std::fs::create_dir_all(root).expect("a temporary directory can be made");
            for arguments in [
                vec!["init", "-q"],
                vec!["config", "user.name", name],
                vec!["config", "user.email", email],
            ] {
                let status = Command::new("git")
                    .arg("-C")
                    .arg(root)
                    .args(&arguments)
                    .env("HOME", &scratch)
                    .env_remove("GIT_AUTHOR_NAME")
                    .env_remove("GIT_AUTHOR_EMAIL")
                    .env_remove("GIT_COMMITTER_NAME")
                    .env_remove("GIT_COMMITTER_EMAIL")
                    .status()
                    .expect("git can be run");
                assert!(status.success(), "git {arguments:?} failed");
            }
        }

        let git = Git {
            program: PathBuf::from("git"),
        };
        assert_eq!(
            git.author_identity(&first).expect("the identity resolves"),
            Identity {
                name: "Alice Example".to_owned(),
                email: "alice@example.com".to_owned(),
            }
        );
        assert_eq!(
            git.author_identity(&second).expect("the identity resolves"),
            Identity {
                name: "Bob Example".to_owned(),
                email: "bob@example.com".to_owned(),
            }
        );

        std::fs::remove_dir_all(&scratch).expect("the fixture can be removed");
    }
}
