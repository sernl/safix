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
use std::process::Stdio;
use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::error::{Error, Result};
use crate::nix::Attribute;
use crate::progress::{Progress, log};
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

/// Regenerate the policy, then re-wrap every governed file to it.
///
/// Returns zero, or the status `sops` exited with when a re-wrap failed.
///
/// # Errors
///
/// [`Error::NixEvalFailed`] when the policy cannot be evaluated,
/// [`Error::FileUnwritable`] when it cannot be put in place, and
/// [`Error::NixSchemaMismatch`] when the governed set is not the shape this
/// reads.
pub fn run(workspace: &Workspace, progress: &dyn Progress, assume_yes: bool) -> Result<i32> {
    write_policy(workspace, progress)?;
    progress.write(NOT_REVOCATION);

    let managed = workspace.governed_files()?.managed.clone();
    let permits = concurrency();

    if assume_yes && permits.get() > 1 {
        rewrap_together(workspace, progress, &managed, permits)
    } else {
        rewrap_one_at_a_time(workspace, progress, &managed, assume_yes)
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
) -> Result<i32> {
    for relative in managed {
        log(progress, &announce(workspace, relative));
        if !workspace.vault_absolute(relative).exists() {
            continue;
        }
        let status = workspace
            .sops()
            .update_keys_command(workspace.root(), relative, assume_yes)
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
                workspace.root(),
                &relative,
                true,
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
