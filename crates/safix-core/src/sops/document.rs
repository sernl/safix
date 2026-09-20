//! What a sops document says about itself without being decrypted.
//!
//! Recipient metadata and encrypted value shapes are public. Readers support
//! SOPS YAML, JSON, dotenv and INI, including age and `GnuPG` recipients.
//! Unsupported recipient providers and threshold groups remain explicit
//! findings rather than being flattened into an ordinary recipient union.
//! Nothing here decrypts or holds a private identity.

use std::collections::{BTreeMap, BTreeSet};

use serde_norway::Value;

use crate::error::{Error, Result};

/// What is reported in place of a recipient list when a governed path holds a
/// document with no recognized SOPS recipient metadata.
///
/// A sentinel rather than a failure: such a path is either plaintext someone
/// committed by mistake or ciphertext from a store with a different metadata
/// shape, and both have to be reported against the declared audience rather
/// than crash the reader that was asked to inspect them.
pub const NO_METADATA: &str = "<file carries no sops recipient metadata>";

pub(crate) const THRESHOLD_GROUPS: &str = "<unsupported sops threshold key groups>";

/// The public recipients a document names, with unsupported semantics marked.
///
/// # Errors
///
/// [`Error::SopsDocumentUnreadable`] for malformed public document structure,
/// or [`Error::SopsStanzaUnreadable`] for malformed recipient metadata.
pub fn recipients_of(text: &str) -> Result<Vec<String>> {
    let document = parse_public_document(text)?;
    let Some(metadata) = document
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String("sops".into())))
        .and_then(Value::as_mapping)
    else {
        return Ok(vec![NO_METADATA.to_owned()]);
    };
    let mut recipients = Vec::new();
    collect_recipients(metadata, &mut recipients)?;
    if recipients.is_empty() {
        recipients.push(NO_METADATA.into());
    }
    recipients.sort();
    recipients.dedup();
    Ok(recipients)
}

fn parse_public_document(text: &str) -> Result<Value> {
    let ini = text.lines().any(|line| line.trim() == "[sops]");
    let dotenv = text
        .lines()
        .any(|line| line.starts_with("sops_") && line.contains('='));
    if !ini && !dotenv {
        return serde_norway::from_str(text).map_err(|cause| Error::SopsDocumentUnreadable {
            cause: cause.to_string(),
        });
    }
    let mut document = serde_norway::Mapping::new();
    let mut metadata = serde_norway::Mapping::new();
    let mut section = "DEFAULT";
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if ini
            && let Some(name) = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
        {
            section = name;
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| Error::SopsDocumentUnreadable {
                cause: "invalid encrypted INI or dotenv assignment".into(),
            })?;
        let key = key.trim();
        let value = Value::String(value.trim().to_owned());
        let metadata_key = if ini {
            (section == "sops").then_some(key)
        } else {
            key.strip_prefix("sops_")
        };
        if let Some(key) = metadata_key {
            if metadata.insert(Value::String(key.into()), value).is_some() {
                return Err(Error::SopsStanzaUnreadable);
            }
        } else if ini {
            let fields = document
                .entry(Value::String(section.into()))
                .or_insert_with(|| Value::Mapping(serde_norway::Mapping::new()));
            let fields = fields.as_mapping_mut().ok_or(Error::SopsStanzaUnreadable)?;
            if fields.insert(Value::String(key.into()), value).is_some() {
                return Err(Error::SopsStanzaUnreadable);
            }
        } else if document.insert(Value::String(key.into()), value).is_some() {
            return Err(Error::SopsStanzaUnreadable);
        }
    }
    document.insert(Value::String("sops".into()), Value::Mapping(metadata));
    Ok(Value::Mapping(document))
}

fn collect_recipients(
    metadata: &serde_norway::Mapping,
    recipients: &mut Vec<String>,
) -> Result<()> {
    for (kind, field, prefix) in [("age", "recipient", ""), ("pgp", "fp", "pgp:")] {
        if let Some(stanzas) = metadata.get(Value::String(kind.into())) {
            for stanza in stanzas.as_sequence().ok_or(Error::SopsStanzaUnreadable)? {
                let recipient = stanza
                    .as_mapping()
                    .and_then(|fields| fields.get(Value::String(field.into())))
                    .and_then(Value::as_str)
                    .ok_or(Error::SopsStanzaUnreadable)?;
                recipients.push(format!("{prefix}{recipient}"));
            }
        }
    }
    if let Some(groups) = metadata.get(Value::String("key_groups".into())) {
        let groups = groups.as_sequence().ok_or(Error::SopsStanzaUnreadable)?;
        if groups.len() > 1 {
            recipients.push(THRESHOLD_GROUPS.into());
        }
        for group in groups {
            collect_recipients(
                group.as_mapping().ok_or(Error::SopsStanzaUnreadable)?,
                recipients,
            )?;
        }
    }
    let mut flattened = BTreeMap::new();
    let mut flattened_groups = BTreeSet::new();
    for (key, value) in metadata {
        let Some(key) = key.as_str() else {
            return Err(Error::SopsStanzaUnreadable);
        };
        if let Some(group) = key.strip_prefix("key_groups__list_") {
            flattened_groups.insert(
                group
                    .split("__map_")
                    .next()
                    .ok_or(Error::SopsStanzaUnreadable)?,
            );
        }
        for provider in ["kms", "gcp_kms", "hckms", "azure_kv", "hc_vault"] {
            if (key == provider && value.as_sequence().is_none_or(|rows| !rows.is_empty()))
                || key.starts_with(&format!("{provider}__list_"))
                || key.contains(&format!("__map_{provider}__list_"))
            {
                recipients.push(format!("<unsupported sops {provider} recipients>"));
            }
        }
        let Some((stem, field)) = key.rsplit_once("__map_") else {
            continue;
        };
        let kind = if stem.starts_with("age__list_") || stem.contains("__map_age__list_") {
            "age"
        } else if stem.starts_with("pgp__list_") || stem.contains("__map_pgp__list_") {
            "pgp"
        } else {
            continue;
        };
        let found = flattened.entry(stem).or_insert(None);
        if (kind == "age" && field == "recipient") || (kind == "pgp" && field == "fp") {
            let recipient = value.as_str().ok_or(Error::SopsStanzaUnreadable)?;
            *found = Some(if kind == "pgp" {
                format!("pgp:{recipient}")
            } else {
                recipient.to_owned()
            });
        }
    }
    if flattened_groups.len() > 1 {
        recipients.push(THRESHOLD_GROUPS.into());
    }
    for recipient in flattened.into_values() {
        recipients.push(recipient.ok_or(Error::SopsStanzaUnreadable)?);
    }
    Ok(())
}

/// Which recipients each side holds that the other does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    /// Can open the file and is not in its audience.
    pub extra: Vec<String>,
    /// Is in the audience and cannot open the file.
    pub missing: Vec<String>,
}

impl Drift {
    /// Whether the two sides agree.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.extra.is_empty() && self.missing.is_empty()
    }
}

/// The two-way difference between what a file names and what its audience
/// declares.
#[must_use]
pub fn drift(actual: &[String], declared: &[String]) -> Drift {
    let actual: BTreeSet<&String> = actual.iter().collect();
    let declared: BTreeSet<&String> = declared.iter().collect();
    Drift {
        extra: actual
            .difference(&declared)
            .map(|key| (*key).clone())
            .collect(),
        missing: declared
            .difference(&actual)
            .map(|key| (*key).clone())
            .collect(),
    }
}

/// Whether a leaf value is sops's encryption of the empty string.
///
/// `ENC[AES256_GCM,data:<base64>,iv:...,tag:...,type:str]` is the envelope sops
/// writes around a leaf. An empty `data:` segment is the encryption of the
/// empty string: AES-GCM is a stream cipher construction, so ciphertext length
/// equals plaintext length and zero bytes in means zero bytes out.
///
/// The equivalent of the python reader's `^ENC\[[A-Z0-9_]+,data:,`, spelled out
/// rather than compiled, because one anchored prefix does not earn a regular
/// expression engine in the dependency graph.
fn is_empty_ciphertext(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("ENC[") else {
        return false;
    };
    let algorithm: String = rest
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    if algorithm.is_empty() {
        return false;
    }
    rest.get(algorithm.len()..)
        .is_some_and(|tail| tail.starts_with(",data:,"))
}

/// One top-level key of a sops document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyState {
    /// Whether the key's ciphertext encrypts the empty string.
    ///
    /// The file `set` creates through sops for a name with no value yet holds
    /// exactly that, so this is the difference between "no value" and "no
    /// value, and a file already exists to put one in".
    pub empty: bool,
}

/// The document's top-level data keys, each flagged empty or not, in key order.
///
/// A document that is not a mapping has no keys to report and yields an empty
/// result rather than failing, for the reason [`NO_METADATA`] exists. Keys that
/// are not strings are not reported: sops writes none, and a mapping keyed by
/// anything else is not a document this can say anything true about.
///
/// # Errors
///
/// [`Error::SopsDocumentUnreadable`] when the bytes are not YAML.
pub fn keys_of(text: &str) -> Result<BTreeMap<String, KeyState>> {
    fn visit(value: &Value, path: String, keys: &mut BTreeMap<String, KeyState>) {
        if let Some(mapping) = value.as_mapping() {
            if mapping.len() == 1
                && mapping
                    .get(Value::String("__safix_bytes_v1".into()))
                    .is_some_and(Value::is_sequence)
            {
                keys.insert(path, KeyState { empty: false });
                return;
            }
            for (key, value) in mapping {
                if let Some(key) = key.as_str() {
                    visit(value, format!("{path}/{key}"), keys);
                }
            }
        } else if let Some(values) = value.as_sequence() {
            for (index, value) in values.iter().enumerate() {
                visit(value, format!("{path}/{index}"), keys);
            }
        } else {
            keys.insert(
                path,
                KeyState {
                    empty: value.as_str().is_some_and(is_empty_ciphertext),
                },
            );
        }
    }
    let document = parse_public_document(text)?;
    let mut keys = BTreeMap::new();
    if let Some(mapping) = document.as_mapping() {
        for (key, value) in mapping {
            if let Some(key) = key.as_str()
                && key != "sops"
            {
                visit(value, key.to_owned(), &mut keys);
            }
        }
    }
    Ok(keys)
}

/// Return a native update index only when existing metadata guarantees that
/// the selected value stays encrypted. Other policies require fresh encryption.
pub(crate) fn encrypted_update_index(text: &str, key: &str) -> Result<Option<String>> {
    let document = parse_public_document(text)?;
    let Some(metadata) = document.get("sops").and_then(Value::as_mapping) else {
        return Ok(None);
    };
    let field = |name: &str| {
        metadata
            .get(Value::String(name.into()))
            .and_then(Value::as_str)
    };
    if [
        "unencrypted_regex",
        "encrypted_suffix",
        "unencrypted_comment_regex",
        "encrypted_comment_regex",
    ]
    .iter()
    .any(|name| field(name).is_some_and(|value| !value.is_empty()))
    {
        return Ok(None);
    }
    let all_encrypted = field("encrypted_regex") == Some(".*");
    let default_policy = field("encrypted_regex").is_none_or(str::is_empty)
        && field("unencrypted_suffix") == Some("_unencrypted")
        && key.split('/').all(|part| !part.ends_with("_unencrypted"));
    if !all_encrypted && !default_policy {
        return Ok(None);
    }
    if all_encrypted && field("unencrypted_suffix").is_some_and(|value| !value.is_empty()) {
        return Ok(None);
    }

    let invalid = || Error::SopsKeyIndex {
        key: key.into(),
        cause: "the key crosses an incompatible document value".into(),
    };
    let mut here = Some(&document);
    let mut index = String::new();
    for component in key.split('/') {
        index.push('[');
        match here {
            Some(Value::Sequence(values)) => {
                let number = component.parse::<usize>().map_err(|_| invalid())?;
                here = Some(values.get(number).ok_or_else(invalid)?);
                index.push_str(&number.to_string());
            }
            Some(Value::Mapping(values)) => {
                here = values.get(Value::String(component.into()));
                index.push_str(&serde_json::to_string(component).map_err(|_| invalid())?);
            }
            None => {
                index.push_str(&serde_json::to_string(component).map_err(|_| invalid())?);
            }
            Some(_) => return Err(invalid()),
        }
        index.push(']');
    }
    Ok(Some(index))
}

/// Whether the document holds a value at this `/`-nested key path.
///
/// Read off the cleartext structure, which is the whole point: sops enciphers
/// leaf values and leaves the mapping keys in the clear, so whether a declared
/// key exists is answerable without an identity. That is what lets the
/// installer's document check mode run inside a nix build sandbox, where there
/// is no key and decryption is not a thing that could be attempted.
///
/// A segment that resolves to something other than a mapping answers `false`
/// rather than failing: the declared path does not reach a value either way,
/// and the caller's refusal names the path it asked for.
///
/// # Errors
///
/// [`Error::SopsDocumentUnreadable`] when the bytes are not YAML.
pub fn holds_key(text: &str, key: &str) -> Result<bool> {
    let document = parse_public_document(text)?;

    if key.is_empty() {
        return Ok(true);
    }
    let mut here = &document;
    for segment in key.split('/') {
        let next = match here {
            Value::Mapping(mapping) => mapping.get(Value::String(segment.to_owned())),
            Value::Sequence(values) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| values.get(index)),
            _ => None,
        };
        let Some(next) = next else {
            return Ok(false);
        };
        here = next;
    }
    Ok((!here.is_mapping() && !here.is_sequence())
        || here.as_mapping().is_some_and(|fields| {
            fields.len() == 1
                && fields
                    .get(Value::String("__safix_bytes_v1".into()))
                    .is_some_and(Value::is_sequence)
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRAPPED: &str = r#"
alice_alone: ENC[AES256_GCM,data:abc,iv:xyz,tag:t,type:str]
blank: ENC[AES256_GCM,data:,iv:xyz,tag:t,type:str]
sops:
    age:
        - recipient: age1bbb
          enc: |
            -----BEGIN AGE ENCRYPTED FILE-----
        - recipient: age1aaa
          enc: |
            -----BEGIN AGE ENCRYPTED FILE-----
    lastmodified: "2026-08-15T00:00:00Z"
"#;

    #[test]
    fn recipients_come_back_sorted_and_without_the_metadata_block() {
        assert_eq!(recipients_of(WRAPPED).unwrap(), ["age1aaa", "age1bbb"]);
    }

    #[test]
    fn threshold_groups_are_not_reported_as_an_any_recipient_audience() {
        let documents = [
            "sops:\n  shamir_threshold: 2\n  key_groups:\n    - age: [{recipient: age1aaa}]\n    - age: [{recipient: age1bbb}]\n",
            "sops_shamir_threshold=2\nsops_key_groups__list_0__map_age__list_0__map_recipient=age1aaa\nsops_key_groups__list_1__map_age__list_0__map_recipient=age1bbb\n",
        ];
        for document in documents {
            let recipients = recipients_of(document).unwrap();
            assert!(!drift(&recipients, &["age1aaa".into(), "age1bbb".into()]).is_empty());
        }
    }

    #[test]
    fn a_document_with_no_age_metadata_reports_the_sentinel() {
        assert_eq!(recipients_of("just: a mapping\n").unwrap(), [NO_METADATA]);
        assert_eq!(recipients_of("- a sequence\n").unwrap(), [NO_METADATA]);
        assert_eq!(recipients_of("sops:\n  kms: []\n").unwrap(), [NO_METADATA]);
    }

    #[test]
    fn a_stanza_with_no_recipient_is_refused_rather_than_reported_as_none() {
        let broken = "sops:\n    age:\n        - enc: something\n";
        assert!(recipients_of(broken).is_err());
    }

    #[test]
    fn keys_exclude_the_metadata_block_and_flag_the_empty_ciphertext() {
        let keys = keys_of(WRAPPED).unwrap();
        assert_eq!(keys.keys().collect::<Vec<_>>(), ["alice_alone", "blank"]);
        assert_eq!(keys.get("alice_alone"), Some(&KeyState { empty: false }));
        assert_eq!(keys.get("blank"), Some(&KeyState { empty: true }));
    }

    #[test]
    fn a_document_that_is_not_a_mapping_has_no_keys() {
        assert!(keys_of("- one\n- two\n").unwrap().is_empty());
        assert!(keys_of("").unwrap().is_empty());
    }

    #[test]
    fn only_an_enc_envelope_with_an_algorithm_and_an_empty_data_segment_is_empty() {
        assert!(is_empty_ciphertext("ENC[AES256_GCM,data:,iv:x]"));
        assert!(is_empty_ciphertext("ENC[A,data:,"));
        assert!(!is_empty_ciphertext("ENC[,data:,iv:x]"));
        assert!(!is_empty_ciphertext("ENC[AES256_GCM,data:a,iv:x]"));
        assert!(!is_empty_ciphertext("ENC[aes,data:,iv:x]"));
        assert!(!is_empty_ciphertext("a plain string"));
    }

    #[test]
    fn drift_is_the_two_way_difference_in_key_order() {
        let found = drift(
            &["age1a".into(), "age1stray".into()],
            &["age1a".into(), "age1missing".into()],
        );
        assert_eq!(found.extra, ["age1stray"]);
        assert_eq!(found.missing, ["age1missing"]);
        assert!(!found.is_empty());
        assert!(drift(&["age1a".into()], &["age1a".into()]).is_empty());
    }
}

#[cfg(test)]
mod properties {
    use std::collections::BTreeSet;

    use proptest::prelude::*;

    use super::{drift, is_empty_ciphertext};

    const KEY: &str = "age1[a-z0-9]{0,12}";

    proptest! {
        /// Drift is the two-way set difference, and it is symmetric under
        /// swapping the sides: what one call reports as extra the mirrored call
        /// reports as missing. A report that named the same key on both sides
        /// would be telling an operator to add and remove one recipient.
        #[test]
        fn drift_is_the_two_way_difference_and_mirrors_under_a_swap(
            actual in proptest::collection::vec(KEY, 0..6),
            declared in proptest::collection::vec(KEY, 0..6),
        ) {
            let found = drift(&actual, &declared);
            let mirrored = drift(&declared, &actual);
            prop_assert_eq!(&found.extra, &mirrored.missing);
            prop_assert_eq!(&found.missing, &mirrored.extra);

            let extra: BTreeSet<&String> = found.extra.iter().collect();
            let missing: BTreeSet<&String> = found.missing.iter().collect();
            prop_assert!(extra.intersection(&missing).next().is_none());

            let mut sorted = found.extra.clone();
            sorted.sort();
            sorted.dedup();
            prop_assert_eq!(&sorted, &found.extra);
        }

        /// Two sides holding the same keys never drift, whatever order or
        /// repetition they arrive in: the report is about sets, and a recipient
        /// listed twice is one recipient.
        #[test]
        fn one_set_never_drifts_from_itself(keys in proptest::collection::vec(KEY, 0..6)) {
            let mut shuffled = keys.clone();
            shuffled.reverse();
            shuffled.extend(keys.clone());
            prop_assert!(drift(&keys, &shuffled).is_empty());
        }

        /// The envelope test accepts exactly what the python reader's anchored
        /// pattern accepts, stated here against an independently written
        /// predicate rather than against the implementation restated.
        #[test]
        fn the_empty_envelope_test_matches_an_independent_reading(
            algorithm in "[A-Z0-9_]{0,8}",
            data in "[a-zA-Z0-9]{0,6}",
        ) {
            let value = format!("ENC[{algorithm},data:{data},iv:x,tag:y,type:str]");
            let expected = !algorithm.is_empty() && data.is_empty();
            prop_assert_eq!(is_empty_ciphertext(&value), expected);
        }
    }
}
