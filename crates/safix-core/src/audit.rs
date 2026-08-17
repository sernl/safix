//! The bridge report: what a mapping declares, against what its two sides hold.
//!
//! One question per declared mapping, asked of both sides and answered by
//! comparing them. Nothing here writes — not into safix, not into clan, and not
//! into this repository. [`crate::bridge`] is what acts on the same
//! declarations, and the two reads and the comparison below are that module's
//! own rather than a second set that could answer differently from the transfer
//! whose divergence they are reporting.
//!
//! # Why this is a verb of its own and not part of `check`
//!
//! [`crate::check`] answers every question from the structure of the ciphertext:
//! it decrypts nothing, which is what lets one machine judge files belonging to
//! people whose keys it does not have, and it needs no clan. Comparing a
//! mapping's two sides needs both of the powers it refuses — the safix side is
//! decrypted, and the clan side is read by running clan's own command, once per
//! mapping. Carrying them here is what keeps both of those properties of
//! `check` unconditionally true; `openspec/changes/clan-bridge/design.md`
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

use crate::bridge;
use crate::clan::{Clan, Reading};
use crate::error::{Error, Result};
use crate::model::{Direction, Mapping};
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
    /// `bridge::held_by_safix` raises [`Error::SourceUnreadable`], whose
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

/// One mapping whose two sides do not agree.
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

/// What one run compared, and what it found.
pub struct Report {
    /// How many mappings were compared.
    ///
    /// Carried because a clean report over no declared mapping and a clean
    /// report over twelve are different statements, and a report that rendered
    /// them alike would say "nothing disagrees" about a bridge it never had.
    pub examined: usize,
    /// One entry per mapping whose two sides do not agree, in declaration
    /// order.
    pub findings: Vec<Finding>,
}

impl Report {
    /// Whether every mapping compared agreed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Every declared mapping's two sides compared, or the one named.
///
/// # Errors
///
/// [`Error::NoClanFlake`] when no clan is declared, [`Error::ClanUnavailable`]
/// when clan's command cannot be run, [`Error::UnknownMapping`] when the named
/// mapping is not one that is declared, and whatever evaluating the
/// declarations failed with. Each of those stops the whole run and is raised
/// before the first mapping is compared, for the reason [`bridge`] raises its
/// own there: a run that discovered them partway through would already have
/// said "agrees" about mappings it never looked at. A mapping that cannot be
/// judged is a finding rather than a refusal.
pub fn run(workspace: &Workspace, only: Option<&str>) -> Result<Report> {
    let mappings = selected(workspace, only)?;
    if mappings.is_empty() {
        return Ok(Report {
            examined: 0,
            findings: Vec::new(),
        });
    }

    let flake = workspace
        .bridge()?
        .clan_flake
        .clone()
        .ok_or(Error::NoClanFlake)?;
    let clan = Clan::new(flake);
    clan.probe()?;

    let mut findings = Vec::new();
    for mapping in &mappings {
        let Some(disagreement) = compare(workspace, &clan, mapping) else {
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

    Ok(Report {
        examined: mappings.len(),
        findings,
    })
}

/// The mappings one run compares, refusing before any of them is touched.
///
/// Both directions, and a named mapping is found in either. The transfer verbs
/// refuse a mapping declared for the other direction because the operator asked
/// the wrong verb of it; there is no wrong verb here, because comparing a
/// mapping is the same act whichever way its value moves.
fn selected<'a>(workspace: &'a Workspace, only: Option<&str>) -> Result<Vec<&'a Mapping>> {
    let bridge = workspace.bridge()?;
    let Some(id) = only else {
        return Ok(bridge.mappings.iter().collect());
    };

    let mapping = bridge.named(id).ok_or_else(|| Error::UnknownMapping {
        mapping: id.to_owned(),
        declared: bridge.declared(),
    })?;
    Ok(vec![mapping])
}

/// One mapping's two sides, read and compared.
///
/// The clan side is read first because it is the read that can fail for a
/// reason the declaration is responsible for — a misspelled triple — and
/// answering that without decrypting anything is worth the ordering.
///
/// Two absences are agreement rather than a finding: a mapping neither side has
/// a value for is one nothing has minted yet, and reporting it would report
/// every mapping of a bridge that has not been bootstrapped, under a remedy
/// this report cannot name.
fn compare(workspace: &Workspace, clan: &Clan, mapping: &Mapping) -> Option<Disagreement> {
    let theirs = match clan.read(
        &mapping.id,
        &mapping.clan.machine,
        &mapping.clan.generator,
        &mapping.clan.file,
    ) {
        Ok(Reading::Present(value)) => Some(value),
        Ok(Reading::AbsentAtSource) => None,
        Err(reason) => return Some(Disagreement::Unjudgeable(reason)),
    };

    let ours = match bridge::held_by_safix(workspace, mapping) {
        Ok(held) => held,
        Err(Error::SourceUnreadable { .. }) => return Some(Disagreement::SafixSideUnreadable),
        Err(reason) => return Some(Disagreement::Unjudgeable(reason)),
    };

    match (theirs, ours) {
        (Some(theirs), Some(ours)) if ours.equals(&theirs) => None,
        (Some(_), Some(_)) => Some(Disagreement::Values),
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
}
