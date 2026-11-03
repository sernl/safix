//! The drift report: what the declarations say, against what the repository
//! holds.
//!
//! Five questions. The first four are the shell runtime's, in the order it asks
//! them, because the order is what the report reads as and a reader who has fixed
//! the policy expects the recipient findings under it. The fifth is newer and sits
//! last: whether a generated value was minted under the declaration that is there
//! now, and whether a value it read has been written since, answered from the
//! records `generate` and every writer leave — see [`crate::definition`] and
//! [`crate::stamps`].
//!
//! Nothing here writes, decrypts, or holds an identity: every question is
//! answered from the declarations, from the structure of the ciphertext, and from
//! plaintext records about declarations and writes, which is what lets one
//! machine judge files belonging to people whose keys it does not have.
//!
//! The structure of the ciphertext says different things per format. A SOPS
//! document's metadata names every recipient in full. A raw age file's header
//! names each SSH recipient by a four-byte tag of its key and every other
//! recipient by nothing — see [`crate::age_header`] — so a raw age file's SSH
//! recipients are compared by tag, a match is reported as a note that does not
//! establish the keys, and any other recipient leaves the roster unverifiable.
//!
//! A finding is data. The prose the command prints for each one is the shell
//! runtime's prose, and it lives in the command beside the other rendering, so
//! that a program embedding this crate gets the finding rather than the
//! paragraph.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::age_audience;
use crate::age_header;
use crate::definition;
use crate::error::{Error, Result};
use crate::model::{Audiences, Holders, Placements, Recipients};
use crate::sops::document::{self, KeyState};
use crate::stamps::{self, Deadline};
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

    /// A raw age file whose roster the header cannot establish: it carries a
    /// stanza that names no recipient, or its audience declares a recipient
    /// whose stanza would name none — X25519, plugin, `scrypt`.
    RecipientRosterUnverifiable {
        /// The repository-relative path of the native age file.
        file: String,
    },

    /// A raw age file's SSH stanzas disagree, by tag, with the SSH recipients
    /// its audience declares.
    ///
    /// A disagreement in a 32-bit tag is a disagreement in the key: the tag is
    /// a function of the key, so a stanza whose tag no declared key has was
    /// written for some other key. The converse does not hold, which is why an
    /// agreement is [`Finding::RecipientRosterTagMatched`] rather than silence.
    RecipientTagDrift {
        /// The repository-relative path of the raw age file.
        file: String,
        /// Declared SSH recipients with fewer stanzas carrying their tag than
        /// the audience declares keys with it.
        missing: Vec<String>,
        /// Stanzas whose tag no declared SSH recipient of the audience has:
        /// the full declared key when exactly one declared key anywhere has
        /// that type and tag, and `<type> <tag>` otherwise.
        extra: Vec<String>,
        /// Stanzas beyond the number of distinct declared keys carrying their
        /// type and tag: the audience's own key when it declares exactly one
        /// with that type and tag, and `<type> <tag>` otherwise.
        duplicate: Vec<String>,
        /// Whose custody the extra keys recovered by tag are.
        narrowed: Holders,
        /// What mints a new value for each name the file holds.
        mints: Vec<Mint>,
    },

    /// A raw age file whose every stanza is an SSH stanza matching, by tag, the
    /// SSH recipients its audience declares, one for one.
    ///
    /// A note rather than a pass, and the one finding that does not fail the
    /// run (see [`Finding::fails`]). A tag is thirty-two bits, so anybody able
    /// to write the file can mint keys until one collides with a declared
    /// key's tag and substitute that stanza; the match says the file is
    /// consistent with the declaration and not that it is wrapped to it.
    RecipientRosterTagMatched {
        /// The repository-relative path of the raw age file.
        file: String,
        /// How many stanzas matched.
        matched: usize,
    },

    /// A raw age file whose audience record names its current bytes and an
    /// audience other than the one declared now.
    ///
    /// The one case in which a raw age file's non-SSH recipients are known to
    /// disagree with the declarations: `fix` recorded who it wrapped these
    /// exact bytes to, and that audience is no longer the declared one.
    AgeAudienceChanged {
        /// The repository-relative path of the raw age file.
        file: String,
        /// The path of the audience record.
        record: String,
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

    /// A public output whose plaintext file holds nothing.
    ///
    /// Apart from [`Finding::ValuelessName`] because the question is asked of a
    /// different file and has a different answer. A public output has no
    /// ciphertext, no key and no creation rule — that is what `secret = false`
    /// means — so the sops document its placement still names holds nothing for
    /// it and never will. Only the plaintext file named by
    /// [`crate::model::Placement::public`] can say whether it has been minted.
    ///
    /// It is never typed, so there is no `set` remedy to offer: a public output
    /// exists only as something a generator writes, and that generator is
    /// declared on another entry, which is the one the remedy has to name.
    ValuelessPublic {
        /// The user who declares it.
        user: String,
        /// The output's name.
        name: String,
        /// The repository-relative path of the plaintext value.
        file: String,
        /// The entry the generator writing it is declared on.
        generator: String,
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

    /// A generated value one of whose dependencies was written after it.
    ///
    /// Answered from the stamp records alone, so it carries no value and no
    /// derivative of one. The remedy re-runs the dependent's generator, never
    /// the dependency's: a dependency set by hand has no generator, and one
    /// that has would discard the value the operator just wrote.
    DerivedValueStale {
        /// The user who holds the generator's entry.
        user: String,
        /// The entry the generator is declared on.
        generator: String,
        /// The dependency written after the generator last ran.
        dependency: String,
        /// The repository-relative path of the dependency's stamp record.
        record: String,
    },

    /// An entry whose rotation deadline has passed.
    ///
    /// Carries no value and no derivative of one: the two numbers it is
    /// computed from are the declared interval and the plaintext stamp record,
    /// and the value itself is never opened to answer this question — which is
    /// what lets one machine report the overdue values of people whose files it
    /// cannot decrypt.
    RotationDue {
        /// The user who holds the entry.
        user: String,
        /// The entry's name.
        name: String,
        /// The `flake.safix.rotation` policy that set the deadline.
        policy: String,
        /// How long ago the deadline passed, in seconds.
        overdue_seconds: u64,
        /// Whether a generator can re-mint it, which decides the remedy.
        generator: bool,
    },

    /// A vault is declared, but its own `.gitignore` does not cover the
    /// scratch creation rules file — design V10's second, independent
    /// guarantee beside the scratch registry's own sweep-on-every-exit-path
    /// guarantee, catching a scratch file that happens to still exist at the
    /// moment `git add`/`git commit` runs.
    VaultGitignoreMissing,

    /// A vault is declared, and this readable-layout path — a ciphertext
    /// document, a public output, or a definition record — still sits at
    /// the declaration root rather than at its opaque vault destination.
    ///
    /// `fix` moves it: decrypting it under the operator's own identity and
    /// re-encrypting it into the vault, or copying it directly when it is
    /// already plaintext, then removing the readable-layout source.
    VaultRelocationPending {
        /// The declaration-root, repository-relative path still pending.
        file: String,
    },
}

impl Finding {
    /// Whether this finding fails the run.
    ///
    /// Every finding does except [`Finding::RecipientRosterTagMatched`], which
    /// qualifies a run that found no drift rather than reporting any.
    #[must_use]
    pub const fn fails(&self) -> bool {
        !matches!(self, Self::RecipientRosterTagMatched { .. })
    }
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
    entry_records(workspace, only, &mut findings)?;
    vault_gitignore(workspace, &mut findings)?;
    vault_relocation(workspace, &mut findings)?;

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
    if !workspace.vault_gitignore_covers_rules()? {
        findings.push(Finding::VaultGitignoreMissing);
    }
    Ok(())
}

/// Every readable-layout path a vault migration has not yet moved.
///
/// A no-op when no vault is declared, for the reason [`vault_gitignore`]
/// gives: every placement's `logical_*` field is `null` then (design V14),
/// so [`crate::relocation`]'s enumerations find nothing to group.
///
/// One finding per distinct readable path — a ciphertext document, a public
/// output, a definition record, a stamp record, or a raw age audience record
/// — still present at the declaration root, deduplicated because a shared
/// secret's readable file is
/// one path named by every carrier's placement.
fn vault_relocation(workspace: &Workspace, findings: &mut Vec<Finding>) -> Result<()> {
    if workspace.vault_root() == workspace.root() {
        return Ok(());
    }
    let placements = workspace.placements()?;
    let mut pending = BTreeSet::new();
    for document in crate::relocation::secret_documents(placements) {
        if workspace.absolute(&document.logical_file).exists() {
            pending.insert(document.logical_file);
        }
    }
    for leaf in crate::relocation::public_leaves(placements)
        .into_iter()
        .chain(crate::relocation::record_leaves(placements))
        .chain(crate::relocation::stamp_leaves(placements))
        .chain(crate::relocation::audience_leaves(placements))
    {
        if workspace.absolute(&leaf.logical).exists() {
            pending.insert(leaf.logical);
        }
    }
    findings.extend(
        pending
            .into_iter()
            .map(|file| Finding::VaultRelocationPending { file }),
    );
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
///
/// The two formats disclose different things, and each is judged by what it
/// discloses. A SOPS document names every recipient, so its drift is exact. A
/// raw age header names each SSH recipient by a four-byte tag of its key and
/// every other recipient by nothing, so [`raw_age`] compares tags and says no
/// more than a tag can.
fn recipients(
    workspace: &Workspace,
    documents: &mut Documents,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let audiences = workspace.audiences()?;
    let holders = workspace.recipients()?;
    let placements = workspace.placements()?;
    for file in &workspace.governed_files()?.managed {
        if crate::ciphertext::Format::from_path(Path::new(file))? == crate::ciphertext::Format::Age
        {
            raw_age(workspace, audiences, holders, placements, file, findings)?;
            continue;
        }
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

/// One raw age file's header, and its audience record, against the audience
/// declared for it.
///
/// Nothing for a file that does not exist, as for a SOPS document. The header
/// is read up to its MAC line and never decrypted; one that cannot be read
/// refuses naming the file, as an unreadable SOPS document does. See
/// [`judge_age`] for the comparison and [`audience_changed`] for the record.
fn raw_age(
    workspace: &Workspace,
    audiences: &Audiences,
    holders: &Recipients,
    placements: &Placements,
    file: &str,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let path = workspace.vault_absolute(file);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(cause) => {
            return Err(Error::FileUnreadable {
                path: path.display().to_string(),
                cause,
            });
        }
    };
    let held_to = audiences
        .for_file(file)
        .or_else(|| audiences.covering_dir(&parent_of(file)));
    let Some(audience) = held_to else {
        findings.push(Finding::UngovernableExtra {
            file: file.to_owned(),
        });
        return Ok(());
    };
    let stanzas = age_header::stanzas(&bytes).map_err(|cause| Error::RecipientsUnreadable {
        file: file.to_owned(),
        cause: Box::new(cause),
    })?;

    findings.extend(judge_age(
        file,
        &stanzas,
        &tagged(&audience.recipients),
        &tagged(holders.0.values().flatten()),
        holders,
        || mints_in(placements, file),
    ));

    if let Some(record) = age_audience::record_for(placements, file) {
        let text = workspace.read_vault_relative(record)?;
        findings.extend(audience_changed(
            file,
            record,
            text.as_deref(),
            &bytes,
            &age_audience::audience_digest(&audience.recipients),
        ));
    }
    Ok(())
}

/// One declared recipient and its SSH tag, when it is an SSH recipient whose
/// key decodes.
#[derive(Debug, Clone)]
struct Tagged {
    key: String,
    tag: Option<(String, String)>,
}

fn tagged<'a>(keys: impl IntoIterator<Item = &'a String>) -> Vec<Tagged> {
    keys.into_iter()
        .map(|key| Tagged {
            key: key.clone(),
            tag: age_header::ssh_tag(key),
        })
        .collect()
}

/// A raw age file's stanzas against its declared audience, by SSH tag.
///
/// Expected multiplicity is keyed by `(type, tag)` and counts distinct declared
/// keys, so two declared keys whose tags collide expect two stanzas rather
/// than reading as a duplicate. A stanza whose `(type, tag)` the audience does
/// not declare is extra; stanzas beyond the expected count are duplicates; a
/// declared key whose `(type, tag)` has fewer stanzas than expected is missing.
///
/// An extra stanza is named as the full declared key when exactly one
/// declared key anywhere has its `(type, tag)` — the recovery agenix's
/// `--check` makes — and as `<type> <tag>` otherwise, because a tag two keys
/// share attributes to neither. A duplicate is named as the audience's own key
/// when the audience declares exactly one with its `(type, tag)`.
///
/// Independently of drift, a non-SSH stanza or a declared recipient with no
/// tag leaves the roster unverifiable; only a file with neither, and no drift,
/// is a tag match.
fn judge_age(
    file: &str,
    stanzas: &[age_header::Stanza],
    declared: &[Tagged],
    everywhere: &[Tagged],
    holders: &Recipients,
    mints: impl FnOnce() -> Vec<Mint>,
) -> Vec<Finding> {
    let mut unverifiable = declared.iter().any(|recipient| recipient.tag.is_none());
    let expected = by_tag(declared);
    let known = by_tag(everywhere);
    let mut actual: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut matched: usize = 0;
    for stanza in stanzas {
        if !stanza.is_ssh() {
            unverifiable = true;
            continue;
        }
        let count = actual
            .entry((
                stanza.kind.as_str(),
                stanza.first_argument.as_deref().unwrap_or(""),
            ))
            .or_default();
        *count = count.saturating_add(1);
        matched = matched.saturating_add(1);
    }

    let attributed = |key: &(&str, &str)| match known.get(key).map(BTreeSet::len) {
        Some(1) => known
            .get(key)
            .and_then(|keys| keys.first())
            .map(|&only| only.to_owned()),
        _ => None,
    };
    let named =
        |key: &(&str, &str)| attributed(key).unwrap_or_else(|| format!("{} {}", key.0, key.1));

    let mut missing = Vec::new();
    for (key, keys) in &expected {
        if actual.get(key).copied().unwrap_or(0) < keys.len() {
            missing.extend(keys.iter().map(|&declared| declared.to_owned()));
        }
    }
    let mut extra = Vec::new();
    let mut recovered = Vec::new();
    let mut duplicate = Vec::new();
    for (key, &count) in &actual {
        match expected.get(key) {
            None => {
                for _ in 0..count {
                    if let Some(holder) = attributed(key) {
                        recovered.push(holder);
                    }
                    extra.push(named(key));
                }
            }
            Some(keys) => {
                // The audience declares this tag, so its own key names the
                // surplus when it declares exactly one with it.
                let declared = match (keys.len(), keys.first()) {
                    (1, Some(&only)) => only.to_owned(),
                    _ => named(key),
                };
                for _ in keys.len()..count {
                    duplicate.push(declared.clone());
                }
            }
        }
    }

    let mut findings = Vec::new();
    let drifted = !(missing.is_empty() && extra.is_empty() && duplicate.is_empty());
    if drifted {
        recovered.dedup();
        findings.push(Finding::RecipientTagDrift {
            file: file.to_owned(),
            missing,
            extra,
            duplicate,
            narrowed: holders.holders_of(&recovered),
            mints: mints(),
        });
    }
    if unverifiable {
        findings.push(Finding::RecipientRosterUnverifiable {
            file: file.to_owned(),
        });
    } else if !drifted {
        findings.push(Finding::RecipientRosterTagMatched {
            file: file.to_owned(),
            matched,
        });
    }
    findings
}

/// The distinct keys carrying each `(type, tag)`, over the recipients that
/// have one.
fn by_tag(recipients: &[Tagged]) -> BTreeMap<(&str, &str), BTreeSet<&str>> {
    let mut keys: BTreeMap<(&str, &str), BTreeSet<&str>> = BTreeMap::new();
    for recipient in recipients {
        if let Some((kind, tag)) = &recipient.tag {
            keys.entry((kind.as_str(), tag.as_str()))
                .or_default()
                .insert(recipient.key.as_str());
        }
    }
    keys
}

/// What a raw age file's audience record says about the file as it is now.
///
/// Nothing for an absent record or one this version cannot read, because a
/// record that cannot be read says nothing; nothing for a record naming other
/// bytes, because it describes a file that is no longer there; and nothing
/// when it names the declared audience. Only a record naming these exact bytes
/// and another audience is a finding.
fn audience_changed(
    file: &str,
    record: &str,
    text: Option<&str>,
    bytes: &[u8],
    declared: &str,
) -> Option<Finding> {
    let (audience, ciphertext) = age_audience::recorded(text?)?;
    if ciphertext != age_audience::ciphertext_digest(bytes) || audience == declared {
        return None;
    }
    Some(Finding::AgeAudienceChanged {
        file: file.to_owned(),
        record: record.to_owned(),
    })
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
        if key.is_empty() {
            continue;
        }
        let declared = audiences
            .for_file(audience_file)
            .map_or(&[] as &[String], |audience| audience.recipients.as_slice());

        for file in managed {
            if file == audience_file
                || crate::ciphertext::Format::from_path(Path::new(file))?
                    == crate::ciphertext::Format::Age
            {
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
/// A public output is asked of its own file rather than of the document its
/// placement names. The placement of an output declared `secret = false` still
/// carries a `file` and a `key`, because every placement does, but they
/// describe a document that is never written — see [`crate::model::Placement`]
/// — so asking whether that document holds a value answers "no" for every
/// public output in the tree, generated or not. The file that can answer is
/// [`crate::model::Placement::public`], which the resolver computes for exactly
/// these entries and which is already the vault-relative path in vault mode.
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
            if let Some(public) = placement.public.as_deref() {
                findings.extend(public_finding(workspace, placements, user, name, public));
                continue;
            }
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
        if claimed.contains(&(file.as_str(), "")) {
            continue;
        }
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

/// What one public output's own file says, and who to name when it says
/// nothing.
///
/// Nothing when the file holds bytes: a minted public output is converged, and
/// the fact that no ciphertext anywhere holds it is what being public means
/// rather than a disagreement to report.
///
/// The remedy names the entry the generator is declared on rather than the
/// output, because that is the entry the run is of: one run of one generator
/// writes every one of its outputs, and naming each output separately would
/// read as several runs to make.
///
/// The producer is looked up rather than assumed, and the output's own name is
/// the fallback, so that a tree whose resolver said "public" and whose
/// placements name no producer still prints a command `generate` accepts.
fn public_finding(
    workspace: &Workspace,
    placements: &Placements,
    user: &str,
    name: &str,
    public: &str,
) -> Option<Finding> {
    if crate::public::holds_a_value(&workspace.vault_absolute(public)) {
        return None;
    }
    Some(Finding::ValuelessPublic {
        user: user.to_owned(),
        name: name.to_owned(),
        file: public.to_owned(),
        generator: placements
            .producer_of(user, name)
            .map_or_else(|| name.to_owned(), |(entry, _)| entry.to_owned()),
    })
}

/// The three questions asked of one entry at a time: whether a generated
/// value's recorded definition is the declared one, whether a value it read
/// has been written since it ran, and whether an opted-in value has outlived
/// its rotation policy.
///
/// Last, because none is about a file's contents or its recipients: the four
/// questions before this one read the tree the operator is converging, and
/// these read what past runs recorded about a declaration and about a
/// write. An operator who has just fixed the policy expects the recipient
/// findings under it, and one who has just edited a generator expects this at
/// the end.
///
/// One walk rather than three. The deadline, the definition and the inputs'
/// dates are facts about one entry, read out of records in one tree, and a
/// second walk over the same placements would be a second order for the report
/// to be read in. For each entry the order is its deadline, its definition,
/// then — on the entry a generator is declared on — its inputs' dates.
///
/// # Definition drift
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
///
/// # Derived values older than their inputs
///
/// Asked of each entry a generator is declared on, by [`stale`]. A value
/// written by `set`, `edit` or `sync` runs no generator, so a generator that
/// read it still holds a value derived from what it replaced — the state the
/// rotation cascade exists to prevent, and one that reads exactly like a
/// value derived after. The stamps date both writes, so the question is
/// answered from them alone; an absent stamp on either side makes no claim,
/// by the rule [`crate::stamps`] states for an absent record.
///
/// One finding per pair of stamp records, for the reason a definition record
/// is reported once: a shared generator held by two carriers is one run with
/// one remedy.
///
/// # The deadline
///
/// The same one-per-record rule, over the stamp record: a shared value has one
/// write, one deadline and one remedy, whoever holds it. An entry with no
/// policy is silent, and an entry with a policy and no stamp record is due —
/// [`crate::stamps::Deadline::of`] is the single place that decides which, so
/// the report and the countdown cannot disagree.
///
/// The clock is read once, before the walk, so every finding in one report is
/// judged against one instant.
fn entry_records(
    workspace: &Workspace,
    only: Option<&str>,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let placements = workspace.placements()?;
    let mut reported: BTreeSet<String> = BTreeSet::new();
    let mut stale_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut dated: BTreeSet<String> = BTreeSet::new();
    let now = stamps::now();

    for user in placements.users() {
        if only.is_some_and(|wanted| wanted != user) {
            continue;
        }
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            let producer = placements.producer_of(user, name);

            if placement.rotation.is_some() && dated.insert(placement.stamp_record.clone()) {
                let recorded = stamps::read(workspace, placement)?;
                if let Some(overdue) = Deadline::at(recorded, placement, now).overdue() {
                    let policy = placement
                        .rotation
                        .as_ref()
                        .map(|rotation| rotation.policy.clone())
                        .unwrap_or_default();
                    findings.push(Finding::RotationDue {
                        user: user.to_owned(),
                        name: name.clone(),
                        policy,
                        overdue_seconds: overdue,
                        generator: producer.is_some(),
                    });
                }
            }

            let Some((generator, declared)) = producer else {
                continue;
            };
            if let Some(drift) = definition_drift(
                workspace,
                user,
                name,
                placement,
                generator,
                declared,
                &mut reported,
            )? {
                findings.push(drift);
            }
            if placement.generator.is_some() {
                findings.extend(stale(workspace, placements, user, name, &mut stale_pairs)?);
            }
        }
    }
    Ok(())
}

/// Whether this entry's recorded definition is not the declared one; see
/// [`entry_records`]'s note.
fn definition_drift(
    workspace: &Workspace,
    user: &str,
    name: &str,
    placement: &crate::model::Placement,
    generator: &str,
    declared: &crate::model::Generator,
    reported: &mut BTreeSet<String>,
) -> Result<Option<Finding>> {
    let record = definition::record_path(placement);
    if !reported.insert(record.clone()) {
        return Ok(None);
    }
    let Some(text) = workspace.read_vault_relative(&record)? else {
        return Ok(None);
    };
    let Some(recorded) = definition::recorded(&text) else {
        return Ok(None);
    };
    if recorded == definition::digest(declared) {
        return Ok(None);
    }
    Ok(Some(Finding::DefinitionDrift {
        user: user.to_owned(),
        name: name.to_owned(),
        generator: generator.to_owned(),
        record,
    }))
}

/// Every dependency of the generator declared on `entry` written after the
/// values that generator wrote.
///
/// The reference is the earliest `updated` among the stamps of every value the
/// generator writes — every held entry whose producer is this generator — that
/// has one. The earliest, because an output re-set by hand moves its own stamp
/// forward, and the oldest output still standing is the one derived from the
/// oldest inputs. A dependency is stale when its own `updated` is strictly
/// later: a cascade stamps the dependency and then the dependent from two
/// readings of the clock that may fall in the same second.
///
/// `reported` holds the `(dependent stamp, dependency stamp)` pairs already
/// reported in this walk.
///
/// # Errors
///
/// [`Error::StampRecordUnparsable`] and [`Error::FileUnreadable`] from
/// [`stamps::read`].
fn stale(
    workspace: &Workspace,
    placements: &Placements,
    user: &str,
    entry: &str,
    reported: &mut BTreeSet<(String, String)>,
) -> Result<Vec<Finding>> {
    let mut found = Vec::new();
    let Some(held) = placements.held_by(user) else {
        return Ok(found);
    };
    let Some((placement, generator)) = held
        .get(entry)
        .and_then(|placement| Some((placement, placement.generator.as_ref()?)))
    else {
        return Ok(found);
    };

    let mut reference: Option<u64> = None;
    for (name, output) in held {
        if placements
            .producer_of(user, name)
            .map(|(producer, _)| producer)
            != Some(entry)
        {
            continue;
        }
        if let Some(written) = stamps::read(workspace, output)? {
            reference =
                Some(reference.map_or(written.updated, |earliest| earliest.min(written.updated)));
        }
    }
    let Some(reference) = reference else {
        return Ok(found);
    };

    for dependency in &generator.dependencies {
        let Some(input) = held.get(dependency) else {
            continue;
        };
        let Some(written) = stamps::read(workspace, input)? else {
            continue;
        };
        if written.updated > reference
            && reported.insert((placement.stamp_record.clone(), input.stamp_record.clone()))
        {
            found.push(Finding::DerivedValueStale {
                user: user.to_owned(),
                generator: entry.to_owned(),
                dependency: dependency.clone(),
                record: input.stamp_record.clone(),
            });
        }
    }
    Ok(found)
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
                Some(text) => document::keys_of(text)?,
                None => BTreeMap::new(),
            };
            self.keys.insert(file.to_owned(), keys);
        }
        self.keys.get(file).ok_or(Error::SopsPipeMissing)
    }

    fn has_value(&mut self, workspace: &Workspace, file: &str, key: &str) -> Result<bool> {
        if key.is_empty() {
            return Ok(crate::public::holds_a_value(
                &workspace.vault_absolute(file),
            ));
        }
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

    /// One generator, two outputs: the entry it is declared on, which is
    /// encrypted, and a public half it writes.
    ///
    /// The shape `resolve.nix` emits, and the one the false positive lived in:
    /// the public entry carries the audience's document and a key inside it
    /// like every placement, carries no generator of its own because the
    /// generator is declared on its sibling, and carries `public` because it is
    /// an output declared `secret = false`. `logicalPublic` is `null`, which is
    /// what every placement outside vault mode carries and what a check reading
    /// it instead of `public` would find nothing in.
    fn keypair() -> Placements {
        serde_json::from_value(serde_json::json!({
            "alice": {
                "wg-private": {
                    "file": "secrets/safix/users/alice/secrets.yaml", "key": "wg-private",
                    "origin": "private", "owner": "alice", "shared": false,
                    "generator": {
                        "script": "true", "network": false, "runtimeInputs": [],
                        "prompts": {}, "dependencies": [],
                        "files": { "wg-public": { "secret": false } },
                        "share": false, "validation": null, "description": null,
                    },
                    "public": null,
                    "definitionRecord": "state/safix/definitions/alice/wg-private",
                    "logicalFile": null, "logicalKey": null, "logicalPublic": null,
                    "logicalRecord": null,
                    "stampRecord": "state/safix/definitions/alice/wg-private.stamps",
                    "logicalStamp": null, "rotation": null,
                },
                "wg-public": {
                    "file": "secrets/safix/users/alice/secrets.yaml", "key": "wg-public",
                    "origin": "private", "owner": "alice", "shared": false,
                    "generator": null,
                    "public": PUBLIC,
                    "definitionRecord": "state/safix/definitions/alice/wg-public",
                    "logicalFile": null, "logicalKey": null, "logicalPublic": null,
                    "logicalRecord": null,
                    "stampRecord": "state/safix/definitions/alice/wg-public.stamps",
                    "logicalStamp": null, "rotation": null,
                },
            }
        }))
        .expect("the fixture is the shape the resolver emits")
    }

    const PUBLIC: &str = "public/safix/users/alice/wg-public/value";

    /// A tree with no git and no nix in it: [`public_finding`] resolves a path
    /// under the vault root and reads a plaintext file, and reaches neither.
    struct Tree(std::path::PathBuf);

    impl Tree {
        fn new(label: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("safix-check-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("a temporary directory can be made");
            Self(root)
        }

        fn workspace(&self) -> Workspace {
            Workspace::at(
                self.0.clone(),
                self.0.clone(),
                crate::git::Git::default(),
                crate::nix::Nix::from_environment(),
                crate::sops::Sops::from_environment(),
            )
        }

        fn mint(&self, relative: &str, bytes: &str) {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().expect("the value has a parent"))
                .expect("the value's directory can be made");
            std::fs::write(path, bytes).expect("the value can be written");
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A minted public output is converged, and the ciphertext that does not
    /// hold it is not a disagreement.
    ///
    /// The regression this pins: every field the old reading consulted says
    /// "no value" here. The document the placement names does not exist, the
    /// key inside it holds nothing, the entry carries no generator of its own,
    /// and `logicalPublic` is `null`. The one field that can answer is
    /// `public`, and the file it names holds bytes.
    #[test]
    fn a_minted_public_output_is_not_a_finding() {
        let tree = Tree::new("minted");
        tree.mint(PUBLIC, "CANARY-a-public-key\n");
        assert_eq!(
            public_finding(&tree.workspace(), &keypair(), "alice", "wg-public", PUBLIC),
            None
        );
    }

    /// An empty file is what a truncated write leaves behind, so it holds
    /// nothing — the same answer [`crate::public::holds_a_value`] gives the
    /// executor, asked here so the two cannot disagree about whether a run is
    /// still owed.
    #[test]
    fn an_empty_public_file_holds_nothing() {
        let tree = Tree::new("empty");
        tree.mint(PUBLIC, "");
        assert!(
            public_finding(&tree.workspace(), &keypair(), "alice", "wg-public", PUBLIC).is_some()
        );
    }

    /// An unminted public output is reported against its own file, and the
    /// remedy names the entry the generator is declared on rather than the
    /// output, which is never typed and has no `set`.
    #[test]
    fn an_unminted_public_output_names_the_generator_that_writes_it() {
        let tree = Tree::new("unminted");
        assert_eq!(
            public_finding(&tree.workspace(), &keypair(), "alice", "wg-public", PUBLIC),
            Some(Finding::ValuelessPublic {
                user: String::from("alice"),
                name: String::from("wg-public"),
                file: String::from(PUBLIC),
                generator: String::from("wg-private"),
            })
        );
    }

    /// One placement of `name` for `user`, with its stamp under the default
    /// root and, when given, a generator declared on it.
    fn placed(user: &str, name: &str, generator: Option<&serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "file": format!("secrets/safix/users/{user}/secrets.yaml"), "key": name,
            "origin": "private", "owner": user, "shared": false,
            "generator": generator, "public": null,
            "definitionRecord": format!("state/safix/definitions/{user}/{name}"),
            "logicalFile": null, "logicalKey": null, "logicalPublic": null,
            "logicalRecord": null,
            "stampRecord": format!("state/safix/definitions/{user}/{name}.stamps"),
            "logicalStamp": null, "rotation": null,
        })
    }

    /// A generator reading `db-password` and writing `db-hash`, the entry it
    /// is declared on, and `db-salt`.
    fn hashing() -> serde_json::Value {
        serde_json::json!({
            "script": "true", "network": false, "runtimeInputs": [],
            "prompts": {}, "dependencies": ["db-password"],
            "files": { "db-salt": { "secret": true } },
            "share": false, "validation": null, "description": null,
        })
    }

    fn chain(user: &str) -> serde_json::Value {
        serde_json::json!({
            "db-password": placed(user, "db-password", None),
            "db-hash": placed(user, "db-hash", Some(&hashing())),
            "db-salt": placed(user, "db-salt", None),
        })
    }

    fn alice_chain() -> Placements {
        serde_json::from_value(serde_json::json!({ "alice": chain("alice") }))
            .expect("the fixture is the shape the resolver emits")
    }

    impl Tree {
        fn stamp(&self, user: &str, name: &str, updated: u64) {
            self.mint(
                &format!("state/safix/definitions/{user}/{name}.stamps"),
                &format!("v1 created=1 updated={updated}\n"),
            );
        }

        fn stale(&self, placements: &Placements, user: &str) -> Vec<Finding> {
            stale(
                &self.workspace(),
                placements,
                user,
                "db-hash",
                &mut BTreeSet::new(),
            )
            .expect("the stamps parse")
        }
    }

    #[test]
    fn a_dependency_updated_after_its_dependent_is_stale() {
        let tree = Tree::new("stale");
        tree.stamp("alice", "db-hash", 100);
        tree.stamp("alice", "db-salt", 100);
        tree.stamp("alice", "db-password", 101);
        assert_eq!(
            tree.stale(&alice_chain(), "alice"),
            vec![Finding::DerivedValueStale {
                user: "alice".into(),
                generator: "db-hash".into(),
                dependency: "db-password".into(),
                record: "state/safix/definitions/alice/db-password.stamps".into(),
            }]
        );
    }

    /// A cascade stamps the dependency and then the dependent, and both may
    /// fall in one second.
    #[test]
    fn equal_stamps_are_not_stale() {
        let tree = Tree::new("equal");
        tree.stamp("alice", "db-hash", 100);
        tree.stamp("alice", "db-salt", 100);
        tree.stamp("alice", "db-password", 100);
        assert_eq!(tree.stale(&alice_chain(), "alice"), Vec::new());
    }

    #[test]
    fn a_missing_stamp_on_either_side_makes_no_claim() {
        let tree = Tree::new("missing-dependency");
        tree.stamp("alice", "db-hash", 100);
        tree.stamp("alice", "db-salt", 100);
        assert_eq!(tree.stale(&alice_chain(), "alice"), Vec::new());

        let tree = Tree::new("missing-outputs");
        tree.stamp("alice", "db-password", 500);
        assert_eq!(tree.stale(&alice_chain(), "alice"), Vec::new());
    }

    /// An output re-set by hand moves its own stamp forward; the oldest output
    /// still standing is what the dependency is compared against.
    #[test]
    fn the_earliest_output_stamp_is_the_reference() {
        let tree = Tree::new("earliest");
        tree.stamp("alice", "db-hash", 300);
        tree.stamp("alice", "db-salt", 100);
        tree.stamp("alice", "db-password", 200);
        assert_eq!(tree.stale(&alice_chain(), "alice").len(), 1);

        let tree = Tree::new("only-one-output");
        tree.stamp("alice", "db-hash", 300);
        tree.stamp("alice", "db-password", 200);
        assert_eq!(tree.stale(&alice_chain(), "alice"), Vec::new());
    }

    /// Two carriers resolving one generator's records to one set of paths are
    /// one run, reported once.
    #[test]
    fn a_shared_generator_is_reported_once() {
        let tree = Tree::new("shared");
        tree.stamp("alice", "db-hash", 100);
        tree.stamp("alice", "db-salt", 100);
        tree.stamp("alice", "db-password", 200);
        let both: Placements = serde_json::from_value(serde_json::json!({
            "alice": chain("alice"),
            "bob": chain("alice"),
        }))
        .expect("the fixture is the shape the resolver emits");
        let workspace = tree.workspace();
        let mut reported = BTreeSet::new();
        let mut found = Vec::new();
        for user in ["alice", "bob"] {
            found.extend(
                stale(&workspace, &both, user, "db-hash", &mut reported).expect("the stamps parse"),
            );
        }
        assert_eq!(found.len(), 1, "{found:?}");
    }

    /// A declared key and the tag its stanza would carry.
    fn key(name: &str, tag: &str) -> Tagged {
        Tagged {
            key: format!("ssh-ed25519 AAAA{name}"),
            tag: Some(("ssh-ed25519".into(), tag.into())),
        }
    }

    fn untagged(key: &str) -> Tagged {
        Tagged {
            key: key.into(),
            tag: None,
        }
    }

    fn stanza(kind: &str, tag: &str) -> age_header::Stanza {
        age_header::Stanza {
            kind: kind.into(),
            first_argument: Some(tag.into()),
        }
    }

    fn ssh(tag: &str) -> age_header::Stanza {
        stanza("ssh-ed25519", tag)
    }

    fn nobody() -> Recipients {
        Recipients(BTreeMap::new())
    }

    const FILE: &str = "secrets/safix/users/alice/token.age";

    fn judged(stanzas: &[age_header::Stanza], declared: &[Tagged]) -> Vec<Finding> {
        judge_age(FILE, stanzas, declared, declared, &nobody(), Vec::new)
    }

    fn no_holders() -> Holders {
        Holders {
            named: Vec::new(),
            orphaned: Vec::new(),
        }
    }

    #[test]
    fn extra_missing_and_duplicate_ssh_stanzas_are_drift() {
        let declared = [key("alice", "AAAAAA"), key("bob", "BBBBBB")];
        let findings = judged(&[ssh("AAAAAA"), ssh("CCCCCC"), ssh("AAAAAA")], &declared);
        assert_eq!(
            findings,
            vec![Finding::RecipientTagDrift {
                file: FILE.into(),
                missing: vec!["ssh-ed25519 AAAAbob".into()],
                extra: vec!["ssh-ed25519 CCCCCC".into()],
                duplicate: vec!["ssh-ed25519 AAAAalice".into()],
                narrowed: no_holders(),
                mints: Vec::new(),
            }]
        );
        assert!(findings.iter().all(Finding::fails));
    }

    /// Two declared keys sharing a tag expect two stanzas: two are a match,
    /// and one leaves both keys missing because the tag cannot say which.
    #[test]
    fn colliding_declared_tags_expect_two_stanzas() {
        let declared = [key("alice", "AAAAAA"), key("carol", "AAAAAA")];
        assert_eq!(
            judged(&[ssh("AAAAAA"), ssh("AAAAAA")], &declared),
            vec![Finding::RecipientRosterTagMatched {
                file: FILE.into(),
                matched: 2,
            }]
        );
        assert_eq!(
            judged(&[ssh("AAAAAA")], &declared),
            vec![Finding::RecipientTagDrift {
                file: FILE.into(),
                missing: vec![
                    "ssh-ed25519 AAAAalice".into(),
                    "ssh-ed25519 AAAAcarol".into()
                ],
                extra: Vec::new(),
                duplicate: Vec::new(),
                narrowed: no_holders(),
                mints: Vec::new(),
            }]
        );
    }

    /// A stanza whose tag exactly one declared key anywhere has is named as
    /// that key, and its holder is the narrowing; a tag two keys share is
    /// named by type and tag.
    #[test]
    fn an_extra_tag_held_by_one_declared_subject_is_named() {
        let alice = key("alice", "AAAAAA");
        let bob = key("bob", "BBBBBB");
        let holders = Recipients(BTreeMap::from([
            ("alice".to_owned(), vec![alice.key.clone()]),
            ("bob".to_owned(), vec![bob.key.clone()]),
        ]));
        let mint = || {
            vec![Mint {
                carrier: "alice".into(),
                name: "token".into(),
                generated: false,
            }]
        };
        let everywhere = [alice.clone(), bob.clone()];
        assert_eq!(
            judge_age(
                FILE,
                &[ssh("AAAAAA"), ssh("BBBBBB")],
                std::slice::from_ref(&alice),
                &everywhere,
                &holders,
                mint,
            ),
            vec![Finding::RecipientTagDrift {
                file: FILE.into(),
                missing: Vec::new(),
                extra: vec![bob.key.clone()],
                duplicate: Vec::new(),
                narrowed: Holders {
                    named: vec!["bob".into()],
                    orphaned: Vec::new(),
                },
                mints: mint(),
            }]
        );

        let everywhere = [alice.clone(), bob, key("dave", "BBBBBB")];
        let findings = judge_age(
            FILE,
            &[ssh("AAAAAA"), ssh("BBBBBB")],
            std::slice::from_ref(&alice),
            &everywhere,
            &holders,
            Vec::new,
        );
        let [
            Finding::RecipientTagDrift {
                extra, narrowed, ..
            },
        ] = findings.as_slice()
        else {
            unreachable!("one drift finding, got {findings:?}");
        };
        assert_eq!(extra, &vec![String::from("ssh-ed25519 BBBBBB")]);
        assert_eq!(narrowed, &no_holders());
    }

    #[test]
    fn a_non_ssh_stanza_keeps_the_file_unverifiable() {
        let declared = [key("alice", "AAAAAA")];
        assert_eq!(
            judged(&[ssh("AAAAAA"), stanza("X25519", "c2hhcmU")], &declared),
            vec![Finding::RecipientRosterUnverifiable { file: FILE.into() }]
        );
        assert_eq!(
            judged(&[ssh("CCCCCC"), stanza("piv-p256", "dGFn")], &declared),
            vec![
                Finding::RecipientTagDrift {
                    file: FILE.into(),
                    missing: vec!["ssh-ed25519 AAAAalice".into()],
                    extra: vec!["ssh-ed25519 CCCCCC".into()],
                    duplicate: Vec::new(),
                    narrowed: no_holders(),
                    mints: Vec::new(),
                },
                Finding::RecipientRosterUnverifiable { file: FILE.into() },
            ],
            "an SSH disagreement in an unverifiable file is reported as well"
        );
    }

    #[test]
    fn a_non_ssh_declared_recipient_keeps_the_file_unverifiable() {
        let declared = [key("alice", "AAAAAA"), untagged("age1someone")];
        assert_eq!(
            judged(&[ssh("AAAAAA")], &declared),
            vec![Finding::RecipientRosterUnverifiable { file: FILE.into() }]
        );
    }

    #[test]
    fn an_exact_all_ssh_match_is_a_note_that_does_not_fail() {
        let declared = [key("alice", "AAAAAA"), key("bob", "BBBBBB")];
        let findings = judged(&[ssh("BBBBBB"), ssh("AAAAAA")], &declared);
        assert_eq!(
            findings,
            vec![Finding::RecipientRosterTagMatched {
                file: FILE.into(),
                matched: 2,
            }]
        );
        assert!(!findings.iter().any(Finding::fails));
    }

    const RECORD: &str = "state/safix/definitions/alice/token.audience";

    fn digest_of(recipients: &[&str]) -> String {
        age_audience::audience_digest(
            &recipients
                .iter()
                .map(|recipient| (*recipient).to_owned())
                .collect::<Vec<_>>(),
        )
    }

    /// The file was rewritten after its record, so the record says nothing
    /// about it — even naming an audience no longer declared.
    #[test]
    fn a_record_for_other_bytes_is_silent() {
        let text = age_audience::line(&digest_of(&["age1alice", "age1bob"]), b"older bytes");
        assert_eq!(
            audience_changed(
                FILE,
                RECORD,
                Some(&text),
                b"newer bytes",
                &digest_of(&["age1alice"])
            ),
            None
        );
    }

    #[test]
    fn an_absent_record_is_silent() {
        assert_eq!(
            audience_changed(FILE, RECORD, None, b"bytes", &digest_of(&["age1alice"])),
            None
        );
        assert_eq!(
            audience_changed(
                FILE,
                RECORD,
                Some("safix-age-audience-v0 x y\n"),
                b"bytes",
                &digest_of(&["age1alice"])
            ),
            None
        );
    }

    #[test]
    fn a_recorded_audience_no_longer_declared_is_reported() {
        let text = age_audience::line(&digest_of(&["age1alice", "age1bob"]), b"bytes");
        assert_eq!(
            audience_changed(
                FILE,
                RECORD,
                Some(&text),
                b"bytes",
                &digest_of(&["age1alice"])
            ),
            Some(Finding::AgeAudienceChanged {
                file: FILE.into(),
                record: RECORD.into(),
            })
        );
        assert_eq!(
            audience_changed(
                FILE,
                RECORD,
                Some(&text),
                b"bytes",
                &digest_of(&["age1bob", "age1alice"])
            ),
            None,
            "the declared audience, in any order, is no finding"
        );
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
