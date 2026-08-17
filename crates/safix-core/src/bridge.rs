//! Moving a declared value across the boundary, in either direction.
//!
//! The mappings are declarations — see `modules/flake/safix/bridge.nix` for why
//! a standing relationship is written down rather than passed as arguments —
//! and this module is what acts on them. [`crate::clan`] is how the far side is
//! reached; nothing here touches a file clan placed.
//!
//! # Both verbs read both sides before writing either
//!
//! For `clan-to-safix` the comparison saves a commit. For `safix-to-clan` it is
//! load-bearing rather than an optimisation, and the reason is a property of
//! clan rather than a preference of ours: `clan vars set` writes
//! unconditionally and commits what it wrote, and a re-encrypting backend — this
//! fleet's is `age` — produces fresh ciphertext for an unchanged value. Without
//! the read-first comparison every `safix export` run would produce a commit in
//! the clan repository for every mapping, forever, each one a diff of ciphertext
//! that decrypts to what it decrypted to before.
//!
//! # The four outcomes
//!
//! Every mapping a run acts on ends as exactly one of [`Outcome`]'s four.
//! `AbsentAtSource` is an outcome rather than a failure because a clan var that
//! has not been generated yet is the normal state during bootstrap, and a bridge
//! run then should say so and carry on. A refusal is also per mapping: one
//! mapping whose clan side is misspelled must not stop the ten that are right.
//!
//! What stops a whole run is exactly the conditions under which no mapping can
//! be judged: no clan declared, clan not installed, or a mapping named that is
//! not declared. Those are raised before the first mapping is touched, because
//! a run that discovered them partway through would already have reported
//! "unchanged" for mappings it never looked at.
//!
//! # No value appears anywhere
//!
//! A report line names the mapping, its endpoints and its outcome. A commit
//! message names the mapping and the direction. Neither names a value, and the
//! comparison that decides between `Unchanged` and `Updated` is
//! [`Secret::equals`] over two values that are zeroed when this returns.
//!
//! # The same reads answer the audit
//!
//! [`crate::audit`] reports the mappings whose two sides no longer agree, and
//! reaches both sides through this module's own reads rather than through a
//! second pair of its own. A report about a transfer that judged agreement
//! differently from the transfer would be a report about nothing.

use crate::clan::{Clan, Reading};
use crate::error::{Error, Result};
use crate::model::{Direction, Mapping};
use crate::progress::{Progress, log};
use crate::scratch;
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::sops::document;
use crate::workspace::Workspace;

/// What happened to one mapping.
///
/// No `Debug` on the whole enum for the reason [`Reading`] has none: nothing
/// here holds a value, and keeping it that way is easier than proving each
/// future variant does not.
pub enum Outcome {
    /// Both sides already held the same bytes. Nothing written, nothing
    /// committed.
    Unchanged,
    /// The destination now holds what the source holds.
    Updated,
    /// The source holds nothing yet. Not a failure: a clan var that has not
    /// been generated is the normal state during bootstrap.
    AbsentAtSource,
    /// This mapping was refused, and this is why.
    Refused(Error),
}

impl Outcome {
    /// The word a report prints for this outcome.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Updated => "updated",
            Self::AbsentAtSource => "absent at source",
            Self::Refused(_) => "refused",
        }
    }

    /// Whether this outcome makes the run a failure.
    #[must_use]
    pub const fn is_refusal(&self) -> bool {
        matches!(self, Self::Refused(_))
    }
}

/// One mapping's line in a run's report.
pub struct Transferred {
    /// The mapping's declared name.
    pub mapping: String,
    /// Which way it moves.
    pub direction: Direction,
    /// The clan endpoint, as `<machine> <generator>/<file>`.
    pub clan: String,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// What happened.
    pub outcome: Outcome,
}

/// Everything a run did, in the order it did it.
pub struct Run {
    /// One entry per mapping acted on.
    pub transferred: Vec<Transferred>,
}

impl Run {
    /// Whether any mapping was refused.
    #[must_use]
    pub fn refused(&self) -> bool {
        self.transferred
            .iter()
            .any(|entry| entry.outcome.is_refusal())
    }

    /// How many mappings ended in each of the three non-refusal outcomes, and
    /// how many were refused.
    #[must_use]
    pub fn tally(&self) -> Tally {
        let count = |wanted: &str| {
            self.transferred
                .iter()
                .filter(|entry| entry.outcome.as_str() == wanted)
                .count()
        };
        Tally {
            unchanged: count(Outcome::Unchanged.as_str()),
            updated: count(Outcome::Updated.as_str()),
            absent: count(Outcome::AbsentAtSource.as_str()),
            refused: count(Outcome::Refused(Error::ClanPipeMissing).as_str()),
        }
    }
}

/// The counts a run's closing line reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Mappings whose two sides already agreed.
    pub unchanged: usize,
    /// Mappings whose destination was written.
    pub updated: usize,
    /// Mappings whose source held nothing yet.
    pub absent: usize,
    /// Mappings refused.
    pub refused: usize,
}

/// A value already in hand, for the write path that expects to read one.
///
/// `set` takes a [`ValueSource`] because how a value arrives differs while what
/// happens to it does not: typed twice at a terminal and compared, piped once by a
/// script, edited in a buffer. A bridged value was read once, from clan, and there
/// is nothing to compare it against — so this hands it over and the rest of the
/// write path is unchanged.
struct Held(Option<Secret>);

impl ValueSource for Held {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        self.0.take().ok_or(Error::NoValueRead)
    }
}

/// Move every `clan-to-safix` mapping, or the one named.
///
/// # Errors
///
/// [`Error::NoClanFlake`] when no clan is declared, [`Error::ClanUnavailable`]
/// when clan's command cannot be run, [`Error::UnknownMapping`] and
/// [`Error::MappingWrongDirection`] when the named mapping is not one of this
/// direction's, and whatever evaluating the declarations failed with.
pub fn import(workspace: &Workspace, progress: &dyn Progress, only: Option<&str>) -> Result<Run> {
    run(workspace, progress, Direction::ClanToSafix, only)
}

/// Move every `safix-to-clan` mapping, or the one named.
///
/// # Errors
///
/// The same run-level refusals [`import`] raises.
pub fn export(workspace: &Workspace, progress: &dyn Progress, only: Option<&str>) -> Result<Run> {
    run(workspace, progress, Direction::SafixToClan, only)
}

/// The mappings one invocation acts on, refusing before any of them is touched.
///
/// A name that is declared in the other direction is refused as such rather than
/// as an unknown name: the operator has spelled the mapping correctly and asked
/// the wrong verb, and a message saying "not a declared mapping" about a mapping
/// sitting three lines above in their own file would be actively misleading.
fn selected<'a>(
    workspace: &'a Workspace,
    direction: Direction,
    only: Option<&str>,
) -> Result<Vec<&'a Mapping>> {
    let bridge = workspace.bridge()?;
    let Some(id) = only else {
        return Ok(bridge.of(direction).collect());
    };

    let mapping = bridge.named(id).ok_or_else(|| Error::UnknownMapping {
        mapping: id.to_owned(),
        declared: bridge.declared(),
    })?;

    if mapping.direction != direction {
        return Err(Error::MappingWrongDirection {
            mapping: id.to_owned(),
            direction: mapping.direction.as_str(),
            verb: mapping.direction.verb(),
            asked: direction.verb(),
        });
    }
    Ok(vec![mapping])
}

fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    direction: Direction,
    only: Option<&str>,
) -> Result<Run> {
    scratch::set_floor(workspace.root());
    let _guard = scratch::Guard;

    let mappings = selected(workspace, direction, only)?;
    if mappings.is_empty() {
        return Ok(Run {
            transferred: Vec::new(),
        });
    }

    let flake = workspace
        .bridge()?
        .clan_flake
        .clone()
        .ok_or(Error::NoClanFlake)?;
    let clan = Clan::new(flake);
    clan.probe()?;

    let mut transferred = Vec::with_capacity(mappings.len());
    for mapping in mappings {
        let outcome = match direction {
            Direction::ClanToSafix => one_import(workspace, progress, &clan, mapping),
            Direction::SafixToClan => one_export(workspace, progress, &clan, mapping),
        };
        let (clan_side, safix_side) = endpoints(mapping);
        transferred.push(Transferred {
            mapping: mapping.id.clone(),
            direction,
            clan: clan_side,
            safix: safix_side,
            outcome: outcome.unwrap_or_else(Outcome::Refused),
        });
        if scratch::interrupted().is_some() {
            break;
        }
    }

    Ok(Run { transferred })
}

/// clan holds the value; safix receives it through the hand-set write path.
fn one_import(
    workspace: &Workspace,
    progress: &dyn Progress,
    clan: &Clan,
    mapping: &Mapping,
) -> Result<Outcome> {
    let incoming = match clan.read(
        &mapping.id,
        &mapping.clan.machine,
        &mapping.clan.generator,
        &mapping.clan.file,
    )? {
        Reading::Present(value) => value,
        Reading::AbsentAtSource => return Ok(Outcome::AbsentAtSource),
    };

    if let Some(held) = held_by_safix(workspace, mapping)?
        && held.equals(&incoming)
    {
        return Ok(Outcome::Unchanged);
    }

    log(
        progress,
        &format!(
            "safix: {} {} -> flake.safix.users.{}.{}",
            mapping.clan.machine,
            Clan::var_id(&mapping.clan.generator, &mapping.clan.file),
            mapping.safix.user,
            mapping.safix.name,
        ),
    );

    let status = set::run_committing(
        workspace,
        progress,
        &mut Held(Some(incoming)),
        &mapping.safix.user,
        &mapping.safix.name,
        &commit_subject(mapping),
    )?;

    if status == 0 {
        Ok(Outcome::Updated)
    } else {
        // sops refused and has said why on its own standard error, which is
        // inherited. Reporting it as this mapping's refusal is what keeps the
        // rest of the run going and the report honest about which mapping it
        // was.
        Ok(Outcome::Refused(Error::SourceUnreadable {
            mapping: mapping.id.clone(),
            user: mapping.safix.user.clone(),
            name: mapping.safix.name.clone(),
            file: workspace
                .resolve(&mapping.safix.user, &mapping.safix.name)?
                .file
                .clone(),
        }))
    }
}

/// safix holds the value; clan receives it through clan's own command.
fn one_export(
    workspace: &Workspace,
    progress: &dyn Progress,
    clan: &Clan,
    mapping: &Mapping,
) -> Result<Outcome> {
    let placement = workspace.resolve(&mapping.safix.user, &mapping.safix.name)?;
    let file = placement.file.clone();
    let generated = placement.generator.is_some();

    let Some(outgoing) = held_by_safix(workspace, mapping)? else {
        return Ok(Outcome::Refused(Error::SourceHasNoValue {
            mapping: mapping.id.clone(),
            user: mapping.safix.user.clone(),
            name: mapping.safix.name.clone(),
            file,
            generated,
        }));
    };

    // The comparison that makes a run converge. See the module note: without it
    // every run commits in the clan repository for every mapping, because clan's
    // write is unconditional and its backend re-encrypts.
    if let Reading::Present(held) = clan.read(
        &mapping.id,
        &mapping.clan.machine,
        &mapping.clan.generator,
        &mapping.clan.file,
    )? && held.equals(&outgoing)
    {
        return Ok(Outcome::Unchanged);
    }

    // Asked only once a write is actually going to happen. A mapping whose two
    // sides already agree writes nothing, so there is no value for a later
    // generation to discard and nothing for this refusal to prevent; refusing
    // there would refuse a no-op and would make a second run of a stale mapping
    // report differently from the first.
    if clan.generator_stale(&mapping.clan.machine, &mapping.clan.generator)? {
        return Ok(Outcome::Refused(Error::GeneratorDefinitionDrifted {
            mapping: mapping.id.clone(),
            machine: mapping.clan.machine.clone(),
            generator: mapping.clan.generator.clone(),
        }));
    }

    log(
        progress,
        &format!(
            "safix: flake.safix.users.{}.{} -> {} {}",
            mapping.safix.user,
            mapping.safix.name,
            mapping.clan.machine,
            Clan::var_id(&mapping.clan.generator, &mapping.clan.file),
        ),
    );

    clan.write(
        &mapping.id,
        &mapping.clan.machine,
        &mapping.clan.generator,
        &mapping.clan.file,
        &outgoing,
    )?;

    // No commit here, and that is not an omission. Nothing in this repository
    // changed: the value went into clan's, and `clan vars set` committed it
    // there — one invocation per mapping, so the single-intent discipline holds
    // across the boundary rather than being restated on this side of it.
    Ok(Outcome::Updated)
}

/// A mapping's two endpoints, as every report of it names them.
///
/// One function rather than a pair of `format!` calls in each report: the
/// transfer's report and the audit's name the same two endpoints, and a
/// difference between them would be a difference with nothing behind it.
pub(crate) fn endpoints(mapping: &Mapping) -> (String, String) {
    (
        format!(
            "{} {}",
            mapping.clan.machine,
            Clan::var_id(&mapping.clan.generator, &mapping.clan.file)
        ),
        format!("{}.{}", mapping.safix.user, mapping.safix.name),
    )
}

/// What safix holds for a mapping's entry, or nothing when the key is not there.
///
/// Absence is answered from the document's own structure rather than from a
/// decryption that came back empty: `set` creates a file holding the target key
/// with an empty value before it asks for one, so "the file exists and the key
/// decrypts to nothing" is a state a value has never been written into and is
/// not distinguishable from a value that is legitimately empty by decrypting.
pub(crate) fn held_by_safix(workspace: &Workspace, mapping: &Mapping) -> Result<Option<Secret>> {
    let placement = workspace.resolve(&mapping.safix.user, &mapping.safix.name)?;
    let key = placement.key.clone();
    let relative = placement.file.clone();
    let absolute = workspace.absolute(&relative);

    let Some(text) = workspace.read_relative(&relative)? else {
        return Ok(None);
    };
    match document::keys_of(&text)?.get(&key) {
        None => return Ok(None),
        Some(state) if state.empty => return Ok(None),
        Some(_) => {}
    }

    let decrypted = workspace.sops().decrypt_key(&absolute, &key)?;
    if decrypted.status != 0 {
        return Err(Error::SourceUnreadable {
            mapping: mapping.id.clone(),
            user: mapping.safix.user.clone(),
            name: mapping.safix.name.clone(),
            file: relative,
        });
    }
    Ok(Some(decrypted.value))
}

/// The commit subject an imported mapping lands under, for a caller that wants
/// to assert it without running a transfer.
#[must_use]
pub fn commit_subject(mapping: &Mapping) -> String {
    format!(
        "chore(safix): {} {} for {}",
        mapping.direction.verb(),
        mapping.id,
        mapping.safix.user
    )
}
