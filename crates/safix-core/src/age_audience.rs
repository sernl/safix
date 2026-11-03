//! The audience a raw age file was last wrapped to, recorded beside it.
//!
//! A raw age header names its SSH recipients by a four-byte tag and every
//! other recipient by nothing (see [`crate::age_header`]), so `fix` cannot
//! tell from the file whether it is already wrapped to the declared audience,
//! and age output is randomised, so re-wrapping it anyway rewrites every raw
//! age file in the tree on every run. This record is what lets it tell: one
//! line, written by `fix` after each raw age re-wrap,
//!
//! ```text
//! safix-age-audience-v1 <audience digest> <ciphertext digest>
//! ```
//!
//! where the audience digest is over the recipients the file was wrapped to
//! and the ciphertext digest is over the bytes that wrap produced. A file
//! whose record reads, names the declared audience and names the file's
//! current bytes is already wrapped to that audience, and `fix` leaves it
//! alone.
//!
//! Both digests are needed. A file re-encrypted by hand to some other roster
//! still matches an audience digest alone, and a record with only that half
//! would make `fix` skip a file whose roster it has never seen.
//!
//! The record carries no value and no derivative of one: the audience digest
//! is over recipient strings already in the declarations, and the ciphertext
//! digest is over bytes already committed. That is what lets it be committed
//! in the clear, beside the definition records.
//!
//! # Where it sits, and who writes it
//!
//! The path is the resolver's, on [`Placement::audience_record`]: under the
//! default root it is the entry's definition record path with an `.audience`
//! suffix, and in vault mode one opaque name. Only `fix` writes it. `set`,
//! `edit`, `generate` and `sync` change a raw age file's bytes and leave the
//! record alone, so the next `fix --yes` re-wraps that file once and records
//! it — one extra re-wrap per content write, against teaching four writers a
//! record whose only reader is `fix`.
//!
//! [`Placement::audience_record`]: crate::model::Placement::audience_record

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::digest::sha256_hex;
use crate::error::{Error, Result};
use crate::model::Placements;
use crate::scratch;
use crate::workspace::Workspace;

/// The tag every audience record begins with, naming the canonical form the
/// audience digest is computed over and the line's shape.
///
/// A record with any other tag is unreadable and says nothing, by the rule
/// [`crate::definition::FORMAT`] states: changing either canonical form means
/// moving this tag in the same commit.
pub(crate) const FORMAT: &str = "safix-age-audience-v1";

/// The digest of an audience: who a file is wrapped to, independent of the
/// order the recipients were named in.
///
/// Sorted and deduplicated, because the order of `--recipient` arguments and a
/// key named twice do not change who can open the file. Then written as
/// [`crate::definition`] writes its canonical form — a count in front, each
/// recipient with its length in front — so no recipient can be spelled to look
/// like the boundary between two.
#[must_use]
pub(crate) fn audience_digest(recipients: &[String]) -> String {
    let mut distinct: Vec<&str> = recipients.iter().map(String::as_str).collect();
    distinct.sort_unstable();
    distinct.dedup();
    let mut canonical = String::new();
    canonical.push_str(FORMAT);
    canonical.push('\n');
    let _ = writeln!(canonical, "recipients {}", distinct.len());
    for recipient in distinct {
        let _ = writeln!(canonical, "recipient {} {recipient}", recipient.len());
    }
    sha256_hex(canonical.as_bytes())
}

/// The digest of a file's bytes, as the record carries it.
#[must_use]
pub(crate) fn ciphertext_digest(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

/// The line a record holds: the tag, the audience digest, the digest of the
/// ciphertext, and a newline.
#[must_use]
pub(crate) fn line(audience: &str, ciphertext: &[u8]) -> String {
    format!("{FORMAT} {audience} {}\n", ciphertext_digest(ciphertext))
}

/// The `(audience digest, ciphertext digest)` a record holds, or nothing when
/// it is not the line [`line`] writes.
///
/// Every deviation is nothing: another tag, a missing field, a field that is
/// not sixty-four lowercase hexadecimal digits, a trailing field. A record this
/// version cannot read says nothing, which for `fix` means a re-wrap as before
/// and for `check` means no finding.
#[must_use]
pub(crate) fn recorded(text: &str) -> Option<(&str, &str)> {
    let mut fields = text.strip_suffix('\n').unwrap_or(text).split(' ');
    if fields.next()? != FORMAT {
        return None;
    }
    let audience = fields.next().filter(|field| is_digest(field))?;
    let ciphertext = fields.next().filter(|field| is_digest(field))?;
    if fields.next().is_some() {
        return None;
    }
    Some((audience, ciphertext))
}

/// Whether this is a digest as [`sha256_hex`] spells one.
fn is_digest(field: &str) -> bool {
    field.len() == 64
        && field
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The audience record of the raw age entry placed in this file, when a
/// placement names one.
///
/// A raw age entry is always one file of its own, so the first placement
/// naming the file and carrying a record is the one. A file named only
/// through `extraGovernedFiles` has no placement and so no record.
#[must_use]
pub(crate) fn record_for<'a>(placements: &'a Placements, file: &str) -> Option<&'a str> {
    placements
        .0
        .values()
        .flat_map(|held| held.values())
        .filter(|placement| placement.file == file)
        .find_map(|placement| placement.audience_record.as_deref())
}

/// Write this line to the record at this vault-relative path.
///
/// Through a candidate beside the target and a rename, registered with
/// [`scratch`] before it exists, the discipline [`crate::stamps::touch`]
/// writes a stamp under: an interrupted run leaves no half-written record.
///
/// # Errors
///
/// [`Error::FileUnwritable`] when the record or a directory above it cannot be
/// written.
pub(crate) fn write(workspace: &Workspace, relative: &str, line: &str) -> Result<()> {
    let absolute = workspace.vault_absolute(relative);

    let mut name = absolute.as_os_str().to_owned();
    name.push(format!(".safix-tmp.{}", std::process::id()));
    let candidate = PathBuf::from(name);
    scratch::register_file(&candidate);

    if let Some(directory) = absolute.parent()
        && !directory.is_dir()
    {
        scratch::register_dir(directory);
        std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
            path: directory.display().to_string(),
            cause,
        })?;
    }

    std::fs::write(&candidate, line).map_err(|cause| Error::FileUnwritable {
        path: candidate.display().to_string(),
        cause,
    })?;
    std::fs::rename(&candidate, &absolute).map_err(|cause| Error::FileUnwritable {
        path: absolute.display().to_string(),
        cause,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audience(recipients: &[&str]) -> String {
        audience_digest(
            &recipients
                .iter()
                .map(|recipient| (*recipient).to_owned())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn the_audience_digest_ignores_order_and_duplicates() {
        let forward = audience(&["age1alice", "ssh-ed25519 AAAAbob"]);
        assert_eq!(forward, audience(&["ssh-ed25519 AAAAbob", "age1alice"]));
        assert_eq!(
            forward,
            audience(&["age1alice", "ssh-ed25519 AAAAbob", "age1alice"])
        );
        assert_ne!(forward, audience(&["age1alice"]));
    }

    /// Two audiences whose recipients concatenate to one string are two
    /// digests, because each recipient carries its length.
    #[test]
    fn the_audience_digest_is_injective_over_lengths() {
        assert_ne!(audience(&["ab", "c"]), audience(&["a", "bc"]));
        assert_ne!(audience(&["a b"]), audience(&["a", "b"]));
        assert_ne!(audience(&[]), audience(&[""]));
    }

    #[test]
    fn a_written_line_reads_back() {
        let digest = audience(&["age1alice"]);
        let written = line(&digest, b"ciphertext");
        assert_eq!(
            recorded(&written),
            Some((digest.as_str(), sha256_hex(b"ciphertext").as_str()))
        );
    }

    #[test]
    fn another_tag_reads_as_nothing() {
        let digest = audience(&["age1alice"]);
        let written = line(&digest, b"ciphertext");
        let older = written.replacen(FORMAT, "safix-age-audience-v0", 1);
        assert_eq!(recorded(&older), None);
        let definition = written.replacen(FORMAT, crate::definition::FORMAT, 1);
        assert_eq!(recorded(&definition), None);
    }

    #[test]
    fn a_truncated_line_reads_as_nothing() {
        let digest = audience(&["age1alice"]);
        let written = line(&digest, b"ciphertext");
        let tag = FORMAT.len();
        for cut in [
            0,
            tag,
            tag.saturating_add(1),
            tag.saturating_add(65),
            written.len().saturating_sub(2),
        ] {
            let truncated = written.get(..cut).expect("the cut is on a character");
            assert_eq!(recorded(truncated), None, "{truncated:?} read");
        }
        assert_eq!(recorded(&format!("{} extra\n", written.trim_end())), None);
        assert_eq!(recorded(&written.to_uppercase()), None);
    }
}
