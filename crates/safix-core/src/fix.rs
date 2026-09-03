//! Converging the recipient policy and the ciphertext onto the declarations.
//!
//! `fix` is the write half of [`check`](crate::check), and it is deliberately
//! not all of it. It regenerates the policy and re-wraps each governed file to
//! the audience that policy declares; it does not mint a value, delete one, or
//! declare a name, because each of those is a decision rather than a
//! convergence.
//!
//! The two halves run in this order and not the other: re-wrapping first
//! re-wraps to a policy that is about to change.
//!
//! It does not commit. Re-wrapping every governed file is a diff worth reading
//! before it becomes history.
//!
//! # Why the fan-out is bounded by one on the interactive path
//!
//! `sops updatekeys` without `--yes` asks the operator to confirm each file's
//! new recipient set on the terminal. A confirmation cannot be fanned out — two
//! prompts competing for one standard input is not a faster confirmation, it is
//! an unanswerable one — so that path runs one file at a time with all three
//! streams inherited, which is also exactly what the retired shell runtime
//! did.
//!
//! With `--yes` there is no prompt, and the files are independent: each is one
//! document, re-wrapped from its own creation rule, and no two of them are the
//! same path. That path runs several at a time, bounded by a semaphore, with
//! each file's two output streams captured and replayed in the order the
//! declarations name the files — so the observable output does not depend on
//! which re-wrap finished first. The bound is [`CONCURRENCY_VARIABLE`], and
//! setting it to `1` returns the `--yes` path to inheriting the streams too.
//!
//! Two things are given up on the fanned-out path and are not given up on the
//! sequential one. sops writes through a logger that colours its output when it
//! sees a terminal and does not when it sees a pipe, so a captured re-wrap is
//! uncoloured where an inherited one is coloured. And a re-wrap that fails stops
//! the run, but the re-wraps already in flight beside it are allowed to finish,
//! where the sequential path never starts them. Both are why `1` is a supported
//! setting rather than a debugging aid.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::error::{Error, Result};
use crate::nix::Attribute;
use crate::progress::{Progress, log, note};
use crate::relocation::{self, PlainLeaf, SecretDocument};
use crate::scratch;
use crate::set;
use crate::workspace::Workspace;

/// The environment variable bounding how many files are re-wrapped at once.
pub const CONCURRENCY_VARIABLE: &str = "SAFIX_FIX_CONCURRENCY";

/// How many are re-wrapped at once when the variable says nothing.
const DEFAULT_CONCURRENCY: usize = 4;

/// The paragraph printed before anything is re-wrapped.
///
/// It is here rather than in the command because it is the one thing `fix`
/// prints that is neither progress nor a refusal: it is the standing correction
/// to what re-wrapping is taken to mean, and an embedder that suppressed it
/// would be running the convergence without it.
const NOT_REVOCATION: &str = "\n\
    Re-wrapping aligns ciphertext with policy. It does not revoke: a person\n\
    removed from an audience has already read every value in the file, and\n\
    re-wrapping the data key does not unread it. Revoking means minting a new\n\
    value — safix generate --regenerate <user> <name>, or sops <file>.\n\
    \n";

/// The note printed after `fix` relocates at least one document, output, or
/// record — the commit order design V4 states, and the lock-bump cost design
/// V6 discloses after an actual vault-root commit, neither of which `fix`
/// performs itself: it writes and removes files and prints this rather than
/// committing or disclosing.
const RELOCATION_NOTE: &str = "\n\
    Relocated content now sits at both roots, neither committed. Commit the\n\
    vault root first, then the declaration root with a trailer naming the\n\
    vault commit — Safix-Vault: <short-id> — and update the declaring\n\
    flake's lock entry for the vault afterward; nothing here commits.\n";

/// Regenerate the policy, then re-wrap every governed file to it.
///
/// Returns zero, or the status `sops` exited with when a re-wrap or a
/// relocation's write refused.
///
/// A vault-root preflight over every managed file runs first, before the
/// policy is regenerated or anything is re-wrapped — design V4's ordering,
/// applied here vault-only since `fix` writes nothing at the declaration
/// root that a commit governs.
///
/// With a vault declared, this also relocates every readable-layout
/// document, public output and definition record still at the declaration
/// root into its opaque vault destination (design V13's dated note),
/// through [`relocate`]. `rollback` runs the same move the other direction
/// and skips the re-wrap entirely, since the files it would touch have just
/// left the vault.
///
/// # Errors
///
/// [`Error::MidOperation`], [`Error::ConflictEntries`] or
/// [`Error::UncommittedChanges`] from the preflight;
/// [`Error::NixEvalFailed`] when the policy cannot be evaluated,
/// [`Error::FileUnwritable`] when it cannot be put in place,
/// [`Error::NixSchemaMismatch`] when the governed set is not the shape this
/// reads, and [`Error::VaultRelocationUnreadable`] when a relocated
/// document's source key will not decrypt.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    assume_yes: bool,
    rollback: bool,
) -> Result<i32> {
    scratch::set_floor(workspace.vault_root());
    scratch::set_floor(workspace.root());
    let _guard = scratch::Guard;

    let managed = workspace.governed_files()?.managed.clone();
    let touches: Vec<(&Path, &str)> = managed
        .iter()
        .map(|file| (workspace.vault_root(), file.as_str()))
        .collect();
    crate::set::refuse_bad_repository_state(workspace, &touches)?;

    write_policy(workspace, progress)?;

    match relocate(workspace, progress, rollback)? {
        RelocateOutcome::Interrupted(status) => return Ok(status),
        RelocateOutcome::Moved => progress.write(RELOCATION_NOTE),
        RelocateOutcome::Nothing => {}
    }
    if rollback {
        // The rollback direction only moves content back to the readable
        // layout; it converges no policy and re-wraps nothing, because the
        // files a re-wrap would touch have just left the vault it would
        // touch them in.
        return Ok(0);
    }

    progress.write(NOT_REVOCATION);

    let permits = concurrency();
    let config = workspace.stage_vault_rules()?;

    if assume_yes && permits.get() > 1 {
        rewrap_together(workspace, progress, &managed, permits, config.as_deref())
    } else {
        rewrap_one_at_a_time(workspace, progress, &managed, assume_yes, config.as_deref())
    }
}

/// Evaluate the policy into a file beside the one it replaces, then rename it
/// over it.
///
/// The destination is created before nix runs and is left behind when nix fails,
/// which is what the shell runtime's redirection does; naming it `.new` beside
/// the real one is what makes the replacement a rename rather than a truncation.
fn write_policy(workspace: &Workspace, progress: &dyn Progress) -> Result<()> {
    let root = workspace.root();
    let destination = root.join(".sops.yaml");
    let staging = root.join(".sops.yaml.new");

    workspace
        .nix()
        .eval_raw_to(root, Attribute::PolicyText, &staging)?;
    std::fs::rename(&staging, &destination).map_err(|cause| Error::FileUnwritable {
        path: destination.display().to_string(),
        cause,
    })?;

    log(
        progress,
        &format!("safix: wrote {}/.sops.yaml", root.display()),
    );
    Ok(())
}

/// What [`relocate`] did.
enum RelocateOutcome {
    /// Nothing was pending: every candidate's destination already existed,
    /// or no vault is declared.
    Nothing,
    /// At least one document, output, or record moved.
    Moved,
    /// A subprocess was interrupted mid-move; the caller exits with this.
    Interrupted(i32),
}

/// Move readable-layout content into a declared vault, or the reverse.
///
/// A no-op when no vault is declared: `vault_root` equals `root` then, and
/// [`relocation::secret_documents`]/`public_leaves`/`record_leaves` find
/// nothing to group, since design V14's `logical_*` fields are `null`
/// outside vault mode.
///
/// Forward (`rollback: false`, the default): for every document, output, or
/// record still present at the declaration root and absent from the vault,
/// decrypt it under the operator's own identity and re-encrypt it into its
/// opaque destination under the vault's disposable creation rules, then
/// remove the readable-layout source. `rollback: true` runs the same move in
/// the other direction, decrypting the vault's opaque form and re-encrypting
/// it into the readable layout at the declaration root under the committed
/// policy [`write_policy`] just regenerated.
///
/// A destination already present is left alone rather than re-created,
/// which is what lets an interrupted run resume: the candidate that would
/// have replaced it is registered with [`scratch`] before it exists, so a
/// signal mid-call leaves the destination absent and the source untouched,
/// and a re-run starts the same document over rather than half-finishing it
/// twice.
///
/// Also writes the vault's `.gitignore` entry for the scratch rules file
/// when it is missing, on every run with a vault declared — completing task
/// 5.8's migration-write half, whether or not anything else was relocated.
///
/// # Errors
///
/// [`Error::MidOperation`] when the declaration root is mid a git
/// operation, [`Error::VaultRelocationUnreadable`] when a source key will
/// not decrypt, and whatever [`crate::sops::Sops::create_empty_document`]
/// raises for a creation-rule refusal.
fn relocate(
    workspace: &Workspace,
    progress: &dyn Progress,
    rollback: bool,
) -> Result<RelocateOutcome> {
    if workspace.vault_root() == workspace.root() {
        return Ok(RelocateOutcome::Nothing);
    }
    if let Some(operation) = workspace.git().operation_in_progress(workspace.root())? {
        return Err(Error::MidOperation {
            state: operation.state,
            marker: operation.marker.display().to_string(),
        });
    }

    let placements = workspace.placements()?;
    let documents = relocation::secret_documents(placements);
    let pending: Vec<&SecretDocument> = documents
        .iter()
        .filter(|document| document_move(workspace, rollback, document).is_some())
        .collect();

    let config = if rollback || pending.is_empty() {
        None
    } else {
        workspace.stage_vault_rules()?
    };

    let mut moved = !pending.is_empty();
    for document in pending {
        if let Some(status) = relocate_document(workspace, rollback, document, config.as_deref())? {
            return Ok(RelocateOutcome::Interrupted(status));
        }
        note(
            progress,
            &format!(
                "relocated {} <-> {} ({} key{})",
                document.logical_file,
                document.opaque_file,
                document.keys.len(),
                if document.keys.len() == 1 { "" } else { "s" }
            ),
        );
    }

    for leaf in relocation::public_leaves(placements) {
        moved |= relocate_leaf(workspace, rollback, &leaf)?;
    }
    for leaf in relocation::record_leaves(placements) {
        moved |= relocate_leaf(workspace, rollback, &leaf)?;
    }

    ensure_vault_gitignore(workspace)?;

    Ok(if moved {
        RelocateOutcome::Moved
    } else {
        RelocateOutcome::Nothing
    })
}

/// The `(source root, source relative, destination root, destination
/// relative)` a document's relocation would touch, or `None` when there is
/// nothing to do: the source is absent, or the destination already exists.
fn document_move<'a>(
    workspace: &'a Workspace,
    rollback: bool,
    document: &'a SecretDocument,
) -> Option<(&'a Path, &'a str, &'a Path, &'a str)> {
    named_move(
        workspace,
        rollback,
        &document.opaque_file,
        &document.logical_file,
    )
}

/// The same shape [`document_move`] returns, for one plaintext leaf.
fn leaf_move<'a>(
    workspace: &'a Workspace,
    rollback: bool,
    leaf: &'a PlainLeaf,
) -> Option<(&'a Path, &'a str, &'a Path, &'a str)> {
    named_move(workspace, rollback, &leaf.opaque, &leaf.logical)
}

/// Resolve one opaque/readable pair to the roots and relative paths a move
/// between them would touch, or `None` when the source is absent or the
/// destination already exists.
fn named_move<'a>(
    workspace: &'a Workspace,
    rollback: bool,
    opaque: &'a str,
    logical: &'a str,
) -> Option<(&'a Path, &'a str, &'a Path, &'a str)> {
    let (source_root, source_relative, dest_root, dest_relative) = if rollback {
        (workspace.vault_root(), opaque, workspace.root(), logical)
    } else {
        (workspace.root(), logical, workspace.vault_root(), opaque)
    };
    if !source_root.join(source_relative).exists() || dest_root.join(dest_relative).exists() {
        return None;
    }
    Some((source_root, source_relative, dest_root, dest_relative))
}

/// Relocate one ciphertext document: decrypt every key from the source under
/// the operator's own identity, re-encrypt each into a candidate beside the
/// destination, rename it into place, then remove the source.
///
/// Returns the sops exit status when a re-encrypt refused, and `None`
/// otherwise — including when there was nothing pending, so a caller can
/// call this unconditionally over every grouped document.
fn relocate_document(
    workspace: &Workspace,
    rollback: bool,
    document: &SecretDocument,
    config: Option<&Path>,
) -> Result<Option<i32>> {
    let Some((source_root, source_relative, dest_root, dest_relative)) =
        document_move(workspace, rollback, document)
    else {
        return Ok(None);
    };
    let Some((first_opaque_key, first_logical_key)) = document.keys.first() else {
        return Ok(None);
    };
    let source_absolute = source_root.join(source_relative);
    let dest_absolute = dest_root.join(dest_relative);

    ensure_parent(&dest_absolute)?;
    let candidate = set::candidate_path(&dest_absolute);
    scratch::register_file(&candidate);

    let first_dest_key = if rollback {
        first_logical_key
    } else {
        first_opaque_key
    };
    {
        let _quiet = scratch::quiet();
        workspace.sops().create_empty_document(
            dest_root,
            dest_relative,
            first_dest_key,
            &candidate,
            config,
        )?;
    }
    if let Some(status) = scratch::interrupted() {
        return Ok(Some(status));
    }

    for (opaque_key, logical_key) in &document.keys {
        let (source_key, dest_key) = if rollback {
            (opaque_key, logical_key)
        } else {
            (logical_key, opaque_key)
        };
        let decrypted = {
            let _quiet = scratch::quiet();
            workspace.sops().decrypt_key(&source_absolute, source_key)?
        };
        if decrypted.status != 0 {
            return Err(Error::VaultRelocationUnreadable {
                file: source_relative.to_owned(),
                key: source_key.clone(),
            });
        }
        if let Some(status) = scratch::interrupted() {
            return Ok(Some(status));
        }
        let status = {
            let _quiet = scratch::quiet();
            workspace
                .sops()
                .set_key(&candidate, dest_key, &decrypted.value)?
        };
        if status != 0 {
            return Ok(Some(status));
        }
        if let Some(status) = scratch::interrupted() {
            return Ok(Some(status));
        }
    }

    std::fs::rename(&candidate, &dest_absolute).map_err(|cause| Error::FileUnwritable {
        path: dest_absolute.display().to_string(),
        cause,
    })?;
    scratch::keep_dirs();
    std::fs::remove_file(&source_absolute).map_err(|cause| Error::FileUnwritable {
        path: source_absolute.display().to_string(),
        cause,
    })?;
    Ok(None)
}

/// Relocate one plaintext leaf — a public output or a definition record — by
/// a staged copy, a rename, and removal of the source; no sops is involved,
/// since both are already plaintext.
///
/// Returns whether anything moved, so [`relocate`] can tell whether to print
/// [`RELOCATION_NOTE`].
fn relocate_leaf(workspace: &Workspace, rollback: bool, leaf: &PlainLeaf) -> Result<bool> {
    let Some((source_root, source_relative, dest_root, dest_relative)) =
        leaf_move(workspace, rollback, leaf)
    else {
        return Ok(false);
    };
    let source_absolute = source_root.join(source_relative);
    let dest_absolute = dest_root.join(dest_relative);

    ensure_parent(&dest_absolute)?;
    let candidate = set::candidate_path(&dest_absolute);
    scratch::register_file(&candidate);
    std::fs::copy(&source_absolute, &candidate).map_err(|cause| Error::FileUnwritable {
        path: candidate.display().to_string(),
        cause,
    })?;
    std::fs::rename(&candidate, &dest_absolute).map_err(|cause| Error::FileUnwritable {
        path: dest_absolute.display().to_string(),
        cause,
    })?;
    scratch::keep_dirs();
    std::fs::remove_file(&source_absolute).map_err(|cause| Error::FileUnwritable {
        path: source_absolute.display().to_string(),
        cause,
    })?;
    Ok(true)
}

/// Create `path`'s parent directory, registered for removal while still
/// empty, when it does not already exist.
fn ensure_parent(path: &Path) -> Result<()> {
    let Some(directory) = path.parent() else {
        return Ok(());
    };
    if directory.is_dir() {
        return Ok(());
    }
    scratch::register_dir(directory);
    std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
        path: directory.display().to_string(),
        cause,
    })
}

/// Append `.gitignore` coverage for the vault's scratch creation rules file
/// when it is missing — completes task 5.8's migration-write half; the
/// check-finding half already exists as `Finding::VaultGitignoreMissing`.
///
/// A no-op when the vault's `.gitignore` already covers it.
fn ensure_vault_gitignore(workspace: &Workspace) -> Result<()> {
    if workspace.vault_gitignore_covers_rules()? {
        return Ok(());
    }
    let path = workspace.vault_absolute(".gitignore");
    let mut text = workspace
        .read_vault_relative(".gitignore")?
        .unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('/');
    text.push_str(crate::workspace::VAULT_RULES_FILE);
    text.push('\n');
    std::fs::write(&path, text).map_err(|cause| Error::FileUnwritable {
        path: path.display().to_string(),
        cause,
    })
}

/// What the environment asks for, or [`DEFAULT_CONCURRENCY`].
///
/// A value that is not a positive number is read as one rather than refused: the
/// setting bounds a fan-out and does not decide anything about custody, and a
/// run that refused to converge over a mistyped environment variable would be a
/// worse outcome than a run that converged serially.
fn concurrency() -> NonZeroUsize {
    std::env::var(CONCURRENCY_VARIABLE)
        .ok()
        .and_then(|value| value.trim().parse::<NonZeroUsize>().ok())
        .unwrap_or_else(|| NonZeroUsize::new(DEFAULT_CONCURRENCY).unwrap_or(NonZeroUsize::MIN))
}

/// The line announcing what is about to happen to one file, or why nothing is.
fn announce(workspace: &Workspace, relative: &str) -> String {
    if workspace.vault_absolute(relative).exists() {
        format!("==> sops updatekeys {relative}")
    } else {
        format!("==> {relative} does not exist yet; create it with: sops {relative}")
    }
}

/// One file at a time, with sops holding the operator's own three streams.
fn rewrap_one_at_a_time(
    workspace: &Workspace,
    progress: &dyn Progress,
    managed: &[String],
    assume_yes: bool,
    config: Option<&Path>,
) -> Result<i32> {
    for relative in managed {
        log(progress, &announce(workspace, relative));
        if !workspace.vault_absolute(relative).exists() {
            continue;
        }
        let status = workspace
            .sops()
            .update_keys_command(workspace.vault_root(), relative, assume_yes, config)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|cause| workspace.sops().unavailable(cause))?;
        let code = status.code().unwrap_or(1);
        if code != 0 {
            return Ok(code);
        }
    }
    finish(progress);
    Ok(0)
}

/// Several at a time, bounded, with each file's output replayed in declaration
/// order.
///
/// The re-wraps are started in declaration order and awaited in declaration
/// order, and nothing is written until the file whose turn it is has finished.
/// That is what makes the output a function of the declarations rather than of
/// which re-wrap the scheduler happened to finish first.
fn rewrap_together(
    workspace: &Workspace,
    progress: &dyn Progress,
    managed: &[String],
    permits: NonZeroUsize,
    config: Option<&Path>,
) -> Result<i32> {
    let sops = workspace.sops();
    let present: Vec<String> = managed
        .iter()
        .filter(|relative| workspace.vault_absolute(relative).exists())
        .cloned()
        .collect();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(permits.get())
        .enable_all()
        .build()
        .map_err(|cause| sops.unavailable(cause))?;

    let produced = runtime.block_on(async {
        let semaphore = Arc::new(Semaphore::new(permits.get()));
        let mut running = Vec::with_capacity(present.len());
        for relative in present {
            let permit = Arc::clone(&semaphore);
            let mut command = tokio::process::Command::from(sops.update_keys_command(
                workspace.vault_root(),
                &relative,
                true,
                config,
            ));
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            running.push(tokio::spawn(async move {
                let held = permit.acquire_owned().await;
                let outcome = command.output().await;
                drop(held);
                (relative, outcome)
            }));
        }

        let mut outcomes = Vec::with_capacity(running.len());
        for handle in running {
            outcomes.push(handle.await);
        }
        outcomes
    });

    let mut by_file: BTreeMap<String, std::process::Output> = BTreeMap::new();
    for outcome in produced {
        let (relative, output) = outcome.map_err(|cause| Error::RewrapUnschedulable {
            cause: cause.to_string(),
        })?;
        by_file.insert(relative, output.map_err(|cause| sops.unavailable(cause))?);
    }

    for relative in managed {
        log(progress, &announce(workspace, relative));
        let Some(output) = by_file.get(relative) else {
            continue;
        };
        progress.write_output(&output.stdout);
        progress.write(&String::from_utf8_lossy(&output.stderr));
        let code = output.status.code().unwrap_or(1);
        if code != 0 {
            return Ok(code);
        }
    }

    finish(progress);
    Ok(0)
}

fn finish(progress: &dyn Progress) {
    log(
        progress,
        "safix: review the diff before committing it: git diff",
    );
}
