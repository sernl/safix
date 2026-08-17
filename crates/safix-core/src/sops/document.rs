//! What a sops document says about itself without being decrypted.
//!
//! Two questions, both answerable from the bytes: which age keys a file's
//! ciphertext is wrapped for, and which keys the document holds with which of
//! them hold nothing. sops leaves the document's shape in the clear — only leaf
//! values are enciphered — so the key names, the stanza list, and the fact that
//! a key's ciphertext encrypts the empty string are all readable.
//!
//! Nothing here decrypts, holds an identity, or produces plaintext. Every
//! string on every path through this module is either a key name or an age
//! public key, and both are public data. That is what lets `check` judge files
//! belonging to people whose identities the machine running it does not hold.
//!
//! This began as a port of two python readers, `sops_recipients.py` and
//! `sops_keys.py`, which the differential harness held it to agreeing with on
//! every fixture before either was retired. All three are gone from the tree and
//! reachable at 8409f15; this reads the same two fields they read and nothing
//! else about the format, and the format itself stays sops's, per
//! `openspec/changes/rewrite-runtime-in-rust` design decision D6.

use std::collections::{BTreeMap, BTreeSet};

use serde_norway::Value;

use crate::error::{Error, Result};

/// What is reported in place of a recipient list when a governed path holds a
/// document with no sops age metadata at all.
///
/// A sentinel rather than a failure: such a path is either plaintext someone
/// committed by mistake or ciphertext from a store with a different metadata
/// shape, and both have to be reported against the declared audience rather
/// than crash the reader that was asked to inspect them.
pub const NO_METADATA: &str = "<file carries no sops age metadata>";

/// The age recipients a document's ciphertext actually names, in key order.
///
/// # Errors
///
/// [`Error::SopsDocumentUnreadable`] when the bytes are not YAML, and
/// [`Error::SopsStanzaUnreadable`] when the `sops.age` list holds something
/// that is not a mapping carrying a string `recipient`.
pub fn recipients_of(text: &str) -> Result<Vec<String>> {
    let document: Value =
        serde_norway::from_str(text).map_err(|cause| Error::SopsDocumentUnreadable {
            cause: cause.to_string(),
        })?;

    let stanzas = document
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String("sops".into())))
        .and_then(Value::as_mapping)
        .and_then(|metadata| metadata.get(Value::String("age".into())))
        .and_then(Value::as_sequence);

    let Some(stanzas) = stanzas else {
        return Ok(vec![NO_METADATA.to_owned()]);
    };

    let mut recipients = Vec::with_capacity(stanzas.len());
    for stanza in stanzas {
        let recipient = stanza
            .as_mapping()
            .and_then(|fields| fields.get(Value::String("recipient".into())))
            .and_then(Value::as_str)
            .ok_or(Error::SopsStanzaUnreadable)?;
        recipients.push(recipient.to_owned());
    }
    recipients.sort();
    Ok(recipients)
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
    let document: Value =
        serde_norway::from_str(text).map_err(|cause| Error::SopsDocumentUnreadable {
            cause: cause.to_string(),
        })?;

    let Some(mapping) = document.as_mapping() else {
        return Ok(BTreeMap::new());
    };

    Ok(mapping
        .iter()
        .filter_map(|(key, value)| key.as_str().map(|key| (key, value)))
        .filter(|(key, _)| *key != "sops")
        .map(|(key, value)| {
            let empty = value.as_str().is_some_and(is_empty_ciphertext);
            (key.to_owned(), KeyState { empty })
        })
        .collect())
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
