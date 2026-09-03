//! Disclosing the lock-bump cost a vault-root commit incurs — design V6.
//!
//! A vault's ciphertext lives at a path nix never evaluates; the declaring
//! flake's lock file is what pins that path for every consuming build, and a
//! commit at the vault root moves it without touching that pin. This module
//! is the sentence naming that gap, printed after such a commit lands, and
//! the best-effort search for the exact remedy: the one lock input, when
//! exactly one settles on the vault root, and the command that bumps it.

use std::path::Path;

use crate::nix::{FlakeMetadata, LockedSource, Nix};
use crate::workspace::canonicalized;

/// The disclosure printed after a vault-root commit lands: the requirement
/// design V6 states, and the exact `nix flake lock --update-input` line when
/// the declaring flake's lock file settles on exactly one matching input.
///
/// Total, by construction: [`Nix::flake_metadata`] and [`Nix::nar_hash`] both
/// fall back to `None` on any failure rather than propagating one, which is
/// what lets a disclosure that cannot name the exact remedy fall back to the
/// general phrasing rather than refuse a write that already committed.
pub(crate) fn disclosure(nix: &Nix, declaration_root: &Path, vault_root: &Path) -> String {
    let input = nix
        .flake_metadata(declaration_root)
        .and_then(|metadata| matching_input(nix, &metadata, vault_root));
    message(input.as_deref())
}

/// The one lock node whose locked source names `vault_root`, when exactly
/// one does — by its own recorded path first, falling back to a freshly
/// computed NAR hash comparison only when the path check settles on none.
fn matching_input(nix: &Nix, metadata: &FlakeMetadata, vault_root: &Path) -> Option<String> {
    let canonical_vault_root = canonicalized(vault_root);
    if let Some(name) = single_match(metadata, |locked| {
        locked
            .path
            .as_deref()
            .is_some_and(|path| canonicalized(Path::new(path)) == canonical_vault_root)
    }) {
        return Some(name);
    }

    // Only reached when no node's own recorded path settled the question:
    // every node the fixture tests declare carries one, so this is the
    // real-world branch a network-fetched lock entry with no local path
    // takes.
    let fresh_hash = nix.nar_hash(vault_root)?;
    single_match(metadata, |locked| {
        locked.nar_hash.as_deref() == Some(fresh_hash.as_str())
    })
}

/// The one node's name whose locked source satisfies `matches`, when exactly
/// one does; `None` on zero matches or on more than one.
fn single_match(
    metadata: &FlakeMetadata,
    matches: impl Fn(&LockedSource) -> bool,
) -> Option<String> {
    let mut found = metadata.nodes().filter(|(_, locked)| matches(locked));
    let (name, _) = found.next()?;
    if found.next().is_some() {
        return None;
    }
    Some(name.to_owned())
}

/// The disclosure's text: the general requirement alone, or with the exact
/// remedy appended when [`matching_input`] settled on one name.
fn message(input: Option<&str>) -> String {
    let requirement = "\nThis change is not visible to any consuming build until the declaring flake's lock entry for the vault is updated.\n";
    match input {
        Some(name) => {
            format!("{requirement}Run `nix flake lock --update-input {name}` to update it.\n")
        }
        None => requirement.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nix::Nix;

    fn stub_nix() -> Nix {
        Nix::from_environment()
    }

    /// A metadata document with no `locked` field on any node — the root
    /// node's own shape — never matches, so the disclosure falls back to the
    /// general phrasing without ever calling nix a second time.
    #[test]
    fn a_metadata_document_naming_no_locked_source_falls_back_to_the_general_message() {
        let metadata: FlakeMetadata = serde_json::from_value(serde_json::json!({
            "locks": { "nodes": { "root": {} } }
        }))
        .expect("this shape parses");
        assert_eq!(metadata.nodes().count(), 0);
        assert!(message(None).contains("lock entry for the vault is updated"));
        assert!(!message(None).contains("--update-input"));
    }

    /// The specific message names the exact remedy; the general one does
    /// not, which is the scenario `secrets-vault/spec.md` states as "the
    /// input name cannot be determined".
    #[test]
    fn the_specific_message_names_the_remedy_the_general_one_omits() {
        assert!(message(Some("vault")).contains("nix flake lock --update-input vault"));
        assert!(!message(None).contains("--update-input"));
    }

    /// `single_match` is the primitive both matching passes share: zero or
    /// more than one satisfying node answers `None`, exactly one answers its
    /// name.
    #[test]
    fn single_match_answers_only_a_lone_satisfying_node() {
        let metadata: FlakeMetadata = serde_json::from_value(serde_json::json!({
            "locks": {
                "nodes": {
                    "root": {},
                    "vault": { "locked": { "path": "/vault" } },
                    "other": { "locked": { "path": "/elsewhere" } },
                }
            }
        }))
        .expect("this shape parses");

        assert_eq!(
            single_match(&metadata, |locked| locked.path.as_deref() == Some("/vault")),
            Some("vault".to_owned())
        );
        assert_eq!(
            single_match(&metadata, |locked| locked.path.is_some()),
            None
        );
        assert_eq!(
            single_match(&metadata, |locked| locked.path.as_deref()
                == Some("/missing")),
            None
        );
    }

    /// [`matching_input`] never calls [`Nix::nar_hash`] when the path check
    /// alone already settles on exactly one node — the stub's own program
    /// path answers `nix hash path` with a refusal, so a call that reached it
    /// would turn this into a failing subprocess rather than a fast, total
    /// comparison.
    #[test]
    fn a_unique_path_match_needs_no_nar_hash_fallback() {
        let metadata: FlakeMetadata = serde_json::from_value(serde_json::json!({
            "locks": { "nodes": { "vault": { "locked": { "path": "/vault" } } } }
        }))
        .expect("this shape parses");
        let nix = stub_nix();
        assert_eq!(
            matching_input(&nix, &metadata, Path::new("/vault")),
            Some("vault".to_owned())
        );
    }
}
