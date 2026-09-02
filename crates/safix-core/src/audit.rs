//! The bridge report: what a mapping declares, against what its two sides hold.
//!
//! One question per declared mapping, asked of both sides and answered by
//! comparing them. Nothing here writes — not into safix, not into clan, not
//! into the password database, and not into this repository. [`crate::bridge`]
//! and [`crate::sync`] are what act on the same declarations, and the reads
//! below are theirs rather than a second pair that could answer differently
//! from a run whose divergence they are reporting.
//!
//! # Two targets, one report
//!
//! [`run`] takes an optional [`Target`]: bare, it compares every declared
//! mapping on both `clan` and `keepassxc`; naming one narrows to that target's
//! own mappings. The two targets' comparisons are different enough — the clan
//! target's is `Values`-or-`OneSided` over a direction, the keepassxc target's
//! is `agreeing`-or-`diverged` over a mode — that [`Report`] carries one section
//! per target rather than folding both into one list, and each section's own
//! `lingering` and exit-status contribution is its own: the clan section's
//! `lingering` names clan vars no currently declared mapping's clan side
//! accounts for, over the machines its own selected mappings name or resolve
//! (`openspec/changes/enumerate-clan-namespace/design.md`'s D2-D5); naming
//! `keepassxc` instead surfaces `keepassxc-sync`'s own lingering report; and
//! neither ever moves either section's exit status.
//!
//! # Why this is a verb of its own and not part of `check`
//!
//! [`crate::check`] answers every question from the structure of the ciphertext:
//! it decrypts nothing, which is what lets one machine judge files belonging to
//! people whose keys it does not have, and it needs no clan and no password
//! database. Comparing a mapping's two sides needs the powers it refuses — a
//! decryption of the safix side, and either clan's own command or the
//! database's password. Carrying them here is what keeps both of `check`'s
//! properties unconditionally true; `openspec/changes/clan-bridge/design.md`
//! records the decision and the two shapes it was taken over.
//!
//! # A mapping that cannot be judged is reported, not skipped
//!
//! A mapping whose safix side this operator cannot decrypt, or whose clan side
//! clan refuses, is a finding of its own. Dropping it would make the report a
//! function of who ran it, and would leave a clean report meaning "the mappings
//! I could look at agree" while reading as "the mappings agree".
//!
//! # No value appears anywhere
//!
//! A finding names the mapping, its two endpoints, and which side holds what.
//! It carries no value and no digest of one: the comparison is
//! [`Secret::equals`](crate::Secret::equals) over two values that are zeroed
//! when it returns, and what leaves it is whether they agreed.

use std::collections::{BTreeSet, HashSet};

use crate::bridge::{self, Addressing, Target};
use crate::clan::{Clan, Reading};
use crate::enroll;
use crate::enroll::custody::DatabasePassword;
use crate::error::{Error, Result};
use crate::model::{ClanPlacement, Direction, Mapping, Mode, SyncMapping};
use crate::store::Database;
use crate::sync;
use crate::workspace::Workspace;

/// The side of a mapping that holds a value the other side does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// clan holds it.
    Clan,
    /// safix holds it.
    Safix,
}

/// Why one mapping's two sides do not agree.
///
/// No `Debug` on the enum for the reason [`bridge::Outcome`] has none: nothing
/// here holds a value, and keeping it that way is easier than proving each
/// future variant does not.
pub enum Disagreement {
    /// Both sides hold a value and the two values differ.
    Values,

    /// One side holds a value and the other holds none.
    OneSided(Side),

    /// safix's side of the mapping did not decrypt for whoever is running.
    ///
    /// Carried as data rather than as the refusal that produced it.
    /// `bridge::held_for` raises [`Error::SourceUnreadable`], whose
    /// sentence is the export path's: it says the mapping exports the entry and
    /// that the mapping was refused rather than transferred. Neither is true
    /// here — the safix side of a `clan-to-safix` mapping is the destination,
    /// and a report transfers nothing — so what the report says about this is
    /// the report's own, and sops has said why on its own standard error either
    /// way.
    SafixSideUnreadable,

    /// The two sides could not be compared for a reason of clan's, and these
    /// are clan's words for it.
    Unjudgeable(Error),
}

/// One clan mapping whose two sides do not agree.
pub struct Finding {
    /// The mapping's declared name.
    pub mapping: String,
    /// Which way it moves, which is what makes one endpoint the source.
    pub direction: Direction,
    /// The clan endpoint, as `<machine> <generator>/<file>`.
    pub clan: String,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// What was found.
    pub disagreement: Disagreement,
}

/// What one run compared on the clan target, and what it found.
pub struct ClanReport {
    /// How many mappings were compared.
    ///
    /// Carried because a clean report over no declared mapping and a clean
    /// report over twelve are different statements, and a report that rendered
    /// them alike would say "nothing disagrees" about a bridge it never had.
    pub examined: usize,
    /// One entry per mapping whose two sides do not agree, in declaration
    /// order.
    pub findings: Vec<Finding>,
    /// Clan vars no currently declared mapping's clan side accounts for, as
    /// `"<machine> <generator>/<file>"`, scoped to the machines the selected
    /// mappings name or resolve and sorted by machine then by id.
    ///
    /// Information rather than a finding: reported alongside the compared
    /// mappings and excluded from [`ClanReport::is_clean`], the same way
    /// [`KeepassxcReport::lingering`] is excluded from its own.
    pub lingering: Vec<String>,
}

impl ClanReport {
    /// Whether every mapping compared agreed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Why one keepassxc mapping's compared outcome is what it is.
///
/// No `Debug`, for the reason [`Disagreement`] has none.
pub enum KeepassxcOutcome {
    /// Both sides hold the same value, or neither holds one — the state
    /// `sync`'s own mode would leave nothing to converge.
    Agreeing,
    /// The two sides hold different content: different values, or exactly one
    /// side holding a value the other does not.
    Diverged,
    /// The two sides could not be compared, and this is why.
    Unjudgeable(Error),
}

impl KeepassxcOutcome {
    /// The word a report prints for this outcome.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Agreeing => "agreeing",
            Self::Diverged => "diverged",
            Self::Unjudgeable(_) => "unjudgeable",
        }
    }
}

/// One keepassxc mapping's compared outcome.
pub struct KeepassxcFinding {
    /// The mapping's declared name.
    pub mapping: String,
    /// How it converges, when it is converged.
    pub mode: Mode,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// The database endpoint, as the entry path under the declared group.
    pub kdbx: String,
    /// What was found.
    pub outcome: KeepassxcOutcome,
}

/// What one run compared on the keepassxc target, and what it found.
pub struct KeepassxcReport {
    /// The database the run compared against.
    pub database: String,
    /// One entry per compared mapping, in declaration order, whatever its
    /// outcome.
    pub compared: Vec<KeepassxcFinding>,
    /// Entries under the declared group that no declared mapping accounts for,
    /// in the same shape [`sync::Report::lingering`] already gives it.
    ///
    /// Information rather than a finding: reported alongside the compared
    /// mappings and excluded from [`KeepassxcReport::is_clean`], the same way
    /// `sync`'s own report excludes it from the exit-status tally.
    pub lingering: Vec<String>,
}

impl KeepassxcReport {
    /// Whether every compared mapping agreed.
    ///
    /// `lingering` never enters this: a lingering entry is information about
    /// the database, not a claim about any declared mapping's two sides.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.compared
            .iter()
            .all(|entry| matches!(entry.outcome, KeepassxcOutcome::Agreeing))
    }
}

/// What one run compared, on whichever target it was scoped to.
pub struct Report {
    /// Present when the run compared the clan target: bare, or `clan` named.
    pub clan: Option<ClanReport>,
    /// Present when the run compared the keepassxc target: bare, or
    /// `keepassxc` named.
    pub keepassxc: Option<KeepassxcReport>,
}

impl Report {
    /// Whether every mapping compared, on every target the run scoped to,
    /// agreed.
    ///
    /// Findings alone decide this. A `lingering` entry on either target's
    /// section never does, for the reason each section's own `is_clean` gives.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.clan.as_ref().is_none_or(ClanReport::is_clean)
            && self
                .keepassxc
                .as_ref()
                .is_none_or(KeepassxcReport::is_clean)
    }
}

/// Every declared mapping's two sides compared, on the named target or both.
///
/// `direction` narrows the clan target's comparison the way it narrows
/// `sync`'s write; it is ignored when `target` is `Keepassxc` and refused
/// earlier, at dispatch, when it is given for that target.
///
/// # Errors
///
/// [`Error::NoClanFlake`] when the clan target is compared and no clan is
/// declared, [`Error::ClanUnavailable`] when clan's command cannot be run,
/// [`Error::NoStoreDatabase`] when the keepassxc target is compared and
/// mappings are declared with no database, [`Error::StoreLocked`] and
/// [`Error::DatabaseUnreadable`] for the keepassxc target's own database
/// opening, [`Error::UnknownMapping`] and [`Error::UnknownSyncMapping`] when a
/// named mapping is not declared on the target it was named under, and
/// whatever evaluating the declarations failed with. Each of those stops the
/// whole run and is raised before the first mapping of its target is compared,
/// for the reason [`bridge`] raises its own there: a run that discovered them
/// partway through would already have said "agrees" about mappings it never
/// looked at. A mapping that cannot be judged is a finding rather than a
/// refusal.
pub fn run(
    workspace: &Workspace,
    password: &mut dyn DatabasePassword,
    target: Option<Target>,
    direction: Option<Direction>,
    only: &[String],
) -> Result<Report> {
    let clan = if matches!(target, None | Some(Target::Clan)) {
        Some(run_clan(workspace, direction, only)?)
    } else {
        None
    };
    let keepassxc = if matches!(target, None | Some(Target::Keepassxc)) {
        Some(run_keepassxc(workspace, password, only)?)
    } else {
        None
    };
    Ok(Report { clan, keepassxc })
}

/// The clan target's own comparison.
fn run_clan(
    workspace: &Workspace,
    direction: Option<Direction>,
    only: &[String],
) -> Result<ClanReport> {
    let mappings = bridge::selected(workspace, direction, only)?;
    if mappings.is_empty() {
        return Ok(ClanReport {
            examined: 0,
            findings: Vec::new(),
            lingering: Vec::new(),
        });
    }

    let flake = workspace
        .bridge()?
        .clan_flake
        .clone()
        .ok_or(Error::NoClanFlake)?;
    let clan = Clan::new(flake);
    clan.probe()?;
    let addressing = Addressing::new(&clan);

    // Enumerated before any mapping is compared: a listing failure raised
    // partway through the findings loop would already have said "agrees"
    // about mappings it never looked at, the same reasoning `run`'s own doc
    // comment gives for its own stopping conditions.
    let lingering = lingering(&clan, &addressing, &mappings)?;

    let mut findings = Vec::new();
    for mapping in &mappings {
        let Some(disagreement) = compare(workspace, &addressing, mapping) else {
            continue;
        };
        let (clan_side, safix_side) = bridge::endpoints(mapping);
        findings.push(Finding {
            mapping: mapping.id.clone(),
            direction: mapping.direction,
            clan: clan_side,
            safix: safix_side,
            disagreement,
        });
    }

    Ok(ClanReport {
        examined: mappings.len(),
        findings,
        lingering,
    })
}

/// Clan vars no currently declared mapping's clan side accounts for, over the
/// machines the selected mappings name or resolve.
///
/// Placement-sensitive rather than a single-field match
/// (`openspec/changes/enumerate-clan-namespace/design.md`'s D2/D3): a
/// per-machine mapping's var is claimed on its declared machine alone, and a
/// shared mapping's is claimed by id, machine-insensitively, because the same
/// shared generator's var can legitimately appear in more than one machine's
/// own listing. `mapping.direction` never enters either comparison.
///
/// # Errors
///
/// Whatever [`Addressing::machine_for`] raises resolving a shared mapping's
/// machine, and [`Error::ClanMachineListFailed`] when a machine's vars cannot
/// be listed.
fn lingering(
    clan: &Clan,
    addressing: &Addressing<'_>,
    mappings: &[&Mapping],
) -> Result<Vec<String>> {
    let mut claimed_on_machine: HashSet<(String, String)> = HashSet::new();
    let mut claimed_anywhere: HashSet<String> = HashSet::new();
    let mut machines: BTreeSet<String> = BTreeSet::new();

    for mapping in mappings {
        let machine = addressing.machine_for(mapping)?;
        let id = Clan::var_id(&mapping.clan.generator, &mapping.clan.file);
        match mapping.clan.placement {
            ClanPlacement::PerMachine => {
                claimed_on_machine.insert((machine.clone(), id));
            }
            ClanPlacement::Shared => {
                claimed_anywhere.insert(id);
            }
        }
        machines.insert(machine);
    }

    let mut found = Vec::new();
    for machine in &machines {
        for id in clan.list(machine)? {
            let claimed = claimed_anywhere.contains(&id)
                || claimed_on_machine.contains(&(machine.clone(), id.clone()));
            if !claimed {
                found.push(format!("{machine} {id}"));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The keepassxc target's own comparison.
///
/// The database is opened even for a scoped run naming one mapping, the same
/// way `sync::run` opens it: a listing taken at open is what answers whether
/// an entry exists at all, and there is no cheaper way to ask.
fn run_keepassxc(
    workspace: &Workspace,
    password: &mut dyn DatabasePassword,
    only: &[String],
) -> Result<KeepassxcReport> {
    let mirror = workspace.keepassxc()?;
    let mappings = sync::selected(mirror, only)?;
    if mappings.is_empty() {
        return Ok(KeepassxcReport {
            database: mirror.database.clone().unwrap_or_default(),
            compared: Vec::new(),
            lingering: Vec::new(),
        });
    }

    let Some(named) = mirror.database.clone() else {
        return Err(Error::NoStoreDatabase {
            mappings: mirror.mappings.len(),
        });
    };
    if !enroll::terminal_present() {
        return Err(Error::StoreLocked { database: named });
    }
    let database = Database::open(std::path::PathBuf::from(&named), mirror, password)?;

    let mut compared = Vec::with_capacity(mappings.len());
    for mapping in &mappings {
        let entry = mirror.entry_of(mapping);
        let outcome = compare_keepassxc(workspace, &database, mapping, &entry);
        compared.push(KeepassxcFinding {
            mapping: mapping.id.clone(),
            mode: mapping.mode,
            safix: format!("{}.{}", mapping.safix.user, mapping.safix.name),
            kdbx: entry,
            outcome,
        });
    }

    Ok(KeepassxcReport {
        database: database.path().display().to_string(),
        lingering: sync::lingering(&database, mirror),
        compared,
    })
}

/// One keepassxc mapping's two sides, read and compared.
///
/// Judged on agreement alone rather than per mode: `sync`'s own mode decides
/// which side a divergence would resolve toward, and this reports that a
/// divergence exists without pre-empting that decision.
fn compare_keepassxc(
    workspace: &Workspace,
    database: &Database,
    mapping: &SyncMapping,
    entry: &str,
) -> KeepassxcOutcome {
    let theirs = match database.read(entry) {
        Ok(held) => held,
        Err(reason) => return KeepassxcOutcome::Unjudgeable(reason),
    };
    let ours = match bridge::held_by_safix(
        workspace,
        &mapping.id,
        &mapping.safix.user,
        &mapping.safix.name,
    ) {
        Ok(held) => held,
        Err(reason) => return KeepassxcOutcome::Unjudgeable(reason),
    };

    match (ours, theirs) {
        (None, None) => KeepassxcOutcome::Agreeing,
        (Some(ours), Some(theirs)) if ours.equals(&theirs) => KeepassxcOutcome::Agreeing,
        _ => KeepassxcOutcome::Diverged,
    }
}

/// One clan mapping's two sides, read and compared.
///
/// The clan side is read first because it is the read that can fail for a
/// reason the declaration is responsible for — a misspelled triple — and
/// answering that without decrypting anything is worth the ordering.
///
/// Two absences are agreement rather than a finding: a mapping neither side has
/// a value for is one nothing has minted yet, and reporting it would report
/// every mapping of a bridge that has not been bootstrapped, under a remedy
/// this report cannot name.
///
/// A two-way mapping's one-sided state is bootstrap rather than a finding as
/// well, matching `bridge_sync`'s own convergence semantics exactly: exactly
/// one side ever having held a value is the ordinary first run of a two-way
/// mapping, not a divergence a person has to resolve. A one-way mapping's
/// one-sided state stays a finding, because its verb moves in one direction
/// only and a destination nothing has written to is a state `sync` resolves,
/// not a bootstrap it enacts unasked.
fn compare(
    workspace: &Workspace,
    addressing: &Addressing<'_>,
    mapping: &Mapping,
) -> Option<Disagreement> {
    let theirs = match addressing.read(mapping) {
        Ok(Reading::Present(value)) => Some(value),
        Ok(Reading::AbsentAtSource) => None,
        Err(reason) => return Some(Disagreement::Unjudgeable(reason)),
    };

    let ours = match bridge::held_for(workspace, mapping) {
        Ok(held) => held,
        Err(Error::SourceUnreadable { .. }) => return Some(Disagreement::SafixSideUnreadable),
        Err(reason) => return Some(Disagreement::Unjudgeable(reason)),
    };

    match (theirs, ours) {
        (Some(theirs), Some(ours)) if ours.equals(&theirs) => None,
        (Some(_), Some(_)) => Some(Disagreement::Values),
        (Some(_), None) | (None, Some(_)) if mapping.direction == Direction::TwoWay => None,
        (Some(_), None) => Some(Disagreement::OneSided(Side::Clan)),
        (None, Some(_)) => Some(Disagreement::OneSided(Side::Safix)),
        (None, None) => None,
    }
}

impl Side {
    /// Whether this side is the one the mapping's direction takes the value
    /// from.
    #[must_use]
    pub const fn is_source_of(self, direction: Direction) -> bool {
        matches!(
            (self, direction),
            (Self::Clan, Direction::ClanToSafix) | (Self::Safix, Direction::SafixToClan)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_of_a_mapping_is_the_side_its_direction_names() {
        assert!(Side::Clan.is_source_of(Direction::ClanToSafix));
        assert!(!Side::Clan.is_source_of(Direction::SafixToClan));
        assert!(Side::Safix.is_source_of(Direction::SafixToClan));
        assert!(!Side::Safix.is_source_of(Direction::ClanToSafix));
    }

    #[test]
    fn a_report_is_clean_only_when_every_present_section_is() {
        let clean_clan = ClanReport {
            examined: 1,
            findings: Vec::new(),
            lingering: vec!["meridian ntfy/orphan".into()],
        };
        let dirty_clan = ClanReport {
            examined: 1,
            findings: vec![Finding {
                mapping: "ntfy-token".into(),
                direction: Direction::ClanToSafix,
                clan: "meridian ntfy/token".into(),
                safix: "alice.ntfy-token".into(),
                disagreement: Disagreement::Values,
            }],
            lingering: Vec::new(),
        };
        let clean_keepassxc = KeepassxcReport {
            database: "/nonexistent/master.kdbx".into(),
            compared: vec![KeepassxcFinding {
                mapping: "grafana".into(),
                mode: Mode::SafixToKeepassxc,
                safix: "alice.grafana-password".into(),
                kdbx: "safix/alice/grafana".into(),
                outcome: KeepassxcOutcome::Agreeing,
            }],
            lingering: vec!["safix/alice/orphan".into()],
        };
        let dirty_keepassxc = KeepassxcReport {
            database: "/nonexistent/master.kdbx".into(),
            compared: vec![KeepassxcFinding {
                mapping: "grafana".into(),
                mode: Mode::SafixToKeepassxc,
                safix: "alice.grafana-password".into(),
                kdbx: "safix/alice/grafana".into(),
                outcome: KeepassxcOutcome::Diverged,
            }],
            lingering: Vec::new(),
        };

        assert!(
            Report {
                clan: Some(clean_clan),
                keepassxc: Some(clean_keepassxc),
            }
            .is_clean(),
            "lingering on either section should not flip the exit status"
        );
        assert!(
            !Report {
                clan: Some(dirty_clan),
                keepassxc: None,
            }
            .is_clean()
        );
        assert!(
            !Report {
                clan: None,
                keepassxc: Some(dirty_keepassxc),
            }
            .is_clean()
        );
    }
}
