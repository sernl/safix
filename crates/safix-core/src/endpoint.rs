//! What a far side of a mapping is asked, and the one decision every target
//! reaches.
//!
//! A far side is asked five things and nothing else: to unlock, to read one
//! address as a value and the metadata declared beside it, to write both
//! together, to enumerate what it holds, and to say what it can carry. That is
//! the whole of [`Endpoint`].
//!
//! Every refusal only one far side can raise stays outside the trait, is raised
//! by that far side alone, and is named here so a later target does not inherit
//! a rule that was never about it:
//!
//! - clan's stale-generator refusal, [`Error::GeneratorDefinitionDrifted`],
//!   raised on both clan write paths — `crate::bridge::write_to_clan` through
//!   `Addressing::generator_stale`, and again on `bridge_sync`'s own push arm.
//! - clan's shared-placement address discovery, `crate::bridge::Addressing`,
//!   which asks clan which machine answers for a shared var rather than taking
//!   one from the declaration.
//! - clan's committing in its own repository rather than in this one, which is
//!   why a transfer toward clan commits nothing here.
//! - clan's ungenerated var as an ordinary absence,
//!   `crate::bridge::Outcome::AbsentAtSource`, rather than a refusal.
//! - the keepassxc value channel's single-line limit,
//!   [`Error::ValueSpansLines`], raised by `crate::store::Database::write` off
//!   this module's own [`Capabilities::multiline`].
//! - the keepassxc write burst, `crate::store::Database`'s one-write-per-entry
//!   discipline, which decides nothing about what a far side is and everything
//!   about how this one is driven.
//!
//! This module exists to delete two copies of one decision — `sync`'s `two_way`
//! and `bridge_sync`'s `judge` were the same four-way match over two different
//! far sides — rather than to anticipate a target. [`judge`] is written once
//! here and read by both callers in the same change.

use crate::bridge;
use crate::error::{Error, Result};
use crate::model::{FieldValue, Fields};
use crate::secret::Secret;
use crate::workspace::Workspace;

/// How a target carries one field's value to its own store.
///
/// The distinction is not a preference: a value that travels an argument vector
/// is readable by every process on the host, so it may carry a literal out of a
/// declaration — already world-readable in the nix store — and never a value
/// read out of an encrypted entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// In the store command's argument vector.
    Argv,
    /// On standard input or a pipe.
    Stdin,
    /// Not at all: this target's own command has no way to write the field.
    Unsupported,
}

/// One of the four fields a mapping's far side may declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldName {
    /// The username to set on the entry.
    Username,
    /// The address the credential is used at.
    Url,
    /// The entry's note.
    Notes,
    /// The entry's tags.
    Tags,
}

impl FieldName {
    /// Every field, in the order `modules/flake/safix/fields.nix`'s
    /// `fieldNames` carries them.
    ///
    /// Fixed rather than incidental: it is the order a diff reports drift in
    /// and the order a fields read asks its attributes in, so two runs over one
    /// mapping read the same.
    pub const ALL: [Self; 4] = [Self::Username, Self::Url, Self::Notes, Self::Tags];

    /// The field as the declaration spells it, and as every report names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Username => "username",
            Self::Url => "url",
            Self::Notes => "notes",
            Self::Tags => "tags",
        }
    }
}

/// What one target can carry, per field, and what shape of value it accepts.
///
/// `multiline` is where a value-shape refusal comes from: [`Error::ValueSpansLines`]
/// is a declared property of a transport whose value channel reads one line,
/// rather than a special case in one writer. A target whose channel takes a
/// whole stream declares `multiline: true` and inherits no refusal it was never
/// subject to.
///
/// Not `Copy`: it is read through a reference by the two rules that consult it,
/// and a table is a thing a caller holds rather than a scalar it passes around.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// How this target carries a username.
    pub username: Channel,
    /// How this target carries a url.
    pub url: Channel,
    /// How this target carries a note.
    pub notes: Channel,
    /// How this target carries tags.
    pub tags: Channel,
    /// Whether this target's value channel accepts more than one line.
    pub multiline: bool,
}

impl Capabilities {
    /// The channel this target carries one field on.
    #[must_use]
    pub const fn channel(&self, field: FieldName) -> Channel {
        match field {
            FieldName::Username => self.username,
            FieldName::Url => self.url,
            FieldName::Notes => self.notes,
            FieldName::Tags => self.tags,
        }
    }
}

/// One field's value, resolved, with its provenance still in the type.
///
/// A [`Field::Literal`] holds a `String`, so an argument vector can carry it. A
/// [`Field::Resolved`] holds a [`Secret`], which has no egress that yields
/// bytes, so nothing can put it in one. The channel rule is enforced by the
/// type rather than by a branch a later edit can forget, and
/// [`Error::FieldSourceInArgv`] is what turns the resulting impossibility into
/// a sentence before a run reaches it.
pub enum Field {
    /// A value written in the declaration, and therefore not a secret.
    Literal(String),
    /// A value read out of another entry the same person holds.
    Resolved(Secret),
}

impl Field {
    /// Whether two fields hold the same value.
    ///
    /// A literal is compared by materialising it through [`Secret::read_from`],
    /// never by taking bytes out of a resolved value: the direction is one-way
    /// by construction, which is the whole reason the two shapes are distinct
    /// types rather than one string and a flag.
    #[must_use]
    pub fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Literal(ours), Self::Literal(theirs)) => ours == theirs,
            (Self::Resolved(ours), Self::Resolved(theirs)) => ours.equals(theirs),
            (Self::Literal(text), Self::Resolved(secret))
            | (Self::Resolved(secret), Self::Literal(text)) => {
                Secret::read_from(&mut text.as_bytes())
                    .is_ok_and(|materialised| materialised.equals(secret))
            }
        }
    }
}

/// The fields of one mapping, resolved: what the declaration says, or what the
/// far side was found holding.
#[derive(Default)]
pub struct ResolvedFields {
    /// The username, or none where the declaration names no username.
    pub username: Option<Field>,
    /// The url, or none.
    pub url: Option<Field>,
    /// The note, or none.
    pub notes: Option<Field>,
    /// The tags, empty where the declaration names none.
    pub tags: Vec<Field>,
}

impl ResolvedFields {
    /// Whether anything at all is declared here.
    ///
    /// What decides whether a fields read happens at all: a mapping declaring
    /// no field issues exactly the argument vectors it issued before fields
    /// existed.
    #[must_use]
    pub fn declares(&self) -> bool {
        self.username.is_some()
            || self.url.is_some()
            || self.notes.is_some()
            || !self.tags.is_empty()
    }

    /// The fields a read has to ask for, in [`FieldName::ALL`] order.
    #[must_use]
    pub fn wanted(&self) -> Vec<FieldName> {
        let mut wanted = Vec::new();
        if self.username.is_some() {
            wanted.push(FieldName::Username);
        }
        if self.url.is_some() {
            wanted.push(FieldName::Url);
        }
        if self.notes.is_some() {
            wanted.push(FieldName::Notes);
        }
        if !self.tags.is_empty() {
            wanted.push(FieldName::Tags);
        }
        wanted
    }

    /// The declared fields the held side does not match, in [`FieldName::ALL`]
    /// order.
    ///
    /// A field the declaration does not name is never compared and never
    /// reported, whatever the far side holds there: safix declares nothing
    /// about it, and an entry's own username is the operator's until a
    /// declaration claims it.
    ///
    /// `tags` compares as an ordered list, and a length difference is a
    /// difference — the far side stores them in an order, so a declaration that
    /// names a different order names a different state.
    #[must_use]
    pub fn diff(&self, held: &Self) -> Vec<&'static str> {
        let mut drifted = Vec::new();
        for field in FieldName::ALL {
            let differs = match field {
                FieldName::Username => differs(self.username.as_ref(), held.username.as_ref()),
                FieldName::Url => differs(self.url.as_ref(), held.url.as_ref()),
                FieldName::Notes => differs(self.notes.as_ref(), held.notes.as_ref()),
                FieldName::Tags => {
                    !self.tags.is_empty()
                        && (self.tags.len() != held.tags.len()
                            || self
                                .tags
                                .iter()
                                .zip(held.tags.iter())
                                .any(|(ours, theirs)| !ours.equals(theirs)))
                }
            };
            if differs {
                drifted.push(field.as_str());
            }
        }
        drifted
    }
}

/// Whether one declared scalar field differs from what the far side holds.
///
/// An undeclared field never differs, which is [`ResolvedFields::diff`]'s
/// stated semantics rather than a shortcut.
fn differs(ours: Option<&Field>, theirs: Option<&Field>) -> bool {
    ours.is_some_and(|ours| !theirs.is_some_and(|theirs| ours.equals(theirs)))
}

/// What one address on a far side holds: a value, and the fields beside it.
pub struct Record {
    /// The value, or none where the far side holds no entry there.
    pub value: Option<Secret>,
    /// The fields the far side carries beside it.
    pub fields: ResolvedFields,
}

/// Whether the far side already holds an entry at the address being written.
///
/// Passed to a write rather than recomputed by it: the caller already read the
/// address to decide there was anything to do, and a second question would be a
/// second answer the two could disagree over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Existing {
    /// Nothing there yet; the write creates the entry.
    Absent,
    /// An entry is there; the write edits it.
    Present,
}

/// What a two-way convergence decided, in words no target owns.
///
/// Each target maps this onto its own report words, because the targets do not
/// report the same set of outcomes — `crate::bridge::bridge_sync::Outcome` has
/// no unjudged variant at all, and `crate::sync::Outcome` has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing to do.
    Unchanged,
    /// Both sides moved, or neither matches an agreement this run can account
    /// for. Nothing is written.
    Conflict,
    /// The far side is written from safix's, remembering the agreement where
    /// the mode records one.
    PushValue {
        /// Whether the new agreement is recorded.
        remember: bool,
    },
    /// safix's side is written from the far side, on the same terms.
    PullValue {
        /// Whether the new agreement is recorded.
        remember: bool,
    },
}

/// The three-way decision, against the agreement a companion entry remembers.
///
/// A memory that is absent, unreadable or written under a tag this version does
/// not know takes bootstrap semantics: write where one side is empty, report
/// everything else. Never a guess — the one thing that cannot happen here is
/// picking a winner from a clock.
///
/// `agrees` is a parameter rather than a derivation because the two targets
/// record their agreement under deliberately distinct format tags —
/// `crate::sync::FORMAT` is `safix-sync-v1` and
/// `crate::bridge::bridge_sync::FORMAT` is `safix-bridge-sync-v1`, so that
/// neither mechanism can read the other's memory. A judge that built the memory
/// line itself would have to know which target it was judging for, which is the
/// coupling it exists to remove.
#[must_use]
pub fn judge(
    ours: Option<&Secret>,
    theirs: Option<&Secret>,
    remembered: Option<&Secret>,
    agrees: &dyn Fn(&Secret, &Secret) -> bool,
) -> Verdict {
    match (ours, theirs) {
        (None, None) => Verdict::Unchanged,
        (Some(_), None) => Verdict::PushValue { remember: true },
        (None, Some(_)) => Verdict::PullValue { remember: true },
        (Some(ours), Some(theirs)) => {
            if ours.equals(theirs) {
                return Verdict::Unchanged;
            }
            let Some(remembered) = remembered else {
                return Verdict::Conflict;
            };
            match (agrees(remembered, ours), agrees(remembered, theirs)) {
                // safix is where the agreement left it, so the far side is the
                // side that moved.
                (true, false) => Verdict::PullValue { remember: true },
                (false, true) => Verdict::PushValue { remember: true },
                // Both moved, or neither matches an agreement this run cannot
                // account for. Either way nothing is written.
                _ => Verdict::Conflict,
            }
        }
    }
}

/// The far side of a mapping, as the five questions a convergence asks it.
pub trait Endpoint {
    /// Whatever the far side needs before the first mapping is touched.
    ///
    /// # Errors
    ///
    /// The far side's own refusal to open.
    fn unlock(&mut self) -> Result<()>;

    /// What the far side holds at one address, or nothing where it holds no
    /// entry there.
    ///
    /// `declared` is what decides which attributes are asked for and whether a
    /// second invocation happens at all: a mapping that declares no field is
    /// read with exactly the argument vector it was read with before fields
    /// existed, and the value and the fields never share a pipe.
    ///
    /// # Errors
    ///
    /// The far side's own refusal over that address.
    fn read(&self, id: &str, declared: &ResolvedFields) -> Result<Option<Record>>;

    /// Write a value and the declared fields beside it, in one write.
    ///
    /// # Errors
    ///
    /// The far side's own refusal over that address, including a value its own
    /// channel cannot carry whole.
    fn write(&mut self, id: &str, record: &Record, existing: Existing) -> Result<()>;

    /// Every address the far side holds under the mapped namespace.
    ///
    /// # Errors
    ///
    /// The far side's own refusal to enumerate.
    fn list(&self) -> Result<Vec<String>>;

    /// What this far side can carry, per field.
    fn capabilities(&self) -> Capabilities;
}

/// Resolve a mapping's declared fields against what the target can carry.
///
/// Three refusals and one read, in that order per field: a field whose channel
/// is [`Channel::Unsupported`] is refused with [`Error::FieldUnsupported`]; an
/// `{ entry = … }` source on a [`Channel::Argv`] field is refused with
/// [`Error::FieldSourceInArgv`], because the resolved value is a secret and an
/// argument vector is not a channel one may travel; a literal becomes a
/// [`Field::Literal`] whatever the channel, because a declaration is evaluated
/// into a world-readable store and is therefore not a secret; and an
/// `{ entry = … }` source on a [`Channel::Stdin`] field is read through the
/// same reader safix's own side of every mapping is read through and becomes a
/// [`Field::Resolved`].
///
/// The `Stdin` arm has no production caller in this change: keepassxc declares
/// no `Stdin` field, so every field it can carry is refused or literal. Its
/// first caller is `add-pass-bridge`, and it is complete and unit-tested here
/// against a fixture whose four channels are all `Stdin`, because shipping the
/// refusal without the resolution would make the refusal a statement about
/// nothing.
///
/// # Errors
///
/// [`Error::FieldUnsupported`] or [`Error::FieldSourceInArgv`] for the two
/// capability refusals; [`Error::UnknownName`] where the named entry is not one
/// that person holds, and [`Error::NoValueYet`] where they hold it and no value
/// has been written into it; and whatever reading that entry refuses with.
pub fn resolve_fields(
    workspace: &Workspace,
    target: &'static str,
    user: &str,
    declared: &Fields,
    capabilities: &Capabilities,
) -> Result<ResolvedFields> {
    let scalar = |field: FieldName, value: Option<&FieldValue>| match value {
        None => Ok(None),
        Some(value) => resolve_one(workspace, target, user, field, value, capabilities).map(Some),
    };

    Ok(ResolvedFields {
        username: scalar(FieldName::Username, declared.username.as_ref())?,
        url: scalar(FieldName::Url, declared.url.as_ref())?,
        notes: scalar(FieldName::Notes, declared.notes.as_ref())?,
        tags: declared
            .tags
            .iter()
            .map(|value| {
                resolve_one(
                    workspace,
                    target,
                    user,
                    FieldName::Tags,
                    value,
                    capabilities,
                )
            })
            .collect::<Result<Vec<_>>>()?,
    })
}

/// One declared field, resolved or refused.
fn resolve_one(
    workspace: &Workspace,
    target: &'static str,
    user: &str,
    field: FieldName,
    value: &FieldValue,
    capabilities: &Capabilities,
) -> Result<Field> {
    match (capabilities.channel(field), value) {
        (Channel::Unsupported, _) => Err(Error::FieldUnsupported {
            target,
            field: field.as_str(),
        }),
        (Channel::Argv, FieldValue::Entry { entry }) => Err(Error::FieldSourceInArgv {
            target,
            field: field.as_str(),
            entry: entry.clone(),
        }),
        (Channel::Argv | Channel::Stdin, FieldValue::Literal(text)) => {
            Ok(Field::Literal(text.clone()))
        }
        (Channel::Stdin, FieldValue::Entry { entry }) => {
            // An entry the person does not hold is refused by name by the
            // resolver this read goes through. An entry they hold that has
            // never been written is the other absence, and its remedy is to
            // write it rather than to rename it.
            let held = bridge::held_by_safix(workspace, target, user, entry)?;
            match held {
                Some(value) => Ok(Field::Resolved(value)),
                None => Err(Error::NoValueYet {
                    file: workspace.resolve(user, entry)?.file.clone(),
                    name: entry.clone(),
                    user: user.to_owned(),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Git;
    use crate::nix::Nix;
    use crate::sops::Sops;

    fn secret(bytes: &str) -> Secret {
        Secret::read_from(&mut bytes.as_bytes()).unwrap()
    }

    /// A workspace over a directory that is not a flake.
    ///
    /// Every refusal asserted below is reached before the workspace is touched
    /// at all, which is itself part of the claim: a declaration the target
    /// cannot carry is refused without a repository, a key or a database in
    /// reach.
    fn unreachable_workspace() -> Workspace {
        Workspace::at(
            std::path::PathBuf::from("/nonexistent/safix-endpoint-fixture"),
            std::path::PathBuf::from("/nonexistent/safix-endpoint-fixture"),
            Git::default(),
            Nix::from_environment(),
            Sops::from_environment(),
        )
    }

    /// Every channel a pipe, so the `Stdin` arm is reachable and neither
    /// refusal is.
    fn all_stdin() -> Capabilities {
        Capabilities {
            username: Channel::Stdin,
            url: Channel::Stdin,
            notes: Channel::Stdin,
            tags: Channel::Stdin,
            multiline: true,
        }
    }

    /// keepassxc's own table, as `modules/flake/safix/fields.nix` declares it.
    fn keepassxc() -> Capabilities {
        Capabilities {
            username: Channel::Argv,
            url: Channel::Argv,
            notes: Channel::Argv,
            tags: Channel::Unsupported,
            multiline: false,
        }
    }

    fn declared(json: &str) -> Fields {
        serde_json::from_str(json).unwrap()
    }

    fn resolve(capabilities: &Capabilities, fields: &Fields) -> Result<ResolvedFields> {
        resolve_fields(
            &unreachable_workspace(),
            "keepassxc",
            "alice",
            fields,
            capabilities,
        )
    }

    /// The two targets' own `agrees` both compare a remembered line against the
    /// one a converging write would record. This is that shape with the format
    /// tag removed, which is the part each target keeps to itself.
    fn agrees(remembered: &Secret, value: &Secret) -> bool {
        remembered.equals(value)
    }

    #[test]
    fn judge_is_the_decision_sync_and_the_bridge_both_reached() {
        let ours = secret("ours");
        let theirs = secret("theirs");
        let same = secret("ours");

        assert_eq!(judge(None, None, None, &agrees), Verdict::Unchanged);
        assert_eq!(
            judge(Some(&ours), None, None, &agrees),
            Verdict::PushValue { remember: true }
        );
        assert_eq!(
            judge(None, Some(&theirs), None, &agrees),
            Verdict::PullValue { remember: true }
        );
        assert_eq!(
            judge(Some(&ours), Some(&same), None, &agrees),
            Verdict::Unchanged
        );
        // Two differing values and no memory is a conflict rather than a guess.
        assert_eq!(
            judge(Some(&ours), Some(&theirs), None, &agrees),
            Verdict::Conflict
        );
        // safix is where the agreement left it, so the far side moved.
        assert_eq!(
            judge(Some(&ours), Some(&theirs), Some(&secret("ours")), &agrees),
            Verdict::PullValue { remember: true }
        );
        assert_eq!(
            judge(Some(&ours), Some(&theirs), Some(&secret("theirs")), &agrees),
            Verdict::PushValue { remember: true }
        );
        // A memory matching neither side is one this run cannot account for,
        // and both sides having moved is the same answer.
        assert_eq!(
            judge(Some(&ours), Some(&theirs), Some(&secret("older")), &agrees),
            Verdict::Conflict
        );
    }

    #[test]
    fn a_field_the_declaration_does_not_name_is_never_compared() {
        let declaration = ResolvedFields {
            username: Some(Field::Literal("alice@example.com".to_owned())),
            ..ResolvedFields::default()
        };
        let held = ResolvedFields {
            username: Some(Field::Literal("alice@example.com".to_owned())),
            url: Some(Field::Literal(
                "https://the-operator-put-this-here".to_owned(),
            )),
            notes: Some(Field::Literal("and so is this".to_owned())),
            tags: vec![Field::Literal("and-this".to_owned())],
        };

        assert!(declaration.diff(&held).is_empty());
        assert_eq!(declaration.wanted(), vec![FieldName::Username]);
        assert!(declaration.declares());
        assert!(!ResolvedFields::default().declares());
    }

    /// The sibling of the tag-order claim: the names a diff returns come back
    /// in [`FieldName::ALL`] order rather than in whatever order the drift was
    /// found in, which is what makes a report's field order fixed.
    #[test]
    fn a_diff_names_the_drifted_fields_in_one_fixed_order() {
        let declaration = ResolvedFields {
            notes: Some(Field::Literal("minted by safix".to_owned())),
            url: Some(Field::Literal("https://grafana.example.com".to_owned())),
            username: Some(Field::Literal("alice@example.com".to_owned())),
            tags: vec![Field::Literal("work".to_owned())],
        };

        assert_eq!(
            declaration.diff(&ResolvedFields::default()),
            vec!["username", "url", "notes", "tags"]
        );
        assert_eq!(
            declaration.wanted(),
            vec![
                FieldName::Username,
                FieldName::Url,
                FieldName::Notes,
                FieldName::Tags
            ]
        );
    }

    #[test]
    fn an_ordered_tag_list_differs_by_order() {
        let declaration = ResolvedFields {
            tags: vec![
                Field::Literal("work".to_owned()),
                Field::Literal("fleet".to_owned()),
            ],
            ..ResolvedFields::default()
        };
        let reordered = ResolvedFields {
            tags: vec![
                Field::Literal("fleet".to_owned()),
                Field::Literal("work".to_owned()),
            ],
            ..ResolvedFields::default()
        };
        let shorter = ResolvedFields {
            tags: vec![Field::Literal("work".to_owned())],
            ..ResolvedFields::default()
        };

        assert_eq!(declaration.diff(&reordered), vec!["tags"]);
        assert_eq!(declaration.diff(&shorter), vec!["tags"]);
        assert!(declaration.diff(&declaration).is_empty());
    }

    #[test]
    fn a_literal_and_a_read_value_compare_without_leaving_the_secret_type() {
        let literal = Field::Literal("alice@example.com".to_owned());
        let read = Field::Resolved(secret("alice@example.com"));
        let other = Field::Resolved(secret("bob@example.com"));

        assert!(literal.equals(&read));
        assert!(read.equals(&literal));
        assert!(!literal.equals(&other));
        assert!(read.equals(&read));

        // And the comparison survives the round trip a held field takes: a
        // declared literal against the same bytes read back off the far side.
        let declaration = ResolvedFields {
            notes: Some(Field::Literal("minted by safix".to_owned())),
            ..ResolvedFields::default()
        };
        let held = ResolvedFields {
            notes: Some(Field::Resolved(secret("minted by safix"))),
            ..ResolvedFields::default()
        };
        assert!(declaration.diff(&held).is_empty());
    }

    #[test]
    fn an_unsupported_field_is_refused_naming_the_target_and_the_field() {
        let refusal = resolve(&keepassxc(), &declared(r#"{"tags": ["work"]}"#));
        assert!(matches!(
            refusal,
            Err(Error::FieldUnsupported {
                target: "keepassxc",
                field: "tags"
            })
        ));

        // The same declaration against a table that carries tags on a pipe is
        // not refused for being tags, which is what makes the refusal the
        // table's rather than the field name's.
        let accepted = resolve(&all_stdin(), &declared(r#"{"tags": ["work"]}"#));
        assert!(!matches!(accepted, Err(Error::FieldUnsupported { .. })));
    }

    #[test]
    fn an_entry_source_on_an_argv_channel_is_refused() {
        let refusal = resolve(
            &keepassxc(),
            &declared(r#"{"notes": {"entry": "grafana-note"}}"#),
        );
        let Err(Error::FieldSourceInArgv {
            target,
            field,
            entry,
        }) = refusal
        else {
            unreachable!("an entry source on an argv channel was not refused")
        };
        assert_eq!(target, "keepassxc");
        assert_eq!(field, "notes");
        assert_eq!(entry, "grafana-note");

        // A literal on the same field and the same channel is admissible, and
        // resolves without a repository in reach: the declaration it came from
        // is already world-readable.
        let Ok(resolved) = resolve(&keepassxc(), &declared(r#"{"notes": "minted by safix"}"#))
        else {
            unreachable!("a literal on an argv channel was refused")
        };
        assert!(matches!(resolved.notes, Some(Field::Literal(_))));
        assert_eq!(resolved.wanted(), vec![FieldName::Notes]);
    }

    #[test]
    fn an_entry_source_on_a_stdin_channel_resolves() {
        // On an all-pipes table an entry source is a read rather than a
        // refusal. The read cannot succeed here — the workspace names no flake
        // — and what is asserted is exactly that: the arm reaches the reader
        // rather than either capability refusal, which is the whole of what the
        // `Stdin` arm's existence claims until `add-pass-bridge` gives it a
        // production caller.
        let refusal = resolve(
            &all_stdin(),
            &declared(r#"{"username": {"entry": "grafana-user"}}"#),
        );
        let Err(reason) = refusal else {
            unreachable!("a read against a workspace naming no flake succeeded")
        };
        assert!(!matches!(
            reason,
            Error::FieldUnsupported { .. } | Error::FieldSourceInArgv { .. }
        ));

        // And a literal on the same all-pipes table resolves, so the arm above
        // is the entry source's alone.
        let Ok(resolved) = resolve(&all_stdin(), &declared(r#"{"username": "alice"}"#)) else {
            unreachable!("a literal on a stdin channel was refused")
        };
        assert!(matches!(resolved.username, Some(Field::Literal(_))));
    }
}
