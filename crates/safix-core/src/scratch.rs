//! What an aborted write must not leave behind.
//!
//! A write lands through a rename from a scratch file beside the target, so an
//! abort leaves either the previous file or no file and never a truncated one.
//! The scratch file holds ciphertext rather than plaintext, and is shredded all
//! the same: a stray `secrets.yaml.safix-tmp.4213.yaml` beside the real one is a
//! file an operator could mistake for it.
//!
//! The registry is process-wide and the removal is driven from one place, for
//! the reason the retired shell runtime gave for its single `EXIT` trap: a
//! cleanup scoped to a function does not run when the process dies between the
//! write and the return, which is the abort a file actually survives. [`Guard`]
//! covers every return and every panic; a signal is the command's to catch, and
//! `safix` catches `SIGINT` and `SIGTERM` and calls [`cleanup`] from the handler
//! before exiting 130 or 143 — the same two codes the shell runtime exits with.
//!
//! A path is registered *before* the file at it is created, never after. The
//! window a registration after creation would open is exactly the one a signal
//! arrives in.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

/// How much is written at a time when overwriting a scratch file.
const SHRED_CHUNK: usize = 4096;

#[derive(Default)]
struct Registry {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
    trees: Vec<PathBuf>,
    floor: Option<PathBuf>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn with_registry<T>(act: impl FnOnce(&mut Registry) -> T) -> T {
    let mut held = registry().lock().unwrap_or_else(PoisonError::into_inner);
    act(&mut held)
}

/// Register a file to be shredded, before creating it.
pub fn register_file(path: &Path) {
    with_registry(|registry| registry.files.push(path.to_path_buf()));
}

/// Register a directory this run created, before creating it.
///
/// Removed only while still empty, so an aborted first write into a new audience
/// directory leaves no evidence and a populated one is never at risk.
pub fn register_dir(path: &Path) {
    with_registry(|registry| registry.dirs.push(path.to_path_buf()));
}

/// Register a whole tree to be overwritten and removed, before creating it.
///
/// Where [`register_dir`] removes a directory only while it is still empty —
/// which is what an aborted first write into a new audience directory needs —
/// this removes the directory and everything under it, shredding each file on
/// the way. It is what a plaintext staging root is registered through, and the
/// difference is that a staging root's whole point is to be populated: leaving
/// it alone because something is in it would leave exactly the plaintext the
/// sweep exists to remove.
///
/// Registered before creation, for the reason the module note gives: a
/// registration after creation opens exactly the window a signal arrives in.
pub fn register_tree(path: &Path) {
    with_registry(|registry| registry.trees.push(path.to_path_buf()));
}

/// The directory the upward sweep stops at, which is the repository root.
///
/// `rmdir -p` stops at the first ancestor that is not empty, and the repository
/// root always holds `.git`, so this is belt and braces — but the sweep walks
/// toward `/` and the cost of the belt is one comparison.
pub fn set_floor(path: &Path) {
    with_registry(|registry| registry.floor = Some(path.to_path_buf()));
}

/// Forget the registered directories, keeping them.
///
/// Called once a write has landed: the directory now holds the file it was made
/// for, and is no longer this run's to remove.
pub fn keep_dirs() {
    with_registry(|registry| registry.dirs.clear());
}

/// The lock a sweep must hold, and that an in-flight subprocess holds against
/// it.
fn quiescent() -> &'static Mutex<()> {
    static QUIESCENT: OnceLock<Mutex<()>> = OnceLock::new();
    QUIESCENT.get_or_init(|| Mutex::new(()))
}

/// The status an interrupted run is to end with, once one has been asked for.
static INTERRUPTED_WITH: AtomicI32 = AtomicI32::new(0);

/// Nothing is swept while this is held.
///
/// Held across each `sops` invocation, and it is what makes an interruption
/// mean the same thing here as it did in the retired shell runtime. Bash
/// runs a trap between commands, so a `SIGINT` arriving while `sops` is writing
/// the candidate document is acted on only once `sops` has been waited on —
/// never while it still has the file open. A sweep from a handler thread has no
/// such discipline of its own, so it borrows this one.
#[must_use]
pub fn quiet() -> Quiet {
    Quiet(Some(
        quiescent().lock().unwrap_or_else(PoisonError::into_inner),
    ))
}

/// The guard [`quiet`] returns.
pub struct Quiet(Option<std::sync::MutexGuard<'static, ()>>);

impl std::fmt::Debug for Quiet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Quiet")
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        self.0.take();
    }
}

/// Record that this run has been asked to stop, and with which status.
///
/// The first request wins: a second signal arriving while the first is waiting
/// for a subprocess does not change what the run will exit with.
pub fn interrupt(status: i32) {
    let _ = INTERRUPTED_WITH.compare_exchange(0, status, Ordering::SeqCst, Ordering::Relaxed);
}

/// The status a run has been asked to stop with, if it has.
///
/// A write checks this immediately after every subprocess it waits on, which is
/// where bash's own trap would have run. Reaching it means the run stops before
/// it renames anything into place, so an interruption during encryption leaves
/// the target file as it was and nothing in the history.
#[must_use]
pub fn interrupted() -> Option<i32> {
    match INTERRUPTED_WITH.load(Ordering::SeqCst) {
        0 => None,
        status => Some(status),
    }
}

/// Shred every registered file and remove every registered directory that is
/// still empty.
///
/// Waits for any in-flight subprocess first — see [`quiet`] — so a sweep never
/// removes a file something else still holds open and is about to write to.
///
/// Best-effort throughout: a file that is already gone, a directory that has
/// since been filled, and a path that cannot be opened are all left alone. This
/// runs on the way out of a failed run and from a signal handler, and a failure
/// here must not mask the failure that led to it.
pub fn cleanup() {
    let _quiet = quiet();
    let (files, dirs, trees, floor) = with_registry(|registry| {
        (
            std::mem::take(&mut registry.files),
            std::mem::take(&mut registry.dirs),
            std::mem::take(&mut registry.trees),
            registry.floor.clone(),
        )
    });

    for file in &files {
        shred(file);
    }
    for tree in &trees {
        sweep_tree(tree);
    }
    for dir in &dirs {
        remove_empty_upwards(dir, floor.as_deref());
    }
}

/// Overwrite every file under a directory, then remove the directory.
///
/// Depth-first over an explicit stack rather than by recursion, and reading each
/// entry with `symlink_metadata` so a link inside the tree is unlinked rather
/// than followed — a staging root a generator dropped a symlink into must not
/// become a way to overwrite whatever it points at.
///
/// Best-effort, like the rest of this module: it runs on the way out of a failed
/// run and from a signal handler, and a failure here must not mask the failure
/// that led to it.
pub fn sweep_tree(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    let mut directories: Vec<PathBuf> = Vec::new();

    while let Some(current) = pending.pop() {
        let Ok(metadata) = std::fs::symlink_metadata(&current) else {
            continue;
        };
        if metadata.is_dir() {
            directories.push(current.clone());
            if let Ok(entries) = std::fs::read_dir(&current) {
                pending.extend(entries.flatten().map(|entry| entry.path()));
            }
        } else if metadata.is_file() {
            shred(&current);
        } else {
            let _ = std::fs::remove_file(&current);
        }
    }

    // A directory is recorded before its children are visited, so a child always
    // sits later in the list than its parent and removing from the back removes
    // depth-first.
    while let Some(directory) = directories.pop() {
        let _ = std::fs::remove_dir(&directory);
    }
}

/// Overwrite a file's bytes, then remove it.
///
/// Not a guarantee that the bytes are unrecoverable: a copy-on-write or
/// log-structured filesystem writes the zeroes elsewhere and leaves the original
/// blocks intact, and this makes no claim about them. It is the same best effort
/// the shell runtime makes with `shred -u`, and its value is that the file is
/// gone from the tree rather than that the medium is clean.
fn shred(path: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && metadata.is_file()
        && let Ok(mut file) = OpenOptions::new().write(true).open(path)
    {
        let zeroes = [0_u8; SHRED_CHUNK];
        let mut remaining = metadata.len();
        while remaining > 0 {
            let take = usize::try_from(remaining)
                .unwrap_or(SHRED_CHUNK)
                .min(SHRED_CHUNK);
            let Some(chunk) = zeroes.get(..take) else {
                break;
            };
            if file.write_all(chunk).is_err() {
                break;
            }
            remaining = remaining.saturating_sub(take as u64);
        }
        let _ = file.sync_all();
    }
    let _ = std::fs::remove_file(path);
}

/// `rmdir -p`: remove this directory and each empty ancestor, stopping at the
/// first that is not empty and at the floor.
fn remove_empty_upwards(leaf: &Path, floor: Option<&Path>) {
    let mut current = leaf.to_path_buf();
    loop {
        if floor == Some(current.as_path()) {
            return;
        }
        if std::fs::remove_dir(&current).is_err() {
            return;
        }
        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => current = parent.to_path_buf(),
            _ => return,
        }
    }
}

/// Runs [`cleanup`] when it goes out of scope, however it goes out of scope.
///
/// Held across a write so that an early return, a refusal and a panic all leave
/// the tree as they found it.
#[derive(Debug, Default)]
pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test per behaviour would race: the registry is process-wide and
    /// `cargo test` runs a module's tests on several threads, so the whole
    /// lifecycle is exercised in one test rather than being made thread-local
    /// for the tests' sake — the process-wide registry is the thing under test.
    #[test]
    fn the_lifecycle_shreds_files_and_removes_only_the_directories_it_made() {
        let root = std::env::temp_dir().join(format!("safix-scratch-{}", std::process::id()));
        let made = root.join("deep").join("audience");
        let kept = root.join("kept");
        std::fs::create_dir_all(&kept).expect("a temporary directory can be made");

        set_floor(&root);

        let doomed = made.join("secrets.yaml.safix-tmp.yaml");
        register_file(&doomed);
        register_dir(&made);
        std::fs::create_dir_all(&made).expect("a temporary directory can be made");
        std::fs::write(&doomed, b"ciphertext").expect("a temporary file can be written");

        let survivor = kept.join("keep-me");
        register_dir(&kept);
        std::fs::write(&survivor, b"not ours").expect("a temporary file can be written");

        cleanup();

        assert!(!doomed.exists(), "the scratch file survived the cleanup");
        assert!(!made.exists(), "the directory this run made survived");
        assert!(
            !root.join("deep").exists(),
            "the empty ancestor survived, so the sweep did not walk up"
        );
        assert!(root.exists(), "the sweep walked past its floor");
        assert!(survivor.exists(), "a populated directory was emptied");

        std::fs::remove_dir_all(&root).expect("the fixture can be removed");
    }
}
