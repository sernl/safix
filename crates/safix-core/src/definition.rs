//! The definition a value was minted under, recorded beside the tree.
//!
//! A generated value carries nothing saying which declaration produced it, so a
//! later edit to that declaration is invisible: the value in the file is a
//! function of a generator that no longer exists, and reads exactly like a value
//! the current one would produce. This module is the record that makes the
//! difference detectable — a digest of the generator's declaration, written in
//! the same commit as the value, read back by [`crate::check`].
//!
//! # Where the record sits, and why it is a third tree
//!
//! ```text
//! state/safix/definitions/<user>/<name>
//! state/safix/definitions/shared/<audience>/<name>
//! ```
//!
//! Neither existing tree can hold it. A path named `secrets` has to mean that
//! everything under it is encrypted, without qualification, because that is the
//! proposition every backup rule, every sync exclusion and every reviewer
//! applies to it — the same reason [`crate::public`] gives for sitting outside
//! it. `public/` is the other candidate and is worse: that prefix means declared
//! public *outputs*, values a nix module reads at evaluation, and putting a
//! bookkeeping file there dilutes it into "plaintext things safix wrote".
//!
//! So a third top-level prefix, named for what it holds: recorded state about
//! the tree, neither a secret nor an output. It is plaintext, it is committed,
//! and it is one line per file.
//!
//! Two alternatives were refused, and are recorded in
//! `openspec/changes/settle-clan-vars-parity/design.md`. A reserved key inside
//! the sops document would put the record behind decryption, which would make
//! `check` decrypt in order to answer a question about declarations — the one
//! property `check` exists not to have. Deriving drift from git history needs no
//! record at all, but a refactor that moved or reformatted a declaration would
//! report drift that is not there, and renaming the defining file would hide
//! drift that is.
//!
//! # What the digest covers
//!
//! The generator record as the runtime receives it, minus the two fields that
//! cannot change what a mint does: `description`, which is a label `list` prints,
//! and `share`, which the resolver derives from the entries rather than from the
//! generator. What is left is everything that decides what a run produces — the
//! script, the tools on its `PATH`, the network grant that decides what the script
//! may reach, the prompts it asks, the dependencies it reads, the outputs it
//! writes with their secrecy, and the validation that judges a candidate.
//!
//! `network` is covered because a grant is a change to what a mint may do, not
//! only to what it happens to do. The value in the file was produced by a fragment
//! that could not reach the network; a declaration that now grants one describes a
//! mint that can, and the two are not the same mint. A record ignoring the grant
//! would call them the same and the flip would be the invisible edit this whole
//! module exists to make visible.
//!
//! An entry's on-disk `mode` is not covered, and cannot be: it is a registry
//! field belonging to the entry rather than to the generator, it does not travel
//! on [`Generator`], and it decides where a decrypted value lands rather than
//! what the mint produces.
//!
//! No value and no derivative of a value enters the record. That is what lets it
//! be committed in the clear, and the canonical form is written to make that
//! evident: everything it reads comes off the declaration.

use std::fmt::Write as _;

use crate::digest::sha256_hex;
use crate::model::{Generator, Placement};

/// The prefix every definition record is under, stated once so a check can
/// quote it.
pub const PREFIX: &str = "state/safix/definitions/";

/// The tag every record file begins with, naming the canonical form the digest
/// was computed over.
///
/// A record is read only when this tag is the one it carries. That is what keeps
/// a change to the canonical form from reading as universal drift: the old records
/// become unknown-version rather than mismatched, and an unknown version is no
/// finding at all. Changing the canonical form means moving this tag in the same
/// commit.
///
/// `v2` is the first exercise of that rule. `v1` covered every field of the
/// generator record that existed when it was written; `network` arrived after it,
/// and covering a new field changes every digest, so the tag moved with it. A `v1`
/// record therefore reads as unknown-version and produces no finding, which is the
/// grandfathering the mechanism was built for rather than a special case for this
/// change.
pub const FORMAT: &str = "safix-definition-v2";

/// The digest of one generator's declaration, as this format records it.
#[must_use]
pub fn digest(record: &Generator) -> String {
    sha256_hex(canonical(record).as_bytes())
}

/// The line a record file holds: the format tag, the digest, and a newline.
#[must_use]
pub fn line(record: &Generator) -> String {
    format!("{FORMAT} {}\n", digest(record))
}

/// The digest a record file records, or nothing when it records none this format
/// can read.
///
/// Nothing is the answer for an empty file, a file whose first token is another
/// format's tag, and a file with no digest after the tag. Each of those is a
/// record this version has nothing to say about, and saying nothing is the
/// documented behaviour rather than a fallback: a finding derived from a record
/// this code cannot read would be a finding about the reader.
#[must_use]
pub fn recorded(text: &str) -> Option<&str> {
    let (tag, rest) = text.trim_end_matches('\n').split_once(' ')?;
    if tag != FORMAT {
        return None;
    }
    let digest = rest.trim();
    if digest.is_empty() {
        return None;
    }
    Some(digest)
}

/// Where the record for one entry lives, repository-relative.
///
/// Two shapes, because a shared entry is one value and a private one is a value
/// per person. A shared name's record is keyed by the directory its audience
/// reads — which is that audience's own name, joined in sorted order by the
/// resolver — so that both carriers resolve one record for the one value they
/// share. Keying it by the carrier would write one record per carrier and then
/// report drift for whichever of them did not mint.
///
/// Everything else is keyed by the entry's owner rather than by whoever holds
/// it, so that a name one person owns and another was granted has one record
/// too. For an entry a user carries or declares privately the owner *is* that
/// user, which is the ordinary case.
#[must_use]
pub fn record_path(name: &str, placement: &Placement) -> String {
    if placement.shared {
        let audience = audience_directory(&placement.file);
        return format!("{PREFIX}shared/{audience}/{name}");
    }
    format!("{PREFIX}{owner}/{name}", owner = placement.owner)
}

/// The last component of the directory a file sits in, which for a shared
/// entry's document is the audience's own name.
///
/// Total, and the degenerate answer is the empty string: a file at the repository
/// root sits in no directory and so names no audience. `resolve.nix` places a
/// shared entry at `secrets/safix/shared/<audience>/`, so nothing it emits reaches
/// that branch, and an embedder that handed one over would get a record path with
/// an empty segment rather than a refusal.
fn audience_directory(file: &str) -> &str {
    let directory = match file.rfind('/') {
        None => "",
        Some(index) => file.get(..index).unwrap_or(""),
    };
    match directory.rfind('/') {
        None => directory,
        Some(index) => directory
            .get(index.saturating_add(1)..)
            .unwrap_or(directory),
    }
}

/// The canonical byte form of one generator's declaration.
///
/// Every string is written with its length in front of it, so no field's content
/// can be spelled to look like the start of the next one: two declarations
/// produce one encoding only when they are equal field for field. Every
/// collection is written with its count in front, and the two maps are
/// [`std::collections::BTreeMap`]s, so the order is the declaration's own rather
/// than a hash map's.
///
/// It is one string rather than a serialisation of the record because it is read
/// by nothing: a byte form nobody parses can be written for injectivity alone,
/// and a reader is what would make its shape a compatibility surface.
fn canonical(record: &Generator) -> String {
    let mut out = String::new();
    out.push_str(FORMAT);
    out.push('\n');

    field(&mut out, "script", &record.script);
    match &record.validation {
        Some(validation) => field(&mut out, "validation", validation),
        None => out.push_str("validation absent\n"),
    }

    // Beside the two fragments it governs rather than among the collections: the
    // grant applies to the script and the validation alike, which is what it has
    // in common with them and not with a list of tool names.
    field(&mut out, "network", if record.network { "1" } else { "0" });

    count(&mut out, "runtimeInputs", record.runtime_inputs.len());
    for input in &record.runtime_inputs {
        field(&mut out, "runtimeInput", input);
    }

    count(&mut out, "dependencies", record.dependencies.len());
    for dependency in &record.dependencies {
        field(&mut out, "dependency", dependency);
    }

    count(&mut out, "prompts", record.prompts.len());
    for (name, prompt) in &record.prompts {
        field(&mut out, "prompt", name);
        field(&mut out, "promptKind", kind_of(prompt.kind));
        field(&mut out, "promptDescription", &prompt.description);
    }

    count(&mut out, "files", record.files.len());
    for (name, file) in &record.files {
        field(&mut out, "file", name);
        field(&mut out, "fileSecret", if file.secret { "1" } else { "0" });
    }

    out
}

/// One named string, length-prefixed.
fn field(out: &mut String, name: &str, value: &str) {
    let _ = writeln!(out, "{name} {} {value}", value.len());
}

/// One collection's element count.
fn count(out: &mut String, name: &str, many: usize) {
    let _ = writeln!(out, "{name} {many}");
}

/// A prompt's kind, as the canonical form spells it.
///
/// Spelled here rather than through a `Display` on the type: this is the digest's
/// own vocabulary, and a rendering meant for an operator that later gained a
/// word would silently move every recorded digest.
const fn kind_of(kind: crate::model::PromptKind) -> &'static str {
    match kind {
        crate::model::PromptKind::Hidden => "hidden",
        crate::model::PromptKind::Line => "line",
        crate::model::PromptKind::Multiline => "multiline",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{FORMAT, PREFIX, canonical, digest, line, record_path, recorded};
    use crate::model::{Generator, Placement};

    /// The declaration every case below perturbs one field of, as the resolver
    /// emits it.
    fn document() -> serde_json::Value {
        json!({
            "dependencies": ["base-pub"],
            "description": "a token",
            "files": { "api-token-pub": { "secret": false } },
            "prompts": { "seed": { "type": "hidden", "description": "the seed" } },
            "network": false,
            "runtimeInputs": ["coreutils", "openssl"],
            "script": "printf '%s' fixture > \"$out/api-token\"",
            "share": false,
            "validation": "test -s /dev/stdin",
        })
    }

    fn generator() -> Generator {
        serde_json::from_value(document()).expect("the fixture is the shape the resolver emits")
    }

    /// The digest of the record with one field replaced by this JSON value.
    fn with(field: &str, value: serde_json::Value) -> String {
        let mut perturbed = document();
        perturbed
            .as_object_mut()
            .expect("the fixture is an object")
            .insert(field.to_owned(), value);
        let record: Generator =
            serde_json::from_value(perturbed).expect("the perturbation is still the schema");
        digest(&record)
    }

    /// Every field the digest claims to cover moves it.
    ///
    /// One case per field of the canonical form, because the failure this is
    /// written against is a field silently left out: the encoding would still be
    /// injective over what it did read, and an edit to the omitted field would
    /// read as no edit at all.
    #[test]
    fn an_edit_to_each_covered_field_changes_the_digest() {
        let base = digest(&generator());

        assert_ne!(
            base,
            with("script", json!("printf '%s' other > \"$out/api-token\""))
        );
        assert_ne!(base, with("runtimeInputs", json!(["coreutils"])));
        assert_ne!(base, with("runtimeInputs", json!(["openssl", "coreutils"])));
        assert_ne!(base, with("network", json!(true)));
        assert_ne!(base, with("dependencies", json!([])));
        assert_ne!(base, with("validation", json!(null)));
        assert_ne!(base, with("validation", json!("test -n /dev/stdin")));
        assert_ne!(
            base,
            with(
                "prompts",
                json!({ "seed": { "type": "line", "description": "the seed" } })
            )
        );
        assert_ne!(
            base,
            with(
                "prompts",
                json!({ "seed": { "type": "hidden", "description": "a seed" } })
            )
        );
        assert_ne!(
            base,
            with(
                "prompts",
                json!({ "salt": { "type": "hidden", "description": "the seed" } })
            )
        );
        assert_ne!(base, with("files", json!({})));
        assert_ne!(
            base,
            with("files", json!({ "api-token-pub": { "secret": true } }))
        );
        assert_ne!(
            base,
            with("files", json!({ "api-token-key": { "secret": false } }))
        );
    }

    /// The two fields the digest deliberately does not cover.
    ///
    /// A description is a label `list` prints and a `share` is derived by the
    /// resolver from the entries, so neither changes what a mint does. Asserted
    /// rather than left implicit: it is the half of the coverage claim that says
    /// what a regeneration is *not* asked for.
    #[test]
    fn the_two_uncovered_fields_leave_the_digest_alone() {
        let base = digest(&generator());
        assert_eq!(base, with("description", json!("a different label")));
        assert_eq!(base, with("description", json!(null)));
        assert_eq!(base, with("share", json!(true)));
    }

    /// Two reads of one declaration agree, which is what makes a mismatch mean
    /// an edit rather than a rerun.
    #[test]
    fn one_declaration_digests_the_same_twice() {
        assert_eq!(digest(&generator()), digest(&generator()));
        assert_eq!(digest(&generator()).len(), 64);
    }

    /// No field's content can be spelled to look like the next field.
    ///
    /// Without the lengths, a script ending in what reads as the start of the
    /// validation line would collide with a declaration that really is split
    /// that way. The two records here differ only in where one boundary falls.
    #[test]
    fn a_field_boundary_cannot_be_forged_from_inside_a_field() {
        let left = digest(&{
            let mut record = generator();
            record.script = "a".to_owned();
            record.dependencies = vec!["bc".to_owned()];
            record
        });
        let right = digest(&{
            let mut record = generator();
            record.script = "a".to_owned();
            record.dependencies = vec!["b".to_owned(), "c".to_owned()];
            record
        });
        assert_ne!(left, right);
    }

    /// The canonical form quotes no value, and there is none in it to quote.
    ///
    /// The generator's own fields are the only thing that reaches it, which is
    /// asserted here by reading the form itself: every line's content comes off
    /// the declaration above.
    #[test]
    fn the_canonical_form_is_the_declaration_and_the_format_tag() {
        let text = canonical(&generator());
        assert!(text.starts_with(FORMAT));
        assert!(text.contains("script 38 printf '%s' fixture > \"$out/api-token\""));
        assert!(text.contains("promptKind 6 hidden"));
        assert!(text.contains("fileSecret 1 0"));
        assert!(text.contains("runtimeInputs 2"));
        assert!(text.contains("network 1 0"));
    }

    /// The grant, and the tag that moved with it.
    ///
    /// One case says the two grants digest apart, which is the coupling this
    /// version exists for: a generator that gains the network describes a mint
    /// that may do something the recorded one could not, so the record has to
    /// move. The other says the tag names v2 and that a v1 record is not read,
    /// which is what keeps every value minted before this from reading as
    /// drifted.
    #[test]
    fn the_network_grant_is_covered_and_the_tag_moved_with_it() {
        let confined = digest(&generator());
        let granted = with("network", json!(true));
        assert_ne!(
            confined, granted,
            "a generator gaining the network digests the same as one without it"
        );

        assert_eq!(FORMAT, "safix-definition-v2");
        assert!(line(&generator()).starts_with("safix-definition-v2 "));
        assert_eq!(
            recorded(&format!("safix-definition-v1 {confined}\n")),
            None,
            "a record written before the grant was covered is read as this version's"
        );
    }

    /// The record file's own line: the tag, one space, the digest, one newline.
    #[test]
    fn a_record_line_round_trips_through_the_reader() {
        let record = generator();
        let text = line(&record);
        assert_eq!(text, format!("{FORMAT} {}\n", digest(&record)));
        assert_eq!(recorded(&text), Some(digest(&record).as_str()));
    }

    /// What the reader says nothing about.
    #[test]
    fn a_record_this_format_cannot_read_records_nothing() {
        assert_eq!(recorded(""), None);
        assert_eq!(recorded("\n"), None);
        assert_eq!(recorded("safix-definition-v1 abc\n"), None);
        assert_eq!(recorded("safix-definition-v3 abc\n"), None);
        assert_eq!(recorded("abc\n"), None);
        assert_eq!(recorded("safix-definition-v2 \n"), None);
        assert_eq!(recorded("safix-definition-v2 abc"), Some("abc"));
    }

    fn placement(file: &str, owner: &str, shared: bool) -> Placement {
        serde_json::from_value(json!({
            "file": file, "key": "k", "origin": "private",
            "owner": owner, "shared": shared, "generator": null, "public": null,
        }))
        .expect("the fixture is the shape the resolver emits")
    }

    /// A private entry's record is under its owner, a shared one's under the
    /// audience both carriers read.
    #[test]
    fn a_record_path_is_keyed_by_owner_or_by_audience() {
        assert_eq!(
            record_path(
                "api-token",
                &placement("secrets/safix/users/alice/secrets.yaml", "alice", false)
            ),
            "state/safix/definitions/alice/api-token"
        );

        // The granted case: bob owns it, alice holds it, and both resolve one
        // record.
        assert_eq!(
            record_path(
                "wifi-psk",
                &placement("secrets/safix/shared/alice,bob/secrets.yaml", "bob", false)
            ),
            "state/safix/definitions/bob/wifi-psk"
        );

        // The shared case: each carrier is its own placement's owner, so the
        // audience directory is what makes the two agree.
        for owner in ["alice", "bob"] {
            assert_eq!(
                record_path(
                    "fleet-token",
                    &placement("secrets/safix/shared/alice,bob/secrets.yaml", owner, true)
                ),
                "state/safix/definitions/shared/alice,bob/fleet-token"
            );
        }
    }

    /// Every record path is under the prefix, and under neither of the other two
    /// trees.
    #[test]
    fn no_record_path_reaches_the_secret_or_the_public_tree() {
        for placement in [
            placement("secrets/safix/users/alice/secrets.yaml", "alice", false),
            placement("secrets/safix/shared/alice,bob/secrets.yaml", "bob", true),
        ] {
            let path = record_path("api-token", &placement);
            assert!(path.starts_with(PREFIX), "{path} is outside {PREFIX}");
            assert!(!path.starts_with("secrets/"), "{path} is under secrets/");
            assert!(!path.starts_with(crate::public::PREFIX), "{path} is public");
        }
    }
}
