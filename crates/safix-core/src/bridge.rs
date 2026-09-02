//! Moving a declared value across the boundary, in either direction.
//!
//! The mappings are declarations — see `modules/flake/safix/bridge.nix` for why
//! a standing relationship is written down rather than passed as arguments —
//! and this module is what acts on them. [`crate::clan`] is how the far side is
//! reached; nothing here touches a file clan placed.
//!
//! # One run, every direction a mapping declares
//!
//! [`sync`] is the clan target's entry point: it converges every declared
//! mapping, or the ones named, each moving in its own declared direction in the
//! same run. An optional `--direction` filter narrows which mappings a run acts
//! on; it never overrides a mapping's own declared direction.
//!
//! # Both directions read both sides before writing either
//!
//! For `clan-to-safix` the comparison saves a commit. For `safix-to-clan` it is
//! load-bearing rather than an optimisation, and the reason is a property of
//! clan rather than a preference of ours: `clan vars set` writes
//! unconditionally and commits what it wrote, and a re-encrypting backend — this
//! fleet's is `age` — produces fresh ciphertext for an unchanged value. Without
//! the read-first comparison every safix-to-clan write would produce a commit in
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

use std::cell::RefCell;
use std::collections::HashMap;

use crate::clan::{Clan, Reading};
use crate::error::{Error, Result};
use crate::model::{Direction, Mapping};
use crate::progress::{Progress, log};
use crate::scratch;
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::sops::document;
use crate::workspace::Workspace;

/// Which of `sync`'s and `audit`'s two targets a run acts on, or both when
/// neither is named.
///
/// `safix-cli`'s dispatch grammar is what owns the "bare means every target"
/// and "a target keyword narrows" rules; this is the value that grammar
/// resolves to once the command line has been read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// `flake.safix.bridge.mappings`.
    Clan,
    /// `flake.safix.keepassxc.mappings`.
    Keepassxc,
}

/// The three words `sync` and `audit` read as a target keyword rather than a
/// mapping name.
///
/// Evaluation refuses a declared mapping id spelled one of these — see
/// `modules/flake/safix/bridge.nix`'s and `keepassxc.nix`'s own `reservedId` —
/// so a name reaching this far that still matches one is not a declared
/// mapping at all; it is the target-keyword role showing up where a mapping
/// name was expected.
pub const RESERVED_MAPPING_WORDS: [&str; 3] = ["clan", "keepassxc", "all"];

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
    /// Which way it moved.
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

/// Discovers and memoizes the machine that addresses a mapping's clan side.
///
/// A per-machine mapping already declares its machine and never searches. A
/// shared mapping's machine is discovered by trying each name
/// [`Clan::machines`] returns in turn against [`Clan::read`], using the
/// [`Error::ClanVarUnknown`] clan raises for "wrong machine" to tell it apart
/// from a genuine failure, and stopping at the first that resolves. The
/// result is memoized per `(generator, file)`, so mappings sharing one clan
/// var search once rather than once per mapping.
pub(crate) struct Addressing<'c> {
    clan: &'c Clan,
    cache: RefCell<HashMap<(String, String), String>>,
}

impl<'c> Addressing<'c> {
    pub(crate) fn new(clan: &'c Clan) -> Self {
        Self {
            clan,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Read a mapping's clan side, discovering and caching its addressing
    /// machine first when its placement is shared.
    ///
    /// # Errors
    ///
    /// Whatever [`Clan::read`] raises, and [`Error::ClanAddressUnresolved`]
    /// for a shared mapping no machine [`Clan::machines`] returned resolves.
    pub(crate) fn read(&self, mapping: &Mapping) -> Result<Reading> {
        let (machine, already_read) = self.resolve(mapping)?;
        match already_read {
            Some(reading) => Ok(reading),
            None => self.clan.read(
                &mapping.id,
                &machine,
                &mapping.clan.generator,
                &mapping.clan.file,
            ),
        }
    }

    /// The machine that addresses this mapping's clan side.
    pub(crate) fn machine_for(&self, mapping: &Mapping) -> Result<String> {
        self.resolve(mapping).map(|(machine, _)| machine)
    }

    /// Write this mapping's clan side, addressed the same way [`Self::read`]
    /// is.
    pub(crate) fn write(&self, mapping: &Mapping, value: &Secret) -> Result<()> {
        let machine = self.machine_for(mapping)?;
        self.clan.write(
            &mapping.id,
            &machine,
            &mapping.clan.generator,
            &mapping.clan.file,
            value,
        )
    }

    /// Whether clan considers this mapping's generator stale, addressed the
    /// same way [`Self::read`] is.
    pub(crate) fn generator_stale(&self, mapping: &Mapping) -> Result<bool> {
        let machine = self.machine_for(mapping)?;
        self.clan.generator_stale(&machine, &mapping.clan.generator)
    }

    /// The machine that addresses this mapping, and the reading a shared
    /// mapping's search already performed while finding it — `None` when the
    /// machine came from the mapping's own declaration or the cache, and
    /// nothing has been read yet.
    fn resolve(&self, mapping: &Mapping) -> Result<(String, Option<Reading>)> {
        if let Some(machine) = &mapping.clan.machine {
            return Ok((machine.clone(), None));
        }
        let key = (mapping.clan.generator.clone(), mapping.clan.file.clone());
        if let Some(machine) = self.cache.borrow().get(&key).cloned() {
            return Ok((machine, None));
        }
        for candidate in self.clan.machines()? {
            match self.clan.read(
                &mapping.id,
                &candidate,
                &mapping.clan.generator,
                &mapping.clan.file,
            ) {
                Ok(reading) => {
                    self.cache.borrow_mut().insert(key, candidate.clone());
                    return Ok((candidate, Some(reading)));
                }
                Err(Error::ClanVarUnknown { .. }) => {}
                Err(other) => return Err(other),
            }
        }
        Err(Error::ClanAddressUnresolved {
            mapping: mapping.id.clone(),
            generator: mapping.clan.generator.clone(),
            file: mapping.clan.file.clone(),
        })
    }
}

/// Converge every declared clan mapping, or the ones named, each moving in its
/// own declared direction.
///
/// `direction` narrows the run to mappings declared with that value; it never
/// overrides a mapping's own declared direction, and a mapping named while it
/// does not match is refused as such rather than acted on.
///
/// # Errors
///
/// [`Error::NoClanFlake`] when no clan is declared, [`Error::ClanUnavailable`]
/// when clan's command cannot be run, [`Error::UnknownMapping`] when a named
/// mapping is not declared, and [`Error::MappingWrongDirection`] when a named
/// mapping does not match the `--direction` filter given.
pub fn sync(
    workspace: &Workspace,
    progress: &dyn Progress,
    direction: Option<Direction>,
    only: &[String],
) -> Result<Run> {
    scratch::set_floor(workspace.root());
    let _guard = scratch::Guard;

    let mappings = selected(workspace, direction, only)?;
    if mappings.iter().all(|m| m.direction == Direction::TwoWay) {
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
    let addressing = Addressing::new(&clan);

    let mut transferred = Vec::with_capacity(mappings.len());
    for mapping in mappings {
        let outcome = match mapping.direction {
            Direction::ClanToSafix => one_import(workspace, progress, &addressing, mapping),
            Direction::SafixToClan => one_export(workspace, progress, &addressing, mapping),
            // Converged by `bridge_sync::converge`, reached separately from
            // the same `sync clan` dispatch; this run reports only the two
            // one-way directions.
            Direction::TwoWay => continue,
        };
        let (clan_side, safix_side) = endpoints(mapping);
        transferred.push(Transferred {
            mapping: mapping.id.clone(),
            direction: mapping.direction,
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

/// The mappings one invocation acts on, refusing before any of them is touched.
///
/// An empty `only` is every mapping of `direction`, or every declared mapping
/// when `direction` is also absent. A named mapping declared with a different
/// direction than `direction` filters for is refused as such rather than as an
/// unknown name: the operator has spelled the mapping correctly, and a message
/// saying "not a declared mapping" about a mapping sitting three lines above in
/// their own file would be actively misleading.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s clan target reuses this
/// exact selection so that scoping a comparison and scoping a write cannot
/// answer "which mappings" differently.
pub(crate) fn selected<'a>(
    workspace: &'a Workspace,
    direction: Option<Direction>,
    only: &[String],
) -> Result<Vec<&'a Mapping>> {
    let bridge = workspace.bridge()?;
    if only.is_empty() {
        return Ok(match direction {
            Some(direction) => bridge.of(direction).collect(),
            None => bridge.mappings.iter().collect(),
        });
    }

    let mut mappings = Vec::with_capacity(only.len());
    for id in only {
        let mapping = bridge.named(id).ok_or_else(|| Error::UnknownMapping {
            mapping: id.clone(),
            declared: bridge.declared(),
        })?;
        if let Some(direction) = direction
            && mapping.direction != direction
        {
            return Err(Error::MappingWrongDirection {
                mapping: id.clone(),
                actual: mapping.direction.as_str(),
                filter: direction.as_str(),
            });
        }
        mappings.push(mapping);
    }
    Ok(mappings)
}

/// clan holds the value; safix receives it through the hand-set write path.
fn one_import(
    workspace: &Workspace,
    progress: &dyn Progress,
    addressing: &Addressing<'_>,
    mapping: &Mapping,
) -> Result<Outcome> {
    let incoming = match addressing.read(mapping)? {
        Reading::Present(value) => value,
        Reading::AbsentAtSource => return Ok(Outcome::AbsentAtSource),
    };

    if let Some(held) = held_for(workspace, mapping)?
        && held.equals(&incoming)
    {
        return Ok(Outcome::Unchanged);
    }

    let (clan_address, safix_address) = endpoints(mapping);
    log(
        progress,
        &format!("safix: {clan_address} -> {safix_address}"),
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
    addressing: &Addressing<'_>,
    mapping: &Mapping,
) -> Result<Outcome> {
    let placement = workspace.resolve(&mapping.safix.user, &mapping.safix.name)?;
    let file = placement.file.clone();
    let generated = placement.generator.is_some();

    let Some(outgoing) = held_for(workspace, mapping)? else {
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
    if let Reading::Present(held) = addressing.read(mapping)?
        && held.equals(&outgoing)
    {
        return Ok(Outcome::Unchanged);
    }

    // Asked only once a write is actually going to happen. A mapping whose two
    // sides already agree writes nothing, so there is no value for a later
    // generation to discard and nothing for this refusal to prevent; refusing
    // there would refuse a no-op and would make a second run of a stale mapping
    // report differently from the first.
    if addressing.generator_stale(mapping)? {
        return Ok(Outcome::Refused(Error::GeneratorDefinitionDrifted {
            mapping: mapping.id.clone(),
            machine: addressing.machine_for(mapping)?,
            generator: mapping.clan.generator.clone(),
        }));
    }

    let (clan_address, safix_address) = endpoints(mapping);
    log(
        progress,
        &format!("safix: {safix_address} -> {clan_address}"),
    );

    addressing.write(mapping, &outgoing)?;

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
///
/// A shared mapping's clan side names `shared` rather than a machine: no
/// machine is declared for one, and the machine an addressing search
/// discovers at run time is not part of the mapping's own declared identity.
pub(crate) fn endpoints(mapping: &Mapping) -> (String, String) {
    let owner = mapping.clan.machine.as_deref().unwrap_or("shared");
    (
        format!(
            "{owner} {}",
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
///
/// Addressed by the three names rather than by a [`Mapping`] because
/// [`crate::sync`] asks the same question about a mapping of its own: what safix
/// holds for one entry is one question, and two readers of it would be two
/// answers a report could disagree with a transfer over.
pub(crate) fn held_by_safix(
    workspace: &Workspace,
    mapping: &str,
    user: &str,
    name: &str,
) -> Result<Option<Secret>> {
    let placement = workspace.resolve(user, name)?;
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
            mapping: mapping.to_owned(),
            user: user.to_owned(),
            name: name.to_owned(),
            file: relative,
        });
    }
    Ok(Some(decrypted.value))
}

/// The same read, addressed by a clan mapping.
pub(crate) fn held_for(workspace: &Workspace, mapping: &Mapping) -> Result<Option<Secret>> {
    held_by_safix(
        workspace,
        &mapping.id,
        &mapping.safix.user,
        &mapping.safix.name,
    )
}

/// The commit subject a written mapping lands under, for a caller that wants
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

/// Converging a two-way mapping toward whichever side changed since the last
/// recorded agreement.
///
/// [`decide`]/[`judge`] mirror [`crate::sync::two_way`] (`sync.rs:451-493`)
/// exactly, adapted to [`Reading`] and [`Secret`] rather than a database read.
/// `push`/`pull` reuse the identical write paths a one-way transfer already
/// has — [`Addressing::write`] under the same stale-generator refusal
/// [`one_export`] carries (D9), and [`set::run_committing`] under the same
/// discipline [`one_import`] carries — and record the agreement afterward as
/// its own, separate commit, never folded into the value's own.
///
/// See `openspec/changes/sync-clan-vars-two-way/design.md`'s D6\u{2013}D11 for
/// why the agreement lives in a companion entry inside safix's own
/// sops-encrypted store rather than anywhere else, and
/// `specs/bridge-sync/spec.md` for the five outcome classes this module's
/// [`Outcome`] carries.
pub mod bridge_sync {
    use super::{
        Addressing, Clan, Direction, Error, Held, Mapping, Progress, Reading, Result, Secret,
        Workspace, endpoints, held_for, log, scratch, selected, set,
    };

    /// The tag the recorded agreement carries.
    ///
    /// Distinct from [`crate::sync::FORMAT`] so the two mechanisms' memories
    /// are never mistaken for one another if a consumer somehow points both
    /// at overlapping entries, and so a future change to one format tag does
    /// not silently reinterpret the other's records.
    pub const FORMAT: &str = "safix-bridge-sync-v1";

    /// The suffix safix reserves for a two-way mapping's companion entry.
    ///
    /// Matches `modules/flake/safix/bridge.nix`'s `stateSuffix` \u{2014} hyphenated
    /// rather than [`crate::store::STATE_SUFFIX`]'s dot-prefixed form: a
    /// companion here is a safix entry name, and `resolve.nix` refuses any
    /// declared name outside `[a-z0-9][a-z0-9_-]*` before a dot-prefixed
    /// reservation could ever collide with a hand declaration.
    const COMPANION_SUFFIX: &str = "-safix-bridge-sync-state";

    /// What happened to one two-way mapping.
    ///
    /// Five classes, exactly D8's. No `Debug`, for the reason
    /// [`super::Outcome`] has none: nothing here holds a value, and keeping
    /// it that way is easier than proving each future variant does not.
    /// [`Self::Conflict`] carries no [`Error`]: a conflict is a finding the
    /// decision function reaches rather than a failed write, so there is no
    /// refusal for a `Code` table entry to attach to.
    pub enum Outcome {
        /// Both sides already held the same bytes, or neither held anything
        /// at all. Nothing written, nothing committed, nothing remembered.
        Unchanged,
        /// clan now holds what safix held; the agreement is remembered.
        UpdatedTowardClan,
        /// safix now holds what clan held; the agreement is remembered.
        UpdatedTowardSafix,
        /// The two sides differ from the last-recorded agreement, or from
        /// each other with none recorded yet. Nothing written.
        Conflict,
        /// This mapping was refused, and this is why.
        Refused(Error),
    }

    impl Outcome {
        /// The word a report prints for this outcome.
        #[must_use]
        pub const fn as_str(&self) -> &'static str {
            match self {
                Self::Unchanged => "unchanged",
                Self::UpdatedTowardClan => "updated toward clan",
                Self::UpdatedTowardSafix => "updated toward safix",
                Self::Conflict => "conflict",
                Self::Refused(_) => "refused",
            }
        }

        /// Whether this outcome makes the run a failure.
        #[must_use]
        pub const fn is_failure(&self) -> bool {
            matches!(self, Self::Conflict | Self::Refused(_))
        }
    }

    /// One mapping's line in a convergence's report.
    pub struct Converged {
        /// The mapping's declared name.
        pub mapping: String,
        /// The clan endpoint, as `<machine> <generator>/<file>`.
        pub clan: String,
        /// The safix endpoint, as `<user>.<name>`.
        pub safix: String,
        /// What happened.
        pub outcome: Outcome,
    }

    /// Everything one convergence judged.
    pub struct Report {
        /// One entry per two-way mapping acted on, in declaration order.
        pub converged: Vec<Converged>,
    }

    impl Report {
        /// Whether every mapping converged without a conflict or a refusal.
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
                unchanged: count(Outcome::Unchanged.as_str()),
                updated_toward_clan: count(Outcome::UpdatedTowardClan.as_str()),
                updated_toward_safix: count(Outcome::UpdatedTowardSafix.as_str()),
                conflict: count(Outcome::Conflict.as_str()),
                refused: count(Outcome::Refused(Error::ClanPipeMissing).as_str()),
            }
        }
    }

    /// The counts a convergence's closing line reports.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Tally {
        /// Mappings whose two sides already agreed.
        pub unchanged: usize,
        /// Mappings whose clan side was written.
        pub updated_toward_clan: usize,
        /// Mappings whose safix side was written.
        pub updated_toward_safix: usize,
        /// Mappings whose two sides disagree with nobody to decide.
        pub conflict: usize,
        /// Mappings refused.
        pub refused: usize,
    }

    /// What a mapping's two current sides, and its remembered agreement, say
    /// to do about it.
    ///
    /// No `Debug`, for the reason [`crate::sync::Decision`] has none: every
    /// value held here is a secret, however briefly.
    enum Decision {
        /// Nothing to do.
        Settled(Outcome),
        /// Write this value into clan, and record the agreement.
        Push { value: Secret, remember: bool },
        /// Write this value into safix, and record the agreement.
        Pull { value: Secret, remember: bool },
    }

    /// Read both sides of one two-way mapping, and the companion's memory
    /// when they differ, and decide what to do about it.
    ///
    /// A read failure on either side is [`Outcome::Refused`] rather than a
    /// `NotJudged` outcome: `bridge_sync::Outcome` has no unjudged variant,
    /// per D8/D10's five classes.
    fn decide(workspace: &Workspace, addressing: &Addressing<'_>, mapping: &Mapping) -> Decision {
        let clan_value = match addressing.read(mapping) {
            Ok(Reading::Present(value)) => Some(value),
            Ok(Reading::AbsentAtSource) => None,
            Err(reason) => return Decision::Settled(Outcome::Refused(reason)),
        };
        let safix_value = match held_for(workspace, mapping) {
            Ok(held) => held,
            Err(reason) => return Decision::Settled(Outcome::Refused(reason)),
        };
        let remembered = match (&safix_value, &clan_value) {
            (Some(safix), Some(clan)) if !safix.equals(clan) => {
                remembered_agreement(workspace, mapping)
            }
            _ => None,
        };
        judge(safix_value, clan_value, remembered)
    }

    /// The pure four-way decision, over values already read: mirrors
    /// [`crate::sync::two_way`] exactly. Both absent is unchanged; exactly
    /// one absent is a bootstrap push or pull, remembered; both present and
    /// equal is unchanged; both present and unequal consults `remembered` \u{2014}
    /// one side still agreeing is a converge toward the other, remembered;
    /// neither agreeing, or no agreement recorded yet, is a conflict.
    fn judge(safix: Option<Secret>, clan: Option<Secret>, remembered: Option<Secret>) -> Decision {
        match (safix, clan) {
            (None, None) => Decision::Settled(Outcome::Unchanged),
            (Some(safix), None) => Decision::Push {
                value: safix,
                remember: true,
            },
            (None, Some(clan)) => Decision::Pull {
                value: clan,
                remember: true,
            },
            (Some(safix), Some(clan)) => {
                if safix.equals(&clan) {
                    return Decision::Settled(Outcome::Unchanged);
                }
                let Some(remembered) = remembered else {
                    return Decision::Settled(Outcome::Conflict);
                };
                match (agrees(&remembered, &safix), agrees(&remembered, &clan)) {
                    // safix is where the agreement left it, so clan is the
                    // side that moved.
                    (true, false) => Decision::Pull {
                        value: clan,
                        remember: true,
                    },
                    (false, true) => Decision::Push {
                        value: safix,
                        remember: true,
                    },
                    // Both moved, or neither matches an agreement this run
                    // cannot account for. Either way nothing is written.
                    _ => Decision::Settled(Outcome::Conflict),
                }
            }
        }
    }

    /// The safix name a mapping's companion entry is declared under, beside
    /// the mapped entry in the same document.
    fn companion_name(mapping: &Mapping) -> String {
        format!("{}{COMPANION_SUFFIX}", mapping.safix.name)
    }

    /// The agreement the companion entry remembers, as the bytes it holds.
    ///
    /// A companion that will not read is treated as absent rather than as a
    /// refusal, mirroring [`crate::sync`]'s own `recorded` exactly: the
    /// memory is safix's own bookkeeping, and a run that stopped over it
    /// would refuse a mapping whose two sides it can see perfectly well.
    fn remembered_agreement(workspace: &Workspace, mapping: &Mapping) -> Option<Secret> {
        held_for_companion(workspace, mapping).ok().flatten()
    }

    /// The companion's own read, addressed the way [`held_for`] addresses
    /// the mapped entry.
    fn held_for_companion(workspace: &Workspace, mapping: &Mapping) -> Result<Option<Secret>> {
        super::held_by_safix(
            workspace,
            &mapping.id,
            &mapping.safix.user,
            &companion_name(mapping),
        )
    }

    /// Whether the remembered agreement is the one this value would have
    /// recorded, compared as bytes against the line a converging write would
    /// have written \u{2014} the same discipline [`crate::sync`]'s own `agrees`
    /// has, for the same reason: a memory written under a tag this version
    /// does not know matches neither side, so every path that consults it
    /// reports a conflict rather than guessing.
    fn agrees(remembered: &Secret, value: &Secret) -> bool {
        let line = memory_of(value);
        Secret::read_from(&mut line.as_bytes()).is_ok_and(|written| written.equals(remembered))
    }

    /// The line a converging write records the agreement as.
    fn memory_of(value: &Secret) -> String {
        format!("{FORMAT} {}", value.fingerprint())
    }

    /// The commit subject a two-way convergence's value write lands under,
    /// for a caller that wants to assert it without running a convergence.
    ///
    /// Not [`super::commit_subject`]: [`Direction::verb`]'s own documentation
    /// states that function is never called for a two-way mapping, because
    /// two-way builds a commit subject of its own.
    #[must_use]
    pub fn commit_subject(mapping: &Mapping) -> String {
        format!(
            "chore(safix): converge {} for {}",
            mapping.id, mapping.safix.user
        )
    }

    /// The commit subject the companion's own write lands under \u{2014} always a
    /// second, separate commit from [`commit_subject`]'s.
    fn companion_commit_subject(mapping: &Mapping) -> String {
        format!(
            "chore(safix): remember the agreement for {} for {}",
            mapping.id, mapping.safix.user
        )
    }

    /// Write clan's side, and the companion's agreement afterward as its own,
    /// separate write.
    ///
    /// The comparison that decides a write is happening at all already ran in
    /// [`decide`]; what remains here is the stale-generator refusal
    /// [`one_export`] has (D9), so a two-way push into clan is refused under
    /// the identical condition and the identical message a safix-to-clan
    /// write of the same mapping would be.
    fn push(
        workspace: &Workspace,
        progress: &dyn Progress,
        addressing: &Addressing<'_>,
        mapping: &Mapping,
        value: &Secret,
        remember: bool,
    ) -> Outcome {
        match addressing.generator_stale(mapping) {
            Ok(true) => {
                let machine = match addressing.machine_for(mapping) {
                    Ok(machine) => machine,
                    Err(reason) => return Outcome::Refused(reason),
                };
                return Outcome::Refused(Error::GeneratorDefinitionDrifted {
                    mapping: mapping.id.clone(),
                    machine,
                    generator: mapping.clan.generator.clone(),
                });
            }
            Ok(false) => {}
            Err(reason) => return Outcome::Refused(reason),
        }

        let (clan_address, safix_address) = endpoints(mapping);
        log(
            progress,
            &format!("safix: {safix_address} -> {clan_address}"),
        );

        if let Err(reason) = addressing.write(mapping, value) {
            return Outcome::Refused(reason);
        }

        if remember {
            let line = memory_of(value);
            if let Err(reason) = remember_agreement(workspace, progress, mapping, &line) {
                return Outcome::Refused(reason);
            }
        }
        Outcome::UpdatedTowardClan
    }

    /// Write safix's side through the ordinary write path, and the
    /// companion's agreement afterward as its own, separate write.
    fn pull(
        workspace: &Workspace,
        progress: &dyn Progress,
        mapping: &Mapping,
        value: Secret,
        remember: bool,
    ) -> Outcome {
        let (clan_address, safix_address) = endpoints(mapping);
        log(
            progress,
            &format!("safix: {clan_address} -> {safix_address}"),
        );

        // Computed from a second reading before the write below consumes it.
        let line = memory_of(&value);

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
            // sops refused and has said why on its own standard error, which
            // is inherited. Reporting it as this mapping's refusal is what
            // keeps the rest of the run going and the report honest about
            // which mapping it was.
            Ok(status) if status != 0 => {
                let file = match workspace.resolve(&mapping.safix.user, &mapping.safix.name) {
                    Ok(placement) => placement.file.clone(),
                    Err(reason) => return Outcome::Refused(reason),
                };
                return Outcome::Refused(Error::SourceUnreadable {
                    mapping: mapping.id.clone(),
                    user: mapping.safix.user.clone(),
                    name: mapping.safix.name.clone(),
                    file,
                });
            }
            Ok(_) => {}
        }

        if remember && let Err(reason) = remember_agreement(workspace, progress, mapping, &line) {
            return Outcome::Refused(reason);
        }
        Outcome::UpdatedTowardSafix
    }

    /// Record the agreement, after the value it is about has landed, as its
    /// own, separate commit.
    ///
    /// This order is load-bearing, per D8: a memory written first and not
    /// followed by its value would say the two sides agreed on a value only
    /// one of them holds, and the next run would read that as "the side
    /// holding the new value never changed" and converge the other way \u{2014}
    /// overwriting the new value with the old one.
    fn remember_agreement(
        workspace: &Workspace,
        progress: &dyn Progress,
        mapping: &Mapping,
        line: &str,
    ) -> Result<()> {
        let recorded = Secret::read_from(&mut line.as_bytes())?;
        let name = companion_name(mapping);
        let status = set::run_committing(
            workspace,
            progress,
            &mut Held(Some(recorded)),
            &mapping.safix.user,
            &name,
            &companion_commit_subject(mapping),
        )?;
        if status != 0 {
            let file = workspace.resolve(&mapping.safix.user, &name)?.file.clone();
            return Err(Error::SourceUnreadable {
                mapping: mapping.id.clone(),
                user: mapping.safix.user.clone(),
                name,
                file,
            });
        }
        Ok(())
    }

    /// Converge every declared two-way clan mapping, or the ones named.
    ///
    /// Reached from the same `sync clan` dispatch as [`super::sync`], with
    /// the same `direction`/`only`: a two-way mapping named under a one-way
    /// `--direction` filter, or a one-way mapping named under `--direction
    /// two-way`, is refused by [`selected`]'s own generic mismatch refusal
    /// before this function's own loop is reached \u{2014} the same refusal
    /// [`super::sync`] raises for its own two directions.
    ///
    /// # Errors
    ///
    /// As [`super::sync`]'s: [`Error::NoClanFlake`],
    /// [`Error::ClanUnavailable`], [`Error::UnknownMapping`] and
    /// [`Error::MappingWrongDirection`].
    pub fn converge(
        workspace: &Workspace,
        progress: &dyn Progress,
        direction: Option<Direction>,
        only: &[String],
    ) -> Result<Report> {
        scratch::set_floor(workspace.root());
        let _guard = scratch::Guard;

        let mappings: Vec<&Mapping> = selected(workspace, direction, only)?
            .into_iter()
            .filter(|mapping| mapping.direction == Direction::TwoWay)
            .collect();
        if mappings.is_empty() {
            return Ok(Report {
                converged: Vec::new(),
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

        let mut decided: Vec<(&Mapping, Decision)> = Vec::with_capacity(mappings.len());
        for mapping in mappings {
            decided.push((mapping, decide(workspace, &addressing, mapping)));
        }

        let mut converged = Vec::with_capacity(decided.len());
        for (mapping, decision) in decided {
            let outcome = match decision {
                Decision::Settled(outcome) => outcome,
                Decision::Push { value, remember } => {
                    push(workspace, progress, &addressing, mapping, &value, remember)
                }
                Decision::Pull { value, remember } => {
                    pull(workspace, progress, mapping, value, remember)
                }
            };
            let (clan_side, safix_side) = endpoints(mapping);
            converged.push(Converged {
                mapping: mapping.id.clone(),
                clan: clan_side,
                safix: safix_side,
                outcome,
            });
            if scratch::interrupted().is_some() {
                break;
            }
        }

        Ok(Report { converged })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// A [`Secret`] built from a literal, for a fixture that never
        /// touches a stream.
        fn secret(bytes: &str) -> Secret {
            Secret::read_from(&mut bytes.as_bytes()).expect("a fixture value")
        }

        /// The agreement line a converging write of `value` would have
        /// recorded, as a [`Secret`] \u{2014} what a companion read would hand
        /// back after such a write.
        fn agreement_of(value: &str) -> Secret {
            secret(&memory_of(&secret(value)))
        }

        #[test]
        fn neither_side_holding_anything_is_unchanged() {
            assert!(matches!(
                judge(None, None, None),
                Decision::Settled(Outcome::Unchanged)
            ));
        }

        #[test]
        fn a_side_that_has_never_held_a_value_is_bootstrap_not_a_failure() {
            match judge(Some(secret("alpha")), None, None) {
                Decision::Push { remember, .. } => assert!(remember),
                other => unreachable!("safix-only became {other:?}", other = describe(&other)),
            }
            match judge(None, Some(secret("alpha")), None) {
                Decision::Pull { remember, .. } => assert!(remember),
                other => unreachable!("clan-only became {other:?}", other = describe(&other)),
            }
        }

        #[test]
        fn agreeing_values_are_unchanged_whatever_the_companion_says() {
            assert!(matches!(
                judge(Some(secret("alpha")), Some(secret("alpha")), None),
                Decision::Settled(Outcome::Unchanged)
            ));
            assert!(matches!(
                judge(
                    Some(secret("alpha")),
                    Some(secret("alpha")),
                    Some(secret("unrelated companion bytes")),
                ),
                Decision::Settled(Outcome::Unchanged)
            ));
        }

        #[test]
        fn disagreeing_with_no_agreement_recorded_yet_is_a_conflict() {
            assert!(matches!(
                judge(Some(secret("alpha")), Some(secret("beta")), None),
                Decision::Settled(Outcome::Conflict)
            ));
        }

        #[test]
        fn one_side_moved_since_the_agreement_converges_toward_it() {
            let remembered = agreement_of("alpha");
            match judge(
                Some(secret("beta")),
                Some(secret("alpha")),
                Some(remembered),
            ) {
                Decision::Push { remember, .. } => assert!(remember),
                other => unreachable!("safix-moved became {other:?}", other = describe(&other)),
            }

            let remembered = agreement_of("alpha");
            match judge(
                Some(secret("alpha")),
                Some(secret("gamma")),
                Some(remembered),
            ) {
                Decision::Pull { remember, .. } => assert!(remember),
                other => unreachable!("clan-moved became {other:?}", other = describe(&other)),
            }
        }

        #[test]
        fn both_sides_moving_since_the_agreement_is_a_conflict_not_a_guess() {
            let remembered = agreement_of("alpha");
            assert!(matches!(
                judge(
                    Some(secret("beta")),
                    Some(secret("gamma")),
                    Some(remembered),
                ),
                Decision::Settled(Outcome::Conflict)
            ));
        }

        /// The interruption case design.md's D8 names: a companion write
        /// interrupted after the value landed leaves the companion holding
        /// an agreement older than either side's current bytes. Both sides
        /// here already differ from the stale companion \u{2014} one because it
        /// is what an interrupted push already landed, the other because it
        /// changed again afterward \u{2014} so this exercises the same conflict
        /// arm as `both_sides_moving_since_the_agreement_is_a_conflict_not_a_guess`
        /// through the branch where one side coincides with what the
        /// interrupted write produced rather than with a value neither side
        /// has ever held, proving the stale agreement is never silently
        /// trusted as still describing the side that looks unchanged since
        /// it.
        #[test]
        fn a_stale_companion_from_an_interrupted_write_never_resolves_the_next_divergence_by_a_guess()
         {
            let remembered = agreement_of("original");
            let landed = secret("landed-by-the-interrupted-push");
            let further_edit = secret("edited-again-after-the-interruption");

            assert!(matches!(
                judge(Some(further_edit), Some(landed), Some(remembered)),
                Decision::Settled(Outcome::Conflict)
            ));
        }

        #[test]
        fn every_outcome_has_a_word_and_only_conflict_and_refused_fail_the_run() {
            let outcomes = [
                (Outcome::Unchanged, "unchanged", false),
                (Outcome::UpdatedTowardClan, "updated toward clan", false),
                (Outcome::UpdatedTowardSafix, "updated toward safix", false),
                (Outcome::Conflict, "conflict", true),
                (Outcome::Refused(Error::ClanPipeMissing), "refused", true),
            ];
            for (outcome, word, failure) in outcomes {
                assert_eq!(outcome.as_str(), word);
                assert_eq!(outcome.is_failure(), failure, "{word} fails the run");
            }
        }

        #[test]
        fn the_memory_carries_a_format_tag_distinct_from_syncs_own() {
            let value = secret("fixture");
            let line = memory_of(&value);
            assert!(line.starts_with("safix-bridge-sync-v1 "));
            assert_ne!(FORMAT, crate::sync::FORMAT);
        }

        #[test]
        fn the_companion_name_carries_the_hyphenated_suffix() {
            let bridge: crate::model::Bridge = serde_json::from_str(
                r#"{
                  "clanFlake": ".",
                  "mappings": [
                    {
                      "id": "tok",
                      "direction": "two-way",
                      "clan": {
                        "placement": "per-machine",
                        "machine": "meridian",
                        "generator": "ntfy",
                        "file": "token"
                      },
                      "safix": { "user": "alice", "name": "tok" }
                    }
                  ]
                }"#,
            )
            .expect("a fixture bridge");
            let mapping = bridge.named("tok").expect("the fixture mapping");
            assert_eq!(companion_name(mapping), "tok-safix-bridge-sync-state");
        }

        /// A `Decision` printed for an `unreachable!` message, since it has
        /// no `Debug` of its own \u{2014} see the type's own doc comment.
        fn describe(decision: &Decision) -> &'static str {
            match decision {
                Decision::Settled(outcome) => outcome.as_str(),
                Decision::Push { .. } => "push",
                Decision::Pull { .. } => "pull",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;

    use super::*;
    use crate::model::Bridge;

    /// A single shared-placement two-way mapping, deserialized the way the
    /// runtime reads one off `nix eval`.
    fn shared_mapping_bridge() -> Bridge {
        serde_json::from_str(
            r#"{
              "clanFlake": ".",
              "mappings": [
                {
                  "id": "ntfy-token",
                  "direction": "two-way",
                  "clan": {
                    "placement": "shared",
                    "machine": null,
                    "generator": "ntfy",
                    "file": "token"
                  },
                  "safix": { "user": "alice", "name": "ntfy-token" }
                }
              ]
            }"#,
        )
        .expect("a fixture bridge")
    }

    /// A scratch directory unique to one test run, not only one process.
    ///
    /// A pid alone collides with a stale directory a differently-timed
    /// earlier run of this same binary left behind under load, whose script
    /// can still be executing when this run's [`stub`] tries to create a new
    /// one at the same path, failing with `ETXTBSY` for a reason that has
    /// nothing to do with [`Addressing`] itself.
    fn addressing_test_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        std::env::temp_dir().join(format!(
            "safix-bridge-addressing-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    /// A stub `clan` answering `machines list` with `machines`, in order, and
    /// `vars get` by appending the machine it was asked about to `log` and
    /// resolving only `right`; every other machine answers "Couldn't find
    /// var", the same substring [`Error::ClanVarUnknown`] matches on the real
    /// command.
    ///
    /// A bash script rather than a compiled binary, following
    /// `enroll::proof::tests`' own pattern of writing a small executable stub
    /// into a per-test directory under [`std::env::temp_dir`]: `Clan::read`
    /// and `Clan::machines` both spawn `self.program` directly, so the
    /// double under test needs to be a real executable rather than a closure.
    fn stub(directory: &Path, machines: &[&str], right: &str, log: &Path) -> std::path::PathBuf {
        let script = format!(
            "#!/usr/bin/env bash\n\
             set -euo pipefail\n\
             case \"$1 $2\" in\n\
             \"machines list\")\n\
             printf '%s\\n' {machines}\n\
             ;;\n\
             \"vars get\")\n\
             printf '%s\\n' \"$5\" >> {log}\n\
             if [ \"$5\" = {right} ]; then\n\
             printf '%s' fixture-value\n\
             else\n\
             echo \"Couldn't find var: $6 for machine: $5\" >&2\n\
             exit 1\n\
             fi\n\
             ;;\n\
             *)\n\
             echo \"stub: unrecognized arguments: $*\" >&2\n\
             exit 1\n\
             ;;\n\
             esac\n",
            machines = machines.join(" "),
            log = log.display(),
            right = right,
        );
        let path = directory.join("clan-addressing-stub");
        let mut file = std::fs::File::create(&path).expect("the stub file can be created");
        file.write_all(script.as_bytes())
            .expect("the stub can be written");
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        path
    }

    /// [`Addressing::read`], retried past a transient `ETXTBSY`.
    ///
    /// This suite runs hundreds of tests concurrently, many of them spawning
    /// subprocesses of their own; under that load a script this test just
    /// wrote and `chmod`ed can be reported busy for a moment by the kernel
    /// before it settles, which is a property of running many `fork`/`exec`
    /// calls at once rather than of [`Addressing`] or of the stub script
    /// itself.
    fn read_past_transient_busy(addressing: &Addressing<'_>, mapping: &Mapping) -> Result<Reading> {
        for _ in 0..49 {
            match addressing.read(mapping) {
                Err(Error::ClanUnavailable { cause, .. })
                    if cause.kind() == std::io::ErrorKind::ExecutableFileBusy =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                other => return other,
            }
        }
        addressing.read(mapping)
    }

    #[test]
    fn a_shared_addressing_search_stops_at_the_first_machine_that_resolves() {
        let directory = addressing_test_dir("stop");
        std::fs::create_dir_all(&directory).expect("a temporary directory can be made");
        let log = directory.join("attempts");
        let program = stub(
            &directory,
            &["wrong-one", "right", "wrong-two"],
            "right",
            &log,
        );

        let clan = Clan::for_tests(program, ".".to_owned());
        let addressing = Addressing::new(&clan);
        let bridge = shared_mapping_bridge();
        let mapping = bridge.named("ntfy-token").expect("the fixture mapping");

        let reading = read_past_transient_busy(&addressing, mapping).expect("the search resolves");
        assert!(matches!(reading, Reading::Present(_)));

        let attempts = std::fs::read_to_string(&log).expect("the log was written");
        assert_eq!(
            attempts.lines().collect::<Vec<_>>(),
            ["wrong-one", "right"],
            "the third candidate was tried although the second already resolved"
        );

        std::fs::remove_dir_all(&directory).expect("it can be removed");
    }

    #[test]
    fn exhausting_every_machine_is_refused_naming_the_mapping_and_tries_each_once() {
        let directory = addressing_test_dir("exhaust");
        std::fs::create_dir_all(&directory).expect("a temporary directory can be made");
        let log = directory.join("attempts");
        let program = stub(&directory, &["one", "two", "three"], "nobody", &log);

        let clan = Clan::for_tests(program, ".".to_owned());
        let addressing = Addressing::new(&clan);
        let bridge = shared_mapping_bridge();
        let mapping = bridge.named("ntfy-token").expect("the fixture mapping");

        match read_past_transient_busy(&addressing, mapping) {
            Ok(_) => unreachable!("no machine should have resolved"),
            Err(Error::ClanAddressUnresolved {
                mapping: id,
                generator,
                file,
            }) => {
                assert_eq!(id, "ntfy-token");
                assert_eq!(generator, "ntfy");
                assert_eq!(file, "token");
            }
            Err(other) => unreachable!("exhaustion became {other:?}"),
        }

        let attempts = std::fs::read_to_string(&log).expect("the log was written");
        assert_eq!(
            attempts.lines().collect::<Vec<_>>(),
            ["one", "two", "three"],
            "each candidate should have been tried exactly once"
        );

        std::fs::remove_dir_all(&directory).expect("it can be removed");
    }
}
