//! Editing a group's membership, which is a declaration and never a value.
//!
//! One subject into or out of one group's `members`, written the way
//! [`enroll`](crate::enroll) writes a recovery recipient: a text edit through
//! [`declaration`], parsed by the real parser before anything is staged, the
//! recipient policy regenerated from the declarations that edit implies, and the
//! two committed together. Nothing here encrypts, decrypts or mints, and nothing
//! here re-wraps a file — that is [`fix`](crate::fix), and a membership change is
//! a reason to run it rather than something this does on the operator's behalf.
//!
//! # Why a verb at all
//!
//! Because the disclosures a hand edit owes are easy to not make. Membership
//! growth is a re-wrap and membership shrink is a revocation, and the shrink's
//! not-retroactive disclosure is the same one every other narrowing in safix
//! carries: a subject that has been in a group has read what that group's
//! audience could read, and no re-wrap unreads it. This verb prints it, and names
//! the report that will carry the shrink.
//!
//! # Where the declaration is
//!
//! `safix/groups/<group>.nix`, which is the layout [`adduser`](crate::adduser)
//! chooses for a person's record and is chosen here for the same reason: safix
//! imposes no layout on declarations, an attrset option merges from anywhere, and
//! an edit still has to have a file to make. A group declared somewhere else is a
//! supported declaration and not one this verb can edit; the refusal says so and
//! names the path it looked at.
//!
//! # What is refused
//!
//! A group or a subject the fleet does not declare, before anything is read: a
//! membership naming either is refused at the next evaluation, so writing one
//! would commit a tree that no longer resolves. And an out-of-scope actor, where
//! the group is one an organization's silo declarations cover — see
//! [`delegation`], and note that a group no silo set names is editable by whoever
//! can commit, exactly as it was before delegation was recorded.

use crate::error::{Error, Result};
use crate::nix::Attribute;
use crate::progress::{Progress, log, note};
use crate::workspace::Workspace;
use crate::{declaration, delegation, git};

/// Which edit an invocation asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// One subject into the group's `members`.
    Add,
    /// One subject out of it.
    Remove,
}

impl Act {
    /// The word the report and the commit use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }
}

/// The file a group's declaration is edited in.
#[must_use]
pub fn declaration_path(group: &str) -> String {
    format!("safix/groups/{group}.nix")
}

/// Add or remove one subject, regenerate the policy, and commit the two.
///
/// # Errors
///
/// [`Error::UnknownGroup`], [`Error::UnknownSubject`],
/// [`Error::ActorUndeclared`], [`Error::ScaffoldOutOfScope`],
/// [`Error::NoGroupDeclaration`], [`Error::Unparsable`],
/// [`Error::PolicyEvalAfterScaffold`] and [`Error::FileUnwritable`], in that
/// order of reachability.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    act: Act,
    group: &str,
    subject: &str,
) -> Result<()> {
    refuse_undeclared(workspace, group, subject)?;

    let scope = delegation::over_group(workspace, group)?;
    scope.announce(progress);

    let relative = declaration_path(group);
    let absolute = workspace.absolute(&relative);
    let original =
        workspace
            .read_relative(&relative)?
            .ok_or_else(|| Error::NoGroupDeclaration {
                group: group.to_owned(),
                file: relative.clone(),
            })?;

    let edited = match act {
        Act::Add => match declaration::add_group_member(&original, group, subject) {
            declaration::Edit::Inserted(edited) => Some(edited),
            declaration::Edit::AlreadyPresent => {
                note(
                    progress,
                    &format!("{subject} is already a member of {group}; nothing was written."),
                );
                None
            }
            declaration::Edit::NoAnchor => {
                return Err(Error::NoGroupDeclaration {
                    group: group.to_owned(),
                    file: relative,
                });
            }
        },
        Act::Remove => match declaration::remove_group_member(&original, group, subject) {
            declaration::Removal::Removed(edited) => Some(edited),
            declaration::Removal::NotPresent => {
                note(
                    progress,
                    &format!("{subject} is not a member of {group}; nothing was removed."),
                );
                None
            }
            declaration::Removal::NoAnchor => {
                return Err(Error::NoGroupDeclaration {
                    group: group.to_owned(),
                    file: relative,
                });
            }
        },
    };

    let Some(edited) = edited else {
        return Ok(());
    };

    log(
        progress,
        &format!("safix: {} {subject} in {relative}", act.as_str()),
    );
    std::fs::write(&absolute, &edited).map_err(|cause| Error::FileUnwritable {
        path: absolute.display().to_string(),
        cause,
    })?;

    // Parsed before anything is staged, and put back when it does not parse: the
    // file was a valid declaration a moment ago and the edit is this module's, so
    // an edit that does not parse is this module's to undo.
    if !workspace.nix().parses(&absolute) {
        let _ = std::fs::write(&absolute, &original);
        return Err(Error::Unparsable {
            path: absolute.display().to_string(),
        });
    }

    // Staged before the policy is regenerated, for the reason `adduser` states at
    // length: an evaluation reads the files git tracks, so regenerating first
    // would write the policy of the membership as it stood before this edit.
    workspace
        .git()
        .stage(workspace.root(), std::slice::from_ref(&relative))?;
    regenerate_policy(workspace)?;

    let mut message = format!(
        "feat(safix): {} {subject} {} the {group} group",
        act.as_str(),
        match act {
            Act::Add => "to",
            Act::Remove => "from",
        }
    );
    if let Some(context) = scope.commit_context() {
        message.push_str("\n\n");
        message.push_str(&context);
    }
    git::commit_written_files(
        workspace.git(),
        workspace.root(),
        progress,
        &message,
        &[relative.clone(), String::from(".sops.yaml")],
    )?;

    progress.write(&report(act, group, subject, &relative));
    Ok(())
}

/// Refuse a group or a subject the fleet does not declare.
///
/// Before the declaration is read and before the delegation is judged, because a
/// membership naming either is refused at the next evaluation: an edit that wrote
/// one would commit a tree that no longer resolves, and the operator's next build
/// would carry a refusal about a file this verb wrote.
fn refuse_undeclared(workspace: &Workspace, group: &str, subject: &str) -> Result<()> {
    let delegation = workspace.delegation()?;
    if delegation.group(group).is_none() {
        return Err(Error::UnknownGroup {
            group: group.to_owned(),
            declared: delegation.groups().map(str::to_owned).collect(),
        });
    }
    if !delegation.declares_subject(subject) {
        return Err(Error::UnknownSubject {
            subject: subject.to_owned(),
            declared: delegation.subjects.clone(),
        });
    }
    Ok(())
}

/// The recipient policy the edited declarations imply, in place of the committed
/// one.
fn regenerate_policy(workspace: &Workspace) -> Result<()> {
    let root = workspace.root();
    let staging = root.join(".sops.yaml.new");
    workspace
        .nix()
        .eval_raw_to(root, Attribute::PolicyText, &staging)
        .map_err(|_| Error::PolicyEvalAfterScaffold {
            root: root.display().to_string(),
        })?;
    let policy = root.join(".sops.yaml");
    std::fs::rename(&staging, &policy).map_err(|cause| Error::FileUnwritable {
        path: policy.display().to_string(),
        cause,
    })
}

/// What the edit did, and — for a removal — what it did not.
///
/// The disclosure is the one every narrowing in safix carries, and it names the
/// report that will carry the shrink rather than offering `fix` as the remedy:
/// `fix` aligns the ciphertext with the policy, which is right and is not
/// revocation.
fn report(act: Act, group: &str, subject: &str, relative: &str) -> String {
    match act {
        Act::Add => format!(
            "\nsafix: {subject} is a member of {group}.\n\
            \n\
            What was done:\n\
            \x20 - {relative}, one inserted line\n\
            \x20 - .sops.yaml regenerated from the declarations that edit implies\n\
            \x20 - both committed together\n\
            \n\
            Membership growth is a re-wrap. Every file the group's audience names is\n\
            still encrypted to the audience that was, until:\n\
            \n\
            \x20   safix fix\n\
            \n"
        ),
        Act::Remove => format!(
            "\nsafix: {subject} is no longer a member of {group}.\n\
            \n\
            What was done:\n\
            \x20 - {relative}, one removed line\n\
            \x20 - .sops.yaml regenerated from the declarations that edit implies\n\
            \x20 - both committed together\n\
            \n\
            What this does NOT do, and no edit can:\n\
            \x20 - it takes nothing back. {subject} has held the data key of every file\n\
            \x20   the group's audience names, so they have read every value in them,\n\
            \x20   and no re-wrap unreads it. Only minting a new value revokes.\n\
            \n\
            safix check reports the shrink as the revocation it is, with rotation named\n\
            as the remedy. safix fix aligns the ciphertext with the policy now declared,\n\
            which is worth doing and is explicitly not that remedy.\n\
            \n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_declaration_is_scaffolded_under_a_directory_of_safixs_own() {
        assert_eq!(declaration_path("oncall"), "safix/groups/oncall.nix");
    }

    #[test]
    fn a_removal_says_what_it_does_not_undo_and_names_the_report_that_carries_it() {
        let rendered = report(Act::Remove, "oncall", "bob", "safix/groups/oncall.nix");
        assert!(rendered.contains("no longer a member"));
        assert!(rendered.contains("takes nothing back"));
        assert!(rendered.contains("no re-wrap unreads it"));
        assert!(rendered.contains("safix check reports the shrink as the revocation it is"));
    }

    #[test]
    fn an_addition_names_the_re_wrap_and_carries_no_revocation_prose() {
        let rendered = report(Act::Add, "oncall", "bob", "safix/groups/oncall.nix");
        assert!(rendered.contains("Membership growth is a re-wrap"));
        assert!(rendered.contains("safix fix"));
        assert!(!rendered.contains("revoke"));
    }
}
