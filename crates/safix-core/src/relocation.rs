//! What a vault migration moves: the opaque and readable name of every
//! document, public output, and definition record a vault-mode fleet holds.
//!
//! Shared by [`crate::check`]'s pending-relocation finding and
//! [`crate::fix`]'s relocate phase, so the two enumerate the same set rather
//! than two that could disagree.
//!
//! Nothing here reads a byte or touches the filesystem, and nothing hashes:
//! every candidate comes straight off [`Placement`], which nix already
//! computed both names for (design V14).

use std::collections::BTreeMap;

use crate::definition;
use crate::model::{Placement, Placements};

/// One ciphertext document's two names, and the `(opaque key, readable key)`
/// pairs it holds.
///
/// Several placements can share one file — a shared secret's own carriers,
/// or two private secrets one person declares in the same document — so this
/// is grouped by file rather than emitted one entry per placement.
#[derive(Debug, Clone)]
pub(crate) struct SecretDocument {
    pub(crate) opaque_file: String,
    pub(crate) logical_file: String,
    /// `(opaque key, readable key)`, one per name the document holds, in the
    /// order first encountered.
    pub(crate) keys: Vec<(String, String)>,
}

/// One plaintext leaf's two names: a public output or a definition record,
/// each a single file with no keys of its own.
#[derive(Debug, Clone)]
pub(crate) struct PlainLeaf {
    pub(crate) opaque: String,
    pub(crate) logical: String,
}

/// Every ciphertext document a vault-mode fleet holds, grouped by file, in
/// file order.
///
/// Empty when no vault is declared: every placement's `logical_file` is
/// `None` then (design V14), so nothing groups. A placement whose `public`
/// is set names no ciphertext at all and is skipped.
pub(crate) fn secret_documents(placements: &Placements) -> Vec<SecretDocument> {
    let mut grouped: BTreeMap<String, (String, Vec<(String, String)>)> = BTreeMap::new();
    for user in placements.users() {
        for (_, placement) in placements.held_by(user).into_iter().flatten() {
            if placement.public.is_some() {
                continue;
            }
            let (Some(logical_file), Some(logical_key)) =
                (&placement.logical_file, &placement.logical_key)
            else {
                continue;
            };
            let entry = grouped
                .entry(placement.file.clone())
                .or_insert_with(|| (logical_file.clone(), Vec::new()));
            let pair = (placement.key.clone(), logical_key.clone());
            if !entry.1.contains(&pair) {
                entry.1.push(pair);
            }
        }
    }
    grouped
        .into_iter()
        .map(|(opaque_file, (logical_file, keys))| SecretDocument {
            opaque_file,
            logical_file,
            keys,
        })
        .collect()
}

/// Every public output a vault-mode fleet holds, in opaque-path order.
///
/// Deduplicated by opaque path, though a public output is not shared the way
/// a secret can be — this keeps the two enumerations the same shape.
pub(crate) fn public_leaves(placements: &Placements) -> Vec<PlainLeaf> {
    leaves_by(placements, |placement| {
        Some((placement.public.clone()?, placement.logical_public.clone()?))
    })
}

/// Every definition record a vault-mode fleet holds, in opaque-path order.
///
/// Deduplicated by opaque path: a shared entry's record is one file both
/// carriers resolve (design V14's `record_path`), so their two placements
/// must not each queue a copy of the same move.
pub(crate) fn record_leaves(placements: &Placements) -> Vec<PlainLeaf> {
    let mut leaves: BTreeMap<String, String> = BTreeMap::new();
    for user in placements.users() {
        for (name, placement) in placements.held_by(user).into_iter().flatten() {
            let Some(opaque) = &placement.definition_record else {
                continue;
            };
            let Some(logical) = definition::logical_record_path(name, placement) else {
                continue;
            };
            leaves.insert(opaque.clone(), logical);
        }
    }
    leaves
        .into_iter()
        .map(|(opaque, logical)| PlainLeaf { opaque, logical })
        .collect()
}

/// The shape [`public_leaves`] and [`record_leaves`] share: derive one
/// `(opaque, logical)` pair per placement when `select` finds one, keyed and
/// deduplicated by the opaque half.
fn leaves_by(
    placements: &Placements,
    select: impl Fn(&Placement) -> Option<(String, String)>,
) -> Vec<PlainLeaf> {
    let mut leaves: BTreeMap<String, String> = BTreeMap::new();
    for user in placements.users() {
        for (_, placement) in placements.held_by(user).into_iter().flatten() {
            if let Some((opaque, logical)) = select(placement) {
                leaves.insert(opaque, logical);
            }
        }
    }
    leaves
        .into_iter()
        .map(|(opaque, logical)| PlainLeaf { opaque, logical })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{public_leaves, record_leaves, secret_documents};
    use crate::model::Placements;

    fn placements(entries: Value) -> Placements {
        serde_json::from_value(entries).expect("the fixture is the shape the resolver emits")
    }

    fn secret(file: &str, key: &str, owner: &str, shared: bool, logical_key: &str) -> Value {
        json!({
            "file": file, "key": key, "origin": "private",
            "owner": owner, "shared": shared, "generator": null, "public": null,
            "definitionRecord": null,
            "logicalFile": "secrets/safix/users/alice/secrets.yaml",
            "logicalKey": logical_key, "logicalPublic": null,
        })
    }

    fn no_vault_secret(file: &str, key: &str, owner: &str) -> Value {
        json!({
            "file": file, "key": key, "origin": "private",
            "owner": owner, "shared": false, "generator": null, "public": null,
            "definitionRecord": null, "logicalFile": null, "logicalKey": null,
            "logicalPublic": null,
        })
    }

    /// Two names sharing one opaque file group into one document, carrying
    /// both key pairs.
    #[test]
    fn secrets_sharing_a_file_group_into_one_document() {
        let held = placements(json!({
            "alice": {
                "api-token": secret("secrets/opaque1.yaml", "opaque-key-1", "alice", false, "api-token"),
                "mail-password": secret("secrets/opaque1.yaml", "opaque-key-2", "alice", false, "mail-password"),
            },
        }));
        let documents = secret_documents(&held);
        let [document] = documents.as_slice() else {
            unreachable!("one distinct opaque file is one document, got {documents:?}");
        };
        assert_eq!(document.opaque_file, "secrets/opaque1.yaml");
        assert_eq!(
            document.logical_file,
            "secrets/safix/users/alice/secrets.yaml"
        );
        assert_eq!(
            document.keys,
            vec![
                ("opaque-key-1".to_owned(), "api-token".to_owned()),
                ("opaque-key-2".to_owned(), "mail-password".to_owned()),
            ]
        );
    }

    /// A shared entry's two carriers name one document once, not twice.
    #[test]
    fn a_shared_secret_s_two_carriers_group_into_one_document() {
        let held = placements(json!({
            "alice": { "fleet-token": secret("secrets/shared-opaque.yaml", "opaque-key", "alice", true, "fleet-token") },
            "bob": { "fleet-token": secret("secrets/shared-opaque.yaml", "opaque-key", "bob", true, "fleet-token") },
        }));
        let documents = secret_documents(&held);
        let [document] = documents.as_slice() else {
            unreachable!("the shared carriers name one document, got {documents:?}");
        };
        assert_eq!(document.keys.len(), 1, "the shared key is queued once");
    }

    /// No vault declared: every `logical_*` field is `None`, and nothing
    /// groups.
    #[test]
    fn no_vault_declared_groups_nothing() {
        let held = placements(json!({
            "alice": { "api-token": no_vault_secret("secrets/safix/users/alice/secrets.yaml", "api-token", "alice") },
        }));
        assert!(secret_documents(&held).is_empty());
        assert!(public_leaves(&held).is_empty());
        assert!(record_leaves(&held).is_empty());
    }

    /// A public output's opaque and readable paths pass through unmodified.
    #[test]
    fn a_public_output_carries_its_two_paths() {
        let entry = json!({
            "file": "secrets/opaque.yaml", "key": "k", "origin": "private",
            "owner": "alice", "shared": false, "generator": null,
            "public": "public/opaque-output",
            "definitionRecord": null,
            "logicalFile": "secrets/safix/users/alice/secrets.yaml",
            "logicalKey": "k", "logicalPublic": "public/safix/users/alice/host-key/value",
        });
        let held = placements(json!({ "alice": { "host-key": entry } }));
        let leaves = public_leaves(&held);
        let [leaf] = leaves.as_slice() else {
            unreachable!("exactly one public leaf, got {leaves:?}");
        };
        assert_eq!(leaf.opaque, "public/opaque-output");
        assert_eq!(leaf.logical, "public/safix/users/alice/host-key/value");
    }

    /// A shared entry's definition record is one leaf, not one per carrier.
    #[test]
    fn a_shared_definition_record_is_one_leaf() {
        let alice = json!({
            "file": "secrets/shared-opaque.yaml", "key": "opaque-key", "origin": "carries",
            "owner": "alice", "shared": true, "generator": null, "public": null,
            "definitionRecord": "state/opaque-record",
            "logicalFile": "secrets/safix/shared/alice,bob/secrets.yaml",
            "logicalKey": "fleet-token", "logicalPublic": null,
        });
        let bob = json!({
            "file": "secrets/shared-opaque.yaml", "key": "opaque-key", "origin": "carries",
            "owner": "bob", "shared": true, "generator": null, "public": null,
            "definitionRecord": "state/opaque-record",
            "logicalFile": "secrets/safix/shared/alice,bob/secrets.yaml",
            "logicalKey": "fleet-token", "logicalPublic": null,
        });
        let held = placements(json!({
            "alice": { "fleet-token": alice },
            "bob": { "fleet-token": bob },
        }));
        let leaves = record_leaves(&held);
        let [leaf] = leaves.as_slice() else {
            unreachable!("both carriers resolve the one record they share, got {leaves:?}");
        };
        assert_eq!(leaf.opaque, "state/opaque-record");
        assert_eq!(
            leaf.logical,
            "state/safix/definitions/shared/alice,bob/fleet-token"
        );
    }
}
