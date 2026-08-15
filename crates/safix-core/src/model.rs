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
    /// The further outputs the script writes beyond the entry carrying it.
    pub files: Vec<String>,
    /// What the operator is asked for, by the name the script addresses.
    pub prompts: BTreeMap<String, Prompt>,
    /// nixpkgs attribute names put on `PATH` while the script runs.
    #[serde(rename = "runtimeInputs")]
    pub runtime_inputs: Vec<String>,
    /// The shell fragment that produces the value.
    pub script: String,
    /// A shell fragment judging a candidate value, or none.
    pub validation: Option<String>,
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
            "dependencies": [], "description": null, "files": [],
            "prompts": {}, "runtimeInputs": ["coreutils"],
            "script": "printf '%s' fixture", "validation": null
          },
          "key": "api-token", "origin": "private", "owner": "ana",
          "shared": false
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
        assert_eq!(
            token.generator.as_ref().unwrap().runtime_inputs,
            ["coreutils"]
        );
        assert!(placements.declares("cy"));
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
