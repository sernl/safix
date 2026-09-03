//! The drift report: what the declarations say, against what the repository
//! holds.
//!
//! Five questions. The first four are the shell runtime's, in the order it asks
//! them, because the order is what the report reads as and a reader who has fixed
//! the policy expects the recipient findings under it. The fifth is newer and sits
//! last: whether a generated value was minted under the declaration that is there
//! now, answered from the record `generate` leaves — see [`crate::definition`].
//!
//! Nothing here writes, decrypts, or holds an identity: every question is
//! answered from the declarations, from the structure of the ciphertext, and from
//! one plaintext record about a declaration, which is what lets one machine judge
//! files belonging to people whose keys it does not have.
//!
//! A finding is data. The prose the command prints for each one is the shell
//! runtime's prose, and it lives in the command beside the other rendering, so
//! that a program embedding this crate gets the finding rather than the
//! paragraph.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::definition;
use crate::error::{Error, Result};
use crate::model::{Holders, Placements};
use crate::sops::document::{self, KeyState};
use crate::workspace::Workspace;

/// How the value a finding asks for is minted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mint {
    /// The user whose custody the command names.
    pub carrier: String,
    /// The secret's name.
    pub name: String,
    /// Whether a generator mints it, which decides between `generate` and
    /// `set`.
    pub generated: bool,
}

/// One disagreement between the declarations and the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Finding {
    /// No committed `.sops.yaml`, so no creation rule covers any file.
    PolicyMissing,

    /// The committed `.sops.yaml` is not the one the declarations imply.
    PolicyDiffers,

    /// A file named through `extraGovernedFiles` that no rule's directory
    /// covers, so nothing declares who should be able to open it.
    UngovernableExtra {
        /// The repository-relative path.
        file: String,
    },

    /// A governed file's stanzas disagree with the audience declared for it.
    RecipientDrift {
        /// The repository-relative path.
        file: String,
        /// Can open it and is not in its audience.
        extra: Vec<String>,
        /// Is in its audience and cannot open it.
        missing: Vec<String>,
        /// Whose custody the extra keys are: declared subjects the audience no
        /// longer names, and keys answering to no declared subject at all.
        ///
        /// A key on a file that is no longer in its audience is what every
        /// narrowing looks like from here — a grant dropped, a member removed
        /// from a group, a machine changed hands, an escrow consent withdrawn, an
        /// organization's custody shrunk — and the declarations record only the
        /// audience that is, never the audience that was. So the narrowing is
        /// read off the ciphertext, and naming its holders is what lets the
        /// report say a re-wrap is not a revocation of them.
        ///
        /// Which half a narrowing lands in follows from whose key it is. A
        /// withdrawn consent leaves the organization's own custody key behind, so
        /// the organization is named; a shrunk custody leaves a key its
        /// declaration no longer holds, so it is orphaned and the organization is
        /// named by the file, whose directory the audience is named for.
        narrowed: Holders,
        /// What mints a new value for each name the file holds, which is the only
        /// thing that revokes.
        mints: Vec<Mint>,
    },

    /// A shared name has a copy outside the file its audience reads, and every
    /// reader of the copy is still in the audience.
    SharedStrayMigration {
        /// The catalogue entry's name.
        name: String,
        /// The file the audience reads.
        audience_file: String,
        /// The file holding the copy.
        stray_file: String,
        /// The key the copy sits under.
        key: String,
        /// What mints the value the audience is to share.
        mint: Mint,
    },

    /// A shared name has a copy someone outside the audience can open.
    SharedStrayRevocation {
        /// The catalogue entry's name.
        name: String,
        /// The file the audience reads.
        audience_file: String,
        /// The file holding the copy.
        stray_file: String,
        /// The key the copy sits under.
        key: String,
        /// Declared users who can open the copy and are no longer carriers.
        named: Vec<String>,
        /// Keys that can open it and answer to no declared user.
        orphaned: Vec<String>,
        /// What mints the value for the audience that remains.
        mint: Mint,
    },

    /// A declared name whose file holds no value for it.
    ValuelessName {
        /// The user who declares it.
        user: String,
        /// The secret's name.
        name: String,
        /// The file that would hold the value.
        file: String,
        /// Whether a generator mints it.
        generated: bool,
    },

    /// A value in a governed file that no declaration claims.
    UnclaimedValue {
        /// The repository-relative path.
        file: String,
        /// The key the value sits under.
        key: String,
    },

    /// A generated value whose recorded definition is not the one the
    /// declarations carry now.
    ///
    /// Carries no value and no derivative of one: the record it is read from is a
    /// digest of the declaration, and the value itself is never opened to answer
    /// this question.
    DefinitionDrift {
        /// The user who holds the entry.
        user: String,
        /// The entry's name.
        name: String,
        /// The entry the generator that minted it is declared on.
        generator: String,
        /// The repository-relative path of the record.
        record: String,
    },

    /// A vault is declared, but its own `.gitignore` does not cover the
    /// scratch creation rules file — design V10's second, independent
    /// guarantee beside the scratch registry's own sweep-on-every-exit-path
    /// guarantee, catching a scratch file that happens to still exist at the
    /// moment `git add`/`git commit` runs.
    VaultGitignoreMissing,
}

/// Every disagreement, in report order, for every user or for one.
///
/// The unclaimed-value half is never narrowed by user: a key no declaration
/// claims belongs to nobody by definition, so there is no custody to filter it
/// by.
///
/// # Errors
///
/// Any refusal from evaluating the nix half, or [`Error::RecipientsUnreadable`]
/// when a governed file's stanzas cannot be read.
pub fn run(workspace: &Workspace, only: Option<&str>) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut documents = Documents::default();

    policy(workspace, &mut findings)?;
    recipients(workspace, &mut documents, &mut findings)?;
    let strays = shared(workspace, &mut documents, &mut findings)?;
    values(workspace, &mut documents, &strays, only, &mut findings)?;
    definitions(workspace, only, &mut findings)?;
    vault_gitignore(workspace, &mut findings)?;

    Ok(findings)
}

/// The vault's `.gitignore` covers the scratch creation rules file.
///
/// A no-op when no vault is declared: `vault_root` equals `root` then, and
/// nothing here is ever created for a `.gitignore` to have to cover.
fn vault_gitignore(workspace: &Workspace, findings: &mut Vec<Finding>) -> Result<()> {
    if workspace.vault_root() == workspace.root() {
        return Ok(());
    }
    let anchored = format!("/{}", crate::workspace::VAULT_RULES_FILE);
    let covered = workspace
        .read_vault_relative(".gitignore")?
        .is_some_and(|text| {
            text.lines()
                .map(str::trim)
                .any(|line| line == crate::workspace::VAULT_RULES_FILE || line == anchored)
        });
    if !covered {
        findings.push(Finding::VaultGitignoreMissing);
    }
    Ok(())
}

/// The committed `.sops.yaml` against the one the declarations imply.
///
/// The sops CLI reads this file off disk rather than out of an evaluation, so
/// it is an artifact that has to be regenerated and committed and can therefore
/// be stale in a way nothing else here can.
fn policy(workspace: &Workspace, findings: &mut Vec<Finding>) -> Result<()> {
    let generated = workspace.policy_text()?;
    let path = workspace.absolute(".sops.yaml");
    let committed = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
            findings.push(Finding::PolicyMissing);
            return Ok(());
        }
        Err(cause) => {
            return Err(Error::FileUnreadable {
                path: path.display().to_string(),
                cause,
            });
        }
    };
    if committed != generated.as_bytes() {
        findings.push(Finding::PolicyDiffers);
    }
    Ok(())
}

/// Each governed file's actual recipients against the audience declared for it.
///
/// The two halves of the governed set are judged differently because they are
/// different claims. A required file has an audience of its own, and drift is
/// its stanzas disagreeing with it. A file named through `extraGovernedFiles`
/// has no audience — no declaration places a secret in it — so what holds it is
/// the rule whose directory covers it, which is also what `fix` will re-wrap it
/// to.
fn recipients(
    workspace: &Workspace,
    documents: &mut Documents,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let audiences = workspace.audiences()?;
    let holders = workspace.recipients()?;
    let placements = workspace.placements()?;
    for file in &workspace.governed_files()?.managed {
        let Some(text) = documents.text(workspace, file)? else {
            continue;
        };

        let held_to = audiences
            .for_file(file)
            .or_else(|| audiences.covering_dir(&parent_of(file)));
        let Some(declared) = held_to.map(|audience| &audience.recipients) else {
            findings.push(Finding::UngovernableExtra { file: file.clone() });
            continue;
        };

        let actual =
            document::recipients_of(text).map_err(|cause| Error::RecipientsUnreadable {
                file: file.clone(),
                cause: Box::new(cause),
            })?;
        let drift = document::drift(&actual, declared);
        if !drift.is_empty() {
            findings.push(Finding::RecipientDrift {
                file: file.clone(),
                narrowed: holders.holders_of(&drift.extra),
                mints: mints_in(placements, file),
                extra: drift.extra,
                missing: drift.missing,
            });
        }
    }
    Ok(())
}

/// What mints a new value for each name one file holds, in name order.
///
/// One per name rather than one per placement: a shared name is one value in one
/// file, and any of its carriers mints it, so a second command naming a second
/// carrier would be the same value minted twice.
fn mints_in(placements: &Placements, file: &str) -> Vec<Mint> {
    let mut mints: BTreeMap<&str, Mint> = BTreeMap::new();
    for user in placements.users() {
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            if placement.file != file || placement.public.is_some() {
                continue;
            }
            mints.entry(name).or_insert_with(|| Mint {
                carrier: user.to_owned(),
                name: name.clone(),
                generated: placement.generator.is_some(),
            });
        }
    }
    mints.into_values().collect()
}

/// A shared entry is one value, so a copy of its key anywhere but the file its
/// audience picks is a second value the audience does not hold.
///
/// Which of the two kinds it is comes off the copy's own stanzas rather than
/// out of any record of what the audience used to be. A copy a non-member can
/// open is a revocation: that reader has held the data key, so re-wrapping does
/// not unread what they read, and only a new value revokes. A copy every one of
/// whose readers is still in the audience is a migration.
///
/// Returns the `(file, key)` pairs reported, because a stray copy of a shared
/// name is an unclaimed value too and reporting it twice under two remedies —
/// one of which is "delete it", the other "declare it" — would invite the wrong
/// one.
fn shared(
    workspace: &Workspace,
    documents: &mut Documents,
    findings: &mut Vec<Finding>,
) -> Result<BTreeSet<(String, String)>> {
    let placements = workspace.placements()?;
    let audiences = workspace.audiences()?;
    let recipients = workspace.recipients()?;
    let managed = &workspace.governed_files()?.managed;

    let mut shared_entries: BTreeMap<String, Mint> = BTreeMap::new();
    let mut shared_files: BTreeMap<String, (String, String)> = BTreeMap::new();
    for user in placements.users() {
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            if !placement.shared || shared_entries.contains_key(name) {
                continue;
            }
            shared_entries.insert(
                name.clone(),
                Mint {
                    carrier: user.to_owned(),
                    name: name.clone(),
                    generated: placement.generator.is_some(),
                },
            );
            shared_files.insert(
                name.clone(),
                (placement.file.clone(), placement.key.clone()),
            );
        }
    }

    let mut strays = BTreeSet::new();
    for (name, mint) in &shared_entries {
        let Some((audience_file, key)) = shared_files.get(name) else {
            continue;
        };
        let declared = audiences
            .for_file(audience_file)
            .map_or(&[] as &[String], |audience| audience.recipients.as_slice());

        for file in managed {
            if file == audience_file {
                continue;
            }
            if !documents.has_value(workspace, file, key)? {
                continue;
            }
            strays.insert((file.clone(), key.clone()));

            let Some(text) = documents.text(workspace, file)? else {
                continue;
            };
            let actual =
                document::recipients_of(text).map_err(|cause| Error::RecipientsUnreadable {
                    file: file.clone(),
                    cause: Box::new(cause),
                })?;
            let extra = document::drift(&actual, declared).extra;

            if extra.is_empty() {
                findings.push(Finding::SharedStrayMigration {
                    name: name.clone(),
                    audience_file: audience_file.clone(),
                    stray_file: file.clone(),
                    key: key.clone(),
                    mint: mint.clone(),
                });
                continue;
            }

            let Holders { named, orphaned } = recipients.holders_of(&extra);
            findings.push(Finding::SharedStrayRevocation {
                name: name.clone(),
                audience_file: audience_file.clone(),
                stray_file: file.clone(),
                key: key.clone(),
                named,
                orphaned,
                mint: mint.clone(),
            });
        }
    }
    Ok(strays)
}

/// Names the declarations make that hold no value, and values in a file the
/// declarations do place secrets in that no name claims.
///
/// The two are opposite directions of the same question and are reported apart
/// because their remedies differ: a valueless name is minted or typed, an
/// unclaimed value is declared or deleted.
///
/// The unclaimed half walks the required files rather than the managed ones.
/// Every key in a file named through `extraGovernedFiles` is unclaimed by
/// construction — that is what naming it there means — so reporting those would
/// be a finding no declaration can ever resolve.
fn values(
    workspace: &Workspace,
    documents: &mut Documents,
    strays: &BTreeSet<(String, String)>,
    only: Option<&str>,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let placements = workspace.placements()?;

    for user in placements.users() {
        if only.is_some_and(|wanted| wanted != user) {
            continue;
        }
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            if documents.has_value(workspace, &placement.file, &placement.key)? {
                continue;
            }
            findings.push(Finding::ValuelessName {
                user: user.to_owned(),
                name: name.clone(),
                file: placement.file.clone(),
                generated: placement.generator.is_some(),
            });
        }
    }

    let claimed: BTreeSet<(&str, &str)> = placements
        .0
        .values()
        .flat_map(|held| held.values())
        .map(|placement| (placement.file.as_str(), placement.key.as_str()))
        .collect();

    for file in &workspace.governed_files()?.required {
        for key in documents.keys(workspace, file)?.keys() {
            if claimed.contains(&(file.as_str(), key.as_str())) {
                continue;
            }
            if strays.contains(&(file.clone(), key.clone())) {
                continue;
            }
            findings.push(Finding::UnclaimedValue {
                file: file.clone(),
                key: key.clone(),
            });
        }
    }
    Ok(())
}

/// Generated values whose recorded definition is not the declared one.
///
/// Last, because it is the only question here that is not about a file's
/// contents or its recipients: the four before it read the tree the operator is
/// converging, and this one reads what a past run recorded about a declaration.
/// An operator who has just fixed the policy expects the recipient findings under
/// it, and one who has just edited a generator expects this at the end.
///
/// Three states are out of scope and produce nothing. An entry nothing generates
/// has no definition to have drifted from. An entry with no record predates the
/// record, and asserting drift over an absent record would report every value
/// minted before this existed — a claim about when the tool changed rather than
/// about the tree. And a record whose leading tag is not the one this version
/// writes is a record this version cannot read, so it says nothing rather than
/// reporting the whole tree as drifted the day the canonical form moves.
///
/// One finding per record rather than per carrier. A shared entry is one value
/// under one record, and reporting it once per person who holds it would be three
/// findings with one remedy between them.
fn definitions(
    workspace: &Workspace,
    only: Option<&str>,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let placements = workspace.placements()?;
    let mut reported: BTreeSet<String> = BTreeSet::new();

    for user in placements.users() {
        if only.is_some_and(|wanted| wanted != user) {
            continue;
        }
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            let Some((generator, declared)) = placements.producer_of(user, name) else {
                continue;
            };
            let record = definition::record_path(name, placement);
            if !reported.insert(record.clone()) {
                continue;
            }
            let Some(text) = workspace.read_vault_relative(&record)? else {
                continue;
            };
            let Some(recorded) = definition::recorded(&text) else {
                continue;
            };
            if recorded == definition::digest(declared) {
                continue;
            }
            findings.push(Finding::DefinitionDrift {
                user: user.to_owned(),
                name: name.clone(),
                generator: generator.to_owned(),
                record,
            });
        }
    }
    Ok(())
}

/// The governed files read during one report, each read once.
///
/// The shell runtime shells out to the key reader once per question and reads
/// each file again every time; caching here changes how often the bytes are
/// read and nothing about the answers, because nothing in a report writes.
#[derive(Debug, Default)]
struct Documents {
    text: HashMap<String, Option<String>>,
    keys: HashMap<String, BTreeMap<String, KeyState>>,
}

impl Documents {
    fn text(&mut self, workspace: &Workspace, file: &str) -> Result<Option<&String>> {
        if !self.text.contains_key(file) {
            let text = workspace.read_vault_relative(file)?;
            self.text.insert(file.to_owned(), text);
        }
        Ok(self.text.get(file).and_then(Option::as_ref))
    }

    /// A document whose keys cannot be read reports none.
    ///
    /// This is the shell runtime's behaviour rather than a choice made here:
    /// its key reader runs inside a pipeline whose failure is a false answer to
    /// "does this hold a value", and inside a process substitution whose
    /// failure ends the loop over keys. Both runtimes are therefore silent
    /// about a governed path holding something that is not a YAML document; the
    /// recipient half of the report is what speaks about it, and does.
    fn keys(&mut self, workspace: &Workspace, file: &str) -> Result<&BTreeMap<String, KeyState>> {
        if !self.keys.contains_key(file) {
            let keys = match self.text(workspace, file)? {
                Some(text) => document::keys_of(text).unwrap_or_default(),
                None => BTreeMap::new(),
            };
            self.keys.insert(file.to_owned(), keys);
        }
        self.keys.get(file).ok_or(Error::SopsPipeMissing)
    }

    fn has_value(&mut self, workspace: &Workspace, file: &str, key: &str) -> Result<bool> {
        Ok(self
            .keys(workspace, file)?
            .get(key)
            .is_some_and(|state| !state.empty))
    }
}

/// `dirname`, as the shell runtime's `dirname` answers it.
///
/// A path with no separator has parent `.`, which is what makes the directory
/// comparison against an audience's `dir` meaningful for a file at the
/// repository root.
fn parent_of(path: &str) -> String {
    match path.rfind('/') {
        None => ".".to_owned(),
        Some(0) => "/".to_owned(),
        Some(index) => path.get(..index).unwrap_or(".").to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_matches_dirname_on_the_shapes_placements_produce() {
        assert_eq!(
            parent_of("secrets/safix/users/alice/secrets.yaml"),
            "secrets/safix/users/alice"
        );
        assert_eq!(parent_of("secrets.yaml"), ".");
        assert_eq!(parent_of("/secrets.yaml"), "/");
    }
}

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::parent_of;

    proptest! {
        /// A file placed in a directory has that directory as its parent, which
        /// is the whole of what the extra-governed-file lookup asks of it: the
        /// directory it computes is matched against an audience's own `dir`, and
        /// a parent that disagreed by a separator would report every consumer
        /// file as covered by no rule.
        #[test]
        fn a_placed_file_has_the_directory_it_was_placed_in(
            directory in "[a-z][a-z0-9/,_-]{0,20}[a-z0-9]",
            name in "[a-z][a-z0-9._-]{0,10}",
        ) {
            prop_assert_eq!(parent_of(&format!("{directory}/{name}")), directory);
        }

        /// A path with no separator is at the root, and reports the directory
        /// `dirname` reports for it.
        #[test]
        fn a_bare_name_is_at_the_current_directory(name in "[a-z][a-z0-9._-]{0,10}") {
            prop_assert_eq!(parent_of(&name), ".");
        }
    }
}
