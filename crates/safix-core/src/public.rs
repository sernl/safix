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
//! ```text
//! public/safix/users/<user>/<name>/value
//! public/safix/shared/<audience>/<name>/value
//! ```
//!
//! The leaf directory is named for the output and holds a file named `value`,
//! which is clan's shape read off `nixosModules/clanCore/vars/public/in_repo.nix`.
//! The prefix is not clan's, because safix's placement axis is audience rather
//! than machine, and it is deliberately *not* `secrets/safix/public/`.
//!
//! A path named `secrets` has to mean that everything under it is encrypted,
//! without qualification, because that is the proposition every backup rule,
//! every sync exclusion, every `rg` invocation and every reviewer applies to it.
//! Putting plaintext inside that tree makes the proposition conditional on
//! reading one more path component. A top-level sibling keeps the two trees
//! separable by prefix, which is what a `.gitignore`, an `rsync --exclude` and a
//! reviewer all actually operate on.
//!
//! The paths themselves are computed by the nix half and arrive on
//! [`crate::model::Placement::public`], so there is one implementation of the
//! layout rather than one here and one in `resolve.nix` that can disagree about
//! where a value is. `flake.safix.lib.publicPaths` is the same set, and it is
//! what the generated recipient policy is checked against.

use std::path::Path;

use crate::error::{Error, Result};
use crate::secret::Secret;

/// The prefix every public path is under, stated once so a check can quote it.
///
/// Asserted against the nix half by `modules/flake/checks/generators.nix`
/// rather than trusted: this constant is what the runtime believes, and the
/// paths it acts on come from the resolver.
pub const PREFIX: &str = "public/safix/";

/// The file name every public value is held in.
pub const LEAF: &str = "value";

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

/// Whether a repository-relative path is inside the public store.
///
/// Used by the runtime to keep a public path out of the code paths that expect
/// a sops document — a public value has no key, no recipients and no creation
/// rule, so every one of those questions is a category error about it.
#[must_use]
pub fn is_public_path(relative: &str) -> bool {
    relative.starts_with(PREFIX) && relative.ends_with(LEAF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_public_prefix_and_the_ciphertext_prefix_are_not_prefixes_of_each_other() {
        const CIPHERTEXT: &str = "secrets/safix/";
        assert!(!PREFIX.starts_with(CIPHERTEXT));
        assert!(!CIPHERTEXT.starts_with(PREFIX));
    }

    #[test]
    fn a_path_is_public_only_under_the_prefix_and_at_the_leaf() {
        assert!(is_public_path("public/safix/users/alice/wg-public/value"));
        assert!(is_public_path(
            "public/safix/shared/alice,bob/wg-public/value"
        ));
        assert!(!is_public_path("secrets/safix/users/alice/secrets.yaml"));
        assert!(!is_public_path(
            "public/safix/users/alice/wg-public/notes.txt"
        ));
    }
}

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::{LEAF, PREFIX, is_public_path};

    /// The alphabet `resolve.nix` admits a user, audience member or secret name
    /// from, plus the separator a shared audience's directory is joined with.
    const NAME: &str = "[a-z0-9][a-z0-9_-]{0,7}";

    /// What `resolve.nix` builds a public path out of, restated here.
    ///
    /// A second implementation on purpose: the runtime never builds one — every
    /// path it acts on comes from the resolver on
    /// [`crate::model::Placement::public`] — so what the properties below are
    /// about is that the *predicate* agrees with the shape, whichever of the two
    /// layouts produced it.
    fn public_path(audience: &[String], name: &str) -> String {
        let where_ = if audience.len() == 1 {
            format!("users/{}", audience.join(""))
        } else {
            format!("shared/{}", audience.join(","))
        };
        format!("{PREFIX}{where_}/{name}/{LEAF}")
    }

    proptest! {
        /// Every path the layout produces is recognised as public.
        #[test]
        fn every_layout_path_is_recognised(
            audience in proptest::collection::vec(NAME, 1..4),
            name in NAME,
        ) {
            prop_assert!(is_public_path(&public_path(&audience, &name)));
        }

        /// No path under the ciphertext prefix ever is.
        ///
        /// This is the separability the store's location was chosen for: an
        /// exclusion, a backup policy or a search scoped to one tree reaches
        /// nothing in the other, whatever the names inside them are.
        #[test]
        fn no_ciphertext_path_is_recognised(
            audience in proptest::collection::vec(NAME, 1..4),
            name in NAME,
        ) {
            let ciphertext = format!("secrets/safix/users/{}/{name}.yaml", audience.join(""));
            prop_assert!(!is_public_path(&ciphertext));
            prop_assert!(!ciphertext.starts_with(PREFIX));
            prop_assert!(!public_path(&audience, &name).starts_with("secrets/"));
        }

        /// The leaf is what makes a path public, not the prefix alone. A file
        /// dropped beside a value — an editor's backup, a note — is not one.
        #[test]
        fn a_sibling_of_a_value_is_not_a_value(
            audience in proptest::collection::vec(NAME, 1..4),
            name in NAME,
            sibling in "[a-z]{1,6}",
        ) {
            prop_assume!(sibling != LEAF);
            let beside = public_path(&audience, &name)
                .replace(&format!("/{LEAF}"), &format!("/{sibling}"));
            prop_assert!(!is_public_path(&beside));
        }
    }
}
