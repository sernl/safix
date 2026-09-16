//! Converging a declared safix entry and a declared database entry, per mode.
//!
//! The mappings are declarations — see `modules/flake/safix/keepassxc.nix` for
//! why a standing relationship is written down rather than passed as arguments —
//! and this module is what acts on them. [`crate::store`] is how the database is
//! reached, and nothing here opens the file itself.
//!
//! # Two phases, because every write rewrites the whole database
//!
//! A kdbx save rewrites and re-uploads the entire file, which on the fleet this
//! was written for is 292 MB. So a run reads both sides of every mapping and
//! decides, and only then writes: every database write is issued consecutively,
//! with no read between two of them, and the safix writes follow. A run over
//! mappings that agree writes nothing anywhere, which is the property the whole
//! shape exists to hold.
//!
//! # A pull is the ordinary write path wearing a different source
//!
//! `keepassxc-to-safix` and the pulling half of `two-way` feed the database's
//! value to [`crate::set::run_committing`] through a [`ValueSource`] holding it.
//! Everything that path does happens: the empty-value refusal, the
//! recipient-drift refusal, the staged write, the rename, and a commit naming the
//! mapping and never the value. An imported value takes the hand-set path's
//! refusals because it *is* the hand-set path.
//!
//! # Two-way remembers inside the encrypted store, and that is a security
//! decision
//!
//! Three-way convergence needs the last agreed state, and a committed digest of
//! a secret value is an oracle: anyone holding the tree could confirm a guess
//! offline. So the memory is a digest of the agreed value held in the *password*
//! of a companion entry beside the mapped one — [`store::companion_of`] — and the
//! repository carries no value-derived state at all.
//!
//! The memory is written only as part of a converging write, never on its own.
//! Two consequences, both stated rather than discovered. A two-way mapping whose
//! sides already agree before safix ever ran has no memory, so its first
//! divergence is a conflict rather than a guess; and a run interrupted between a
//! converging write and its memory leaves a memory of the older agreement, whose
//! next divergence is also a conflict. Both are the safe direction: a conflict
//! writes nothing and names two remedies, where a guess would pick a side.
//!
//! # No value appears anywhere
//!
//! A report line names the mapping, its two endpoints and its outcome. A commit
//! message names the mapping. The comparison that decides between `Unchanged` and
//! a write is [`Secret::equals`] over two values that are zeroed when this
//! returns, and the digest that decides a two-way tiebreak never leaves the
//! database.

use crate::endpoint::{self, ResolvedFields, Verdict};
use crate::enroll::custody::DatabasePassword;
use crate::error::{Error, Result};
use crate::model::{Keepassxc, Mode, SyncMapping};
use crate::progress::{Progress, log};
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::store::{self, Database};
use crate::workspace::Workspace;
use crate::{bridge, enroll, scratch};

/// The tag the recorded agreement carries, and the whole of how a change to what
/// it covers is told apart from universal drift.
///
/// The reasoning [`crate::definition::FORMAT`] gives, applied to a second
/// record: a memory whose tag this version does not write is read as no memory
/// at all, which takes the mapping to bootstrap semantics rather than to a
/// conflict on every entry.
pub const FORMAT: &str = "safix-sync-v1";

/// What happened to one mapping.
///
/// No `Debug` on the enum for the reason [`crate::bridge::Outcome`] has none:
/// nothing here holds a value, and keeping it that way is easier than proving
/// each future variant does not.
pub enum Outcome {
    /// Both sides already held the same bytes. Nothing written anywhere.
    Unchanged,
    /// The database now holds what safix holds.
    Updated,
    /// safix now holds what the database holds, through the ordinary write path.
    Pulled,
    /// The two sides' values already agreed and the declared fields were
    /// written, in one write carrying the value the database already held.
    ///
    /// The names of the fields written, never their contents: a note may itself
    /// be sensitive, so the report is a `&'static str` per field and no
    /// run-time string can reach it.
    FieldsUpdated(Vec<&'static str>),
    /// The declared fields differ from the entry's and this mode does not write
    /// them, so nothing was written.
    ///
    /// The names of the fields, for the reason [`Self::FieldsUpdated`] carries
    /// them.
    FieldsDiverged(Vec<&'static str>),
    /// The two sides disagree and the mode does not say who wins, so nothing was
    /// written.
    Conflict,
    /// This mapping was refused, and this is why.
    Refused(Error),
    /// This mapping could not be judged, and this is why.
    ///
    /// Reported rather than skipped, for the reason [`crate::audit`] gives:
    /// dropping it would make the report a function of who ran it, and a clean
    /// report would read as "the mappings agree" while meaning "the ones I could
    /// look at agree".
    NotJudged(Error),
}

impl Outcome {
    /// The word a report prints for this outcome.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Updated => "updated",
            Self::Pulled => "pulled",
            Self::FieldsUpdated(_) => "fields updated",
            Self::FieldsDiverged(_) => "fields diverged",
            Self::Conflict => "conflict",
            Self::Refused(_) => "refused",
            Self::NotJudged(_) => "not judged",
        }
    }

    /// Whether this outcome makes the run a failure.
    ///
    /// A conflict does, because it is a state the operator has to resolve; a
    /// mapping that could not be judged does, because a clean exit would say
    /// something the run does not know. A field divergence does, on the same
    /// footing: a declared field that is not there is a declaration that is not
    /// true, and answering whether the declarations are true is the whole
    /// purpose of reporting at all. A field that *was* written does not, for
    /// the reason an updated value does not: the run resolved it.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Conflict | Self::Refused(_) | Self::NotJudged(_) | Self::FieldsDiverged(_)
        )
    }
}

/// One mapping's line in a run's report.
pub struct Converged {
    /// The mapping's declared name.
    pub mapping: String,
    /// How it converges.
    pub mode: Mode,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// The database endpoint, as the entry path under the declared group.
    pub kdbx: String,
    /// What happened.
    pub outcome: Outcome,
}

/// Everything a run judged, and what it found beside it.
pub struct Report {
    /// The database the run converged against.
    pub database: String,
    /// One entry per declared mapping, in declaration order. Every declared
    /// mapping is here, whatever happened to it.
    pub converged: Vec<Converged>,
    /// Entries under the declared group that no declared mapping names,
    /// including the companions of mappings that are gone.
    ///
    /// Information rather than a finding: no mode deletes an entry, so a mapping
    /// that was removed leaves its last value behind on purpose, and only a
    /// person removes it.
    pub lingering: Vec<String>,
}

impl Report {
    /// Whether every mapping converged without a conflict, a refusal or an
    /// unjudgeable side.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        !self
            .converged
            .iter()
            .any(|entry| entry.outcome.is_failure())
    }

    /// How many mappings ended in each outcome.
    #[must_use]
    pub fn tally(&self) -> Tally {
        let count = |wanted: &str| {
            self.converged
                .iter()
                .filter(|entry| entry.outcome.as_str() == wanted)
                .count()
        };
        Tally {
            unchanged: count("unchanged"),
            updated: count("updated"),
            pulled: count("pulled"),
            fields_updated: count("fields updated"),
            fields_diverged: count("fields diverged"),
            conflict: count("conflict"),
            refused: count("refused"),
            not_judged: count("not judged"),
        }
    }
}

/// The counts a run's closing line reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Mappings whose two sides already agreed.
    pub unchanged: usize,
    /// Mappings whose database side was written.
    pub updated: usize,
    /// Mappings whose safix side was written.
    pub pulled: usize,
    /// Mappings whose value already agreed and whose declared fields were
    /// written.
    pub fields_updated: usize,
    /// Mappings whose declared fields differ and whose mode does not write
    /// them.
    pub fields_diverged: usize,
    /// Mappings whose two sides disagree with nobody to decide.
    pub conflict: usize,
    /// Mappings refused.
    pub refused: usize,
    /// Mappings that could not be judged.
    pub not_judged: usize,
}

/// A value already in hand, for the write path that expects to read one.
///
/// The same seam [`crate::bridge`] uses and for the same reason: a mirrored value
/// was read once, from the database, and there is nothing to compare it against,
/// so this hands it over and the rest of the write path is unchanged.
struct Held(Option<Secret>);

impl ValueSource for Held {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        self.0.take().ok_or(Error::NoValueRead)
    }
}

/// What a mapping's two sides came back as, and what is to be done about it.
///
/// No `Debug`, and every value here is zeroed when the run returns.
enum Decision {
    /// Nothing to do.
    Settled(Outcome),
    /// Write the database's side.
    Push(Pushing),
    /// Write this value into safix, and record the agreement when the mode
    /// remembers one.
    Pull { value: Secret, remember: bool },
}

/// One database write: the value, the declared fields beside it, and whether
/// the agreement is recorded afterwards.
///
/// One push arm and one struct rather than a second arm for a field repair,
/// because a value repair and a field repair are the same single `add` or
/// `edit`: a kdbx save rewrites the whole file, so a second write path to the
/// same command would be a second whole-file save of the same entry.
///
/// A struct rather than four fields on the variant so that [`push`] takes one
/// parameter for them rather than four; the four are what
/// `openspec/changes/extend-bridge-fields/tasks.md` names.
struct Pushing {
    /// The value the entry will hold, which is the one it already holds when
    /// only the fields moved.
    value: Secret,
    /// Whether this mode records the agreement afterwards.
    remember: bool,
    /// The declared fields, resolved, that ride along in the same write.
    resolved: ResolvedFields,
    /// The declared fields the entry did not match, when the value itself
    /// agreed; empty when the value is what moved.
    ///
    /// Empty is what makes the report say `updated`: a value divergence is
    /// decided first and a field divergence never displaces it, so the only
    /// push that reports `fields updated` is one whose value agreed.
    drifted: Vec<&'static str>,
}

impl Pushing {
    /// What a completed write of this push is reported as.
    ///
    /// The whole of the precedence between a value and a field, in one place:
    /// an empty `drifted` is a value that moved, and a non-empty one is a value
    /// that agreed and fields that did not.
    fn outcome(self) -> Outcome {
        if self.drifted.is_empty() {
            return Outcome::Updated;
        }
        Outcome::FieldsUpdated(self.drifted)
    }
}

/// Converge every declared mapping, or the ones named.
///
/// # Errors
///
/// [`Error::NoStoreDatabase`] when mappings are declared and no database is,
/// [`Error::UnknownSyncMapping`] when a named mapping is not one that is
/// declared, [`Error::StoreLocked`] when there is no terminal to ask for the
/// database's password on, [`Error::DatabaseUnreadable`] when it will not open,
/// and whatever evaluating the declarations failed with. Each of those stops the
/// whole run and is raised before the first mapping is read, for the reason
/// [`crate::bridge`] raises its own there: a run that discovered them partway
/// through would already have said "unchanged" about mappings it never looked
/// at. Anything about one mapping is that mapping's outcome rather than the
/// run's.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    password: &mut dyn DatabasePassword,
    only: &[String],
) -> Result<Report> {
    scratch::set_floor(workspace.vault_root());
    let _guard = scratch::Guard;

    let mirror = workspace.keepassxc()?;
    let selected = selected(mirror, only)?;
    if selected.is_empty() {
        return Ok(Report {
            database: mirror.database.clone().unwrap_or_default(),
            converged: Vec::new(),
            lingering: Vec::new(),
        });
    }

    let Some(named) = mirror.database.clone() else {
        return Err(Error::NoStoreDatabase {
            mappings: mirror.mappings.len(),
        });
    };
    // Before the password is asked for and before any side is read, which is what
    // makes this a refusal that costs the operator nothing: a run that prompted
    // into the void would already have decrypted safix's side of every mapping.
    if !enroll::terminal_present() {
        return Err(Error::StoreLocked { database: named });
    }
    let mut database = Database::open(std::path::PathBuf::from(&named), mirror, password)?;

    let mut decided: Vec<(&SyncMapping, Decision)> = Vec::with_capacity(selected.len());
    for mapping in &selected {
        decided.push((*mapping, decide(workspace, &database, mirror, mapping)));
    }

    // Every database write, consecutively. The reads are all behind us, so this
    // is the burst the whole-file rewrite cost is bounded by.
    let mut written: Vec<(&SyncMapping, Outcome)> = Vec::with_capacity(decided.len());
    let mut pulls: Vec<(&SyncMapping, Secret, bool)> = Vec::new();
    for (mapping, decision) in decided {
        match decision {
            Decision::Settled(outcome) => written.push((mapping, outcome)),
            Decision::Push(pushing) => {
                let outcome = push(progress, &mut database, mirror, mapping, pushing);
                written.push((mapping, outcome));
            }
            // Held rather than acted on here, so that the database writes stay
            // one burst: a pull commits in this repository, and a commit between
            // two database writes is a commit inside the window the burst exists
            // to keep one save wide.
            Decision::Pull { value, remember } => pulls.push((mapping, value, remember)),
        }
    }

    for (mapping, value, remember) in pulls {
        let outcome = pull(
            workspace,
            progress,
            &mut database,
            mirror,
            mapping,
            value,
            remember,
        );
        written.push((mapping, outcome));
    }

    // Back into declaration order, which is the order the mappings were selected
    // in: the report is about the declarations rather than about the order a run
    // happened to act in.
    let mut converged: Vec<Converged> = Vec::with_capacity(written.len());
    for mapping in &selected {
        let Some(at) = written.iter().position(|(done, _)| done.id == mapping.id) else {
            continue;
        };
        let (_, outcome) = written.remove(at);
        converged.push(Converged {
            mapping: mapping.id.clone(),
            mode: mapping.mode,
            safix: format!("{}.{}", mapping.safix.user, mapping.safix.name),
            kdbx: mirror.entry_of(mapping),
            outcome,
        });
    }

    Ok(Report {
        database: database.path().display().to_string(),
        lingering: lingering(&database, mirror),
        converged,
    })
}

/// The mappings one run acts on, refusing before any of them is touched.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s keepassxc target reuses
/// this exact selection so that scoping a comparison and scoping a write cannot
/// answer "which mappings" differently.
///
/// Deliberately not shared with [`crate::bridge::selected`], which answers the
/// same question for the clan target. Their refusals name different declared
/// lists and that one additionally refuses `MappingWrongDirection`, so one
/// generic selection would cost a per-target refusal closure and a per-target
/// mapping trait to save these lines. The duplication that [`crate::endpoint`]
/// exists to delete is the judge, not this.
pub(crate) fn selected<'a>(mirror: &'a Keepassxc, only: &[String]) -> Result<Vec<&'a SyncMapping>> {
    if only.is_empty() {
        return Ok(mirror.mappings.iter().collect());
    }
    let mut mappings = Vec::with_capacity(only.len());
    for id in only {
        let mapping = mirror.named(id).ok_or_else(|| Error::UnknownSyncMapping {
            mapping: id.clone(),
            declared: mirror.declared(),
        })?;
        mappings.push(mapping);
    }
    Ok(mappings)
}

/// Entries under the declared group that no declared mapping accounts for.
///
/// Every mapping accounts for its own entry and for the companion beside it, so a
/// companion whose mapping is gone lingers exactly as its entry does — which is
/// the point of computing this from the listing rather than from the mappings.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s keepassxc target reports
/// the identical list, over a database it opened itself, rather than a second
/// computation of it.
pub(crate) fn lingering(database: &Database, mirror: &Keepassxc) -> Vec<String> {
    let mut claimed: Vec<String> = Vec::new();
    for mapping in &mirror.mappings {
        let entry = mirror.entry_of(mapping);
        claimed.push(store::companion_of(&entry));
        claimed.push(entry);
    }
    database
        .under(&mirror.group)
        .filter(|entry| !claimed.iter().any(|held| held == entry))
        .map(str::to_owned)
        .collect()
}

/// Read both sides of one mapping and decide what its mode says to do.
///
/// # The order, which is a precedence rather than a sequence
///
/// The value verdict is decided first and a field divergence never displaces
/// it: a mapping whose value and whose declared fields both differ is `updated`
/// or `conflict`, never `fields diverged`, because the same write that repairs
/// the value carries the fields, and reporting the lesser fact would bury the
/// greater one. That is held in the code by where `drifted` is filled in: only
/// the arms whose two values already agree carry it, so a push that repairs a
/// value reports `updated` by construction rather than by a later check.
///
/// # Two reads, and the second only sometimes
///
/// The fields read is its own invocation and happens only for a mapping that
/// declares a field over an entry that is there. Both reads are in this
/// function, which is entirely before the first write of a run, so the write
/// burst the whole-file save requires is unaffected.
fn decide(
    workspace: &Workspace,
    database: &Database,
    mirror: &Keepassxc,
    mapping: &SyncMapping,
) -> Decision {
    let entry = mirror.entry_of(mapping);

    // Before either side is read: a declaration this target cannot honour is
    // refused rather than partly written, and the refusal costs no read.
    let declared = match endpoint::resolve_fields(
        workspace,
        "keepassxc",
        &mapping.safix.user,
        &mapping.kdbx.fields,
        &database.capabilities(),
    ) {
        Ok(declared) => declared,
        Err(reason) => return Decision::Settled(Outcome::Refused(reason)),
    };

    let theirs = match database.read(&entry) {
        Ok(held) => held,
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
    };
    // An entry that is not there carries no field, and asking the store about
    // one would be asking about an entry its own listing says is absent.
    let held_fields = if theirs.is_some() {
        match database.read_fields(&entry, &declared) {
            Ok(held) => held,
            Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
        }
    } else {
        ResolvedFields::default()
    };
    let ours = match bridge::held_by_safix(
        workspace,
        &mapping.id,
        &mapping.safix.user,
        &mapping.safix.name,
    ) {
        Ok(held) => held,
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
    };

    let drifted = declared.diff(&held_fields);

    match mapping.mode {
        Mode::SafixToKeepassxc => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                repairing_fields(ours, declared, drifted, false)
            }
            (Some(ours), _) => Decision::Push(Pushing {
                value: ours,
                remember: false,
                resolved: declared,
                drifted: Vec::new(),
            }),
        },

        Mode::KeepassxcToSafix => match (ours, theirs) {
            (_, None) => Decision::Settled(Outcome::Refused(Error::StoreEntryAbsent {
                mapping: mapping.id.clone(),
                entry,
                mode: mapping.mode.as_str(),
            })),
            // The declaration is the author of a field, and this mode writes
            // safix rather than the database, so a field difference is reported
            // and nothing is written.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            (_, Some(theirs)) => Decision::Pull {
                value: theirs,
                remember: false,
            },
        },

        Mode::Backup => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            // Absence is the one case `backup` writes, so the declared fields
            // ride along with the value that creates the entry.
            (Some(ours), None) => Decision::Push(Pushing {
                value: ours,
                remember: false,
                resolved: declared,
                drifted: Vec::new(),
            }),
            // `backup` never overwrites an existing entry, and writing its
            // fields while refusing its value would make `backup` half a mode.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            // The whole of what `backup` is: an existing differing value is
            // reported and never overwritten.
            (Some(_), Some(_)) => Decision::Settled(Outcome::Conflict),
        },

        Mode::TwoWay => two_way(database, &entry, ours, theirs, declared, drifted),
    }
}

/// A two-way mapping's decision: the shared judge's verdict over the two
/// values and the agreement the companion entry remembers, worded as this
/// target's own.
///
/// `agrees` is passed into the judge rather than derived inside it, because
/// this mechanism's memory carries [`FORMAT`] and the bridge target's carries
/// its own, deliberately distinct tag.
fn two_way(
    database: &Database,
    entry: &str,
    ours: Option<Secret>,
    theirs: Option<Secret>,
    declared: ResolvedFields,
    drifted: Vec<&'static str>,
) -> Decision {
    let remembered = recorded(database, entry);
    let agreement = |remembered: &Secret, value: &Secret| agrees(remembered, value);
    let verdict = endpoint::judge(
        ours.as_ref(),
        theirs.as_ref(),
        remembered.as_ref(),
        &agreement,
    );
    match verdict {
        Verdict::Unchanged => match ours {
            // A field has one author, so a two-way mapping's fields converge
            // toward the declaration — and no agreement is recorded for one,
            // because there is nothing for a memory to arbitrate.
            Some(ours) => repairing_fields(ours, declared, drifted, false),
            None => Decision::Settled(Outcome::Unchanged),
        },
        Verdict::Conflict => Decision::Settled(Outcome::Conflict),
        Verdict::PushValue { remember } => match ours {
            Some(value) => Decision::Push(Pushing {
                value,
                remember,
                resolved: declared,
                drifted: Vec::new(),
            }),
            None => unreachable!("a push was asked for where safix holds no value"),
        },
        Verdict::PullValue { remember } => match theirs {
            Some(value) => Decision::Pull { value, remember },
            None => unreachable!("a pull was asked for where the database holds no value"),
        },
    }
}

/// The two sides' values agree: either there is nothing to do, or the declared
/// fields moved and this mode writes them.
fn repairing_fields(
    ours: Secret,
    resolved: ResolvedFields,
    drifted: Vec<&'static str>,
    remember: bool,
) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Push(Pushing {
        value: ours,
        remember,
        resolved,
        drifted,
    })
}

/// The two sides' values agree and this mode does not write the database's
/// side, so a field difference is the finding.
fn diverging_fields(drifted: Vec<&'static str>) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Settled(Outcome::FieldsDiverged(drifted))
}

/// The agreement the companion entry remembers, as the bytes it holds.
///
/// A companion that will not read is treated as absent rather than as a refusal:
/// the memory is safix's own bookkeeping, and a run that stopped over it would
/// refuse a mapping whose two sides it can see perfectly well.
fn recorded(database: &Database, entry: &str) -> Option<Secret> {
    database.read(&store::companion_of(entry)).ok().flatten()
}

/// Whether the remembered agreement is the one this value would have recorded.
///
/// Compared as bytes against the line a converging write would have written,
/// rather than by parsing the memory into a tag and a digest. That is what makes
/// a memory written under a tag this version does not know behave as no memory:
/// it matches neither side, and every path that consults it then reports a
/// conflict rather than guessing — which is what bootstrap semantics do with two
/// differing values too.
fn agrees(remembered: &Secret, value: &Secret) -> bool {
    let line = memory_of(value);
    Secret::read_from(&mut line.as_bytes()).is_ok_and(|written| written.equals(remembered))
}

/// The line a converging two-way write records the agreement as.
///
/// `pub(crate)` rather than public: it is a derivative of a value, and the one
/// place it may land is the encrypted database.
pub(crate) fn memory_of(value: &Secret) -> String {
    format!("{FORMAT} {}", value.fingerprint())
}

/// safix holds nothing for this mapping, and its mode makes safix the source.
fn empty_source(workspace: &Workspace, mapping: &SyncMapping) -> Error {
    let (file, generated) = workspace
        .resolve(&mapping.safix.user, &mapping.safix.name)
        .map_or_else(
            |_| (String::new(), false),
            |placement| (placement.file.clone(), placement.generator.is_some()),
        );
    Error::SyncSourceEmpty {
        mapping: mapping.id.clone(),
        user: mapping.safix.user.clone(),
        name: mapping.safix.name.clone(),
        file,
        generated,
    }
}

/// Write the database's side — the value and the declared fields in one write —
/// and the agreement when the mode remembers one.
///
/// The log line names the two endpoints and no field, because the log is about
/// which sides a value moved between; which fields were written is the report's
/// own sentence, under the mapping's line.
fn push(
    progress: &dyn Progress,
    database: &mut Database,
    mirror: &Keepassxc,
    mapping: &SyncMapping,
    pushing: Pushing,
) -> Outcome {
    let entry = mirror.entry_of(mapping);
    log(
        progress,
        &format!(
            "safix: flake.safix.users.{}.{} -> {entry}",
            mapping.safix.user, mapping.safix.name,
        ),
    );
    if let Err(reason) = database.write(&entry, &pushing.value, &pushing.resolved) {
        return Outcome::Refused(reason);
    }
    if let Err(reason) = remembered_after(database, &entry, &pushing.value, pushing.remember) {
        return Outcome::Refused(reason);
    }
    pushing.outcome()
}

/// Write safix's side through the ordinary write path, and the agreement when
/// the mode remembers one.
///
/// Only the value crosses. A field is a declaration and safix's side has
/// nowhere to hold one — a safix entry is a file, a key inside it and an
/// audience — so this function is unchanged by the field axis except for the
/// companion's own write, which carries no field because a companion is
/// safix's bookkeeping rather than a mirrored entry.
fn pull(
    workspace: &Workspace,
    progress: &dyn Progress,
    database: &mut Database,
    mirror: &Keepassxc,
    mapping: &SyncMapping,
    value: Secret,
    remember: bool,
) -> Outcome {
    let entry = mirror.entry_of(mapping);
    log(
        progress,
        &format!(
            "safix: {entry} -> flake.safix.users.{}.{}",
            mapping.safix.user, mapping.safix.name,
        ),
    );

    // The memory is written from a second copy, because the write path below
    // consumes the value it is handed.
    let memory = memory_of(&value);
    let status = set::run_committing(
        workspace,
        progress,
        &mut Held(Some(value)),
        &mapping.safix.user,
        &mapping.safix.name,
        &commit_subject(mapping),
    );
    match status {
        Err(reason) => return Outcome::Refused(reason),
        // sops refused and has said why on its own standard error, which is
        // inherited. Reporting it as this mapping's refusal is what keeps the
        // rest of the run going and the report honest about which mapping it was.
        Ok(status) if status != 0 => {
            return Outcome::Refused(Error::StoreCommandFailed {
                entry: entry.clone(),
                arguments: String::from("<the safix write path>"),
                output: format!("sops exited {status}; its own message is above"),
            });
        }
        Ok(_) => {}
    }

    if remember {
        let recorded = match Secret::read_from(&mut memory.as_bytes()) {
            Ok(recorded) => recorded,
            Err(reason) => return Outcome::Refused(reason),
        };
        if let Err(reason) = database.write(
            &store::companion_of(&entry),
            &recorded,
            &ResolvedFields::default(),
        ) {
            return Outcome::Refused(reason);
        }
    }
    Outcome::Pulled
}

/// Record the agreement, after the value it is about has landed.
///
/// This order is load-bearing and the other one loses data. A memory written
/// first and then not followed by its value would say the two sides agreed on a
/// value only one of them holds, and the next run would read that as "the side
/// holding the new value never changed" and converge the other way — overwriting
/// the new value with the old one.
///
/// The companion carries no declared field, because it is safix's own
/// bookkeeping rather than an entry a mapping mirrors: nothing declares it, so
/// there is nothing to declare about it.
fn remembered_after(
    database: &mut Database,
    entry: &str,
    value: &Secret,
    remember: bool,
) -> Result<()> {
    if !remember {
        return Ok(());
    }
    let line = memory_of(value);
    let recorded = Secret::read_from(&mut line.as_bytes())?;
    database.write(
        &store::companion_of(entry),
        &recorded,
        &ResolvedFields::default(),
    )
}

/// The commit subject a mirrored value lands under, for a caller that wants to
/// assert it without running a sync.
///
/// The shape [`crate::bridge::commit_subject`] has, because a pulled value is
/// not "set by hand" either and a commit saying it was would be the one sentence
/// in the history that is wrong about where the value came from.
#[must_use]
pub fn commit_subject(mapping: &SyncMapping) -> String {
    format!(
        "chore(safix): sync {} for {}",
        mapping.id, mapping.safix.user
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mirror(mode: &str) -> Keepassxc {
        let document = format!(
            r#"{{
              "database": "/keys/master.kdbx",
              "group": "safix",
              "mappings": [
                {{
                  "id": "grafana",
                  "mode": "{mode}",
                  "safix": {{ "user": "alice", "name": "grafana-password" }},
                  "kdbx": {{ "path": "alice/grafana", "fields": {{}} }}
                }}
              ]
            }}"#
        );
        serde_json::from_str(&document).expect("a fixture mirror")
    }

    #[test]
    fn the_memory_carries_a_format_tag_and_a_digest_of_the_value() {
        let value = Secret::read_from(&mut b"fixture".as_slice()).expect("a fixture value");
        let line = memory_of(&value);
        assert!(line.starts_with("safix-sync-v1 "));
        // The digest of the seven bytes `fixture`, taken with `sha256sum` rather
        // than with this crate's own function: a literal re-derived through the
        // code under test would assert only that the code agrees with itself.
        assert_eq!(
            line,
            "safix-sync-v1 f16d05ec6b29248d2c61adb1e9263f78e4f7bace1b955014a2d17872cfe4064d"
        );
    }

    #[test]
    fn a_commit_subject_names_the_mapping_and_the_person_and_never_a_value() {
        let mirror = mirror("keepassxc-to-safix");
        let mapping = mirror.named("grafana").expect("the fixture mapping");
        assert_eq!(
            commit_subject(mapping),
            "chore(safix): sync grafana for alice"
        );
    }

    #[test]
    fn a_named_mapping_nothing_declares_is_refused_naming_what_is_declared() {
        let mirror = mirror("two-way");
        let refusal = selected(&mirror, &["grafana-typo".to_owned()]).expect_err("no such mapping");
        match refusal {
            Error::UnknownSyncMapping { mapping, declared } => {
                assert_eq!(mapping, "grafana-typo");
                assert_eq!(declared, ["grafana"]);
            }
            other => unreachable!("a misspelled mapping became {other:?}"),
        }
    }

    #[test]
    fn every_outcome_has_a_word_and_only_the_ones_needing_an_operator_fail_the_run() {
        let outcomes = [
            (Outcome::Unchanged, "unchanged", false),
            (Outcome::Updated, "updated", false),
            (Outcome::Pulled, "pulled", false),
            (
                Outcome::FieldsUpdated(vec!["notes"]),
                "fields updated",
                false,
            ),
            (
                Outcome::FieldsDiverged(vec!["url"]),
                "fields diverged",
                true,
            ),
            (Outcome::Conflict, "conflict", true),
            (Outcome::Refused(Error::StorePipeMissing), "refused", true),
            (
                Outcome::NotJudged(Error::StorePipeMissing),
                "not judged",
                true,
            ),
        ];
        for (outcome, word, failure) in outcomes {
            assert_eq!(outcome.as_str(), word);
            assert_eq!(outcome.is_failure(), failure, "{word} fails the run");
        }
    }

    /// A value divergence is decided first and a field divergence never
    /// displaces it.
    ///
    /// Held here at the seam where the precedence lives: `decide` fills
    /// `drifted` only in the arms whose two values already agree, so a push
    /// that repairs a value carries an empty one and reports the value's
    /// outcome however far the fields have drifted. The other half of this
    /// claim — that a run really does report `updated` for a mapping whose
    /// value and declared fields both differ — is
    /// `crates/safix/tests/sync_path.rs`'s
    /// `each_mode_converges_exactly_as_its_name_says`, whose pushing mapping
    /// declares a field and differs in its value.
    #[test]
    fn a_field_drift_does_not_displace_a_value_divergence() {
        let declared = || endpoint::ResolvedFields {
            username: Some(endpoint::Field::Literal("alice@example".to_owned())),
            ..endpoint::ResolvedFields::default()
        };
        let value = || Secret::read_from(&mut b"safix-side".as_slice()).expect("a fixture value");

        let moved = Pushing {
            value: value(),
            remember: false,
            resolved: declared(),
            drifted: Vec::new(),
        };
        assert_eq!(moved.outcome().as_str(), "updated");

        let repaired = Pushing {
            value: value(),
            remember: false,
            resolved: declared(),
            drifted: vec!["username"],
        };
        assert_eq!(repaired.outcome().as_str(), "fields updated");
    }
}
