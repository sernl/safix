//! What an interrupted run leaves behind, and how it comes not to.
//!
//! A write prepares a candidate document beside its target and renames it into
//! place. Between those two moments the tree holds a file the operator did not
//! ask for, and the two ways a run ends there are a refusal — which
//! [`scratch::Guard`](safix_core::scratch::Guard) covers, because it is a return
//! — and a signal, which nothing in the language covers, because the process is
//! killed rather than unwound.
//!
//! So `SIGINT` and `SIGTERM` are caught. The handler sweeps the scratch registry
//! and exits 130 or 143, which are the codes `modules/flake/safix/safix.sh`
//! routes the same two signals to and therefore the codes an operator's shell
//! already reports for an interrupted `safix`.
//!
//! The sweep runs on an ordinary thread rather than inside a signal handler:
//! `signal-hook`'s iterator delivers through a self-pipe, so what runs here is
//! not restricted to async-signal-safe calls and may open files, take a lock and
//! exit. The cost is that a signal arriving in the microseconds between a path
//! being registered and the file at it being created finds nothing to sweep,
//! which is why the registration happens first and the creation second.

use std::process::ExitCode;

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

/// What the process exits with when interrupted at the keyboard.
pub const INTERRUPTED: i32 = 130;

/// What the process exits with when asked to terminate.
pub const TERMINATED: i32 = 143;

/// Catch the two signals a write has to be swept up after.
///
/// Installed for every subcommand rather than only the writing ones: the
/// registry is empty on a read path, so the sweep is a no-op there, and a
/// handler installed conditionally is one that is absent exactly when a future
/// subcommand forgets to ask for it.
///
/// A failure to install is not fatal. The signals keep their default
/// dispositions, which means an interrupted write can leave its candidate
/// document behind — the same outcome as a `SIGKILL`, which nothing can catch.
pub fn catch_signals() {
    let Ok(mut signals) = Signals::new([SIGINT, SIGTERM]) else {
        return;
    };
    std::thread::spawn(move || {
        // The first signal is the last: the sweep runs and the process ends, so
        // this reads one and never comes back for a second.
        let first = signals.forever().next();
        safix_core::scratch::cleanup();
        std::process::exit(if first == Some(SIGINT) {
            INTERRUPTED
        } else {
            TERMINATED
        });
    });
}

/// A status from the runtime as this process's own.
///
/// `sops` exits with codes above 255 for nothing, but a status that does not fit
/// is reported as a plain failure rather than truncated to whatever its low
/// eight bits happen to be — a truncation that landed on zero would report a
/// refusal as a success.
#[must_use]
pub fn exit_code(status: i32) -> ExitCode {
    ExitCode::from(u8::try_from(status).unwrap_or(1))
}
