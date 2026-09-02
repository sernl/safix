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
            Self::Conflict => "conflict",
            Self::Refused(_) => "refused",
            Self::NotJudged(_) => "not judged",
        }
    }

    /// Whether this outcome makes the run a failure.
    ///
    /// A conflict does, because it is a state the operator has to resolve; a
    /// mapping that could not be judged does, because a clean exit would say
    /// something the run does not know.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Conflict | Self::Refused(_) | Self::NotJudged(_))
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
    /// Write this value into the database, and record the agreement when the
    /// mode remembers one.
    Push { value: Secret, remember: bool },
    /// Write this value into safix, and record the agreement when the mode
    /// remembers one.
    Pull { value: Secret, remember: bool },
}

/// Converge every declared mapping, or the one named.
///
/// # Errors
///
/// [`Error::NoStoreDatabase`] when mappings are declared and no database is,
/// [`Error::UnknownSyncMapping`] when the named mapping is not one that is
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
    only: Option<&str>,
) -> Result<Report> {
    scratch::set_floor(workspace.root());
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
            Decision::Push { value, remember } => {
                let outcome = push(progress, &mut database, mirror, mapping, &value, remember);
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
fn selected<'a>(mirror: &'a Keepassxc, only: Option<&str>) -> Result<Vec<&'a SyncMapping>> {
    let Some(id) = only else {
        return Ok(mirror.mappings.iter().collect());
    };
    let mapping = mirror.named(id).ok_or_else(|| Error::UnknownSyncMapping {
        mapping: id.to_owned(),
        declared: mirror.declared(),
    })?;
    Ok(vec![mapping])
}

/// Entries under the declared group that no declared mapping accounts for.
///
/// Every mapping accounts for its own entry and for the companion beside it, so a
/// companion whose mapping is gone lingers exactly as its entry does — which is
/// the point of computing this from the listing rather than from the mappings.
fn lingering(database: &Database, mirror: &Keepassxc) -> Vec<String> {
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
fn decide(
    workspace: &Workspace,
    database: &Database,
    mirror: &Keepassxc,
    mapping: &SyncMapping,
) -> Decision {
    let entry = mirror.entry_of(mapping);

    let theirs = match database.read(&entry) {
        Ok(held) => held,
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
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

    match mapping.mode {
        Mode::SafixToKeepassxc => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                Decision::Settled(Outcome::Unchanged)
            }
            (Some(ours), _) => Decision::Push {
                value: ours,
                remember: false,
            },
        },

        Mode::KeepassxcToSafix => match (ours, theirs) {
            (_, None) => Decision::Settled(Outcome::Refused(Error::StoreEntryAbsent {
                mapping: mapping.id.clone(),
                entry,
                mode: mapping.mode.as_str(),
            })),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                Decision::Settled(Outcome::Unchanged)
            }
            (_, Some(theirs)) => Decision::Pull {
                value: theirs,
                remember: false,
            },
        },

        Mode::Backup => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            (Some(ours), None) => Decision::Push {
                value: ours,
                remember: false,
            },
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                Decision::Settled(Outcome::Unchanged)
            }
            // The whole of what `backup` is: an existing differing value is
            // reported and never overwritten.
            (Some(_), Some(_)) => Decision::Settled(Outcome::Conflict),
        },

        Mode::TwoWay => two_way(database, &entry, ours, theirs),
    }
}

/// The three-way decision, against the agreement the companion entry remembers.
///
/// A memory that is absent, unreadable or written under a tag this version does
/// not know takes bootstrap semantics: write where one side is empty, report
/// everything else. Never a guess — the one thing that cannot happen here is
/// picking a winner from a clock.
fn two_way(
    database: &Database,
    entry: &str,
    ours: Option<Secret>,
    theirs: Option<Secret>,
) -> Decision {
    let remembered = recorded(database, entry);

    match (ours, theirs) {
        (None, None) => Decision::Settled(Outcome::Unchanged),
        (Some(ours), None) => Decision::Push {
            value: ours,
            remember: true,
        },
        (None, Some(theirs)) => Decision::Pull {
            value: theirs,
            remember: true,
        },
        (Some(ours), Some(theirs)) => {
            if ours.equals(&theirs) {
                return Decision::Settled(Outcome::Unchanged);
            }
            let Some(remembered) = remembered else {
                return Decision::Settled(Outcome::Conflict);
            };
            match (agrees(&remembered, &ours), agrees(&remembered, &theirs)) {
                // safix is where the agreement left it, so the database is the
                // side that moved.
                (true, false) => Decision::Pull {
                    value: theirs,
                    remember: true,
                },
                (false, true) => Decision::Push {
                    value: ours,
                    remember: true,
                },
                // Both moved, or neither matches an agreement this run cannot
                // account for. Either way nothing is written.
                _ => Decision::Settled(Outcome::Conflict),
            }
        }
    }
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

/// Write the database's side, and the agreement when the mode remembers one.
fn push(
    progress: &dyn Progress,
    database: &mut Database,
    mirror: &Keepassxc,
    mapping: &SyncMapping,
    value: &Secret,
    remember: bool,
) -> Outcome {
    let entry = mirror.entry_of(mapping);
    log(
        progress,
        &format!(
            "safix: flake.safix.users.{}.{} -> {entry}",
            mapping.safix.user, mapping.safix.name,
        ),
    );
    if let Err(reason) = database.write(&entry, value, mapping.kdbx.username.as_deref()) {
        return Outcome::Refused(reason);
    }
    if let Err(reason) = remembered_after(database, &entry, value, remember) {
        return Outcome::Refused(reason);
    }
    Outcome::Updated
}

/// Write safix's side through the ordinary write path, and the agreement when
/// the mode remembers one.
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
        if let Err(reason) = database.write(&store::companion_of(&entry), &recorded, None) {
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
    database.write(&store::companion_of(entry), &recorded, None)
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
                  "kdbx": {{ "path": "alice/grafana", "username": null }}
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
        let refusal = selected(&mirror, Some("grafana-typo")).expect_err("no such mapping");
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
}
