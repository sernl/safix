//! Where plaintext is allowed to be while a run is in progress.
//!
//! safix 0.1 held an absolute: a generated value travelled a pipe and was never
//! a file. The clan-compatible generator contract is a filesystem contract —
//! `$out/<name>`, `$in/<generator>/<file>` and `$prompts/<key>` are paths, and
//! an editor edits a file — so that absolute cannot survive it. What replaces it
//! is this module, and the replacement is materially weaker rather than
//! equivalent.
//!
//! # The rule
//!
//! Plaintext materialized during generation or editing lives inside a directory
//! created mode `0700` for one run, on a filesystem this module has asked the
//! kernel about rather than inferred from a path. Every file inside it is
//! `0600`. The root is registered for removal *before* it is created, and it is
//! overwritten and removed on return, on error, on panic, and from both signal
//! handlers.
//!
//! There is no disk-backed fallback. `/tmp` is not tried, and the reason is
//! concrete rather than stylistic: this fleet's `/tmp` is ext4, so a silent
//! fallback to it would be the exact failure this rule exists to prevent,
//! occurring under a code path that looks like it succeeded. When no
//! memory-backed mount can be found the run refuses, and proceeds only when the
//! operator passes `--allow-disk-staging`, whose name states what is being
//! accepted.
//!
//! # What this achieves, and what it does not
//!
//! Two residual exposures are recorded here rather than smoothed over, in the
//! voice `types.nix` already uses for the equivalent 0.1 limit.
//!
//! Overwriting a page of a memory-backed filesystem does not reach a copy of
//! that page written to swap before the overwrite. tmpfs bounds plaintext to
//! memory and swap, not to memory alone, and closing the swap half is an
//! encrypted-swap decision on the host, outside safix.
//!
//! A mode-`0700` directory is reachable by every process running as its owner
//! for the run's duration, which on a workstation includes the operator's shell,
//! their editor and any agent process they are running. The pipe this replaces
//! was reachable by neither a third process nor a shell. That is a real
//! reduction, and the two are not equivalent.
//!
//! Beyond those: what a generator script or an editor does with a value it has
//! been handed is the author's to get right. This module removes the whole
//! staging root, including whatever a script or an editor left beside the file
//! it was given — but a script redirecting `$in/dep/file` elsewhere, or an
//! editor configured to write undo history or backups to a directory of its own,
//! has put plaintext where safix does not look.

use std::ffi::OsStr;
use std::fs::{DirBuilder, File, OpenOptions};
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Result};
use crate::scratch;
use crate::secret::Secret;

/// The environment variable naming a staging location to use ahead of the
/// conventional ones.
///
/// Its purpose is the severity drill this module's rule is verified by: pointing
/// staging at a disk-backed directory must refuse. It weakens nothing, because
/// what it names is verified exactly as the conventional candidates are — an
/// operator who points it at ext4 gets the refusal, not a silent disk write.
pub const OVERRIDE_VARIABLE: &str = "SAFIX_STAGING_DIR";

/// The flag that accepts disk-backed staging, spelled once.
pub const ACKNOWLEDGEMENT: &str = "--allow-disk-staging";

/// `statfs`'s answer for tmpfs, from `linux/magic.h`.
const TMPFS_MAGIC: i128 = 0x0102_1994;

/// `statfs`'s answer for ramfs, from `linux/magic.h`.
///
/// Accepted alongside tmpfs because ramfs is the stronger of the two — it cannot
/// be swapped at all — so refusing it would refuse the better filesystem.
const RAMFS_MAGIC: i128 = 0x8584_58f6;

/// Distinguishes two staging roots made by one process.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Whether the filesystem mounted at this path is memory-backed.
///
/// Asks the kernel with `statfs` and reads the filesystem's type. Nothing here
/// looks at the path's name: `/dev/shm` being a tmpfs is the overwhelmingly
/// common case and is not the case this check exists for. The case it exists for
/// is a container or a hardened host where `/dev/shm` has been remounted or
/// replaced, where the name still reads as shared memory and the bytes land on a
/// disk.
///
/// `None` when the path cannot be interrogated at all, which is not the same
/// answer as "not memory-backed" and is not treated as one: a candidate that
/// cannot be stat'd is skipped rather than refused, and running out of
/// candidates is what raises the refusal.
#[must_use]
pub fn memory_backed(path: &Path) -> Option<bool> {
    let answer = rustix::fs::statfs(path).ok()?;
    let kind = i128::from(answer.f_type);
    Some(kind == TMPFS_MAGIC || kind == RAMFS_MAGIC)
}

/// The mounts a staging root is looked for in, in order.
///
/// The override first, so a drill can point staging anywhere and watch the same
/// verification run over it. Then the conventional shared-memory mount, then the
/// per-user runtime directory, which is a tmpfs on a systemd host and is already
/// mode `0700`.
fn candidates() -> Vec<PathBuf> {
    let named = |variable: &str| {
        std::env::var_os(variable)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let mut found = Vec::new();
    found.extend(named(OVERRIDE_VARIABLE));
    found.push(PathBuf::from("/dev/shm"));
    found.extend(named("XDG_RUNTIME_DIR"));
    found
}

/// One run's private directory for plaintext, and the guard that removes it.
///
/// Dropping this overwrites and removes the whole tree, whether the drop comes
/// from a return, from an early error, or from a panic unwinding through it.
/// Signals are covered separately and by the same code: the root is registered
/// with [`crate::scratch`]'s process-wide registry before it is created, so the
/// handler's sweep reaches it too.
pub struct Staging {
    root: PathBuf,
}

impl std::fmt::Debug for Staging {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Staging")
            .field("root", &self.root)
            .finish()
    }
}

impl Staging {
    /// Establish a private staging root on a verified memory-backed mount.
    ///
    /// Verified twice, and the second time is the one that matters. The
    /// candidate mount is interrogated before anything is created, which is what
    /// makes the refusal cheap and its message able to name every candidate that
    /// was tried. The created root is then interrogated through its own opened
    /// descriptor, which is what closes the window between the two: a mount
    /// swapped underneath the first answer does not survive the second.
    ///
    /// # Errors
    ///
    /// [`Error::StagingNotMemoryBacked`] when no candidate is memory-backed and
    /// the acknowledgement was not given, and [`Error::StagingUnusable`] when a
    /// mount that is memory-backed cannot be written into.
    pub fn establish(allow_disk_staging: bool) -> Result<Self> {
        let tried = candidates();
        let mut refusals: Vec<String> = Vec::new();

        for mount in &tried {
            match memory_backed(mount) {
                Some(true) => return Self::create_in(mount, true),
                Some(false) => refusals.push(mount.display().to_string()),
                None => {}
            }
        }

        if allow_disk_staging && let Some(mount) = tried.first() {
            return Self::create_in(mount, false);
        }

        Err(Error::StagingNotMemoryBacked {
            candidates: tried
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            disk_backed: refusals,
        })
    }

    /// Make the root under a chosen mount, registering before creating.
    ///
    /// The order is the one [`crate::scratch`] states and is not tidiness:
    /// registering after creation opens exactly the window a signal arrives in.
    fn create_in(mount: &Path, verify: bool) -> Result<Self> {
        let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let root = mount.join(format!("safix-stage-{}-{sequence}", std::process::id()));

        scratch::register_tree(&root);
        DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .map_err(|cause| Error::StagingUnusable {
                path: root.display().to_string(),
                cause,
            })?;

        let staging = Self { root };

        // The mount was interrogated by path a moment ago; this asks the same
        // question of the directory that now exists, so a mount replaced between
        // the two answers is caught rather than staged into.
        if verify && memory_backed(&staging.root) != Some(true) {
            return Err(Error::StagingNotMemoryBacked {
                candidates: vec![mount.display().to_string()],
                disk_backed: vec![mount.display().to_string()],
            });
        }
        Ok(staging)
    }

    /// The directory every staged path is under.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Make a directory inside the root, and every missing parent under it.
    ///
    /// # Errors
    ///
    /// [`Error::StagingUnusable`] when it cannot be made.
    pub fn directory(&self, relative: &Path) -> Result<PathBuf> {
        let path = self.root.join(relative);
        DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(&path)
            .map_err(|cause| Error::StagingUnusable {
                path: path.display().to_string(),
                cause,
            })?;
        Ok(path)
    }

    /// Write one value into the root, owner-readable and nothing more.
    ///
    /// Exactly the bytes given: nothing is appended and nothing is stripped.
    /// This is clan's behaviour for a prompt's answer — `write_text(value)` — and
    /// matching it is load-bearing rather than incidental, because a newline
    /// convention on either side of this boundary silently corrupts a key whose
    /// last byte matters.
    ///
    /// # Errors
    ///
    /// [`Error::StagingUnusable`] when the file cannot be created or written.
    pub fn write(&self, relative: &Path, value: &Secret) -> Result<PathBuf> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent()
            && !parent.is_dir()
        {
            let inside = parent.strip_prefix(&self.root).unwrap_or(Path::new(""));
            self.directory(inside)?;
        }

        let unusable = |cause: std::io::Error| Error::StagingUnusable {
            path: path.display().to_string(),
            cause,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(unusable)?;
        value.write_to(&mut file).map_err(unusable)?;
        Ok(path)
    }

    /// Read one staged file back, as a value that zeroes itself.
    ///
    /// Exactly the bytes on disk. The symmetry with [`Staging::write`] is the
    /// contract: what a generator wrote to `$out/<name>` is what is stored.
    ///
    /// # Errors
    ///
    /// [`Error::StagingUnusable`] when the file cannot be opened, and whatever
    /// reading it failed with otherwise.
    pub fn read(&self, path: &Path) -> Result<Secret> {
        let mut file = File::open(path).map_err(|cause| Error::StagingUnusable {
            path: path.display().to_string(),
            cause,
        })?;
        Secret::read_from(&mut file)
    }
}

/// The names directly inside a staged directory, sorted, for a refusal that has
/// to say what *was* produced.
///
/// Copied from clan, which lists the output directory's contents in the message
/// it raises for a missing output. A refusal naming only what is absent leaves
/// the operator to guess whether the script wrote nothing, wrote to the wrong
/// place, or misspelled one name; listing what is there answers all three.
#[must_use]
pub fn names_in(directory: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .flatten()
        .map(|entry| {
            entry.file_name().to_str().map_or_else(
                || entry.file_name().to_string_lossy().into_owned(),
                str::to_owned,
            )
        })
        .collect();
    found.sort();
    found
}

/// Whether a staged output is present, by clan's own test.
///
/// clan asks `Path.is_file()`, which follows a symlink and answers for its
/// target, and which answers false for a directory. Both are matched rather than
/// tightened, and the reason the first is not a hole worth closing is that the
/// script has the caller's filesystem either way: a generator that wanted to
/// place `/etc/passwd` in an output could copy it as easily as link to it, so
/// refusing the link would move the hazard rather than remove it.
#[must_use]
pub fn is_output(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// One path component, refused if it is anything else.
///
/// Every name reaching a staged path comes from the resolver, which admits
/// `[a-z0-9][a-z0-9_-]*` and nothing else, so this can never fire on a declared
/// name. It is here because "can never fire" is a claim about a file two
/// modules away, and a staged path is the one place where a name containing `/`
/// or `..` would write outside the root.
#[must_use]
pub fn is_one_component(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name).components().count() == 1
        && Path::new(name).file_name() == Some(OsStr::new(name))
}

impl Drop for Staging {
    fn drop(&mut self) {
        scratch::sweep_tree(&self.root);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn value(bytes: &[u8]) -> Secret {
        Secret::read_from(&mut Cursor::new(bytes.to_vec())).unwrap_or_else(|_| {
            unreachable!("a cursor over a literal reads to its end");
        })
    }

    #[test]
    fn a_root_is_private_and_its_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let _exclusive = crate::scratch::exclusive();
        let Ok(staging) = Staging::establish(false) else {
            return;
        };
        let staged = staging
            .write(Path::new("out/api-token"), &value(b"fixture"))
            .unwrap_or_else(|_| unreachable!("a fresh staging root accepts a write"));

        let root_mode = std::fs::metadata(staging.root())
            .map(|metadata| metadata.permissions().mode() & 0o777)
            .unwrap_or_default();
        let file_mode = std::fs::metadata(&staged)
            .map(|metadata| metadata.permissions().mode() & 0o777)
            .unwrap_or_default();

        assert_eq!(root_mode, 0o700, "the staging root is not owner-only");
        assert_eq!(file_mode, 0o600, "a staged file is not owner-only");
    }

    #[test]
    fn two_roots_do_not_collide() {
        let _exclusive = crate::scratch::exclusive();
        let (Ok(first), Ok(second)) = (Staging::establish(false), Staging::establish(false)) else {
            return;
        };
        assert_ne!(first.root(), second.root(), "two runs share one root");
    }

    #[test]
    fn dropping_the_guard_removes_the_tree() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(staging) = Staging::establish(false) else {
            return;
        };
        let root = staging.root().to_path_buf();
        let _ = staging.write(Path::new("in/base/base"), &value(b"fixture"));
        drop(staging);
        assert!(!root.exists(), "the staging root survived its guard's drop");
    }

    /// The panic path, proved rather than argued.
    ///
    /// [`Staging`]'s removal is a `Drop`, so the claim that a panic removes the
    /// tree is a claim about unwinding, and the only severe way to test it is to
    /// panic. The panic is caught so the test process survives to make the
    /// assertion, and the root's path is carried out of the unwinding closure
    /// through a channel rather than through the caught payload, because a
    /// payload is what a panic hook could render.
    ///
    /// The `panic` lint is denied across this workspace because a panic can end
    /// a process while it holds a decrypted value, and every failure is a
    /// returned error instead. This is the one place the construction is the
    /// subject rather than a defect: the claim under test is that unwinding
    /// through the guard shreds the tree, and it cannot be made without one.
    #[test]
    #[expect(
        clippy::panic,
        reason = "the panic is the behaviour under test, not a failure path"
    )]
    fn a_panic_unwinding_through_the_guard_removes_the_tree() {
        let (announce, root) = std::sync::mpsc::channel();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let outcome = std::panic::catch_unwind(move || {
            let _exclusive = crate::scratch::exclusive();
            let Ok(staging) = Staging::establish(false) else {
                return;
            };
            let _ = announce.send(staging.root().to_path_buf());
            let _ = staging.write(Path::new("out/api-token"), &value(b"fixture"));
            panic!("the generator's own failure, mid-run");
        });
        std::panic::set_hook(previous);

        assert!(outcome.is_err(), "the deliberate panic did not happen");
        if let Ok(path) = root.try_recv() {
            assert!(!path.exists(), "the staging root survived a panic");
        }
    }

    #[test]
    fn a_name_that_is_not_one_component_is_refused() {
        assert!(is_one_component("api-token"));
        assert!(!is_one_component("../escape"));
        assert!(!is_one_component("nested/name"));
        assert!(!is_one_component(""));
        assert!(!is_one_component("."));
    }

    #[test]
    fn a_disk_backed_candidate_is_not_memory_backed() {
        // The repository this test runs from is on whatever the checkout is on,
        // and the assertion is about the answer being *some* answer rather than
        // about which: a tmpfs checkout would make the literal false.
        assert!(memory_backed(Path::new("/")).is_some());
        assert_eq!(memory_backed(Path::new("/nonexistent-mount-point")), None);
    }
}
