//! Whose act a scaffold is.
//!
//! Two declarations decide it, and both are read out of the tree rather than
//! passed on a command line: an organization names the people who scaffold for
//! it, and a person's own record consents to being scaffolded for. When the
//! target of a scaffold is covered by neither, nothing here is consulted at all
//! and the verb behaves exactly as it did before delegation was recorded.
//!
//! # The acting identity is the commit's
//!
//! Every scaffolding verb commits what it wrote, so whoever the commit will name
//! is who is acting, and that is the identity this reads — `user.name` and
//! `user.email` as the repository resolves them, through
//! [`Git::author_identity`](crate::git::Git::author_identity). There is no flag
//! that names somebody else, deliberately: a flag would let the check and the
//! commit disagree, and the whole value of the record is that a scaffold and its
//! attribution cannot be separated.
//!
//! The identity is matched to a declared person by name, because that is the only
//! correspondence the declarations hold — nothing maps a git identity to a
//! person — and where the match fails the refusal says so rather than guessing at
//! an address's local part.
//!
//! # What these refusals are not
//!
//! Authorization. They guard the cooperative path and nothing else, which is
//! stated at every surface the feature has — on both options, in the README, and
//! in [`BOUNDARY`], which the refusals themselves carry. This is the one refusal
//! family in safix that guards a process rather than a structure: evaluation
//! cannot refuse a hostile edit here, because the tree that edit produces is
//! structurally valid, and saying so is what keeps the guard from being trusted
//! with what it cannot do.

use crate::error::{Error, Result};
use crate::model::Delegation;
use crate::progress::{Progress, note};
use crate::workspace::Workspace;

/// The one sentence every delegation surface carries.
///
/// The nix options say it where they are declared, the README says it, and every
/// refusal below ends with it. `crates/safix/src/usage.rs` carries it into the
/// verbs' help text, held to this string by a test rather than by inspection.
pub const BOUNDARY: &str = "\
These refusals bind the cooperative path and are not authorization. The tree is\n\
the authorization: anyone who can commit can edit these declarations by hand,\n\
evaluation refuses structure rather than people, and no delegation record places\n\
a key in any audience. What they buy is that a scaffold and the identity it is\n\
attributed to cannot disagree.";

/// How a delegation reaches what is about to be edited.
///
/// Two shapes because the model has two: a person is delegated by their own
/// consent, and a group by the silo sets that cover it. Both name the target and
/// the declaration a refusal has to send the operator to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Through {
    /// One person's custody record, delegated by that person's own `managedBy`.
    Consent {
        /// The person whose record is being edited.
        person: String,
    },
    /// One group's declaration, delegated by the silo sets covering it.
    Silo {
        /// The group whose membership is being edited.
        group: String,
    },
}

impl Through {
    /// What is being edited, as the option path declaring it.
    #[must_use]
    pub fn subject(&self) -> String {
        match self {
            Self::Consent { person } => format!("flake.safix.users.{person}"),
            Self::Silo { group } => format!("flake.safix.groups.{group}"),
        }
    }

    /// Where the delegation over it is written, as a refusal names it.
    ///
    /// A noun phrase rather than a bare path, because the silo shape has no single
    /// path to name: what delegates a group is every set that holds it, and a
    /// refusal sending an operator to one of them would send them to the wrong
    /// half of the boundary as often as to the right one.
    #[must_use]
    pub fn site(&self) -> String {
        match self {
            Self::Consent { person } => format!("flake.safix.users.{person}.managedBy"),
            Self::Silo { .. } => String::from("the flake.safix.silos sets that hold it"),
        }
    }

    /// How the organization came to be the one, as a clause about it.
    ///
    /// One phrasing shared by the commit's own line and the note the run prints,
    /// so the sentence an operator reads and the sentence history keeps cannot
    /// drift apart.
    #[must_use]
    pub fn clause(&self) -> String {
        match self {
            Self::Consent { person } => {
                format!("which flake.safix.users.{person}.managedBy names")
            }
            Self::Silo { group } => {
                format!("whose silo declarations cover flake.safix.groups.{group}")
            }
        }
    }
}

/// What a delegation refusal names: how the delegation reached what was about to
/// be edited, and the organizations it names.
///
/// One value for both refusals below, because both say the same three things about
/// the delegation and differ only in what they say about the identity. It reaches
/// [`Error`] boxed, which keeps the refusal type small enough for the lint that
/// watches how large a `Result`'s error half is — a refusal is returned by every
/// fallible function in the crate, and a large one is copied along every one of
/// those returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// How the delegation reached the target.
    pub through: Through,
    /// The organizations it names, in name order.
    pub organizations: Vec<String>,
}

/// What a delegation check answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// No delegation covers the target, so none was consulted.
    ///
    /// The ordinary answer, and the whole of the compatibility promise: an
    /// unmanaged person and a group no silo set names are edited by whoever can
    /// commit, exactly as they were before.
    Unmanaged,
    /// A delegation covers the target and the acting identity is within it.
    Delegated {
        /// The organization the scaffold is performed for.
        organization: String,
        /// The declared person the resulting commit will name.
        manager: String,
        /// How the delegation reached the target.
        through: Through,
    },
}

impl Scope {
    /// The line a permitted scaffold's commit carries, and nothing at all where
    /// no delegation was consulted.
    ///
    /// A body line rather than a trailer, because it is a sentence about the act
    /// rather than a key anything parses: what makes it worth recording is that
    /// the commit already names who made it, so this is the other half — what
    /// they made it as.
    #[must_use]
    pub fn commit_context(&self) -> Option<String> {
        match self {
            Self::Unmanaged => None,
            Self::Delegated {
                organization,
                manager,
                through,
            } => Some(format!(
                "Scaffolded by {manager} for {organization}, {}.",
                through.clause()
            )),
        }
    }

    /// Tell the operator which delegation the run is proceeding under, where one
    /// was consulted.
    pub fn announce(&self, progress: &dyn Progress) {
        if let Self::Delegated {
            organization,
            manager,
            through,
        } = self
        {
            note(
                progress,
                &format!(
                    "{manager} is a declared manager of {organization}, {}, so this scaffold is \
                     recorded as {organization}'s.",
                    through.clause()
                ),
            );
        }
    }
}

/// Whether the acting identity may edit this person's custody record.
///
/// # Errors
///
/// [`Error::ActorUndeclared`] when the commit's identity names no declared
/// person, [`Error::ScaffoldOutOfScope`] when it names one the delegation does
/// not, and whatever reading the declarations or the git identity failed with.
pub fn over_person(workspace: &Workspace, person: &str) -> Result<Scope> {
    let delegation = workspace.delegation()?;
    let organizations: Vec<String> = delegation
        .managing(person)
        .map(|organization| vec![organization.to_owned()])
        .unwrap_or_default();
    judge(
        workspace,
        delegation,
        &organizations,
        Through::Consent {
            person: person.to_owned(),
        },
    )
}

/// Whether the acting identity may edit this group's declaration.
///
/// # Errors
///
/// The same three as [`over_person`].
pub fn over_group(workspace: &Workspace, group: &str) -> Result<Scope> {
    let delegation = workspace.delegation()?;
    let organizations = delegation
        .group(group)
        .map(|declared| declared.organizations.clone())
        .unwrap_or_default();
    judge(
        workspace,
        delegation,
        &organizations,
        Through::Silo {
            group: group.to_owned(),
        },
    )
}

/// The one judgement, over however many organizations cover the target.
///
/// No covering organization is the first branch and it returns before anything
/// else is read: an unmanaged target must not be able to fail on a git identity
/// nobody declared, because that would make declaring a delegation somewhere else
/// in the fleet a refusal for a person nobody manages.
///
/// Where several organizations cover one group — two silo sets naming it is
/// refused, but one set can reach two organizations' people — a manager of any one
/// of them is within scope, and the scaffold is recorded as that organization's.
/// The alternative, demanding every one of them, would refuse a manager acting
/// squarely within their own remit because somebody else's remit overlaps it, and
/// the guard is here to prevent a mistake rather than to arbitrate between two
/// organizations that have declared themselves one boundary.
fn judge(
    workspace: &Workspace,
    delegation: &Delegation,
    organizations: &[String],
    through: Through,
) -> Result<Scope> {
    if organizations.is_empty() {
        return Ok(Scope::Unmanaged);
    }

    let identity = workspace.git().author_identity(workspace.root())?;
    let placements = workspace.placements()?;
    let refused = || {
        Box::new(Refused {
            through: through.clone(),
            organizations: organizations.to_vec(),
        })
    };

    if !placements.declares(&identity.name) {
        return Err(Error::ActorUndeclared {
            name: identity.name,
            email: identity.email,
            delegation: refused(),
            declared: placements.users().map(str::to_owned).collect(),
        });
    }

    let within = organizations
        .iter()
        .find(|organization| delegation.is_manager(organization, &identity.name));
    if let Some(organization) = within {
        return Ok(Scope::Delegated {
            organization: organization.clone(),
            manager: identity.name,
            through,
        });
    }

    let mut managers: Vec<String> = organizations
        .iter()
        .flat_map(|organization| delegation.managers_of(organization))
        .cloned()
        .collect();
    managers.sort_unstable();
    managers.dedup();
    Err(Error::ScaffoldOutOfScope {
        actor: identity.name,
        delegation: refused(),
        managers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unmanaged_scope_records_nothing_in_the_commit() {
        assert_eq!(Scope::Unmanaged.commit_context(), None);
    }

    #[test]
    fn a_persons_context_names_the_consent_that_delegated_the_act() {
        let scope = Scope::Delegated {
            organization: String::from("acme"),
            manager: String::from("alice"),
            through: Through::Consent {
                person: String::from("bob"),
            },
        };
        assert_eq!(
            scope.commit_context().as_deref(),
            Some("Scaffolded by alice for acme, which flake.safix.users.bob.managedBy names.")
        );
    }

    #[test]
    fn a_groups_context_names_the_silo_coverage_that_delegated_the_act() {
        let scope = Scope::Delegated {
            organization: String::from("acme"),
            manager: String::from("alice"),
            through: Through::Silo {
                group: String::from("oncall"),
            },
        };
        assert_eq!(
            scope.commit_context().as_deref(),
            Some(
                "Scaffolded by alice for acme, whose silo declarations cover \
                 flake.safix.groups.oncall."
            )
        );
    }

    #[test]
    fn each_shape_names_the_declaration_a_refusal_sends_the_operator_to() {
        let consent = Through::Consent {
            person: String::from("bob"),
        };
        assert_eq!(consent.subject(), "flake.safix.users.bob");
        assert_eq!(consent.site(), "flake.safix.users.bob.managedBy");
        assert_eq!(
            consent.clause(),
            "which flake.safix.users.bob.managedBy names"
        );

        let silo = Through::Silo {
            group: String::from("oncall"),
        };
        assert_eq!(silo.subject(), "flake.safix.groups.oncall");
        assert_eq!(silo.site(), "the flake.safix.silos sets that hold it");
        assert_eq!(
            silo.clause(),
            "whose silo declarations cover flake.safix.groups.oncall"
        );
    }
}
