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
    /// The explicitly declared ciphertext format.
    #[serde(default)]
    pub format: crate::ciphertext::Format,
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
    /// Where this entry's definition record lives: repository-relative under
    /// `flake.safix.storage.generatorRecords` with no vault declared,
    /// vault-rooted-relative and opaque when one is.
    ///
    /// Always set, because the resolver computes it the way it computes
    /// [`Placement::file`] and [`Placement::public`] — from a configured
    /// root rather than from a literal this crate could spell too. That is
    /// what leaves one implementation of the layout, and it is why
    /// [`crate::definition::record_path`] neither derives a path nor, in
    /// vault mode, a hash.
    #[serde(rename = "definitionRecord")]
    pub definition_record: String,
    /// The readable, unhashed relative path [`Placement::file`] would carry
    /// with no vault declared; `null` when no vault is declared.
    ///
    /// Carried alongside the opaque [`Placement::file`] so that a migration
    /// or a rollback can enumerate both forms of one document without the
    /// runtime computing either hash itself — nothing in `crates/safix-core`
    /// hashes, per design V14.
    #[serde(rename = "logicalFile")]
    pub logical_file: Option<String>,
    /// The readable, unhashed in-document key [`Placement::key`] would carry
    /// with no vault declared; `null` when no vault is declared.
    #[serde(rename = "logicalKey")]
    pub logical_key: Option<String>,
    /// The readable, unhashed relative public path [`Placement::public`]
    /// would carry with no vault declared; `null` when either no vault is
    /// declared or this entry has no public output.
    #[serde(rename = "logicalPublic")]
    pub logical_public: Option<String>,
    /// The readable, unhashed relative record path
    /// [`Placement::definition_record`] would carry with no vault declared;
    /// `null` when no vault is declared.
    ///
    /// Carried for the same reason [`Placement::logical_file`] is: a
    /// migration enumerates both forms of one record without this crate
    /// computing either hash.
    #[serde(rename = "logicalRecord")]
    pub logical_record: Option<String>,
    /// Where this entry's created/updated stamp lives: repository-relative
    /// under `flake.safix.storage.generatorRecords` with no vault declared,
    /// vault-rooted-relative and opaque when one is.
    ///
    /// Always set, for the reason [`Placement::definition_record`] is: the
    /// resolver owns the layout, and the readable form carries a
    /// `generatorRecords` root this crate never sees. The two records sit in
    /// one tree because they are one kind of thing — plaintext bookkeeping
    /// about a value, carrying no value — and this one differs from the
    /// definition record's path by a `.stamps` suffix no declared name can
    /// carry.
    #[serde(rename = "stampRecord")]
    pub stamp_record: String,
    /// The readable, unhashed relative stamp path [`Placement::stamp_record`]
    /// would carry with no vault declared; `null` when no vault is declared.
    ///
    /// Carried for the same reason [`Placement::logical_record`] is, and it is
    /// what gates a stamp's relocation: a migration enumerates both forms of
    /// one stamp without this crate computing either hash.
    #[serde(rename = "logicalStamp")]
    pub logical_stamp: Option<String>,
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
    /// Whether recipient policy covers the directory's shared keyed YAML files.
    #[serde(default)]
    pub legacy: bool,
    /// The declared users the file serves, in name order.
    pub audience: Vec<String>,
    /// The directory the file sits in, which is what a creation rule covers.
    pub dir: String,
    /// Every age public key the file's data key should be wrapped for.
    pub recipients: Vec<String>,
    /// The container whose recipient metadata is inspected.
    #[serde(default)]
    pub format: crate::ciphertext::Format,
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

/// `subject -> every age key that subject holds of its own`.
///
/// A person's own recipient and their recovery keys, a machine's host identity, an
/// organization's custody. [`Audiences`] answers the same question per file and
/// loses which key is whose; a report that has found a stanza and wants to say who
/// left it there needs the direction this way round.
///
/// A person's escrow consent is not folded into their row, because the keys are
/// the organization's: that is what lets a withdrawn consent be reported as the
/// organization's access rather than as the person's own.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Recipients(pub BTreeMap<String, Vec<String>>);

impl Recipients {
    /// Which declared subjects hold any of these keys, and which keys answer to no
    /// declared subject at all.
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
    /// Declared subjects holding at least one of the keys, in name order.
    pub named: Vec<String>,
    /// Keys belonging to no declared subject, in the order they were given.
    pub orphaned: Vec<String>,
}

/// One group, as a delegation reads it: who is in it, and whose it is.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelegatedGroup {
    /// The subjects the declaration names, as it names them.
    pub members: Vec<String>,
    /// Every organization whose silo declarations cover this group, in name
    /// order.
    ///
    /// Empty is the ordinary case and the whole of the unmanaged default: a group
    /// no silo set names is nobody's, and an edit to it is refused by nothing.
    pub organizations: Vec<String>,
}

/// Who may scaffold for whom, and over which groups.
///
/// Not part of the audience algebra and deliberately kept out of it: no value
/// here places a key in any audience, so a fleet that declares a delegation
/// derives the byte-identical tree. What it decides is which acting identity the
/// scaffolding verbs accept, and the acting identity is the one the commit those
/// verbs make would carry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delegation {
    /// `organization -> the people it declares as managers`, in name order.
    pub managers: BTreeMap<String, Vec<String>>,
    /// `person -> the organization their own record consents to be managed by`.
    ///
    /// Only the people who declare one, so an unmanaged fleet is an empty map
    /// rather than a map of nulls.
    #[serde(rename = "managedBy")]
    pub managed_by: BTreeMap<String, String>,
    /// `group -> its membership and the organizations covering it`.
    pub groups: BTreeMap<String, DelegatedGroup>,
    /// Every declared subject's name, in name order.
    ///
    /// The name space a group edit is checked against, which travels here because
    /// the verb that reads a delegation is the verb that needs it: a membership
    /// naming a subject the fleet does not declare is refused at evaluation, and
    /// a verb that wrote one would have committed a tree that no longer resolves.
    pub subjects: Vec<String>,
}

impl Delegation {
    /// The organization whose managers scaffold for this person, if one does.
    #[must_use]
    pub fn managing(&self, person: &str) -> Option<&str> {
        self.managed_by.get(person).map(String::as_str)
    }

    /// Whether this person is among an organization's managers.
    #[must_use]
    pub fn is_manager(&self, organization: &str, person: &str) -> bool {
        self.managers
            .get(organization)
            .is_some_and(|managers| managers.iter().any(|manager| manager == person))
    }

    /// The managers one organization declares, empty for one that declares none.
    #[must_use]
    pub fn managers_of(&self, organization: &str) -> &[String] {
        self.managers
            .get(organization)
            .map_or(&[], |managers| managers.as_slice())
    }

    /// One group's record, if the fleet declares it.
    #[must_use]
    pub fn group(&self, group: &str) -> Option<&DelegatedGroup> {
        self.groups.get(group)
    }

    /// Every group the fleet declares, in name order.
    pub fn groups(&self) -> impl Iterator<Item = &str> {
        self.groups.keys().map(String::as_str)
    }

    /// Whether the fleet declares a subject of this name.
    #[must_use]
    pub fn declares_subject(&self, subject: &str) -> bool {
        self.subjects.iter().any(|declared| declared == subject)
    }
}

/// Which way a mapping's value moves, written as its endpoints.
///
/// Not `import` and `export`: `clan vars export` moves values out of clan and
/// a safix-to-clan mapping's convergence moves a value the opposite way, so a
/// direction spelled with either word means opposite things depending on
/// which tool the reader has in mind. `two-way` names neither a source nor a
/// destination, because the value may originate on either side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Direction {
    /// clan holds the value and safix receives it.
    #[serde(rename = "clan-to-safix")]
    ClanToSafix,
    /// safix holds the value and clan receives it.
    #[serde(rename = "safix-to-clan")]
    SafixToClan,
    /// Neither side is a fixed source or destination: the value converges
    /// toward whichever side changed since the last recorded agreement.
    #[serde(rename = "two-way")]
    TwoWay,
}

impl Direction {
    /// The direction as it is declared, reported and committed.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClanToSafix => "clan-to-safix",
            Self::SafixToClan => "safix-to-clan",
            Self::TwoWay => "two-way",
        }
    }

    /// The verb that acts on mappings of this direction.
    ///
    /// `two-way` has no verb of its own to report: [`crate::bridge::commit_subject`]
    /// is never called for a two-way mapping, which builds its own
    /// direction-neutral commit subject instead. The arm exists only because
    /// the match must be exhaustive.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::ClanToSafix => "import",
            Self::SafixToClan => "export",
            Self::TwoWay => "sync",
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Which of clan's own placements a var is declared under.
///
/// clan's placement is a three-way sum — `Shared`, `PerMachine`, `PerExport`
/// — of which `clan vars get`/`set` can resolve only the first two through a
/// machine: `get_machine_generators` never constructs a `PerExport`
/// placement for any machine it is asked about, so a `PerExport` var is
/// unreachable through the two contracts this runtime uses regardless of
/// what a mapping declared. Only the two placements safix can actually
/// address are represented here.
///
/// Named apart from [`Placement`], which is safix's own per-entry file/key/
/// public shape: the two are unrelated concepts that happen to share a
/// domain (secret custody) but not a vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ClanPlacement {
    /// One generator declares the var once for the whole fleet. The machine
    /// that addresses it on clan's command line is discovered at run time
    /// rather than declared.
    #[serde(rename = "shared")]
    Shared,
    /// The var belongs to exactly one machine, named by [`ClanSide::machine`].
    #[serde(rename = "per-machine")]
    PerMachine,
}

/// The clan half of a mapping: the triple clan's own command line takes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClanSide {
    /// Which of clan's own placements the var is declared under.
    pub placement: ClanPlacement,
    /// The clan machine the var belongs to.
    ///
    /// `Some` when [`Self::placement`] is [`ClanPlacement::PerMachine`], and
    /// `None` when it is [`ClanPlacement::Shared`]: a shared var is not owned
    /// by any one machine, so evaluation refuses a mapping that declares one
    /// anyway.
    pub machine: Option<String>,
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

/// One field's declared value, and where it came from.
///
/// A [`FieldValue::Literal`] was written in a declaration, which is evaluated
/// into a world-readable nix store, so it is not a secret and may travel any
/// channel a target offers. A [`FieldValue::Entry`] names another entry of the
/// mapping's own person, is read at run time rather than at evaluation, and is
/// therefore a secret value — which is what decides the channels it may
/// travel.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// A value written in the declaration itself.
    Literal(String),
    /// A value read out of another entry the same person holds.
    Entry {
        /// The entry's name, as that person holds it.
        entry: String,
    },
}

/// What a mapping's far side carries beside its value.
///
/// The other half of `modules/flake/safix/fields.nix`'s `fields` submodule,
/// coupled to it the way this module's own header describes: a field added
/// there is a schema change, and this struct denying unknown fields is what
/// makes a half-migrated declaration an evaluation failure rather than a run
/// that silently wrote less than it said.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    /// The username to set, or none to leave the field alone.
    pub username: Option<FieldValue>,
    /// The address the credential is used at, or none.
    pub url: Option<FieldValue>,
    /// The entry's note, or none.
    pub notes: Option<FieldValue>,
    /// The tags to set, empty to leave them alone.
    #[serde(default)]
    pub tags: Vec<FieldValue>,
}

impl Fields {
    /// Whether the mapping says anything at all about a field.
    #[must_use]
    pub fn declares(&self) -> bool {
        self.username.is_some()
            || self.url.is_some()
            || self.notes.is_some()
            || !self.tags.is_empty()
    }

    /// The declared fields, paired with the name every report and every diff
    /// names them by, in `fields.nix`'s own `fieldNames` order.
    ///
    /// `tags` yields one pair per element, because a tag list is written as a
    /// list and a refusal about it names the field rather than the element.
    #[must_use]
    pub fn named(&self) -> Vec<(&'static str, &FieldValue)> {
        let mut named = Vec::new();
        if let Some(value) = self.username.as_ref() {
            named.push(("username", value));
        }
        if let Some(value) = self.url.as_ref() {
            named.push(("url", value));
        }
        if let Some(value) = self.notes.as_ref() {
            named.push(("notes", value));
        }
        for value in &self.tags {
            named.push(("tags", value));
        }
        named
    }
}

/// The database half of a mapping: where the entry sits, and what it carries
/// beside the value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KdbxSide {
    /// The entry's path under the declared group, as the store's own command
    /// line spells one.
    pub path: String,
    /// What the entry carries beside its value.
    pub fields: Fields,
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

/// A `YubiKey` challenge-response slot a database's own composite key requires
/// to open, and the card serial that disambiguates it when more than one is
/// connected.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Yubikey {
    /// The slot number, as keepassxc-cli's `-y` flag takes it.
    pub slot: String,
    /// The card's serial number, or none to accept whichever card answers.
    pub serial: Option<String>,
}

/// The declared mirror: the database, the group, every mapping under it, and
/// the composite-key factors the database itself requires to open.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keepassxc {
    /// The database `sync` converges against, or none when the consumer named
    /// no database.
    pub database: Option<String>,
    /// The group every mapping's entry path is relative to.
    pub group: String,
    /// A `YubiKey` challenge-response slot the database requires to open, or
    /// none when the database opens on its password alone.
    pub yubikey: Option<Yubikey>,
    /// A key file the database requires to open, as an absolute path on the
    /// machine the verb runs on, or none when the database opens on its
    /// password alone.
    #[serde(rename = "keyFile")]
    pub key_file: Option<String>,
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

/// How one mapping between a safix entry and a `pass` entry converges.
///
/// The same four modes [`Mode`] carries, spelled with this target's own word:
/// a declaration is read by someone with no tool in hand to be relative to, so
/// the endpoints are named rather than a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum PassMode {
    /// The store converges to safix's value. `sync` overwrites a store-side
    /// edit and reports that it did.
    #[serde(rename = "safix-to-pass")]
    SafixToPass,
    /// safix converges to the store's value, through the write path a hand-set
    /// value takes.
    #[serde(rename = "pass-to-safix")]
    PassToSafix,
    /// Whichever side changed since the last agreement wins; both changed is a
    /// conflict that writes nothing.
    #[serde(rename = "two-way")]
    TwoWay,
    /// safix's value is written where the store holds no entry, and a differing
    /// store value is reported rather than overwritten.
    #[serde(rename = "backup")]
    Backup,
}

impl PassMode {
    /// The mode as it is declared and as every report of it names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafixToPass => "safix-to-pass",
            Self::PassToSafix => "pass-to-safix",
            Self::TwoWay => "two-way",
            Self::Backup => "backup",
        }
    }

    /// Whether this mode can write safix's side, which is what makes a
    /// generator on that side a second producer.
    ///
    /// The same predicate `modules/flake/safix/pass.nix` refuses on, here as
    /// well for the reason [`Mode::pulls`] is: evaluation refuses the
    /// declaration, and the runtime decides which half of a converging run may
    /// write.
    #[must_use]
    pub const fn pulls(self) -> bool {
        matches!(self, Self::PassToSafix | Self::TwoWay)
    }

    /// Whether this mode can write the store's side.
    #[must_use]
    pub const fn pushes(self) -> bool {
        matches!(self, Self::SafixToPass | Self::TwoWay | Self::Backup)
    }
}

impl std::fmt::Display for PassMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The store half of a mapping: which entry, and what the record carries
/// beside the value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassSide {
    /// The entry path inside the declared store, as `pass` itself spells one:
    /// no leading slash and no `.gpg` suffix.
    pub path: String,
    /// What the record carries beside its value. All four are carried, because
    /// the whole body crosses on standard input.
    pub fields: Fields,
}

/// One declared relationship between a safix entry and a `pass` entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassMapping {
    /// The attribute name the mapping was declared under, for the reason
    /// [`Mapping::id`] carries one.
    pub id: String,
    /// How it converges.
    pub mode: PassMode,
    /// The safix endpoint, which evaluation did verify.
    pub safix: SafixSide,
    /// The store endpoint. Nothing at evaluation verified any of it.
    pub pass: PassSide,
}

/// The declared `pass` store and every mapping into it.
///
/// No group, where [`Keepassxc`] has one: a `pass` path is already absolute
/// within the store.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pass {
    /// The store root, as the declaration names it — a leading `~` is expanded
    /// by the runtime, because evaluation has no home to expand against.
    pub store: String,
    /// Every mapping, in the order the attribute names sort.
    pub mappings: Vec<PassMapping>,
}

impl Pass {
    /// One mapping by its declared name.
    #[must_use]
    pub fn named(&self, id: &str) -> Option<&PassMapping> {
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

    /// The entry path this mapping names, unchanged.
    ///
    /// No group prefix, where [`Keepassxc::entry_of`] has one: a `pass` path is
    /// absolute within the store and there is no group option for it to be
    /// relative to. The function exists anyway, so that the report, the
    /// refusals and the reads all name the entry through one place — a
    /// difference between them would be a difference with nothing behind it.
    #[must_use]
    pub fn entry_of(&self, mapping: &PassMapping) -> String {
        mapping.pass.path.clone()
    }
}

/// How one mapping between a safix entry and a Bitwarden item converges.
///
/// The same four modes [`Mode`] carries, spelled with this target's own word,
/// for [`PassMode`]'s reason: a declaration is read by someone with no tool in
/// hand to be relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BitwardenMode {
    /// The vault converges to safix's value. `sync` overwrites a vault-side
    /// edit and reports that it did.
    #[serde(rename = "safix-to-bitwarden")]
    SafixToBitwarden,
    /// safix converges to the vault's value, through the write path a hand-set
    /// value takes.
    #[serde(rename = "bitwarden-to-safix")]
    BitwardenToSafix,
    /// Whichever side changed since the last agreement wins; both changed is a
    /// conflict that writes nothing.
    #[serde(rename = "two-way")]
    TwoWay,
    /// safix's value is written where the vault holds no such item, and a
    /// differing vault value is reported rather than overwritten.
    #[serde(rename = "backup")]
    Backup,
}

impl BitwardenMode {
    /// The mode as it is declared and as every report of it names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafixToBitwarden => "safix-to-bitwarden",
            Self::BitwardenToSafix => "bitwarden-to-safix",
            Self::TwoWay => "two-way",
            Self::Backup => "backup",
        }
    }

    /// Whether this mode can write safix's side, which is what makes a
    /// generator on that side a second producer.
    ///
    /// The same predicate `modules/flake/safix/bitwarden.nix` refuses on, here
    /// as well for the reason [`Mode::pulls`] is: evaluation refuses the
    /// declaration, and the runtime decides which half of a converging run may
    /// write.
    #[must_use]
    pub const fn pulls(self) -> bool {
        matches!(self, Self::BitwardenToSafix | Self::TwoWay)
    }

    /// Whether this mode can write the vault's side.
    #[must_use]
    pub const fn pushes(self) -> bool {
        matches!(self, Self::SafixToBitwarden | Self::TwoWay | Self::Backup)
    }
}

impl std::fmt::Display for BitwardenMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The vault half of a mapping: where the item sits, and what it carries
/// beside the value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BitwardenSide {
    /// The folder the item sits in, or none for the vault's root.
    pub folder: Option<String>,
    /// The item's name, as the person holding it sees it.
    ///
    /// Never the vault's own item identifier: an identifier is opaque to
    /// review and is reissued by a restore of the vault, which
    /// `modules/flake/safix/bitwarden.nix` records as the reason this target
    /// addresses by folder and name.
    pub item: String,
    /// What the item carries beside its value. Three of the four are carried,
    /// because this vault has no tag concept at all.
    pub fields: Fields,
}

/// One declared relationship between a safix entry and a Bitwarden item.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BitwardenMapping {
    /// The attribute name the mapping was declared under, for the reason
    /// [`Mapping::id`] carries one.
    pub id: String,
    /// How it converges.
    pub mode: BitwardenMode,
    /// The safix endpoint, which evaluation did verify.
    pub safix: SafixSide,
    /// The vault endpoint. Nothing at evaluation verified any of it.
    pub bitwarden: BitwardenSide,
}

/// The declared vault and every mapping into it.
///
/// No group and no store root, where [`Keepassxc`] and [`Pass`] have one: the
/// far side is a network service, and the one thing declared about it is which
/// server — optionally, because a client an operator logged into already holds
/// that configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bitwarden {
    /// The server every mapping is converged against, or none for whichever
    /// one the operator's own client is configured against.
    pub server: Option<String>,
    /// Every mapping, in the order the attribute names sort.
    pub mappings: Vec<BitwardenMapping>,
}

impl Bitwarden {
    /// One mapping by its declared name.
    #[must_use]
    pub fn named(&self, id: &str) -> Option<&BitwardenMapping> {
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

    /// The address this mapping names: `folder/item`, or the item alone when it
    /// lives in the vault's root.
    ///
    /// The same two branches `modules/flake/safix/bitwarden.nix`'s
    /// `itemPathOf` has, and the address every report and every refusal names.
    /// It is not an argument any command takes: the transport resolves it to
    /// the vault's own item identifier at run time.
    #[must_use]
    pub fn address_of(&self, mapping: &BitwardenMapping) -> String {
        match mapping.bitwarden.folder.as_deref() {
            Some(folder) => format!("{folder}/{}", mapping.bitwarden.item),
            None => mapping.bitwarden.item.clone(),
        }
    }
}

/// How one mapping between a safix entry and a 1Password item converges.
///
/// The same four modes [`Mode`] carries, spelled with this target's own word.
/// `1password` and never `op`: one spelling per target, and `op` is the name of
/// the program the runtime invokes rather than of the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum OnePasswordMode {
    /// The item converges to safix's value. `sync` overwrites an item-side edit
    /// and reports that it did.
    #[serde(rename = "safix-to-1password")]
    SafixToOnePassword,
    /// safix converges to the item's value, through the write path a hand-set
    /// value takes.
    #[serde(rename = "1password-to-safix")]
    OnePasswordToSafix,
    /// Whichever side changed since the last agreement wins; both changed is a
    /// conflict that writes nothing.
    #[serde(rename = "two-way")]
    TwoWay,
    /// safix's value is written where the item does not exist or holds none,
    /// and a differing item value is reported rather than overwritten.
    #[serde(rename = "backup")]
    Backup,
}

impl OnePasswordMode {
    /// The mode as it is declared and as every report of it names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafixToOnePassword => "safix-to-1password",
            Self::OnePasswordToSafix => "1password-to-safix",
            Self::TwoWay => "two-way",
            Self::Backup => "backup",
        }
    }

    /// Whether this mode can write safix's side, which is what makes a
    /// generator on that side a second producer.
    ///
    /// The same predicate `modules/flake/safix/onepassword.nix` refuses on,
    /// here as well for the reason [`Mode::pulls`] is: evaluation refuses the
    /// declaration, and the runtime decides which half of a converging run may
    /// write.
    #[must_use]
    pub const fn pulls(self) -> bool {
        matches!(self, Self::OnePasswordToSafix | Self::TwoWay)
    }

    /// Whether this mode can write the item's side.
    #[must_use]
    pub const fn pushes(self) -> bool {
        matches!(self, Self::SafixToOnePassword | Self::TwoWay | Self::Backup)
    }
}

impl std::fmt::Display for OnePasswordMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The 1Password half of a mapping: which vault, which item, and what the item
/// carries beside the value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpSide {
    /// The vault the item lives in, by name. Required rather than defaulted: a
    /// service account cannot reach a built-in vault at all.
    pub vault: String,
    /// The item's title inside that vault.
    pub item: String,
    /// What the item carries beside its value. All four are carried, because
    /// the whole item crosses as one JSON object on standard input.
    pub fields: Fields,
}

/// One declared relationship between a safix entry and a 1Password item.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnePasswordMapping {
    /// The attribute name the mapping was declared under, for the reason
    /// [`Mapping::id`] carries one.
    pub id: String,
    /// How it converges.
    pub mode: OnePasswordMode,
    /// The safix endpoint, which evaluation did verify.
    pub safix: SafixSide,
    /// The 1Password endpoint. Nothing at evaluation verified any of it: both
    /// halves are content of a remote service.
    pub onepassword: OpSide,
}

/// The declared 1Password mirror and every mapping into it.
///
/// No database and no store root, where [`Keepassxc`] and [`Pass`] have one:
/// the far side is a service, and what addresses it is the account the
/// operator's own session resolves.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnePassword {
    /// The account shorthand every invocation names, or none to let `op`
    /// resolve its own — which a service-account token does implicitly, so an
    /// undeclared account is a working configuration rather than a missing one.
    pub account: Option<String>,
    /// Every mapping, in the order the attribute names sort.
    pub mappings: Vec<OnePasswordMapping>,
}

impl OnePassword {
    /// One mapping by its declared name.
    #[must_use]
    pub fn named(&self, id: &str) -> Option<&OnePasswordMapping> {
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

    /// The vault-qualified item this mapping names, the counterpart of
    /// [`Keepassxc::entry_of`].
    ///
    /// The vault is part of the address rather than decoration: two mappings
    /// naming one item name in two vaults name two items, and an identity that
    /// dropped the vault would call them a collision.
    #[must_use]
    pub fn item_of(&self, mapping: &OnePasswordMapping) -> String {
        format!("{}/{}", mapping.onepassword.vault, mapping.onepassword.item)
    }
}

/// One declared machine, as `flake.safix.lib.subjects.machines` projects it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineSubject {
    /// The `flake.safix.users` or `flake.safix.organizations` entry this
    /// machine belongs to, or none.
    pub owner: Option<String>,
    /// The tags this machine carries.
    pub tags: Vec<String>,
}

/// One declared service, as `flake.safix.lib.subjects.services` projects it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceSubject {
    /// The machines this service runs on.
    pub machines: Vec<String>,
    /// The `flake.safix.users` or `flake.safix.organizations` entry this
    /// service belongs to, or none.
    pub owner: Option<String>,
    /// The unix account its landed entries belong to, or none.
    pub user: Option<String>,
    /// The unix group its landed entries belong to, or none.
    pub group: Option<String>,
}

/// One declared group, as `flake.safix.lib.subjects.groups` projects it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupSubject {
    /// The subjects this group's audience expands to.
    pub members: Vec<String>,
}

/// The subject records themselves, as the consumption modules read them —
/// see `flake.safix.lib.subjects` in `modules/flake/safix/default.nix`.
///
/// `safix upload` is this crate's one reader: it looks a machine name up in
/// [`Subjects::machines`] to tell a declared machine apart from an
/// undeclared name or a person's, without consulting placements or
/// recipients for that distinction — see `crates/safix-core/src/upload.rs`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subjects {
    /// Every declared machine, by name.
    pub machines: BTreeMap<String, MachineSubject>,
    /// Every declared service, by name.
    pub services: BTreeMap<String, ServiceSubject>,
    /// Every declared group, by name.
    pub groups: BTreeMap<String, GroupSubject>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACEMENT: &str = r#"{
      "alice": {
        "api-token": {
          "file": "secrets/safix/users/alice/secrets.yaml",
          "generator": {
            "dependencies": [], "description": null,
            "files": { "api-token-pub": { "secret": false } },
            "network": false,
            "prompts": {}, "runtimeInputs": ["coreutils"],
            "script": "printf '%s' fixture > $out/api-token",
            "share": false, "validation": null
          },
          "key": "api-token", "origin": "private", "owner": "alice",
          "public": null, "shared": false,
          "definitionRecord": "state/safix/definitions/alice/api-token",
          "logicalFile": null,
          "logicalKey": null, "logicalPublic": null, "logicalRecord": null,
          "stampRecord": "state/safix/definitions/alice/api-token.stamps",
          "logicalStamp": null
        }
      },
      "carol": {}
    }"#;

    #[test]
    fn placements_deserialize_from_the_shape_nix_emits() {
        let placements: Placements = serde_json::from_str(PLACEMENT).unwrap();
        let alice = placements.held_by("alice").unwrap();
        let token = alice.get("api-token").unwrap();
        assert_eq!(token.origin, Origin::Private);
        let generator = token.generator.as_ref().unwrap();
        assert_eq!(generator.runtime_inputs, ["coreutils"]);
        assert!(placements.declares("carol"));

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
              "alice": {
                "order": ["api-token"],
                "outputs": { "api-token": ["api-token", "api-token-pub"] },
                "inputs": { "api-token": {} }
              },
              "carol": { "order": [], "outputs": {}, "inputs": {} }
            }"#,
        )
        .unwrap();

        for name in ["api-token", "api-token-pub", "nobody-writes-this"] {
            let off_the_plan = plan.for_user("alice").unwrap().producer_of(name);
            let off_the_placements = placements
                .producer_of("alice", name)
                .map(|(entry, _)| entry);
            assert_eq!(
                off_the_plan, off_the_placements,
                "the two readings disagree about what writes '{name}'"
            );
        }

        // A user who holds nothing, and one the declarations do not name at all.
        assert!(placements.producer_of("carol", "api-token").is_none());
        assert!(placements.producer_of("nobody", "api-token").is_none());
    }

    #[test]
    fn holders_lists_users_with_no_secrets_and_holders_does_not() {
        let placements: Placements = serde_json::from_str(PLACEMENT).unwrap();
        assert_eq!(placements.users().collect::<Vec<_>>(), ["alice", "carol"]);
        assert_eq!(placements.holders().collect::<Vec<_>>(), ["alice"]);
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
      "alice": {
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
        let alice = plan.for_user("alice").unwrap();
        assert_eq!(alice.producer_of("base-pub"), Some("base"));
        assert_eq!(alice.producer_of("derived"), Some("derived"));
        assert_eq!(alice.producer_of("nobody-writes-this"), None);
    }

    #[test]
    fn a_cascade_is_transitive_and_stays_in_the_plans_order() {
        let plan: GeneratorPlan = serde_json::from_str(PLAN).unwrap();
        let alice = plan.for_user("alice").unwrap();
        assert_eq!(alice.cascade("base"), ["base", "derived", "far"]);
        assert_eq!(alice.cascade("derived"), ["derived", "far"]);
        assert_eq!(alice.cascade("aside"), ["aside"]);
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
        assert_eq!(plan.for_user("alice").unwrap().cycle(), None);
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
          "safix": { "user": "alice", "name": "grafana-password" },
          "kdbx": {
            "path": "alice/grafana",
            "fields": { "username": "alice@example.com", "url": null, "notes": null, "tags": [] }
          }
        },
        {
          "id": "router",
          "mode": "two-way",
          "safix": { "user": "bob", "name": "router" },
          "kdbx": { "path": "bob/router", "fields": {} }
        }
      ]
    }"#;

    #[test]
    fn the_mirror_deserializes_from_the_shape_nix_emits() {
        let mirror: Keepassxc = serde_json::from_str(MIRROR).unwrap();
        assert_eq!(mirror.database.as_deref(), Some("/keys/master.kdbx"));
        assert_eq!(mirror.declared(), ["grafana", "router"]);
        assert!(mirror.yubikey.is_none());
        assert!(mirror.key_file.is_none());

        let grafana = mirror.named("grafana").unwrap();
        assert_eq!(grafana.mode, Mode::SafixToKeepassxc);
        assert_eq!(mirror.entry_of(grafana), "safix/alice/grafana");
        assert!(matches!(
            &grafana.kdbx.fields.username,
            Some(FieldValue::Literal(username)) if username == "alice@example.com"
        ));
        assert!(grafana.kdbx.fields.declares());

        let router = mirror.named("router").unwrap();
        assert_eq!(router.mode, Mode::TwoWay);
        assert!(!router.kdbx.fields.declares());
        assert!(mirror.named("absent").is_none());
    }

    #[test]
    fn a_field_is_a_literal_or_a_named_entry_and_nothing_else() {
        let fields: Fields = serde_json::from_str(
            r#"{
              "username": "alice@example",
              "url": {"entry": "grafana-url"},
              "notes": null,
              "tags": []
            }"#,
        )
        .unwrap();

        assert!(matches!(
            &fields.username,
            Some(FieldValue::Literal(text)) if text == "alice@example"
        ));
        assert!(matches!(
            &fields.url,
            Some(FieldValue::Entry { entry }) if entry == "grafana-url"
        ));
        assert!(fields.notes.is_none());

        // The declared fields come back in the order every report and every
        // diff names them in, whatever order they were written in.
        assert_eq!(
            fields
                .named()
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["username", "url"]
        );

        // A third shape is not a field. `{ "name": … }` is the mistake this
        // refuses: it is the spelling an operator reaches for, and accepting it
        // as a literal would write the object's rendering into the entry.
        assert!(serde_json::from_str::<Fields>(r#"{"username": {"name": "x"}}"#).is_err());
    }

    #[test]
    fn an_unknown_field_name_is_refused_by_the_model() {
        assert!(
            serde_json::from_str::<KdbxSide>(r#"{"path": "a/b", "fields": {"pin": "1234"}}"#)
                .is_err()
        );
        // And the spelling this change deleted is refused the same way, which
        // is what makes the cutover clean rather than silently half-applied.
        assert!(
            serde_json::from_str::<KdbxSide>(r#"{"path": "a/b", "username": "alice"}"#).is_err()
        );
        assert!(serde_json::from_str::<KdbxSide>(r#"{"path": "a/b", "fields": {}}"#).is_ok());
    }

    /// A declared composite key deserializes field-for-field, and a mirror
    /// naming neither factor reads both as `None` — asserted at [`MIRROR`]
    /// above, which declares neither.
    #[test]
    fn a_declared_composite_key_deserializes_field_for_field() {
        let with_factors = r#"{
          "database": "/keys/master.kdbx",
          "group": "safix",
          "yubikey": { "slot": "1", "serial": "12345678" },
          "keyFile": "/home/alice/.keys/master.keyx",
          "mappings": []
        }"#;
        let mirror: Keepassxc = serde_json::from_str(with_factors).unwrap();
        let yubikey = mirror.yubikey.as_ref().unwrap();
        assert_eq!(yubikey.slot, "1");
        assert_eq!(yubikey.serial.as_deref(), Some("12345678"));
        assert_eq!(
            mirror.key_file.as_deref(),
            Some("/home/alice/.keys/master.keyx")
        );

        let bare_slot = r#"{"slot": "2", "serial": null}"#;
        let yubikey: Yubikey = serde_json::from_str(bare_slot).unwrap();
        assert_eq!(yubikey.slot, "2");
        assert!(yubikey.serial.is_none());
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
            r#""path": "bob/router""#,
            r#""path": "bob/router", "url": "x""#,
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

    const PASS_STORE: &str = r#"{
      "store": "~/.password-store",
      "mappings": [
        {
          "id": "grafana",
          "mode": "safix-to-pass",
          "safix": { "user": "alice", "name": "grafana-password" },
          "pass": {
            "path": "alice/grafana",
            "fields": {
              "username": "alice@example.com",
              "url": "https://grafana.example.invalid",
              "notes": "minted by safix",
              "tags": ["work", "fleet"]
            }
          }
        },
        {
          "id": "sourced",
          "mode": "two-way",
          "safix": { "user": "alice", "name": "deck-password" },
          "pass": {
            "path": "alice/deck",
            "fields": { "username": { "entry": "other" } }
          }
        },
        {
          "id": "bare",
          "mode": "backup",
          "safix": { "user": "bob", "name": "router" },
          "pass": { "path": "bob/router", "fields": {} }
        }
      ]
    }"#;

    #[test]
    fn the_pass_store_deserializes_from_the_shape_nix_emits() {
        let store: Pass = serde_json::from_str(PASS_STORE).unwrap();
        assert_eq!(store.store, "~/.password-store");
        assert_eq!(store.declared(), ["grafana", "sourced", "bare"]);

        let grafana = store.named("grafana").unwrap();
        assert_eq!(grafana.mode, PassMode::SafixToPass);
        assert_eq!(store.entry_of(grafana), "alice/grafana");
        assert!(matches!(
            &grafana.pass.fields.username,
            Some(FieldValue::Literal(username)) if username == "alice@example.com"
        ));
        assert!(matches!(
            &grafana.pass.fields.url,
            Some(FieldValue::Literal(url)) if url == "https://grafana.example.invalid"
        ));
        assert!(matches!(
            &grafana.pass.fields.notes,
            Some(FieldValue::Literal(notes)) if notes == "minted by safix"
        ));
        assert_eq!(grafana.pass.fields.tags.len(), 2);
        assert!(matches!(
            grafana.pass.fields.tags.first(),
            Some(FieldValue::Literal(tag)) if tag == "work"
        ));

        // The one target where an `{ entry = … }` source is admissible, so the
        // variant it lands in is what the transport reads to decide whether a
        // field's value is a secret.
        let sourced = store.named("sourced").unwrap();
        assert_eq!(sourced.mode, PassMode::TwoWay);
        assert!(matches!(
            &sourced.pass.fields.username,
            Some(FieldValue::Entry { entry }) if entry == "other"
        ));

        let bare = store.named("bare").unwrap();
        assert_eq!(bare.mode, PassMode::Backup);
        assert!(!bare.pass.fields.declares());
        assert!(store.named("absent").is_none());
    }

    /// What makes the nix half and this one a single change: a key the
    /// declaration does not have is an evaluation-time failure here rather than
    /// a run that silently wrote less than the declaration said.
    #[test]
    fn an_unknown_key_on_a_pass_mapping_is_refused_rather_than_dropped() {
        let with_extra = PASS_STORE.replace(
            r#""path": "bob/router""#,
            r#""path": "bob/router", "group": "safix""#,
        );
        assert!(serde_json::from_str::<Pass>(&with_extra).is_err());
    }

    #[test]
    fn an_unknown_pass_mode_is_refused_rather_than_read_as_another() {
        let with_mode = PASS_STORE.replace(r#""mode": "two-way""#, r#""mode": "push""#);
        assert!(serde_json::from_str::<Pass>(&with_mode).is_err());
    }

    #[test]
    fn each_pass_mode_writes_the_sides_its_name_says() {
        let modes = [
            (PassMode::SafixToPass, false, true),
            (PassMode::PassToSafix, true, false),
            (PassMode::TwoWay, true, true),
            (PassMode::Backup, false, true),
        ];
        for (mode, pulls, pushes) in modes {
            assert_eq!(mode.pulls(), pulls, "{mode} pulls");
            assert_eq!(mode.pushes(), pushes, "{mode} pushes");
        }
    }

    /// Exactly what `modules/flake/safix/default.nix`'s `bitwarden` record
    /// emits: an optional server, one mapping under a folder, one folderless,
    /// and the fields record — `tags` included, because the refusal for it is
    /// evaluation's rather than serde's.
    const VAULT: &str = r#"{
      "server": "http://127.0.0.1:8222",
      "mappings": [
        {
          "id": "grafana",
          "mode": "safix-to-bitwarden",
          "safix": { "user": "alice", "name": "grafana-password" },
          "bitwarden": {
            "folder": "fleet",
            "item": "grafana",
            "fields": {
              "username": "alice@example.com",
              "url": "https://grafana.example.invalid",
              "notes": "minted by safix",
              "tags": []
            }
          }
        },
        {
          "id": "sourced",
          "mode": "two-way",
          "safix": { "user": "alice", "name": "router" },
          "bitwarden": {
            "folder": null,
            "item": "router",
            "fields": { "username": { "entry": "other" }, "url": null, "notes": null, "tags": [] }
          }
        },
        {
          "id": "bare",
          "mode": "backup",
          "safix": { "user": "bob", "name": "router" },
          "bitwarden": { "folder": null, "item": "bob-router", "fields": {} }
        }
      ]
    }"#;

    #[test]
    fn the_vault_deserializes_from_the_shape_nix_emits() {
        let vault: Bitwarden = serde_json::from_str(VAULT).unwrap();
        assert_eq!(vault.server.as_deref(), Some("http://127.0.0.1:8222"));
        assert_eq!(vault.declared(), ["grafana", "sourced", "bare"]);

        let grafana = vault.named("grafana").unwrap();
        assert_eq!(grafana.mode, BitwardenMode::SafixToBitwarden);
        assert!(matches!(
            &grafana.bitwarden.fields.username,
            Some(FieldValue::Literal(username)) if username == "alice@example.com"
        ));
        assert!(matches!(
            &grafana.bitwarden.fields.url,
            Some(FieldValue::Literal(url)) if url == "https://grafana.example.invalid"
        ));
        assert!(matches!(
            &grafana.bitwarden.fields.notes,
            Some(FieldValue::Literal(notes)) if notes == "minted by safix"
        ));

        let sourced = vault.named("sourced").unwrap();
        assert_eq!(sourced.mode, BitwardenMode::TwoWay);
        assert!(matches!(
            &sourced.bitwarden.fields.username,
            Some(FieldValue::Entry { entry }) if entry == "other"
        ));

        let bare = vault.named("bare").unwrap();
        assert_eq!(bare.mode, BitwardenMode::Backup);
        assert!(!bare.bitwarden.fields.declares());
        assert!(vault.named("absent").is_none());
    }

    /// The two branches of the address, asserted against the literals
    /// `modules/flake/safix/bitwarden.nix`'s `itemPathOf` builds.
    #[test]
    fn an_address_is_the_folder_and_the_item_or_the_item_alone() {
        let vault: Bitwarden = serde_json::from_str(VAULT).unwrap();
        assert_eq!(
            vault.address_of(vault.named("grafana").unwrap()),
            "fleet/grafana"
        );
        assert_eq!(vault.address_of(vault.named("bare").unwrap()), "bob-router");
    }

    /// What makes the nix half and this one a single change: a key the
    /// declaration does not have is a deserialization failure rather than a run
    /// that silently wrote less than it said. Drill 2.8 is removing
    /// `deny_unknown_fields` from `BitwardenSide` and watching this turn green.
    #[test]
    fn an_unknown_key_on_a_vault_mapping_is_refused_rather_than_dropped() {
        let with_extra = VAULT.replace(
            r#""item": "bob-router""#,
            r#""item": "bob-router", "collection": "fleet""#,
        );
        assert!(serde_json::from_str::<Bitwarden>(&with_extra).is_err());
    }

    #[test]
    fn an_unknown_bitwarden_mode_is_refused_rather_than_read_as_another() {
        let with_mode = VAULT.replace(r#""mode": "two-way""#, r#""mode": "push""#);
        assert!(serde_json::from_str::<Bitwarden>(&with_mode).is_err());
    }

    #[test]
    fn each_bitwarden_mode_writes_the_sides_its_name_says() {
        let modes = [
            (BitwardenMode::SafixToBitwarden, false, true),
            (BitwardenMode::BitwardenToSafix, true, false),
            (BitwardenMode::TwoWay, true, true),
            (BitwardenMode::Backup, false, true),
        ];
        for (mode, pulls, pushes) in modes {
            assert_eq!(mode.pulls(), pulls, "{mode} pulls");
            assert_eq!(mode.pushes(), pushes, "{mode} pushes");
        }
    }

    /// A declared `tags` deserializes — the shared `Fields` type carries it —
    /// and the transport's own capability table is what reports it uncarried.
    /// The refusal is evaluation's, and confusing the two would make serde the
    /// place a consumer hears about a field this vault has no concept of.
    #[test]
    fn a_declared_tag_deserializes_and_is_reported_uncarried_by_the_capabilities() {
        let with_tags = VAULT.replace(
            r#""fields": { "username": { "entry": "other" }, "url": null, "notes": null, "tags": [] }"#,
            r#""fields": { "username": null, "url": null, "notes": null, "tags": ["work"] }"#,
        );
        let vault: Bitwarden = serde_json::from_str(&with_tags).unwrap();
        assert_eq!(
            vault.named("sourced").unwrap().bitwarden.fields.tags.len(),
            1
        );
        assert_eq!(
            crate::bitwarden::CAPABILITIES.channel(crate::endpoint::FieldName::Tags),
            crate::endpoint::Channel::Unsupported
        );
    }

    #[test]
    fn holders_of_separates_named_users_from_orphaned_keys() {
        let recipients: Recipients = serde_json::from_str(
            r#"{"alice": ["age1a", "age1escrow"], "bob": ["age1b"], "carol": ["age1c"]}"#,
        )
        .unwrap();
        let found = recipients.holders_of(&["age1b".into(), "age1stray".into()]);
        assert_eq!(found.named, ["bob"]);
        assert_eq!(found.orphaned, ["age1stray"]);
    }

    #[test]
    fn a_shared_mapping_deserializes_with_a_null_machine_and_a_per_machine_one_with_a_declared_one()
    {
        let bridge: Bridge = serde_json::from_str(
            r#"{
              "clanFlake": ".",
              "mappings": [
                {
                  "id": "shared-rel",
                  "direction": "two-way",
                  "clan": {
                    "placement": "shared",
                    "machine": null,
                    "generator": "ntfy",
                    "file": "token"
                  },
                  "safix": { "user": "alice", "name": "ntfy-token" }
                },
                {
                  "id": "per-machine-rel",
                  "direction": "clan-to-safix",
                  "clan": {
                    "placement": "per-machine",
                    "machine": "meridian",
                    "generator": "wg",
                    "file": "private"
                  },
                  "safix": { "user": "bob", "name": "wg-key" }
                }
              ]
            }"#,
        )
        .unwrap();

        let shared = bridge.named("shared-rel").unwrap();
        assert_eq!(shared.clan.placement, ClanPlacement::Shared);
        assert_eq!(shared.clan.machine, None);
        assert_eq!(shared.direction, Direction::TwoWay);

        let per_machine = bridge.named("per-machine-rel").unwrap();
        assert_eq!(per_machine.clan.placement, ClanPlacement::PerMachine);
        assert_eq!(per_machine.clan.machine.as_deref(), Some("meridian"));
    }

    const ONEPASSWORD: &str = r#"{
      "account": "fixture.example.com",
      "mappings": [
        {
          "id": "grafana",
          "mode": "safix-to-1password",
          "safix": { "user": "alice", "name": "grafana-password" },
          "onepassword": {
            "vault": "fixture-vault",
            "item": "grafana",
            "fields": {
              "username": "alice@example.com",
              "url": "https://grafana.example.invalid",
              "notes": "minted by safix",
              "tags": ["work", "fleet"]
            }
          }
        },
        {
          "id": "sourced",
          "mode": "1password-to-safix",
          "safix": { "user": "alice", "name": "deck-password" },
          "onepassword": {
            "vault": "fixture-vault",
            "item": "deck",
            "fields": { "notes": { "entry": "other" } }
          }
        },
        {
          "id": "bare",
          "mode": "backup",
          "safix": { "user": "bob", "name": "router" },
          "onepassword": { "vault": "other-vault", "item": "grafana", "fields": {} }
        },
        {
          "id": "paired",
          "mode": "two-way",
          "safix": { "user": "carol", "name": "wiki" },
          "onepassword": { "vault": "fixture-vault", "item": "wiki", "fields": {} }
        }
      ]
    }"#;

    #[test]
    fn the_onepassword_mirror_deserializes_from_the_shape_nix_emits() {
        let mirror: OnePassword = serde_json::from_str(ONEPASSWORD).unwrap();
        assert_eq!(mirror.account.as_deref(), Some("fixture.example.com"));
        assert_eq!(mirror.declared(), ["grafana", "sourced", "bare", "paired"]);

        let grafana = mirror.named("grafana").unwrap();
        assert_eq!(grafana.mode, OnePasswordMode::SafixToOnePassword);
        assert_eq!(mirror.item_of(grafana), "fixture-vault/grafana");
        assert!(matches!(
            &grafana.onepassword.fields.username,
            Some(FieldValue::Literal(username)) if username == "alice@example.com"
        ));
        assert_eq!(grafana.onepassword.fields.tags.len(), 2);

        // The far side's address is the vault and the item together: the same
        // item name in a second vault is a second item.
        let bare = mirror.named("bare").unwrap();
        assert_eq!(mirror.item_of(bare), "other-vault/grafana");
        assert!(!bare.onepassword.fields.declares());

        // Every field of this target crosses standard input, so an
        // `{ entry = … }` source is admissible on all four.
        let sourced = mirror.named("sourced").unwrap();
        assert_eq!(sourced.mode, OnePasswordMode::OnePasswordToSafix);
        assert!(matches!(
            &sourced.onepassword.fields.notes,
            Some(FieldValue::Entry { entry }) if entry == "other"
        ));

        assert_eq!(
            mirror.named("paired").unwrap().mode,
            OnePasswordMode::TwoWay
        );
        assert!(mirror.named("absent").is_none());
    }

    /// An account nobody declared is a working configuration rather than a
    /// missing declaration, so the projection's null has to survive the read.
    #[test]
    fn an_undeclared_onepassword_account_deserializes_as_none() {
        let mirror: OnePassword =
            serde_json::from_str(r#"{"account": null, "mappings": []}"#).unwrap();
        assert!(mirror.account.is_none());
        assert!(mirror.declared().is_empty());
    }

    /// What makes the nix half and this one a single change: a key the
    /// declaration does not have is refused here rather than read as a run
    /// that silently wrote less than the declaration said.
    #[test]
    fn an_unknown_member_of_the_onepassword_side_is_refused_rather_than_dropped() {
        let with_extra = ONEPASSWORD.replace(
            r#""item": "wiki""#,
            r#""item": "wiki", "category": "login""#,
        );
        assert!(serde_json::from_str::<OnePassword>(&with_extra).is_err());
    }

    #[test]
    fn an_unknown_top_level_member_of_the_onepassword_mirror_is_refused() {
        assert!(
            serde_json::from_str::<OnePassword>(
                r#"{"account": null, "mappings": [], "vault": "fixture-vault"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn an_unknown_onepassword_mode_is_refused_rather_than_read_as_another() {
        let with_mode = ONEPASSWORD.replace(r#""mode": "two-way""#, r#""mode": "op-to-safix""#);
        assert!(serde_json::from_str::<OnePassword>(&with_mode).is_err());
    }

    #[test]
    fn each_onepassword_mode_writes_the_sides_its_name_says() {
        let modes = [
            (OnePasswordMode::SafixToOnePassword, false, true),
            (OnePasswordMode::OnePasswordToSafix, true, false),
            (OnePasswordMode::TwoWay, true, true),
            (OnePasswordMode::Backup, false, true),
        ];
        for (mode, pulls, pushes) in modes {
            assert_eq!(mode.pulls(), pulls, "{mode} pulls");
            assert_eq!(mode.pushes(), pushes, "{mode} pushes");
        }
        // Each of the four spellings round-trips under its own word.
        for spelling in [
            "safix-to-1password",
            "1password-to-safix",
            "two-way",
            "backup",
        ] {
            let one =
                ONEPASSWORD.replace(r#""mode": "two-way""#, &format!(r#""mode": "{spelling}""#));
            let mirror: OnePassword = serde_json::from_str(&one).unwrap();
            assert_eq!(mirror.named("paired").unwrap().mode.as_str(), spelling);
        }
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
/// a group, the owner a machine records, a service, and an organization.
///
/// Named here for the same reason as the separator, and load-bearing for the same
/// claim. A directory is joined from elements, so the alphabet injectivity rests
/// on is the marked forms as well as the bare names, and the property test below
/// is where this crate states that rather than inheriting it.
pub const AUDIENCE_MARKERS: [&str; 4] = ["@", "@~", "%", "="];

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::AUDIENCE_SEPARATOR;

    use super::AUDIENCE_MARKERS;

    /// The alphabet `resolve.nix` admits a user, anchor or secret name from.
    const NAME: &str = "[a-z0-9][a-z0-9_-]{0,7}";

    /// Every form an audience element takes: a subject named in place, a group,
    /// the owner a machine records, a service, or an organization. The markers are
    /// part of the alphabet a directory is joined from, so the property below has
    /// to be over elements rather than over names.
    ///
    /// Built by mapping the marker set rather than by naming each marker, which is
    /// what covered the organization element the day the constant grew rather than
    /// the day someone remembered this strategy.
    fn element() -> impl Strategy<Value = String> {
        let marked: Vec<BoxedStrategy<String>> = std::iter::once(NAME.prop_map(|n| n).boxed())
            .chain(
                AUDIENCE_MARKERS
                    .into_iter()
                    .map(|marker| NAME.prop_map(move |name| format!("{marker}{name}")).boxed()),
            )
            .collect();
        proptest::strategy::Union::new(marked)
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
        let group = ["alice".to_owned(), "xops".to_owned()];
        let person = ["alice".to_owned(), "ops".to_owned()];

        let marked = |audience: &[String], marker: &str| {
            let mut marked = audience.to_vec();
            if let Some(last) = marked.last_mut() {
                *last = format!("{marker}{}", last.trim_start_matches('x'));
            }
            directory_of(&marked, AUDIENCE_SEPARATOR)
        };

        // A marker of `x` is inside the alphabet, so marking `ops` reaches the
        // directory `alice,xops` that a person named `xops` already has.
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
        let pair = ["alice".to_owned(), "bob-carol".to_owned()];
        let other = ["alice-bob".to_owned(), "carol".to_owned()];

        assert_eq!(directory_of(&pair, "-"), directory_of(&other, "-"));
        assert_ne!(
            directory_of(&pair, AUDIENCE_SEPARATOR),
            directory_of(&other, AUDIENCE_SEPARATOR)
        );
    }
}
