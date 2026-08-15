//! The three shims the residue and drill checks put in the runtime's way.
//!
//! Each is a deliberate perturbation, and each exists so an assertion is shown
//! to fail rather than assumed to be capable of failing. The role arrives in
//! `SAFIX_SHIM_ROLE` because two of the three stand in for a program the runtime
//! invokes with that program's own arguments, so there is no argument left to
//! carry it.
//!
//! - `spy` records the argument vector and environment it was handed and then
//!   becomes the real sops. It reads them from itself rather than from `/proc`,
//!   so the claim — that a plaintext value reached sops down neither channel —
//!   is made the same way on every platform.
//! - `interrupt` signals whoever invoked it and then does the real work, which
//!   turns "interrupted while sops holds the candidate document open" from a
//!   race into a fixture. It signals its parent alone rather than a process
//!   group, and it acts only before the sops subcommand `SAFIX_SHIM_HOLD`
//!   names, because a signal in front of every invocation would land in
//!   whichever window the run reached first rather than in the one being
//!   drilled.
//! - `mutate` runs the real binary and then damages exactly one channel: a line
//!   on standard output, a line on standard error, a different exit status, a
//!   file left in the repository, a value left in the temporary directory, or a
//!   plaintext value written to a regular file in the repository. Each is a
//!   mutation some assertion is supposed to catch, and the drill fails unless
//!   the assertion that catches it is the one that exists to.

use std::io::Write as _;
use std::process::Command;

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match environment("SAFIX_SHIM_ROLE").as_str() {
        "spy" => spy(&arguments),
        "interrupt" => interrupt(&arguments),
        "mutate" => mutate(&arguments),
        other => refuse(&format!("unknown shim role: {other}")),
    }
}

/// Record what sops was handed, then be sops.
fn spy(arguments: &[String]) -> ! {
    let spool = std::path::PathBuf::from(environment("SAFIX_SHIM_SPY"));
    if let Err(cause) = std::fs::create_dir_all(&spool) {
        refuse(&format!("could not open the spool: {cause}"));
    }

    let argv: Vec<String> = std::env::args().collect();
    append(&spool.join("argv"), &argv.join("\n"));

    let environ: Vec<String> = std::env::vars()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    append(&spool.join("environ"), &environ.join("\n"));

    become_sops(arguments)
}

/// Interrupt whoever invoked this, then be sops.
///
/// Settled into waiting for this child before the signal arrives, and still
/// running for a moment afterwards, so the window drilled is the one where the
/// candidate document is open rather than either edge of it. Then the real work
/// happens and this exits normally, which is what makes the run's own status the
/// thing under test: a runtime that reported its child's failure would be
/// answering a different question.
fn interrupt(arguments: &[String]) -> ! {
    let hold = environment("SAFIX_SHIM_HOLD");
    if arguments.first().is_some_and(|first| *first == hold) {
        pause();
        signal_parent();
        pause();
    }
    become_sops(arguments)
}

/// Long enough that the run is waiting rather than arriving or leaving.
fn pause() {
    let millis = std::env::var("SAFIX_SHIM_PAUSE_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(500);
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

/// `SIGINT` to the process that invoked this, and to nothing else.
///
/// The parent alone, rather than a process group: a group signal would reach
/// this process too, and a sops that died of the signal would make the run
/// report its child's failure instead of its own interruption, which is a
/// different claim.
///
/// `kill` is coreutils', which the suite already needs for `timeout` and `id`,
/// and `parent_id` is the standard library's — so no signal-sending dependency
/// is added to a crate whose shipped binary needs none.
fn signal_parent() {
    let parent = std::os::unix::process::parent_id();
    match Command::new("kill")
        .arg("-INT")
        .arg(parent.to_string())
        .status()
    {
        Ok(status) if status.success() => (),
        Ok(status) => refuse(&format!("kill -INT {parent} exited {status}")),
        Err(cause) => refuse(&format!("could not run kill: {cause}")),
    }
}

/// Run the real binary, then damage one channel.
fn mutate(arguments: &[String]) -> ! {
    let real = environment("SAFIX_SHIM_TARGET");
    let status = Command::new(&real).args(arguments).status();
    let code = match status {
        Ok(status) => status.code().unwrap_or(128),
        Err(cause) => refuse(&format!("could not run '{real}': {cause}")),
    };

    match environment("SAFIX_SHIM_MUTATION").as_str() {
        "stdout" => println!("a line the runtime does not print"),
        "stderr" => eprintln!("a note the runtime does not write"),
        "status" => std::process::exit(3),
        "effects" => {
            let repository = std::path::PathBuf::from(environment("SAFIX_REPO_ROOT"));
            write(&repository.join("an-extra-file"), "");
        }
        "residue" => {
            let temporary = std::path::PathBuf::from(environment("TMPDIR"));
            write(&temporary.join("leaked"), &environment("SAFIX_SHIM_VALUE"));
        }
        // Neither in the temporary directory nor named like a candidate
        // document, so the residue sweep and the scratch sweep both pass over
        // it and only a trace of the write itself can catch it.
        "plaintext" => {
            let repository = std::path::PathBuf::from(environment("SAFIX_REPO_ROOT"));
            write(
                &repository.join("a-plaintext-note"),
                &environment("SAFIX_SHIM_VALUE"),
            );
        }
        other => refuse(&format!("unknown mutation: {other}")),
    }
    std::process::exit(code)
}

/// Hand the arguments to the sops the fixture actually has.
fn become_sops(arguments: &[String]) -> ! {
    let real = environment("SAFIX_SHIM_SOPS");
    match Command::new(&real).args(arguments).status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(128)),
        Err(cause) => refuse(&format!("could not run '{real}': {cause}")),
    }
}

/// One environment variable the caller is required to have set.
fn environment(variable: &str) -> String {
    match std::env::var(variable) {
        Ok(value) => value,
        Err(_) => refuse(&format!("{variable} is unset")),
    }
}

/// Append a record to a spool file.
fn append(path: &std::path::Path, text: &str) {
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path);
    match opened {
        Ok(mut file) => {
            if writeln!(file, "{text}").is_err() {
                refuse("could not write the spool");
            }
        }
        Err(cause) => refuse(&format!("could not open {}: {cause}", path.display())),
    }
}

/// Write a file the drill's assertion is expected to find.
fn write(path: &std::path::Path, text: &str) {
    if let Err(cause) = std::fs::write(path, text) {
        refuse(&format!("could not write {}: {cause}", path.display()));
    }
}

/// Refuse loudly: a shim that failed quietly would make a drill pass by not
/// running.
fn refuse(reason: &str) -> ! {
    eprintln!("shim: {reason}");
    std::process::exit(127)
}
