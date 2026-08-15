//! Authoring one value in the operator's own editor.
//!
//! `edit` is a verb rather than an option on `set`, and the reason is custody
//! rather than taste. `set` never reads the existing value: it takes a new one
//! from a stream and writes it, and its requirement is that the value arrives on
//! a stream and never touches a filesystem. `edit` must decrypt the existing
//! value, materialize it as a file, hand that file to a program safix does not
//! control, and read the result back. Those are different enough to deserve
//! different refusals, and an option would make custody a function of a flag.
//!
//! What the two share is the write. A changed, non-empty buffer goes through
//! [`crate::set::run`] — the same sequence, the same candidate-and-rename, the
//! same recipient-drift refusal — rather than through a second write path that
//! would have to be kept in step with it.
//!
//! # The buffer
//!
//! The file handed to the editor lives inside the private staging root
//! [`crate::staging`] governs, so it is mode `0600` on a filesystem verified to
//! be memory-backed, and it is shredded however the run ends — along with
//! whatever the editor left beside it, because what is removed is the root and
//! not the one file safix created.
//!
//! The limit that leaves is real and is stated rather than left to be
//! discovered: an editor configured to write undo history, swap files or backups
//! to a directory of its own has put plaintext where safix does not look.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};
use crate::progress::{Progress, log, note};
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::sops::document;
use crate::staging::Staging;
use crate::workspace::Workspace;

/// The variable consulted first, then the one consulted second.
///
/// There is no third. safix opens no editor of its own choosing: dropping an
/// operator who has never used it into `vi`, with a secret in the buffer,
/// produces either an accidental write or an accidental abandonment — and
/// nothing here can tell the two apart, so the value stored would be one nobody
/// chose. sops falls back; that is the precedent being declined.
pub const PREFERRED: &str = "VISUAL";

/// The variable consulted when [`PREFERRED`] is unset.
pub const FALLBACK: &str = "EDITOR";

/// How an editing run was asked for.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Accept that the buffer will be staged on a disk-backed filesystem.
    pub allow_disk_staging: bool,
}

/// The editor the operator chose, split into a program and its arguments.
///
/// Split on whitespace and executed directly rather than through a shell, so
/// `EDITOR="code --wait"` works and `EDITOR="rm -rf /; vi"` is a program named
/// `rm` with three arguments rather than two commands. The staged file's path is
/// appended as the last argument: the *path* reaches argv, and the *value* does
/// not.
#[derive(Debug, Clone)]
pub struct Editor {
    program: String,
    arguments: Vec<String>,
}

impl Editor {
    /// Read the editor out of the environment.
    ///
    /// # Errors
    ///
    /// [`Error::NoEditor`] when neither variable names anything, raised before
    /// anything is decrypted or staged.
    pub fn from_environment() -> Result<Self> {
        Self::choose(std::env::var(PREFERRED).ok(), std::env::var(FALLBACK).ok())
    }

    /// The selection itself, over two values rather than over the environment.
    ///
    /// Split out because a process's environment is process-wide, so a test that
    /// set it would race every other test in this crate. The rule is here and
    /// the reading of it is one line above.
    ///
    /// # Errors
    ///
    /// [`Error::NoEditor`] when neither names a non-blank command.
    fn choose(preferred: Option<String>, fallback: Option<String>) -> Result<Self> {
        let chosen = [preferred, fallback]
            .into_iter()
            .flatten()
            .find(|value| !value.trim().is_empty())
            .ok_or(Error::NoEditor)?;

        let mut words = chosen.split_whitespace().map(str::to_owned);
        let program = words.next().ok_or(Error::NoEditor)?;
        Ok(Self {
            program,
            arguments: words.collect(),
        })
    }

    /// Open the editor on a staged path, and wait for it.
    ///
    /// The three streams are inherited, because a terminal editor without a
    /// terminal is an editor that cannot be used.
    ///
    /// # Errors
    ///
    /// [`Error::EditorFailed`] when it exits non-zero or cannot be run at all.
    fn open(&self, path: &Path) -> Result<()> {
        let status = Command::new(&self.program)
            .args(&self.arguments)
            .arg(path)
            .status()
            .map_err(|_| Error::EditorFailed { status: 127 })?;

        match status.code() {
            Some(0) => Ok(()),
            Some(code) => Err(Error::EditorFailed { status: code }),
            None => Err(Error::EditorFailed { status: 1 }),
        }
    }
}

/// A value the operator has already edited, handed to `set`'s write path.
///
/// The editing has happened by the time this is constructed, which is what makes
/// the unchanged and the emptied outcomes decidable before any write begins.
struct Edited(Secret);

impl ValueSource for Edited {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        Ok(std::mem::replace(&mut self.0, Secret::empty()))
    }
}

/// Edit one value in the operator's editor, and commit the file holding it.
///
/// Four outcomes, and three of them write nothing:
///
/// - the editor exits non-zero: nothing is written, nothing is committed, and
///   the refusal names the status;
/// - the buffer comes back byte-identical: nothing is written and nothing is
///   committed, which matches the idempotent re-run `set` already has;
/// - the buffer comes back empty: refused with the same refusal an empty value
///   produces anywhere else, because an empty value is the state a truncated
///   write leaves behind;
/// - the buffer comes back changed and non-empty: written through `set`'s own
///   path.
///
/// An entry that holds no value yet opens on an empty buffer, so `edit` is
/// usable for authoring rather than only for amendment.
///
/// # Errors
///
/// [`Error::NoEditor`] when no editor is chosen, [`Error::EditorFailed`] when it
/// refuses, [`Error::EmptyValue`] for an emptied buffer, and every refusal
/// [`crate::set::run`] can raise.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    user: &str,
    name: &str,
    options: Options,
) -> Result<i32> {
    // Before anything is decrypted or staged: a refusal that arrived after the
    // value was in a file would have materialized plaintext for a run that was
    // never going to write.
    let editor = Editor::from_environment()?;

    let placement = workspace.resolve(user, name)?;
    if let Some(path) = &placement.public {
        return Err(Error::PublicNotEditable {
            name: name.to_owned(),
            path: path.clone(),
        });
    }
    let relative = placement.file.clone();
    let key = placement.key.clone();

    log(
        progress,
        &format!("safix: editing {name} for {user} -> {relative} [{key}]"),
    );

    let existing = read_existing(workspace, &relative, &key)?;

    let staging = Staging::establish(options.allow_disk_staging)?;
    let buffer = staging.write(Path::new(name), &existing)?;
    editor.open(&buffer)?;
    let edited = staging.read(&buffer)?;

    if edited.is_empty() {
        return Err(Error::EmptyValue);
    }
    if edited.equals(&existing) {
        note(
            progress,
            "unchanged — the buffer came back byte-identical, so nothing was written.",
        );
        return Ok(0);
    }

    // The staging root is dropped here rather than held across the write: the
    // value is in memory by now, and every byte the editor touched is shredded
    // before sops is invoked.
    drop(staging);

    set::run(workspace, progress, &mut Edited(edited), user, name)
}

/// What the entry holds today, or nothing when it holds nothing yet.
///
/// A file that does not exist, and a key that is absent or empty inside one that
/// does, are all "nothing yet" — the same three states `generate` treats as an
/// output with no value, answered the same way so the two cannot disagree about
/// one entry.
fn read_existing(workspace: &Workspace, relative: &str, key: &str) -> Result<Secret> {
    let absolute = workspace.absolute(relative);
    if !absolute.exists() {
        return Ok(Secret::empty());
    }
    let Some(text) = workspace.read_relative(relative)? else {
        return Ok(Secret::empty());
    };
    if document::keys_of(&text)?
        .get(key)
        .is_none_or(|state| state.empty)
    {
        return Ok(Secret::empty());
    }
    Ok(workspace.sops().decrypt_key(&absolute, key)?.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chosen(preferred: Option<&str>, fallback: Option<&str>) -> Result<Editor> {
        Editor::choose(preferred.map(str::to_owned), fallback.map(str::to_owned))
    }

    #[test]
    fn the_preferred_variable_wins() {
        let Ok(editor) = chosen(Some("preferred"), Some("fallback")) else {
            unreachable!("a set variable names an editor");
        };
        assert_eq!(editor.program, "preferred");
    }

    #[test]
    fn the_fallback_is_used_when_the_preferred_is_unset_or_blank() {
        assert_eq!(
            chosen(None, Some("fallback")).map(|e| e.program).ok(),
            Some("fallback".to_owned())
        );
        assert_eq!(
            chosen(Some("   "), Some("fallback"))
                .map(|e| e.program)
                .ok(),
            Some("fallback".to_owned())
        );
    }

    #[test]
    fn neither_set_is_a_refusal_rather_than_a_choice() {
        assert!(matches!(chosen(None, None), Err(Error::NoEditor)));
        assert!(matches!(chosen(Some(""), Some("  ")), Err(Error::NoEditor)));
    }

    #[test]
    fn a_command_with_arguments_is_split_and_not_handed_to_a_shell() {
        let Ok(editor) = chosen(Some("code --wait --new-window"), None) else {
            unreachable!("a set variable names an editor");
        };
        assert_eq!(editor.program, "code");
        assert_eq!(editor.arguments, ["--wait", "--new-window"]);
    }

    #[test]
    fn there_is_no_fallback_to_a_named_program() {
        let source = include_str!("edit.rs");
        for named in ["\"vi\"", "\"vim\"", "\"nano\"", "\"emacs\""] {
            assert!(
                !source.contains(named),
                "a fallback editor {named} was added; the refusal is the decision"
            );
        }
    }
}
