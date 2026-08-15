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

use std::collections::BTreeMap;

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
/// The map key is the identifier the script addresses — `$in_<key>` — and
/// [`PlanInput::name`] is the declared name it was derived from, which is the
/// one a refusal quotes and the one a dependency is resolved by.
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
/// postcondition. Nothing here re-derives either.
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

    /// Every generator that would derive from this one's output, it first, in
    /// the plan's own order.
    ///
    /// One forward pass over [`UserPlan::order`] is sufficient because that
    /// order is topological — a generator appears after everything it reads —
    /// which is the resolver's claim and is what its cycle refusal guarantees. A
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

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::AUDIENCE_SEPARATOR;

    /// The alphabet `resolve.nix` admits a user, anchor or secret name from.
    const NAME: &str = "[a-z0-9][a-z0-9_-]{0,7}";

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
            left in proptest::collection::vec(NAME, 1..4),
            right in proptest::collection::vec(NAME, 1..4),
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
