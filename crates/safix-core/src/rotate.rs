//! Re-minting a value because of its age, and declaring how long values live.
//!
//! Two verbs over one idea. `rotate` acts on the deadline
//! [`crate::stamps::Deadline`] computes: one entry now, or everything past its
//! deadline in one run. `rotation set`/`unset` edits the declaration that sets
//! the deadline, the way [`crate::group`] edits a membership.
//!
//! # Nothing here mints
//!
//! Every value this verb replaces travels [`crate::generate`]'s own path: the
//! same sandbox, the same staging root, the same pipe, the same per-generator
//! commit, the same definition and stamp records. What this module adds is
//! which generators run and in which order — the deadline is a reason to run
//! them and not a second way of running them.
//!
//! # What it refuses
//!
//! An entry nothing mints. `rotate` on a typed value would have to ask a
//! person for it, and a verb that sometimes prompts and sometimes does not is
//! a verb a timer cannot run; the refusal names `safix set`. The bulk form
//! does not refuse over one — it lists every due typed entry and exits zero,
//! because a run that refused would leave the values it could mint unminted.

use crate::error::{Error, Result};
use crate::model::UserPlan;
use crate::progress::{Progress, log, note};
use crate::stamps::Deadline;
use crate::workspace::Workspace;
use crate::{declaration, delegation, generate, git, stamps};

/// One entry whose deadline has passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    /// The user who holds it.
    pub user: String,
    /// The entry's name.
    pub name: String,
    /// The policy that set the deadline.
    pub policy: String,
    /// How long ago the deadline passed, in seconds.
    pub overdue_seconds: u64,
    /// The generator that mints it, when one does.
    pub producer: Option<String>,
}

/// Everything past its deadline, in placement order, for every user or for
/// one.
///
/// Read through [`Deadline::at`] like every other answer about a deadline, and
/// deduplicated by stamp record: a shared value has one write and one deadline
/// however many people hold it, and rotating it once is rotating it.
///
/// # Errors
///
/// Any refusal from evaluating the nix half, and
/// [`Error::StampRecordUnparsable`] for a record that cannot be read.
pub fn due(workspace: &Workspace, only: Option<&str>) -> Result<Vec<Due>> {
    let placements = workspace.placements()?;
    let plan = workspace.generator_plan()?;
    let now = stamps::now();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut found = Vec::new();

    for user in placements.users() {
        if only.is_some_and(|wanted| wanted != user) {
            continue;
        }
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            let Some(rotation) = placement.rotation.as_ref() else {
                continue;
            };
            if !seen.insert(placement.stamp_record.clone()) {
                continue;
            }
            let recorded = stamps::read(workspace, placement)?;
            let Some(overdue) = Deadline::at(recorded, placement, now).overdue() else {
                continue;
            };
            found.push(Due {
                user: user.to_owned(),
                name: name.clone(),
                policy: rotation.policy.clone(),
                overdue_seconds: overdue,
                producer: plan
                    .for_user(user)
                    .and_then(|mine| mine.producer_of(name))
                    .map(str::to_owned),
            });
        }
    }
    Ok(found)
}

/// Re-mint one generator-backed entry now, with the cascade its dependents
/// require.
///
/// [`generate::run`] with `regenerate` set, which is the whole of it: the
/// cascade, its confirmation, the sandbox and the commits are that function's,
/// so a value rotated here and a value regenerated there are the same write.
///
/// # Errors
///
/// [`Error::NoGenerator`] for an entry nothing mints, naming `safix set`, and
/// every refusal [`generate::run`] can raise.
pub fn one(
    workspace: &Workspace,
    progress: &dyn Progress,
    interaction: &mut dyn generate::Interaction,
    user: &str,
    name: &str,
    options: generate::Options,
) -> Result<i32> {
    generate::run(
        workspace,
        progress,
        interaction,
        user,
        Some(name),
        generate::Options {
            regenerate: true,
            ..options
        },
    )
}

/// Re-mint everything past its deadline that a generator can mint, and list
/// what it cannot.
///
/// The order is the union of the due generators' cascades, taken in the plan's
/// own order so that a generator reading another's output still runs after it.
/// One confirmation covers the whole set, before the first commit: declining
/// afterwards takes nothing back out of history.
///
/// Exits zero when nothing is due, and zero when the only due entries are ones
/// no generator mints: both are reports rather than failures, and the second
/// prints the `safix set` each of them needs.
///
/// # Errors
///
/// [`Error::CascadeDeclined`] when the set is declined, and every refusal one
/// generator's run can raise.
pub fn all(
    workspace: &Workspace,
    progress: &dyn Progress,
    interaction: &mut dyn generate::Interaction,
    only: Option<&str>,
    options: generate::Options,
) -> Result<i32> {
    let overdue = due(workspace, only)?;
    if overdue.is_empty() {
        note(progress, "Nothing is past its rotation deadline.");
        return Ok(0);
    }

    let plan = workspace.generator_plan()?;
    let mut status = 0;
    let users: Vec<String> = {
        let mut names: Vec<String> = overdue.iter().map(|entry| entry.user.clone()).collect();
        names.dedup();
        names
    };

    for user in users {
        let Some(mine) = plan.for_user(&user) else {
            continue;
        };
        if let Some(cycle) = UserPlan::cycle(mine) {
            return Err(Error::GeneratorCycle {
                user: user.clone(),
                cycle,
            });
        }
        let producers: Vec<&str> = overdue
            .iter()
            .filter(|entry| entry.user == user)
            .filter_map(|entry| entry.producer.as_deref())
            .collect();
        if producers.is_empty() {
            continue;
        }
        // The union of the cascades, ordered by the plan rather than by the
        // order the due entries were found in: a generator reading another's
        // output has to run after it, and only the plan knows that.
        let marked: Vec<String> = producers
            .iter()
            .flat_map(|producer| mine.cascade(producer))
            .collect();
        let order: Vec<String> = mine
            .order
            .iter()
            .filter(|name| marked.contains(name))
            .cloned()
            .collect();

        generate::confirm_rotation(progress, interaction, &order, options.assume_yes)?;
        let ran = generate::run_order(
            workspace,
            progress,
            interaction,
            &user,
            &order,
            generate::Options {
                regenerate: true,
                ..options
            },
        )?;
        if ran != 0 {
            status = ran;
        }
    }

    report_typed(progress, &overdue);
    Ok(status)
}

/// The due entries no generator can mint, with the command each of them needs.
///
/// Printed after the run rather than before it, so the last thing an operator
/// reads is the work that is still theirs.
fn report_typed(progress: &dyn Progress, overdue: &[Due]) {
    let typed: Vec<&Due> = overdue
        .iter()
        .filter(|entry| entry.producer.is_none())
        .collect();
    if typed.is_empty() {
        return;
    }
    let mut report = format!(
        "\nsafix: {} due value(s) have no generator, so nothing here can mint them:\n\n",
        typed.len()
    );
    for entry in typed {
        let _ = std::fmt::Write::write_fmt(
            &mut report,
            format_args!(
                "    safix set {} {}    ({}, overdue)\n",
                entry.user, entry.name, entry.policy
            ),
        );
    }
    report.push('\n');
    progress.write(&report);
}

/// Which edit an invocation of `rotation` asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    /// Assign or replace the entry's policy.
    Set(String),
    /// Remove it.
    Unset,
}

impl Act {
    /// The word the report and the commit use.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Set(_) => "set",
            Self::Unset => "unset",
        }
    }
}

/// The file an entry's declaration is edited in.
///
/// The layout [`crate::adduser`] chooses for a person's record, because that
/// is where [`declaration::add_private_entry`] writes an entry and where a
/// scaffolded one therefore is. An entry declared elsewhere is a supported
/// declaration and not one this verb can edit; the refusal says so and names
/// the path it looked at.
#[must_use]
pub fn declaration_path(user: &str) -> String {
    crate::adduser::scaffold_path(user)
}

/// Assign or remove one entry's rotation policy, and commit the edit.
///
/// The mechanism is [`crate::group`]'s, act for act: refuse what the
/// declarations do not define before anything is read, judge the delegation,
/// refuse a repository that is mid-operation or dirty, edit the file as text,
/// parse it with the real parser and put it back when it does not parse, then
/// stage and commit.
///
/// No policy is regenerated and nothing is re-wrapped: a deadline places no
/// key in any audience, so the `.sops.yaml` this edit implies is the one
/// already committed.
///
/// # Errors
///
/// [`Error::UnknownUser`], [`Error::UnknownName`],
/// [`Error::UnknownRotationPolicy`], [`Error::ActorUndeclared`],
/// [`Error::ScaffoldOutOfScope`], [`Error::MidOperation`],
/// [`Error::UncommittedChanges`], [`Error::NoEntryDeclaration`],
/// [`Error::Unparsable`] and [`Error::FileUnwritable`], in that order of
/// reachability.
pub fn scaffold(
    workspace: &Workspace,
    progress: &dyn Progress,
    act: &Act,
    user: &str,
    name: &str,
) -> Result<()> {
    refuse_undeclared(workspace, act, user, name)?;

    let scope = delegation::over_person(workspace, user)?;
    scope.announce(progress);

    let relative = declaration_path(user);
    crate::set::refuse_bad_repository_state(workspace, &[(workspace.root(), relative.as_str())])?;

    let absolute = workspace.absolute(&relative);
    let original =
        workspace
            .read_relative(&relative)?
            .ok_or_else(|| Error::NoEntryDeclaration {
                user: user.to_owned(),
                name: name.to_owned(),
                file: relative.clone(),
            })?;

    let edited = match act {
        Act::Set(policy) => match declaration::set_entry_rotation(&original, name, policy) {
            declaration::Edit::Inserted(edited) => Some(edited),
            declaration::Edit::AlreadyPresent => {
                note(
                    progress,
                    &format!("'{name}' already rotates on '{policy}'; nothing was written."),
                );
                None
            }
            declaration::Edit::NoAnchor => {
                return Err(Error::NoEntryDeclaration {
                    user: user.to_owned(),
                    name: name.to_owned(),
                    file: relative,
                });
            }
        },
        Act::Unset => match declaration::remove_entry_rotation(&original, name) {
            declaration::Removal::Removed(edited) => Some(edited),
            declaration::Removal::NotPresent => {
                note(
                    progress,
                    &format!("'{name}' declares no rotation policy; nothing was removed."),
                );
                None
            }
            declaration::Removal::NoAnchor => {
                return Err(Error::NoEntryDeclaration {
                    user: user.to_owned(),
                    name: name.to_owned(),
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
        &match act {
            Act::Set(policy) => {
                format!("safix: declaring {name} rotates on {policy} in {relative}")
            }
            Act::Unset => format!("safix: removing {name}'s rotation policy from {relative}"),
        },
    );
    std::fs::write(&absolute, &edited).map_err(|cause| Error::FileUnwritable {
        path: absolute.display().to_string(),
        cause,
    })?;

    // Parsed before anything is staged, and put back when it does not parse:
    // the file was a valid declaration a moment ago and the edit is this
    // module's, so an edit that does not parse is this module's to undo.
    if !workspace.nix().parses(&absolute) {
        let _ = std::fs::write(&absolute, &original);
        return Err(Error::Unparsable {
            path: absolute.display().to_string(),
        });
    }

    workspace
        .git()
        .stage(workspace.root(), std::slice::from_ref(&relative))?;

    let mut message = match act {
        Act::Set(policy) => {
            format!("feat(safix): rotate {user}'s {name} on the {policy} policy")
        }
        Act::Unset => format!("feat(safix): unset {user}'s {name} rotation policy"),
    };
    if let Some(context) = scope.commit_context() {
        message.push_str("\n\n");
        message.push_str(&context);
    }
    let identity = workspace.git().author_identity(workspace.root())?;
    git::commit_two_roots(
        workspace.git(),
        workspace.vault_root(),
        workspace.root(),
        progress,
        "",
        &message,
        &[],
        std::slice::from_ref(&relative),
        &identity,
    )?;

    progress.write(&report(act, user, name, &relative));
    Ok(())
}

/// Refuse an entry or a policy the declarations do not carry.
///
/// Before the declaration is read and before the delegation is judged, for
/// [`crate::group`]'s reason: an entry naming an undeclared policy is refused
/// at the next evaluation, so writing one would commit a tree that no longer
/// resolves.
fn refuse_undeclared(workspace: &Workspace, act: &Act, user: &str, name: &str) -> Result<()> {
    let placements = workspace.placements()?;
    let held = placements.held_by(user).ok_or_else(|| Error::UnknownUser {
        user: user.to_owned(),
        declared: placements.users().map(str::to_owned).collect(),
    })?;
    if !held.contains_key(name) {
        return Err(Error::UnknownName {
            user: user.to_owned(),
            name: name.to_owned(),
            held: held.keys().cloned().collect(),
        });
    }
    if let Act::Set(policy) = act {
        let policies = workspace.rotation_policies()?;
        if !policies.declares(policy) {
            return Err(Error::UnknownRotationPolicy {
                policy: policy.clone(),
                declared: policies.names().map(str::to_owned).collect(),
            });
        }
    }
    Ok(())
}

/// What the edit did, and what it does not do.
///
/// A deadline is not custody: assigning one encrypts nothing and revokes
/// nothing, and the report says so rather than leaving an operator to wonder
/// whether the value just changed hands.
fn report(act: &Act, user: &str, name: &str, relative: &str) -> String {
    match act {
        Act::Set(policy) => format!(
            "\nsafix: {user}'s '{name}' rotates on the '{policy}' policy.\n\
            \n\
            What was done:\n\
            \x20 - {relative}, one line\n\
            \x20 - committed\n\
            \n\
            No value moved and no file was re-wrapped. The deadline is the value's\n\
            last write plus the policy's interval, and an entry with no recorded\n\
            write is due at once:\n\
            \n\
            \x20   safix check\n\
            \n"
        ),
        Act::Unset => format!(
            "\nsafix: {user}'s '{name}' has no rotation policy.\n\
            \n\
            What was done:\n\
            \x20 - {relative}, one removed line\n\
            \x20 - committed\n\
            \n\
            What this does NOT do: shorten anything already read. It stops the\n\
            reporting, not the ageing — a value nobody rotates is a value whose\n\
            readers keep it.\n\
            \n"
        ),
    }
}
