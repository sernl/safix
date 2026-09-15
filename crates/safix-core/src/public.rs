//! Outputs a generator declares as not secret, stored in the clear.
//!
//! A generator output declared `secret = false` is written to the repository as
//! plaintext, is never handed to sops, and is never given a creation rule. That
//! is what makes it readable at evaluation, which is the whole reason the
//! declaration exists: a public key, a fingerprint or a derived identifier
//! reaches a nix module through `.value` without a deployment-time indirection,
//! and clan's own service modules are written against exactly that.
//!
//! # Where the store sits, and why not beside the ciphertext
//!
//! Under the default root:
//!
//! ```text
//! public/safix/users/<user>/<name>/value
//! public/safix/shared/<audience>/<name>/value
//! ```
//!
//! The leaf directory is named for the output and holds a file named `value`,
//! which is clan's shape read off `nixosModules/clanCore/vars/public/in_repo.nix`.
//! The root is not clan's, because safix's placement axis is audience rather
//! than machine, and it is `flake.safix.storage.plaintextOutputs` rather than
//! a literal: the tree is the consumer's to name.
//!
//! That is why this module spells no path and holds no prefix. The promise
//! "everything under here is ciphertext" used to be carried by a path
//! component called `secrets` — by the name, not by any mechanism, since
//! nothing in safix behaved differently. Once the root is the consumer's, a
//! root called `.vault/` or `store/` carries no promise at all, so the
//! promise moves to where the consumer states it: the option they set, and
//! the subtree name `encrypted`, which describes the representation rather
//! than the subject matter (design S3).
//!
//! What stays mechanically enforced is stronger than a constant here ever
//! was. Evaluation refuses any two of the three storage roots that overlap,
//! for every configuration rather than for three literals that happen not
//! to collide. `safix-public-no-rule` refuses a generated creation rule
//! matching any resolved public path, and `safix-no-catch-all` refuses a
//! rule reaching into any of the three trees — both behaviourally, against
//! resolved paths, and both gating a build.
//!
//! The paths themselves are computed by the nix half and arrive on
//! [`crate::model::Placement::public`], so there is one implementation of the
//! layout rather than one here and one in `resolve.nix` that can disagree about
//! where a value is. `flake.safix.lib.publicPaths` is the same set, and it is
//! what the generated recipient policy is checked against.

use std::path::Path;

use crate::error::{Error, Result};
use crate::secret::Secret;

/// Write a public value to its path, through a candidate and a rename.
///
/// The same staging discipline a ciphertext write uses, and for the same
/// reason: an abort leaves either the previous value or no file, never a
/// truncated one. The candidate is registered for shredding before it is
/// created even though the bytes are public, because a stray candidate beside
/// the real file is a file an operator could mistake for it.
///
/// # Errors
///
/// [`Error::FileUnwritable`] when the directory or the file cannot be written.
pub fn stage(candidate: &Path, value: &Secret) -> Result<()> {
    if let Some(directory) = candidate.parent()
        && !directory.is_dir()
    {
        crate::scratch::register_dir(directory);
        std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
            path: directory.display().to_string(),
            cause,
        })?;
    }

    let unwritable = |cause: std::io::Error| Error::FileUnwritable {
        path: candidate.display().to_string(),
        cause,
    };
    let mut file = std::fs::File::create(candidate).map_err(unwritable)?;
    value.write_to(&mut file).map_err(unwritable)
}

/// Whether a public output already holds a value.
///
/// Answered off the file rather than off a record of the run, which is the same
/// question `holds_a_value` asks of a ciphertext and has to be answered the same
/// way: an empty file is the state a truncated write leaves behind, so it
/// counts as holding nothing.
#[must_use]
pub fn holds_a_value(absolute: &Path) -> bool {
    std::fs::metadata(absolute).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}
