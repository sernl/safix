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
//! - `slow` waits before becoming the real sops, which holds open the window an
//!   interrupt has to arrive in for the scratch discipline to be under test.
//! - `mutate` runs the real binary and then damages exactly one channel: a line
//!   on standard output, a line on standard error, a different exit status, a
//!   file left in the repository, or a value left in the temporary directory.
//!   Each is a mutation some assertion is supposed to catch, and the drill fails
//!   unless the assertion that catches it is the one that exists to.

use std::io::Write as _;
use std::process::Command;

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match environment("SAFIX_SHIM_ROLE").as_str() {
        "spy" => spy(&arguments),
        "slow" => slow(&arguments),
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

/// Hold the window open, then be sops.
fn slow(arguments: &[String]) -> ! {
    let millis = environment("SAFIX_SHIM_DELAY_MS").parse().unwrap_or(2_000);
    std::thread::sleep(std::time::Duration::from_millis(millis));
    become_sops(arguments)
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
