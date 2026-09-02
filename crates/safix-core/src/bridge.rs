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

    #[test]
    fn a_shared_addressing_search_stops_at_the_first_machine_that_resolves() {
        let directory = std::env::temp_dir().join(format!(
            "safix-bridge-addressing-stop-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("a temporary directory can be made");
        let log = directory.join("attempts");
        let program = stub(&directory, &["wrong-one", "right", "wrong-two"], "right", &log);

        let clan = Clan::for_tests(program, ".".to_owned());
        let addressing = Addressing::new(&clan);
        let bridge = shared_mapping_bridge();
        let mapping = bridge.named("ntfy-token").expect("the fixture mapping");

        let reading = addressing.read(mapping).expect("the search resolves");
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
        let directory = std::env::temp_dir().join(format!(
            "safix-bridge-addressing-exhaust-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("a temporary directory can be made");
        let log = directory.join("attempts");
        let program = stub(&directory, &["one", "two", "three"], "nobody", &log);

        let clan = Clan::for_tests(program, ".".to_owned());
        let addressing = Addressing::new(&clan);
        let bridge = shared_mapping_bridge();
        let mapping = bridge.named("ntfy-token").expect("the fixture mapping");

        let refusal = addressing.read(mapping).expect_err("no machine resolves");
        match refusal {
            Error::ClanAddressUnresolved {
                mapping: id,
                generator,
                file,
            } => {
                assert_eq!(id, "ntfy-token");
                assert_eq!(generator, "ntfy");
                assert_eq!(file, "token");
            }
            other => unreachable!("exhaustion became {other:?}"),
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
