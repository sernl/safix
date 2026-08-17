//! The nix half's output, as types.
//!
//! safix is two halves, and this module is the seam between them. The nix half
//! resolves declarations into placements, audiences, a governed file set and a
//! recipient policy; the runtime reads those four as JSON from `nix eval` and
//! decides nothing about them. Every type here is therefore a schema rather
//! than a model: it says what the nix half emits, and its only job is to refuse
//! anything else.
//!
//! Every struct denies unknown fields. A field added on the nix side is a
//! schema change, and a reader that silently dropped it would keep working
//! while answering an older question — the failure mode this exists to prevent.
//! The cost is that adding an option to the nix half requires a matching field
//! here, in the same change, which is the intended coupling.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

/// Where a placement's declaration came from.
///
/// The three sources a secret can be declared in, and the only three the
/// resolver emits. `list` prints this and `set` logs it, so the rendering is
/// part of the command's output and is fixed by [`Origin::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// Selected out of the shared catalogue by `users.<user>.carries`.
    Carries,
    /// Declared as this user's own `users.<user>.private` entry.
    Private,
    /// Granted from outside by another user's `sharedWith.<user>`.
    Shared,
}

impl Origin {
    /// The word the command prints for this origin.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Carries => "carries",
            Self::Private => "private",
            Self::Shared => "shared",
        }
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// How a generator's prompt reads the operator's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptKind {
    /// One line, not echoed.
    Hidden,
    /// One line, echoed.
    Line,
    /// Every line until end of input.
    Multiline,
}

/// One value a generator asks the operator for.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prompt {
    /// How the input is read.
    #[serde(rename = "type")]
    pub kind: PromptKind,
    /// What the operator is being asked for.
    pub description: String,
}

/// One further output a generator writes, and whether it is encrypted.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorFile {
    /// Whether the value is encrypted, or stored in the repository in the clear.
    pub secret: bool,
}

/// What mints an entry's value, as data rather than as a derivation.
///
/// The whole generator travels inside the placement map, because the command
/// reads placements and generators out of one evaluation and so cannot resolve
/// a file by one computation and a generator by another.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    /// Other secrets of the same user whose plaintext the script reads.
    pub dependencies: Vec<String>,
    /// What this generator mints, shown by `list` and `check`.
    pub description: Option<String>,
    /// The further outputs the script writes beyond the entry carrying it,
    /// each with its own secrecy.
    pub files: BTreeMap<String, GeneratorFile>,
    /// Whether this generator's fragments reach the network.
    ///
    /// The grant travels with the rest of the declaration because it is part of
    /// what running the generator means, and it is read at evaluation because
    /// that is the audit: which generators may reach the network is a question
    /// the declarations answer with no runtime consulted. It re-shares the
    /// network and nothing else — see [`crate::sandbox`] — and governs the
    /// script and the validation fragments alike, because a validation that
    /// verifies a minted token against the API that issued it has the same need
    /// its script had.
    pub network: bool,
    /// What the operator is asked for, by the name the script addresses.
    pub prompts: BTreeMap<String, Prompt>,
    /// nixpkgs attribute names put on `PATH` while the script runs.
    #[serde(rename = "runtimeInputs")]
    pub runtime_inputs: Vec<String>,
    /// The shell fragment that produces the value.
    pub script: String,
    /// Whether every entry this generator writes is shared.
    ///
    /// Derived by the resolver from the entries rather than authored here, and
    /// refused at evaluation when the outputs disagree. It is the field a bridge
    /// to clan compares against clan's own `share`, and deriving it is what
    /// keeps one fact from having two authoring surfaces.
    pub share: bool,
    /// A shell fragment judging a candidate value, or none.
    pub validation: Option<String>,
}

impl Generator {
    /// Whether this output's value is encrypted.
    ///
    /// The entry a generator is declared on is always secret and has no slot to
    /// say otherwise: its placement — a file, a key inside it and an audience —
    /// is what the whole custody model is expressed in, and an entry with no
    /// ciphertext would have none of the three. A generator that wants to mint a
    /// public value declares it under `files`, which is how clan's own keypair
    /// samples are written.
    #[must_use]
    pub fn is_secret(&self, output: &str) -> bool {
        self.files.get(output).is_none_or(|file| file.secret)
    }
}

/// Where one name lives for one user, and what serves it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// The repository-relative path of the file holding the value.
    pub file: String,
    /// The key the value is read under inside that file.
    pub key: String,
    /// Which of the three declaration sources placed it.
    pub origin: Origin,
    /// The user whose declaration owns the entry.
    pub owner: String,
    /// Whether one value in this file serves every carrier.
    pub shared: bool,
    /// What mints the value, when anything does.
    pub generator: Option<Generator>,
    /// The repository-relative path of the plaintext value, when this entry is
    /// an output some generator declares as not secret.
    ///
    /// Computed by the resolver so that the layout has one implementation rather
    /// than one here and one in `resolve.nix`, and `null` for every entry whose
    /// value is encrypted. When it is set, [`Placement::file`] and
    /// [`Placement::key`] describe a document that is never written: a public
    /// value has no ciphertext, no recipients and no creation rule.
    pub public: Option<String>,
}

/// `user -> name -> placement`, the whole of what the command resolves against.
///
/// A [`BTreeMap`] rather than a hash map at both levels: `list` and `check`
/// walk this in order, and the shell runtime walks `jq`'s output, which is
/// sorted by key. The ordering is part of the output, so it is part of the
/// type.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Placements(pub BTreeMap<String, BTreeMap<String, Placement>>);

impl Placements {
    /// Every declared user, in name order.
    pub fn users(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Whether the declarations name this user at all.
    #[must_use]
    pub fn declares(&self, user: &str) -> bool {
        self.0.contains_key(user)
    }

    /// What this user holds, in name order, or nothing when no such user.
    #[must_use]
    pub fn held_by(&self, user: &str) -> Option<&BTreeMap<String, Placement>> {
        self.0.get(user)
    }

    /// Every user holding at least one secret, in name order.
    pub fn holders(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .filter(|(_, held)| !held.is_empty())
            .map(|(user, _)| user.as_str())
    }

    /// The generator that writes this name for this user, and the entry it is
    /// declared on.
    ///
    /// The same relation [`UserPlan::producer_of`] reads out of the run plan,
    /// read here out of the placements instead. Two readings of one fact is a
    /// cost, and the test binding them is what pays it: `resolve.nix` computes a
    /// generator's outputs as the entry it is declared on followed by the names
    /// under `files`, and both readings are exactly that.
    ///
    /// It exists because [`crate::check`] has to answer on a tree the run plan
    /// refuses. `flake.safix.lib.generatorPlan` is guarded — a cycle, a
    /// self-dependency or two producers for one output throws rather than
    /// returning an order — while `placements` is not, so a drift report that
    /// read the plan would fall silent on exactly the trees whose declarations
    /// are wrong.
    #[must_use]
    pub fn producer_of(&self, user: &str, name: &str) -> Option<(&str, &Generator)> {
        let held = self.held_by(user)?;
        if let Some((entry, generator)) = held
            .get_key_value(name)
            .and_then(|(entry, placement)| Some((entry, placement.generator.as_ref()?)))
        {
            return Some((entry.as_str(), generator));
        }
        held.iter().find_map(|(entry, placement)| {
            let generator = placement.generator.as_ref()?;
            generator
                .files
                .contains_key(name)
                .then_some((entry.as_str(), generator))
        })
    }
}

/// Which side of a generator's name space one input came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    /// Another secret of the same user, whose plaintext the script reads.
    Dependency,
    /// A value the operator is asked for.
    Prompt,
}

/// One entry of a generator's script-facing name space.
///
/// The map key is the declared name, and so is [`PlanInput::name`]: the
/// hyphen-to-underscore mapping the descriptor interface needed went with it,
/// because a prompt is now addressed as `$prompts/<name>` and a dependency as
/// `$in/<generator>/<name>` — two directories rather than one shell name space,
/// so a prompt and a dependency of the same name no longer collide. The field is
/// kept beside the key so that this runtime quotes the resolver's own spelling
/// in a refusal rather than trusting a map key to be one.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanInput {
    /// Whether the value comes from another secret or from the operator.
    pub kind: InputKind,
    /// The declared name, before the hyphen-to-underscore mapping.
    pub name: String,
}

/// One user's run plan: what may run, in which order, reading and writing what.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPlan {
    /// `generator -> script identifier -> what that identifier carries`.
    pub inputs: BTreeMap<String, BTreeMap<String, PlanInput>>,
    /// Every generator this user has, in an order that puts each after
    /// everything it reads.
    pub order: Vec<String>,
    /// `generator -> every name it writes`, the entry carrying it first.
    pub outputs: BTreeMap<String, Vec<String>>,
}

/// `user -> run plan`, as `flake.safix.lib.generatorPlan` computes it.
///
/// The order and the edges are the resolver's, not this runtime's: the nix half
/// is what refuses a cycle, and an order existing at all is that refusal's
/// postcondition. Nothing here derives an order of its own; what it does do is
/// check that postcondition on the order it was handed, which is
/// [`UserPlan::cycle`].
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct GeneratorPlan(pub BTreeMap<String, UserPlan>);

impl GeneratorPlan {
    /// This user's plan, or nothing when the declarations name no such user.
    #[must_use]
    pub fn for_user(&self, user: &str) -> Option<&UserPlan> {
        self.0.get(user)
    }
}

impl UserPlan {
    /// The generator writing this name, when one does.
    ///
    /// An output of a multi-output generator is named by its own name rather
    /// than by the entry the generator hangs off, so naming either half of a
    /// keypair resolves to the one generator that mints both.
    #[must_use]
    pub fn producer_of(&self, name: &str) -> Option<&str> {
        self.outputs
            .iter()
            .find(|(_, written)| written.iter().any(|output| output == name))
            .map(|(generator, _)| generator.as_str())
    }

    /// The generators one has to run after, out of the ones the order carries.
    ///
    /// The producers of every dependency it reads. A dependency nobody
    /// generates resolves to no producer and contributes no edge, exactly as it
    /// contributes none at evaluation. A dependency this generator produces
    /// itself does contribute one, where `resolve.nix` drops that edge and
    /// refuses the declaration by name instead: the two arrive at the same
    /// refusal from opposite ends, and a plan reaching this runtime with a
    /// self-edge in it came from neither.
    ///
    /// Restricted to the generators [`UserPlan::order`] carries, because those
    /// are the ones a run walks. Sorted and then reversed so that the caller
    /// below, which pops, meets them in name order: the cycle a refusal names
    /// is then a function of the plan rather than of the traversal.
    fn prerequisites(&self, generator: &str) -> Vec<&str> {
        let Some(inputs) = self.inputs.get(generator) else {
            return Vec::new();
        };
        let mut producers: Vec<&str> = inputs
            .values()
            .filter(|input| input.kind == InputKind::Dependency)
            .filter_map(|input| self.producer_of(&input.name))
            .filter(|producer| self.order.iter().any(|name| name == producer))
            .collect();
        producers.sort_unstable();
        producers.dedup();
        producers.reverse();
        producers
    }

    /// One cycle among the generators the order carries, when it carries one,
    /// as the participating generators with the one it closes on repeated.
    ///
    /// The resolver answers this question at evaluation and refuses there, and
    /// the generators inside a cycle are then left out of the order rather than
    /// placed in it, so a plan that reached this runtime through
    /// `flake.safix.lib.generatorPlan` never carries one. Two callers are not
    /// that plan: a stand-in for nix, and a program embedding this crate, for
    /// which [`GeneratorPlan`] is a value with public fields rather than
    /// something a refusal has already been thrown over. This is where the
    /// order's own claim is checked for them.
    ///
    /// A depth-first walk rather than the resolver's own trick of following one
    /// prerequisite per node, which is sound only inside its stuck set: a
    /// generator reached through its second prerequisite is on no path that
    /// following first prerequisites alone ever takes.
    #[must_use]
    pub fn cycle(&self) -> Option<Vec<String>> {
        let mut settled: BTreeSet<&str> = BTreeSet::new();
        for start in &self.order {
            if settled.contains(start.as_str()) {
                continue;
            }
            let mut open: Vec<(&str, Vec<&str>)> =
                vec![(start.as_str(), self.prerequisites(start))];
            while !open.is_empty() {
                let descend = open.last_mut().and_then(|(_, pending)| pending.pop());
                let Some(next) = descend else {
                    if let Some((exhausted, _)) = open.pop() {
                        settled.insert(exhausted);
                    }
                    continue;
                };
                if settled.contains(next) {
                    continue;
                }
                if let Some(from) = open.iter().position(|(node, _)| *node == next) {
                    let mut cycle: Vec<String> = open
                        .iter()
                        .skip(from)
                        .map(|(node, _)| (*node).to_owned())
                        .collect();
                    cycle.push(next.to_owned());
                    return Some(cycle);
                }
                let prerequisites = self.prerequisites(next);
                open.push((next, prerequisites));
            }
        }
        None
    }

    /// Every generator that would derive from this one's output, it first, in
    /// the plan's own order.
    ///
    /// One forward pass over [`UserPlan::order`] is sufficient because that
    /// order is topological — a generator appears after everything it reads —
    /// which is the resolver's claim, is what its cycle refusal guarantees, and
    /// is what [`UserPlan::cycle`] checks before a run walks anything. A
    /// dependency nobody generates resolves to no producer and contributes no
    /// edge, exactly as it contributes none at evaluation.
    #[must_use]
    pub fn cascade(&self, generator: &str) -> Vec<String> {
        let mut marked: Vec<&str> = vec![generator];
        for candidate in &self.order {
            if marked.contains(&candidate.as_str()) {
                continue;
            }
            let derives = self.inputs.get(candidate).is_some_and(|inputs| {
                inputs
                    .values()
                    .filter(|input| input.kind == InputKind::Dependency)
                    .filter_map(|input| self.producer_of(&input.name))
                    .any(|producer| marked.contains(&producer))
            });
            if derives {
                marked.push(candidate);
            }
        }
        self.order
            .iter()
            .filter(|name| marked.contains(&name.as_str()))
            .cloned()
            .collect()
    }
}

/// Who can open one encrypted file, and where it sits.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audience {
    /// The declared users the file serves, in name order.
    pub audience: Vec<String>,
    /// The directory the file sits in, which is what a creation rule covers.
    pub dir: String,
    /// Every age public key the file's data key should be wrapped for.
    pub recipients: Vec<String>,
}

/// `file -> audience`, over every file a declaration places a secret in.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Audiences(pub BTreeMap<String, Audience>);

impl Audiences {
    /// The audience declared for this file, when one is.
    #[must_use]
    pub fn for_file(&self, file: &str) -> Option<&Audience> {
        self.0.get(file)
    }

    /// The first audience whose directory covers this path, in file order.
    ///
    /// What holds a file named through `extraGovernedFiles`: it has no audience
    /// of its own, so the rule covering its directory is both what encrypts
    /// into it and what `fix` re-wraps it to.
    #[must_use]
    pub fn covering_dir(&self, dir: &str) -> Option<&Audience> {
        self.0.values().find(|entry| entry.dir == dir)
    }
}

/// Which files the recipient policy governs, split by where they come from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernedFiles {
    /// What the consumer named through `flake.safix.extraGovernedFiles`.
    pub extra: Vec<String>,
    /// The union, which is what `fix` re-wraps.
    pub managed: Vec<String>,
    /// What the audiences the declarations imply require.
    pub required: Vec<String>,
}

/// `user -> every age key that user can open a file with`.
///
/// Their own and their recovery keys alike. [`Audiences`] answers the same
/// question per file and loses which key is whose; a report that has found a
/// stanza and wants to say who left it there needs the direction this way
/// round.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Recipients(pub BTreeMap<String, Vec<String>>);

impl Recipients {
    /// Which declared users hold any of these keys, and which keys answer to no
    /// declared user at all.
    ///
    /// Both halves come back because a key on a file that no longer answers to
    /// a name is the more alarming of the two and must not be swallowed by
    /// reporting only the names that matched.
    #[must_use]
    pub fn holders_of(&self, keys: &[String]) -> Holders {
        let named = self
            .0
            .iter()
            .filter(|(_, held)| held.iter().any(|key| keys.contains(key)))
            .map(|(user, _)| user.clone())
            .collect();
        let known: Vec<&String> = self.0.values().flatten().collect();
        let orphaned = keys
            .iter()
            .filter(|key| !known.contains(key))
            .cloned()
            .collect();
        Holders { named, orphaned }
    }
}

/// The answer [`Recipients::holders_of`] gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holders {
    /// Declared users holding at least one of the keys, in name order.
    pub named: Vec<String>,
    /// Keys belonging to no declared user, in the order they were given.
    pub orphaned: Vec<String>,
}

/// Which way a mapping's value moves, written as its endpoints.
///
/// Not `import` and `export`: `clan vars export` moves values out of clan and
/// `safix export` moves them in, so a direction spelled with either word means
/// opposite things depending on which tool the reader has in mind. The verbs
/// stay relative because they sit on safix's own command line; the declaration
/// does not, because it is read without a tool in hand to be relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Direction {
    /// clan holds the value and safix receives it. `safix import` acts on these.
    #[serde(rename = "clan-to-safix")]
    ClanToSafix,
    /// safix holds the value and clan receives it. `safix export` acts on these.
    #[serde(rename = "safix-to-clan")]
    SafixToClan,
}

impl Direction {
    /// The direction as it is declared, reported and committed.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClanToSafix => "clan-to-safix",
            Self::SafixToClan => "safix-to-clan",
        }
    }

    /// The verb that acts on mappings of this direction.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::ClanToSafix => "import",
            Self::SafixToClan => "export",
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The clan half of a mapping: the triple clan's own command line takes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClanSide {
    /// The clan machine the var belongs to.
    pub machine: String,
    /// The clan generator that declares the var.
    pub generator: String,
    /// The file that generator declares, named as clan names it.
    pub file: String,
}

/// The safix half of a mapping: a user and a name that user holds.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafixSide {
    /// The `flake.safix.users` entry holding the value.
    pub user: String,
    /// The secret that user holds, as they hold it.
    pub name: String,
}

/// One declared relationship between a clan var and a safix entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    /// The attribute name the mapping was declared under.
    ///
    /// The mapping's own identifier rather than anything derived from an
    /// endpoint: it appears in reports, in commit messages and in refusals, and
    /// a name taken from one side reads wrongly in a sentence about the other.
    pub id: String,
    /// Which way the value moves.
    pub direction: Direction,
    /// The clan endpoint. Nothing at evaluation verified any of it.
    pub clan: ClanSide,
    /// The safix endpoint, which evaluation did verify.
    pub safix: SafixSide,
}

/// Every declared mapping, and the clan they reach.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bridge {
    /// The flake reference clan's own command takes for `--flake`, or none when
    /// the consumer declared no clan.
    #[serde(rename = "clanFlake")]
    pub clan_flake: Option<String>,
    /// Every mapping, in the order the attribute names sort.
    pub mappings: Vec<Mapping>,
}

impl Bridge {
    /// The mappings of one direction, in declaration order.
    pub fn of(&self, direction: Direction) -> impl Iterator<Item = &Mapping> {
        self.mappings
            .iter()
            .filter(move |mapping| mapping.direction == direction)
    }

    /// One mapping by its declared name, whichever direction it runs.
    ///
    /// Found across both directions rather than within the verb's own, so that
    /// naming an export mapping to `import` is refused as a direction mistake
    /// with the mapping named, rather than as an unknown id.
    #[must_use]
    pub fn named(&self, id: &str) -> Option<&Mapping> {
        self.mappings.iter().find(|mapping| mapping.id == id)
    }

    /// Every declared mapping's name, for a refusal that has to list them.
    #[must_use]
    pub fn declared(&self) -> Vec<String> {
        self.mappings
            .iter()
            .map(|mapping| mapping.id.clone())
            .collect()
    }
}

/// How one mapping between a safix entry and a database entry converges.
///
/// Named by its endpoints where a direction is what it is, and by the
/// relationship where it is not: `two-way` and `backup` are not directions. The
/// vocabulary is the one the fleet's own sync declaration already uses for
/// pairs, minus deletion propagation in every mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Mode {
    /// The database converges to safix's value. `sync` overwrites a
    /// database-side edit and reports that it did.
    #[serde(rename = "safix-to-keepassxc")]
    SafixToKeepassxc,
    /// safix converges to the database's value, through the write path a
    /// hand-set value takes.
    #[serde(rename = "keepassxc-to-safix")]
    KeepassxcToSafix,
    /// Whichever side changed since the last agreement wins; both changed is a
    /// conflict that writes nothing.
    #[serde(rename = "two-way")]
    TwoWay,
    /// safix's value is written where the database holds none, and a differing
    /// database value is reported rather than overwritten.
    #[serde(rename = "backup")]
    Backup,
}

impl Mode {
    /// The mode as it is declared and as every report of it names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafixToKeepassxc => "safix-to-keepassxc",
            Self::KeepassxcToSafix => "keepassxc-to-safix",
            Self::TwoWay => "two-way",
            Self::Backup => "backup",
        }
    }

    /// Whether this mode can write safix's side, which is what makes a
    /// generator on that side a second producer.
    ///
    /// The same predicate `modules/flake/safix/keepassxc.nix` refuses on, and
    /// the reason it is here as well is that the two answer different questions
    /// about it: evaluation refuses the declaration, and the runtime decides
    /// which half of a converging run may write.
    #[must_use]
    pub const fn pulls(self) -> bool {
        matches!(self, Self::KeepassxcToSafix | Self::TwoWay)
    }

    /// Whether this mode can write the database's side.
    #[must_use]
    pub const fn pushes(self) -> bool {
        matches!(self, Self::SafixToKeepassxc | Self::TwoWay | Self::Backup)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The database half of a mapping: where the entry sits, and what to call its
/// user.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KdbxSide {
    /// The entry's path under the declared group, as the store's own command
    /// line spells one.
    pub path: String,
    /// The username to set on the entry, or none to leave the field alone.
    pub username: Option<String>,
}

/// One declared relationship between a safix entry and a database entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncMapping {
    /// The attribute name the mapping was declared under, for the reason
    /// [`Mapping::id`] carries one.
    pub id: String,
    /// How it converges.
    pub mode: Mode,
    /// The safix endpoint, which evaluation did verify.
    pub safix: SafixSide,
    /// The database endpoint. Nothing at evaluation verified any of it.
    pub kdbx: KdbxSide,
}

/// The declared mirror: the database, the group, and every mapping under it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keepassxc {
    /// The database `sync` converges against, or none when the consumer named
    /// no database.
    pub database: Option<String>,
    /// The group every mapping's entry path is relative to.
    pub group: String,
    /// Every mapping, in the order the attribute names sort.
    pub mappings: Vec<SyncMapping>,
}

impl Keepassxc {
    /// One mapping by its declared name.
    #[must_use]
    pub fn named(&self, id: &str) -> Option<&SyncMapping> {
        self.mappings.iter().find(|mapping| mapping.id == id)
    }

    /// Every declared mapping's name, for a refusal that has to list them.
    #[must_use]
    pub fn declared(&self) -> Vec<String> {
        self.mappings
            .iter()
            .map(|mapping| mapping.id.clone())
            .collect()
    }

    /// The entry path this mapping names, under the declared group.
    ///
    /// One function rather than a `format!` at each caller: the report, the
    /// refusals and the two reads all name the same entry, and a difference
    /// between them would be a difference with nothing behind it.
    #[must_use]
    pub fn entry_of(&self, mapping: &SyncMapping) -> String {
        format!("{}/{}", self.group, mapping.kdbx.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACEMENT: &str = r#"{
      "ana": {
        "api-token": {
          "file": "secrets/safix/users/ana/secrets.yaml",
          "generator": {
            "dependencies": [], "description": null,
            "files": { "api-token-pub": { "secret": false } },
            "network": false,
            "prompts": {}, "runtimeInputs": ["coreutils"],
            "script": "printf '%s' fixture > $out/api-token",
            "share": false, "validation": null
          },
          "key": "api-token", "origin": "private", "owner": "ana",
          "public": null, "shared": false
        }
      },
      "cy": {}
    }"#;

    #[test]
    fn placements_deserialize_from_the_shape_nix_emits() {
        let placements: Placements = serde_json::from_str(PLACEMENT).unwrap();
        let ana = placements.held_by("ana").unwrap();
        let token = ana.get("api-token").unwrap();
        assert_eq!(token.origin, Origin::Private);
        let generator = token.generator.as_ref().unwrap();
        assert_eq!(generator.runtime_inputs, ["coreutils"]);
        assert!(placements.declares("cy"));

        // The entry a generator hangs off has no slot to say otherwise and is
        // always encrypted; a further output says for itself.
        assert!(generator.is_secret("api-token"));
        assert!(!generator.is_secret("api-token-pub"));
    }

    /// The two readings of "which generator writes this name" agree.
    ///
    /// [`Placements::producer_of`] reads the placements and
    /// [`UserPlan::producer_of`] reads the run plan. They are two projections of
    /// one declaration, and this is what holds them to it: the plan below is built
    /// the way `resolve.nix` builds it — the entry a generator is declared on,
    /// then the names under `files` — so a reading that answered differently for
    /// any name in either direction fails here.
    #[test]
    fn the_two_readings_of_a_producer_agree() {
        let placements: Placements = serde_json::from_str(PLACEMENT).unwrap();
        let plan: GeneratorPlan = serde_json::from_str(
            r#"{
              "ana": {
                "order": ["api-token"],
                "outputs": { "api-token": ["api-token", "api-token-pub"] },
                "inputs": { "api-token": {} }
              },
              "cy": { "order": [], "outputs": {}, "inputs": {} }
            }"#,
        )
        .unwrap();

        for name in ["api-token", "api-token-pub", "nobody-writes-this"] {
            let off_the_plan = plan.for_user("ana").unwrap().producer_of(name);
            let off_the_placements = placements.producer_of("ana", name).map(|(entry, _)| entry);
            assert_eq!(
                off_the_plan, off_the_placements,
                "the two readings disagree about what writes '{name}'"
            );
        }

        // A user who holds nothing, and one the declarations do not name at all.
        assert!(placements.producer_of("cy", "api-token").is_none());
        assert!(placements.producer_of("nobody", "api-token").is_none());
    }

    #[test]
    fn holders_lists_users_with_no_secrets_and_holders_does_not() {
        let placements: Placements = serde_json::from_str(PLACEMENT).unwrap();
        assert_eq!(placements.users().collect::<Vec<_>>(), ["ana", "cy"]);
        assert_eq!(placements.holders().collect::<Vec<_>>(), ["ana"]);
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_dropped() {
        let with_extra =
            PLACEMENT.replace(r#""shared": false"#, r#""shared": false, "mode": "0400""#);
        let refused = serde_json::from_str::<Placements>(&with_extra);
        assert!(refused.is_err());
    }

    #[test]
    fn an_unknown_origin_is_refused() {
        let with_origin = PLACEMENT.replace(r#""origin": "private""#, r#""origin": "inherited""#);
        assert!(serde_json::from_str::<Placements>(&with_origin).is_err());
    }

    const PLAN: &str = r#"{
      "ana": {
        "order": ["base", "derived", "aside", "far"],
        "outputs": {
          "base": ["base", "base-pub"], "derived": ["derived"],
          "aside": ["aside"], "far": ["far"]
        },
        "inputs": {
          "base": { "seed": { "kind": "prompt", "name": "seed" } },
          "derived": { "base-pub": { "kind": "dependency", "name": "base-pub" } },
          "aside": {},
          "far": { "derived": { "kind": "dependency", "name": "derived" } }
        }
      }
    }"#;

    #[test]
    fn the_plan_deserializes_and_resolves_an_output_to_the_generator_writing_it() {
        let plan: GeneratorPlan = serde_json::from_str(PLAN).unwrap();
        let ana = plan.for_user("ana").unwrap();
        assert_eq!(ana.producer_of("base-pub"), Some("base"));
        assert_eq!(ana.producer_of("derived"), Some("derived"));
        assert_eq!(ana.producer_of("nobody-writes-this"), None);
    }

    #[test]
    fn a_cascade_is_transitive_and_stays_in_the_plans_order() {
        let plan: GeneratorPlan = serde_json::from_str(PLAN).unwrap();
        let ana = plan.for_user("ana").unwrap();
        assert_eq!(ana.cascade("base"), ["base", "derived", "far"]);
        assert_eq!(ana.cascade("derived"), ["derived", "far"]);
        assert_eq!(ana.cascade("aside"), ["aside"]);
    }

    /// One user's plan, as the four fields and nothing else.
    fn plan_of(order: &str, outputs: &str, inputs: &str) -> UserPlan {
        let document =
            format!(r#"{{ "order": {order}, "outputs": {outputs}, "inputs": {inputs} }}"#);
        serde_json::from_str(&document).unwrap()
    }

    /// The plan every other test here drives carries no cycle.
    ///
    /// Asserted rather than assumed, because a cycle check answering "yes"
    /// unconditionally would refuse every run and one answering "no"
    /// unconditionally would be the vacuous half of the pair below.
    #[test]
    fn a_topological_order_carries_no_cycle() {
        let plan: GeneratorPlan = serde_json::from_str(PLAN).unwrap();
        assert_eq!(plan.for_user("ana").unwrap().cycle(), None);
    }

    /// Two generators each reading the other's output.
    #[test]
    fn a_cycle_in_the_order_is_reported_as_the_generators_participating_in_it() {
        let plan = plan_of(
            r#"["a", "b"]"#,
            r#"{ "a": ["a"], "b": ["b"] }"#,
            r#"{
              "a": { "b": { "kind": "dependency", "name": "b" } },
              "b": { "a": { "kind": "dependency", "name": "a" } }
            }"#,
        );

        assert_eq!(plan.cycle(), Some(vec!["a".into(), "b".into(), "a".into()]));
    }

    /// A cycle reached only through a second prerequisite.
    ///
    /// `resolve.nix` finds its cycle by following one prerequisite per node,
    /// which is sound inside the set it has already established is stuck and
    /// unsound over a graph in general. Here `a` reads what `b` and `c` write and
    /// `c` reads what `a` writes: following `a`'s first prerequisite alone ends
    /// at `b`, which reads nothing, and no walk from `b` or from `c` returns to
    /// its own start either. The cycle is real and only a walk that backtracks
    /// meets it.
    #[test]
    fn a_cycle_behind_a_second_prerequisite_is_still_reported() {
        let plan = plan_of(
            r#"["a", "b", "c"]"#,
            r#"{ "a": ["a"], "b": ["b"], "c": ["c"] }"#,
            r#"{
              "a": {
                "b": { "kind": "dependency", "name": "b" },
                "c": { "kind": "dependency", "name": "c" }
              },
              "b": {},
              "c": { "a": { "kind": "dependency", "name": "a" } }
            }"#,
        );

        assert_eq!(plan.cycle(), Some(vec!["a".into(), "c".into(), "a".into()]));
    }

    /// A generator reading an output of its own is the cycle of length one.
    ///
    /// `resolve.nix` refuses that declaration by name before it walks the graph,
    /// and drops the self-edge so the walk cannot report it as a cycle nobody
    /// wrote. This runtime cannot tell the two refusals apart from the plan
    /// alone, and does not need to: either way the generator would be waiting on
    /// a value it is the one to write.
    #[test]
    fn a_generator_reading_its_own_output_is_a_cycle_of_length_one() {
        let plan = plan_of(
            r#"["solo"]"#,
            r#"{ "solo": ["solo", "solo-pub"] }"#,
            r#"{ "solo": { "solo-pub": { "kind": "dependency", "name": "solo-pub" } } }"#,
        );

        assert_eq!(plan.cycle(), Some(vec!["solo".into(), "solo".into()]));
    }

    /// A cycle among generators the order leaves out is not this walk's.
    ///
    /// The refusal is about the order a run walks, and a generator absent from
    /// it never runs. `resolve.nix` emits no such plan — it refuses the whole
    /// evaluation when anything is stuck rather than emitting an order for the
    /// rest — so this is about what the check does not claim rather than about a
    /// shape it has to tolerate.
    #[test]
    fn a_cycle_outside_the_order_is_not_reported() {
        let plan = plan_of(
            r#"["ok"]"#,
            r#"{ "ok": ["ok"], "x": ["x"], "y": ["y"] }"#,
            r#"{
              "ok": {},
              "x": { "y": { "kind": "dependency", "name": "y" } },
              "y": { "x": { "kind": "dependency", "name": "x" } }
            }"#,
        );

        assert_eq!(plan.cycle(), None);
    }

    const MIRROR: &str = r#"{
      "database": "/keys/master.kdbx",
      "group": "safix",
      "mappings": [
        {
          "id": "grafana",
          "mode": "safix-to-keepassxc",
          "safix": { "user": "ana", "name": "grafana-password" },
          "kdbx": { "path": "ana/grafana", "username": "ana@example.invalid" }
        },
        {
          "id": "router",
          "mode": "two-way",
          "safix": { "user": "bo", "name": "router" },
          "kdbx": { "path": "bo/router", "username": null }
        }
      ]
    }"#;

    #[test]
    fn the_mirror_deserializes_from_the_shape_nix_emits() {
        let mirror: Keepassxc = serde_json::from_str(MIRROR).unwrap();
        assert_eq!(mirror.database.as_deref(), Some("/keys/master.kdbx"));
        assert_eq!(mirror.declared(), ["grafana", "router"]);

        let grafana = mirror.named("grafana").unwrap();
        assert_eq!(grafana.mode, Mode::SafixToKeepassxc);
        assert_eq!(mirror.entry_of(grafana), "safix/ana/grafana");
        assert_eq!(
            grafana.kdbx.username.as_deref(),
            Some("ana@example.invalid")
        );

        let router = mirror.named("router").unwrap();
        assert_eq!(router.mode, Mode::TwoWay);
        assert!(router.kdbx.username.is_none());
        assert!(mirror.named("absent").is_none());
    }

    /// A consumer who has never heard of this evaluates exactly this.
    #[test]
    fn a_mirror_with_no_database_and_no_mapping_is_a_shape_this_reads() {
        let mirror: Keepassxc =
            serde_json::from_str(r#"{"database": null, "group": "safix", "mappings": []}"#)
                .unwrap();
        assert!(mirror.database.is_none());
        assert!(mirror.mappings.is_empty());
        assert!(mirror.declared().is_empty());
    }

    #[test]
    fn an_unknown_mode_is_refused_rather_than_read_as_another() {
        let with_mode = MIRROR.replace(r#""mode": "two-way""#, r#""mode": "push""#);
        assert!(serde_json::from_str::<Keepassxc>(&with_mode).is_err());
    }

    #[test]
    fn an_unknown_field_on_a_mapping_is_refused_rather_than_dropped() {
        let with_extra = MIRROR.replace(
            r#""path": "bo/router""#,
            r#""path": "bo/router", "url": "x""#,
        );
        assert!(serde_json::from_str::<Keepassxc>(&with_extra).is_err());
    }

    /// Which side each mode may write, asserted over every mode rather than over
    /// the ones a test happened to name.
    #[test]
    fn each_mode_writes_the_sides_its_name_says() {
        let modes = [
            (Mode::SafixToKeepassxc, false, true),
            (Mode::KeepassxcToSafix, true, false),
            (Mode::TwoWay, true, true),
            (Mode::Backup, false, true),
        ];
        for (mode, pulls, pushes) in modes {
            assert_eq!(mode.pulls(), pulls, "{mode} pulls");
            assert_eq!(mode.pushes(), pushes, "{mode} pushes");
        }
    }

    #[test]
    fn holders_of_separates_named_users_from_orphaned_keys() {
        let recipients: Recipients = serde_json::from_str(
            r#"{"ana": ["age1a", "age1escrow"], "bo": ["age1b"], "cy": ["age1c"]}"#,
        )
        .unwrap();
        let found = recipients.holders_of(&["age1b".into(), "age1stray".into()]);
        assert_eq!(found.named, ["bo"]);
        assert_eq!(found.orphaned, ["age1stray"]);
    }
}

/// The character the nix half joins a shared audience's members with when it
/// names their directory.
///
/// Not used to build anything here — the directory arrives already built, in
/// [`Audience::dir`]. It is named because [`Audiences::covering_dir`] resolves a
/// file to an audience *by* that directory, which is sound only while the join
/// is injective, and the property test below is where this crate states the
/// assumption it is relying on rather than inheriting it silently.
pub const AUDIENCE_SEPARATOR: &str = ",";

/// The markers the nix half writes an audience element with when the element is a
/// reference resolved through a declaration rather than a subject named in place:
/// a group, and the owner a machine records.
///
/// Named here for the same reason as the separator, and load-bearing for the same
/// claim. A directory is joined from elements, so the alphabet injectivity rests
/// on is the marked forms as well as the bare names, and the property test below
/// is where this crate states that rather than inheriting it.
pub const AUDIENCE_MARKERS: [&str; 2] = ["@", "@~"];

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::AUDIENCE_SEPARATOR;

    use super::AUDIENCE_MARKERS;

    /// The alphabet `resolve.nix` admits a user, anchor or secret name from.
    const NAME: &str = "[a-z0-9][a-z0-9_-]{0,7}";

    /// Every form an audience element takes: a subject named in place, a group,
    /// or the owner a machine records. The markers are part of the alphabet a
    /// directory is joined from, so the property below has to be over elements
    /// rather than over names.
    fn element() -> impl Strategy<Value = String> {
        prop_oneof![
            NAME.prop_map(|name| name),
            NAME.prop_map(|name| format!("{}{name}", AUDIENCE_MARKERS[0])),
            NAME.prop_map(|name| format!("{}{name}", AUDIENCE_MARKERS[1])),
        ]
    }

    /// How a shared audience's directory is named: its members, sorted, joined.
    fn directory_of(audience: &[String], separator: &str) -> String {
        let mut sorted = audience.to_vec();
        sorted.sort();
        sorted.dedup();
        sorted.join(separator)
    }

    proptest! {
        /// Two distinct audiences never reach one directory.
        ///
        /// This is what `covering_dir` needs to be true: a file with no audience
        /// of its own is held to the rule covering its directory, and two
        /// audiences sharing a directory would be one rule over two audiences'
        /// secrets — a wider readership than either was declared with.
        #[test]
        fn distinct_audiences_reach_distinct_directories(
            left in proptest::collection::vec(element(), 1..4),
            right in proptest::collection::vec(element(), 1..4),
        ) {
            let mut left_set = left.clone();
            left_set.sort();
            left_set.dedup();
            let mut right_set = right.clone();
            right_set.sort();
            right_set.dedup();

            let collides = directory_of(&left, AUDIENCE_SEPARATOR)
                == directory_of(&right, AUDIENCE_SEPARATOR);
            prop_assert_eq!(collides, left_set == right_set);
        }

    }

    /// The separator is what makes the property above true, not the names.
    ///
    /// A separator drawn from the alphabet a name is drawn from is forgeable
    /// across an element boundary: two audiences that share no member list reach
    /// one directory, because the character that was supposed to separate them
    /// can sit inside a name. That is why `resolve.nix` chooses a separator
    /// outside the alphabet rather than refusing names that contain the chosen
    /// one — no refusal restores injectivity once the character is forgeable.
    /// A marker inside the alphabet collapses a resolved reference onto a subject
    /// of that name.
    ///
    /// The same argument the separator rests on, over the other half of the
    /// alphabet: `resolve.nix` marks a group audience and an owner reference
    /// because the readership those name is a declaration rather than the list in
    /// the path, and it draws the markers from outside the name alphabet because
    /// a marker a name could carry would put the group `ops` and the person
    /// `@ops` — or, with a marker of `x`, the group `ops` and the person `xops` —
    /// in one directory, under one rule.
    #[test]
    fn a_marker_inside_the_alphabet_collapses_a_reference_onto_a_subject() {
        let group = ["ana".to_owned(), "xops".to_owned()];
        let person = ["ana".to_owned(), "ops".to_owned()];

        let marked = |audience: &[String], marker: &str| {
            let mut marked = audience.to_vec();
            if let Some(last) = marked.last_mut() {
                *last = format!("{marker}{}", last.trim_start_matches('x'));
            }
            directory_of(&marked, AUDIENCE_SEPARATOR)
        };

        // A marker of `x` is inside the alphabet, so marking `ops` reaches the
        // directory `ana,xops` that a person named `xops` already has.
        assert_eq!(
            marked(&person, "x"),
            directory_of(&group, AUDIENCE_SEPARATOR)
        );

        // The markers the nix half uses are outside it, so nothing does.
        for marker in AUDIENCE_MARKERS {
            assert_ne!(
                marked(&person, marker),
                directory_of(&group, AUDIENCE_SEPARATOR)
            );
        }
    }

    #[test]
    fn a_separator_inside_the_alphabet_is_forgeable_across_an_element_boundary() {
        let pair = ["ana".to_owned(), "bo-cy".to_owned()];
        let other = ["ana-bo".to_owned(), "cy".to_owned()];

        assert_eq!(directory_of(&pair, "-"), directory_of(&other, "-"));
        assert_ne!(
            directory_of(&pair, AUDIENCE_SEPARATOR),
            directory_of(&other, AUDIENCE_SEPARATOR)
        );
    }
}
