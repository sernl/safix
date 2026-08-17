//! A clan whose behaviour is asserted rather than assumed.
//!
//! # Why stubbing clan is permitted where stubbing sops is not
//!
//! sops is what safix's claims are *about*. "The value was encrypted to this
//! audience" is a statement about a sops document, so a suite that stubbed sops
//! would be asserting that safix said the right things to a program that agreed
//! with it, which is not the claim.
//!
//! clan is a boundary safix delegates across. The claim is about the delegation:
//! that the read runs clan's command and takes what came back on the pipe, that
//! the write runs clan's command and puts the value on standard input and
//! nowhere else, that clan's own refusals reach the operator as clan's words,
//! and that nothing on this side ever reads a file clan placed. Every one of
//! those is a statement about what safix does at the boundary, and a stub that
//! records what it was handed is a better instrument for them than a real clan,
//! because it can be asked what it saw.
//!
//! What a stub cannot establish is that the arguments mean to clan what safix
//! thinks they mean. That is what the real-clan check is for, and it is a
//! separate check for exactly that reason.
//!
//! # The contract this stands in for
//!
//! Read out of clan-cli rather than out of its documentation:
//!
//! - `clan vars get <machine> <generator>/<file>` writes the value to standard
//!   output — `var.value` when that output is not a terminal, and
//!   `var.printable_value` when it is (`clan_cli/vars/get.py`).
//! - `clan vars set <machine> <generator>/<file>` reads the value from standard
//!   input (`clan_lib/vars/set.py`).
//! - `clan vars check <machine> --generator <g>` exits non-zero and reports
//!   "outdated invalidation hash" for a generator whose recorded validation no
//!   longer matches its definition (`clan_lib/vars/check.py`).
//! - "Couldn't find var" for an id clan has nothing under, and "has not been
//!   generated yet" for one it knows and that holds nothing
//!   (`clan_lib/vars/get.py`, `clan_cli/vars/get.py`).
//! - `clan secrets users add <name> --age-key <key>` declares a person clan does
//!   not have and refuses one it does; `clan secrets users add-key` is the second
//!   form, for a person who already exists.
//!
//! # What it records
//!
//! Every invocation's argument vector, so a test can assert that no value ever
//! reached one. Every value a write received, so a test can assert what landed.
//! And whether standard output was a terminal on each read, which is the one
//! thing the pipe assertion cannot be made without: the runtime's claim is that
//! it took the raw bytes rather than clan's printable rendering, and that claim
//! is only true because the output was a pipe.

use std::io::{Read as _, Write as _};

/// Where the store, the spool and the failure switches live.
const SPOOL: &str = "SAFIX_CLAN_STUB_SPOOL";

/// A var id clan has no declaration for at all.
///
/// The distinction matters and the real clan makes it. A var whose generator is
/// declared in the machine's configuration but which holds nothing yet answers
/// "has not been generated yet" — the ordinary state during bootstrap, and the
/// state every first export writes into. "Couldn't find var" is for an id
/// nothing declares, which is a typo in the mapping. Confirmed against the real
/// clan CLI over a miniature clan: a declared-and-ungenerated var gives the
/// first, and an invented id gives the second.
///
/// So the stub declares every var the fixture names, and this is how a test
/// says one is not declared.
const UNKNOWN: &str = "SAFIX_CLAN_STUB_UNKNOWN";

/// The generator whose recorded validation is reported as stale.
const STALE: &str = "SAFIX_CLAN_STUB_STALE";

/// A var id whose read or write fails the way any other clan refusal does.
const REFUSES: &str = "SAFIX_CLAN_STUB_REFUSES";

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();

    // `--help` is the probe: the runtime establishes that clan can be run at all
    // before it touches the first mapping.
    if words.first() == Some(&"--help") {
        println!("usage: clan [-h] [SUBCOMMAND]");
        std::process::exit(0);
    }

    record("argv", &arguments.join(" "));

    match words.as_slice() {
        ["vars", "get", "--flake", _flake, machine, id] => get(machine, id),
        ["vars", "set", "--flake", _flake, machine, id] => set(machine, id),
        [
            "vars",
            "check",
            "--flake",
            _flake,
            machine,
            "--generator",
            generator,
        ] => {
            check(machine, generator);
        }
        [
            "secrets",
            "users",
            verb,
            "--flake",
            _flake,
            user,
            "--age-key",
            key,
        ] => {
            register(verb, user, key);
        }
        other => refuse(&format!(
            "clan: unrecognized arguments: {}",
            other.join(" ")
        )),
    }
}

/// One var, written to standard output.
fn get(machine: &str, id: &str) -> ! {
    // Recorded before anything is written, because it is the fact the whole
    // raw-capture claim rests on and a stub that only recorded it on the
    // success path would say nothing about the runs that refused.
    record(
        "isatty",
        if rustix::termios::isatty(std::io::stdout()) {
            "terminal"
        } else {
            "pipe"
        },
    );

    if named(REFUSES) == Some(id.to_owned()) {
        refuse(&format!(
            "Error: clan refused to read {id} for {machine}, for a reason of its own"
        ));
    }
    if named(UNKNOWN) == Some(id.to_owned()) {
        refuse(&format!("Couldn't find var: {id} for machine: {machine}"));
    }

    let Some(value) = stored(machine, id) else {
        refuse(&format!("Var {id} has not been generated yet"));
    };

    // Bytes rather than a line, and no trailing newline added: the file is the
    // value, and a newline convention on either side of this boundary silently
    // corrupts a key whose last byte matters.
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&value);
    let _ = out.flush();
    std::process::exit(0);
}

/// One var, read from standard input.
fn set(machine: &str, id: &str) -> ! {
    if named(REFUSES) == Some(id.to_owned()) {
        refuse(&format!(
            "Error: clan refused to write {id} for {machine}, for a reason of its own"
        ));
    }

    let mut value = Vec::new();
    if std::io::stdin().lock().read_to_end(&mut value).is_err() {
        refuse("Error: clan could not read the value from standard input");
    }

    // Counted rather than overwritten, so "clan's write is unconditional and a
    // second run would commit again" is a claim a test can make: the count is
    // how many times clan was asked to write, and convergence is the claim that
    // it stops at one.
    let writes = spool().join("writes");
    let seen = std::fs::read_to_string(&writes)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let _ = std::fs::write(&writes, format!("{}", seen.saturating_add(1)));

    let _ = std::fs::create_dir_all(store_dir(machine, id).0);
    let _ = std::fs::write(store_dir(machine, id).1, encoded(&value));
    std::process::exit(0);
}

/// Whether clan considers a generator's recorded validation stale.
///
/// The sentence is the one `clan_lib/vars/check.py` logs, on standard error, at
/// the default level.
fn check(machine: &str, generator: &str) -> ! {
    if named(STALE).as_deref() == Some(generator) {
        eprintln!("Generator '{generator}' in machine {machine} has outdated invalidation hash.");
        eprintln!("Invalid generators (outdated invalidation hash):");
        eprintln!("  - {generator}");
        std::process::exit(1);
    }
    eprintln!("Check results for machine '{machine}': \nAll vars are present and valid.");
    std::process::exit(0);
}

/// One person's key, registered the way clan registers one.
///
/// `add` refuses a person clan already has, which is what makes the runtime try
/// `add-key` after it: which of the two applies is a fact about clan's store, and
/// the runtime reaches the second by outcome rather than by reading a message.
/// `SAFIX_CLAN_STUB_EXISTING` is how a test says the person is already there.
fn register(verb: &str, user: &str, key: &str) -> ! {
    let exists = named("SAFIX_CLAN_STUB_EXISTING").as_deref() == Some(user);
    match (verb, exists) {
        ("add", false) | ("add-key", true) => {
            record("registered", &format!("{verb} {user} {key}"));
            std::process::exit(0);
        }
        ("add", true) => refuse(&format!("Error: user {user} already exists")),
        ("add-key", false) => refuse(&format!("Error: no such user: {user}")),
        (other, _) => refuse(&format!("clan: unrecognized users verb: {other}")),
    }
}

/// The directory and file one var is stored under, in the stub's own layout.
///
/// Deliberately not clan's layout. Nothing in the runtime may read a file clan
/// placed, so a stub that reproduced clan's directory scheme would make a
/// runtime that cheated pass — the cheat would find the file where it expected
/// it. A layout of the stub's own means any read that is not `clan vars get`
/// finds nothing.
fn store_dir(machine: &str, id: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let directory = spool().join("store").join(machine);
    let file = directory.join(id.replace('/', "%"));
    (directory, file)
}

fn stored(machine: &str, id: &str) -> Option<Vec<u8>> {
    decoded(&std::fs::read_to_string(store_dir(machine, id).1).ok()?)
}

/// The store holds an encoding of the value rather than the value.
///
/// Hex, which is not encryption and is not pretending to be. What it reproduces
/// is the one property of clan's store that matters to the reading in
/// `syscall_proof.rs`: a real clan writes ciphertext, so the plaintext bytes
/// never appear in a write to a regular file. A stub that wrote them would make
/// that reading fail on the stub's own behaviour rather than on safix's, and the
/// only way to keep it green would be to widen the reading to admit a plaintext
/// write to an arbitrary path — which is the whole thing it exists to refuse.
fn encoded(value: &[u8]) -> String {
    use std::fmt::Write as _;
    value.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn decoded(text: &str) -> Option<Vec<u8>> {
    let digits: Vec<char> = text.trim().chars().collect();
    digits
        .chunks(2)
        .map(|pair| {
            let text: String = pair.iter().collect();
            u8::from_str_radix(&text, 16).ok()
        })
        .collect()
}

fn spool() -> std::path::PathBuf {
    std::path::PathBuf::from(
        named(SPOOL).unwrap_or_else(|| refuse(&format!("{SPOOL} names no directory"))),
    )
}

fn named(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .filter(|value| !value.is_empty())
}

fn record(name: &str, line: &str) {
    let spool = spool();
    let _ = std::fs::create_dir_all(&spool);
    let path = spool.join(name);
    let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
    existing.push_str(line);
    existing.push('\n');
    let _ = std::fs::write(path, existing);
}

fn refuse(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
