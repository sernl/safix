//! When a value was first written, and when it last changed.
//!
//! A ciphertext document says nothing about its own history: a rewrap moves
//! every byte of it, a shared document holds several values whose writes are
//! months apart, and git's own dates are the dates of commits that touch a
//! file rather than of writes to one key inside it. So the picker had nothing
//! to show for "when did this arrive" and "when did it last move" except the
//! file's mtime, which answers neither question.
//!
//! This module is the record that answers them: two unix seconds per value,
//! written in the same commit as the value itself, read back by the picker.
//! It sits in the generator-record tree beside [`crate::definition`]'s digest
//! because it is the same kind of thing — plaintext bookkeeping about a value,
//! carrying no value and no derivative of one, which is what lets it be
//! committed in the clear.
//!
//! # Where a stamp sits, and who decides
//!
//! Not here. The path arrives on [`Placement::stamp_record`], computed by the
//! resolver from `flake.safix.storage.generatorRecords` the way every other
//! path it emits is computed, so the layout has one implementation rather than
//! one here and one in `resolve.nix` that can disagree. Under the default root
//! that reads
//!
//! ```text
//! state/safix/definitions/<user>/<name>.stamps
//! state/safix/definitions/shared/<audience>/<name>.stamps
//! ```
//!
//! and in vault mode it is one opaque name under the vault's own `state/`
//! bucket. The `.stamps` suffix needs no injectivity argument of its own: a
//! declared name is `[a-z0-9][a-z0-9_-]*`, so no name can end in `.stamps` and
//! no entry's definition record can be another entry's stamp record.
//!
//! # Why a missing record is not a zero
//!
//! Every value written before this record existed has none, and inventing a
//! date for it would be a claim this repository cannot make. [`read`] answers
//! [`None`] for that, and the picker renders it as `-`: no record, no claim —
//! the discipline [`crate::definition`] already applies to a digest it cannot
//! read.
//!
//! A record that exists and does not parse is the opposite case and is a
//! refusal. [`crate::definition::recorded`] can be silent about a digest it
//! cannot read because the worst outcome is an unreported drift; here silence
//! would make [`touch`] mint a fresh `created` over a value that plainly has
//! one, rewriting history rather than failing to report it.

use std::path::PathBuf;

use crate::model::Placement;
use crate::{Error, Result, Workspace, scratch};

/// The tag every stamp record begins with, naming the format the two numbers
/// are written in.
///
/// Read for the same reason [`crate::definition::FORMAT`] is written: a change
/// to what a record holds moves this tag in the same commit, so an older
/// record is refused as unreadable rather than parsed under a newer meaning.
const FORMAT: &str = "v1";

/// When one value was first written, and when it last changed.
///
/// Unix seconds, UTC, which is what a clock reads without a timezone database
/// and without a rendering decision: the formatting a person sees is the
/// command's, and putting it here would freeze one locale into a committed
/// file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamps {
    /// The unix second the first write of this value landed in.
    pub created: u64,
    /// The unix second the most recent write of it landed in.
    pub updated: u64,
}

/// The wall clock, as a stamp records it.
///
/// A clock reading before the epoch gives zero rather than a refusal: the
/// stamp is bookkeeping beside a value, and a machine whose clock is set wrong
/// is not a reason to refuse the write of the value itself.
#[must_use]
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The stamps recorded for one entry, or nothing when it has no record.
///
/// # Errors
///
/// [`Error::FileUnreadable`] when the record exists and cannot be read, and
/// [`Error::StampRecordUnparsable`] when it can be read and is not the one
/// line this format writes.
pub fn read(workspace: &Workspace, placement: &Placement) -> Result<Option<Stamps>> {
    let relative = placement.stamp_record.as_str();
    let Some(text) = workspace.read_vault_relative(relative)? else {
        return Ok(None);
    };
    parse(&text)
        .ok_or_else(|| Error::StampRecordUnparsable {
            path: relative.to_owned(),
        })
        .map(Some)
}

/// Record that this value has just been written, at `now`.
///
/// Creates the record with `created` and `updated` both `now` when there is
/// none, and moves `updated` alone when there is one. The caller passes the
/// clock rather than this reading it, so that every path a single command
/// writes carries one instant rather than several a few milliseconds apart.
///
/// The record lands through a candidate beside its target and a rename, the
/// discipline [`crate::generate`] writes a definition record under: the
/// candidate is registered with [`scratch`] before it exists, so an
/// interrupted run leaves no half-written record, and the file at the real
/// path is only ever a complete one. The caller names the path in the same
/// commit as the value — which is the property the record rests on, since a
/// stamp committed without its value would date a write that never landed.
///
/// # Errors
///
/// Whatever [`read`] raises for the existing record, and
/// [`Error::FileUnwritable`] when the record or a directory above it cannot be
/// written.
pub fn touch(workspace: &Workspace, placement: &Placement, now: u64) -> Result<()> {
    let created = read(workspace, placement)?.map_or(now, |stamps| stamps.created);
    let absolute = workspace.vault_absolute(&placement.stamp_record);

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

    std::fs::write(
        &candidate,
        line(Stamps {
            created,
            updated: now,
        }),
    )
    .map_err(|cause| Error::FileUnwritable {
        path: candidate.display().to_string(),
        cause,
    })?;
    std::fs::rename(&candidate, &absolute).map_err(|cause| Error::FileUnwritable {
        path: absolute.display().to_string(),
        cause,
    })
}

/// The line a stamp record holds: the format tag, both stamps, and a newline.
fn line(stamps: Stamps) -> String {
    format!(
        "{FORMAT} created={} updated={}\n",
        stamps.created, stamps.updated
    )
}

/// The stamps one record's text holds, or nothing when it is not the line
/// [`line`] writes.
///
/// Every deviation is nothing rather than a partial answer: an unknown tag, a
/// field out of order, a field that is not a number, a missing field and a
/// fourth field after the two. A record is two numbers whose meaning depends
/// on which is which, so a tolerant reader here would be one that guesses.
fn parse(text: &str) -> Option<Stamps> {
    let mut fields = text.trim_end_matches('\n').split(' ');
    if fields.next()? != FORMAT {
        return None;
    }
    let created = fields.next()?.strip_prefix("created=")?.parse().ok()?;
    let updated = fields.next()?.strip_prefix("updated=")?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    Some(Stamps { created, updated })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::{Stamps, line, parse, read, touch};
    use crate::git::Git;
    use crate::model::Placement;
    use crate::nix::Nix;
    use crate::sops::Sops;
    use crate::{Error, Workspace};

    /// A tree with no git and no nix in it: [`read`] and [`touch`] resolve a
    /// path under the vault root and write a plaintext file, and reach neither.
    struct Tree(PathBuf);

    impl Tree {
        fn new(label: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("safix-stamps-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("a temporary directory can be made");
            Self(root)
        }

        fn workspace(&self) -> Workspace {
            Workspace::at(
                self.0.clone(),
                self.0.clone(),
                Git::default(),
                Nix::from_environment(),
                Sops::from_environment(),
            )
        }

        fn path(&self) -> PathBuf {
            self.0.join(RECORD)
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The record path the fixture placement carries, under the default root.
    const RECORD: &str = "state/safix/definitions/alice/api-token.stamps";

    fn placement() -> Placement {
        serde_json::from_value(json!({
            "file": "secrets/safix/users/alice/secrets.yaml", "key": "api-token",
            "origin": "private", "owner": "alice", "shared": false,
            "generator": null, "public": null,
            "definitionRecord": "state/safix/definitions/alice/api-token",
            "logicalFile": null, "logicalKey": null, "logicalPublic": null,
            "logicalRecord": null,
            "stampRecord": RECORD,
            "logicalStamp": null,
        }))
        .expect("the fixture is the shape the resolver emits")
    }

    fn write_record(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("the record has a parent"))
            .expect("the record's directory can be made");
        std::fs::write(path, text).expect("the record can be written");
    }

    /// The line a record holds and the stamps it is read back as are the same
    /// pair, so a record this version writes is one it can read.
    #[test]
    fn a_written_line_reads_back_as_the_stamps_it_was_written_from() {
        let stamps = Stamps {
            created: 1_700_000_000,
            updated: 1_758_000_123,
        };
        assert_eq!(line(stamps), "v1 created=1700000000 updated=1758000123\n");
        assert_eq!(parse(&line(stamps)), Some(stamps));
    }

    /// Every value written before this record existed has none, and that is an
    /// absent answer rather than a zero or a refusal.
    #[test]
    fn an_absent_record_reads_as_no_stamps() {
        let tree = Tree::new("absent");
        assert_eq!(
            read(&tree.workspace(), &placement()).expect("an absent record is not a refusal"),
            None
        );
    }

    /// The first touch dates both stamps; the second moves `updated` and
    /// leaves `created` where the first write put it.
    #[test]
    fn a_first_touch_creates_and_a_second_moves_updated_alone() {
        let tree = Tree::new("touch");
        let workspace = tree.workspace();
        let entry = placement();

        touch(&workspace, &entry, 1_700_000_000).expect("the first touch writes a record");
        assert_eq!(
            read(&workspace, &entry).expect("the record reads back"),
            Some(Stamps {
                created: 1_700_000_000,
                updated: 1_700_000_000
            })
        );

        touch(&workspace, &entry, 1_800_000_000).expect("the second touch rewrites the record");
        assert_eq!(
            read(&workspace, &entry).expect("the record reads back"),
            Some(Stamps {
                created: 1_700_000_000,
                updated: 1_800_000_000
            })
        );
    }

    /// A record that exists and does not parse is refused, naming the path, so
    /// that a hand-edited record is reported rather than overwritten with a
    /// fresh `created`.
    #[test]
    fn a_malformed_record_is_refused_naming_the_path() {
        let tree = Tree::new("malformed");
        write_record(&tree.path(), "v1 created=yesterday updated=today\n");
        let workspace = tree.workspace();

        let error = read(&workspace, &placement()).expect_err("a malformed record refuses");
        assert!(
            matches!(&error, Error::StampRecordUnparsable { path } if path == RECORD),
            "the refusal names the record: {error:?}"
        );

        let error = touch(&workspace, &placement(), 1_800_000_000)
            .expect_err("a touch over a malformed record refuses rather than reminting created");
        assert!(
            matches!(&error, Error::StampRecordUnparsable { path } if path == RECORD),
            "the refusal names the record: {error:?}"
        );
        assert_eq!(
            std::fs::read_to_string(tree.path()).expect("the record is still there"),
            "v1 created=yesterday updated=today\n",
            "the refused touch left the record as it found it"
        );
    }

    /// An unknown tag is unreadable rather than parsed under this version's
    /// meaning, which is what lets the format move without every older record
    /// being read as something it is not.
    #[test]
    fn an_unknown_tag_does_not_parse() {
        assert_eq!(parse("v2 created=1 updated=2\n"), None);
        assert_eq!(parse("v1 updated=2 created=1\n"), None);
        assert_eq!(parse("v1 created=1\n"), None);
        assert_eq!(parse("v1 created=1 updated=2 extra=3\n"), None);
        assert_eq!(parse(""), None);
    }
}
