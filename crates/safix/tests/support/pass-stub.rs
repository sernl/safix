//! A `pass` whose behaviour is asserted rather than assumed.
//!
//! # Why stubbing this boundary is permitted where stubbing sops is not
//!
//! sops is what safix's claims are *about*. "The value was encrypted to this
//! audience" is a statement about a sops document, so a suite that stubbed sops
//! would be asserting that safix said the right things to a program that agreed
//! with it, which is not the claim.
//!
//! `pass` is a boundary safix delegates across. The claim is about the
//! delegation: that a read runs the store's own command and takes what came
//! back on the pipe, that a write runs it with the whole record body on
//! standard input and nowhere else, that the store's own refusals reach the
//! operator as gpg's words, and that nothing on this side ever reads a file the
//! store placed. Every one of those is a statement about what safix does at the
//! boundary, and a stub that records what it was handed is a better instrument
//! for them than a real store, because it can be asked what it saw.
//!
//! # What a stub cannot establish, and what does establish it here
//!
//! A stub cannot establish that the argument vectors mean to `pass` what safix
//! thinks they mean: it answers the vectors safix sends because it was written
//! to, and it would go on answering them after the tool changed its options.
//! For this target that gap is closed rather than accepted —
//! `checks.safix-pass-cli` drives the real `pass` over a store it mints in its
//! own `GNUPGHOME`, and `crates/safix/tests/pass_cli.rs` is the file it runs.
//! This is the one of the five targets where that is possible, because the tool
//! is free-licensed and needs neither a network nor an account.
//!
//! # The contract this stands in for
//!
//! Read out of `pass`'s own manual page and its `platform`-independent script
//! body rather than out of third-party documentation:
//!
//! - `pass show <path>` writes the decrypted file to standard output, whole. It
//!   is `gpg -d` on one file, so what arrives is the file's bytes and nothing
//!   is appended.
//! - `pass insert --multiline <path>` reads the entry from standard input until
//!   EOF; without `--multiline` it reads a single line, twice, as a
//!   confirmation prompt. `--force` is what suppresses the "already exists"
//!   question, so a non-interactive run never blocks on one.
//! - `pass ls` renders the store through `tree(1)` — box-drawing glyphs and
//!   colour — which is why the runtime never parses it and answers absence from
//!   the `*.gpg` filenames instead. The verb is dispatched here anyway, so that
//!   a runtime which started parsing it would be recorded doing so.
//! - `pass git …` forwards to git inside the store. safix never runs it; the
//!   verb is here for the same reason `ls` is.
//! - a decrypt the operator's agent declines exits non-zero with gpg's own
//!   words about a passphrase, a secret key or the agent.
//!
//! # What it records
//!
//! Every invocation's argument vector, so a test can assert that no value and
//! no field ever reached one. Every invocation's whole environment, so a test
//! can assert that the only variable safix added is the store's location. And
//! every body a write received, so a test can assert what landed, byte for
//! byte.

use std::io::{Read as _, Write as _};

/// Where the stub's own store, the spool and the failure switches live.
const SPOOL: &str = "SAFIX_PASS_STUB_SPOOL";

/// An entry whose `show` exits non-zero with gpg's passphrase wording.
///
/// The state a run meets when the operator's agent declines: the entry is
/// there, its name is in the listing, and it did not decrypt. Distinct from
/// [`UNKNOWN`] on purpose — absence and a declined decrypt both exit non-zero
/// against the real tool, and conflating them would let a `backup` mapping
/// write over an entry it merely could not read.
const LOCKED: &str = "SAFIX_PASS_STUB_LOCKED";

/// An entry the stub's store does not hold.
const UNKNOWN: &str = "SAFIX_PASS_STUB_UNKNOWN";

/// An entry whose every command fails the way any other refusal does.
const REFUSES: &str = "SAFIX_PASS_STUB_REFUSES";

/// The store root the runtime points every child at.
const STORE_DIR: &str = "PASSWORD_STORE_DIR";

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();

    record("argv", &arguments.join(" "));
    record_environment();
    refuse_a_value_in_argv(&arguments);

    match words.as_slice() {
        ["--version"] => version(),
        ["show", path] => show(path),
        ["ls", ..] => list(),
        ["git", ..] => forwarded(),
        ["insert", rest @ ..] => insert(rest),
        other => refuse(&format!(
            "Usage: pass [show|insert|ls|git] {}",
            other.join(" ")
        )),
    }
}

/// The probe a caller uses to establish that the store's command can be run.
fn version() -> ! {
    println!("============================================");
    println!("= pass: the standard unix password manager =");
    println!("=                                          =");
    println!("=                  v1.7.4                  =");
    println!("============================================");
    std::process::exit(0);
}

/// One entry's whole body, written to standard output.
fn show(path: &str) -> ! {
    if named(REFUSES).as_deref() == Some(path) {
        refuse(&format!(
            "Error: pass refused to read {path}, for a reason of its own"
        ));
    }
    // gpg's own wording, which is what `Error::PassLocked` carries verbatim and
    // what the runtime reads to tell a declined decrypt from any other failure.
    if named(LOCKED).as_deref() == Some(path) {
        refuse(&format!(
            "gpg: decryption failed: No secret key\n\
             gpg: public key decryption failed: No passphrase given\n\
             Error: {path} could not be decrypted"
        ));
    }
    if named(UNKNOWN).as_deref() == Some(path) {
        refuse(&format!("Error: {path} is not in the password store."));
    }

    let Some(body) = stored(path) else {
        refuse(&format!("Error: {path} is not in the password store."));
    };

    // Bytes rather than a line, and no trailing newline added: the file is the
    // record, and a newline convention on either side of this boundary silently
    // corrupts a value whose last byte matters. That absence is the whole of
    // what makes the byte-exactness claim testable here.
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&body);
    let _ = out.flush();
    std::process::exit(0);
}

/// One entry's whole body, read from standard input.
///
/// Both flags are required rather than tolerated. Without `--multiline` the
/// real tool reads one line twice as a confirmation prompt, so a body carrying
/// a newline would be truncated at the first one and the confirmation would
/// never arrive; without `--force` it asks whether to overwrite, and a
/// non-interactive run would block on the question. Refusing here is what makes
/// "the runtime sends the vector the tool needs" a claim the thing safix runs
/// enforces, rather than one a stub quietly forgave.
fn insert(rest: &[&str]) -> ! {
    let multiline = rest.contains(&"--multiline") || rest.contains(&"-m");
    let force = rest.contains(&"--force") || rest.contains(&"-f");
    let Some(path) = rest.iter().find(|word| !word.starts_with('-')) else {
        refuse("Usage: pass insert [--multiline,-m] [--force,-f] pass-name");
    };

    if !multiline {
        refuse(&format!(
            "Error: pass insert {path} carries no --multiline, so this tool would read \
             one line and prompt for a confirmation of it"
        ));
    }
    if !force {
        refuse(&format!(
            "Error: pass insert {path} carries no --force, so this tool would ask \
             whether to overwrite and wait for an answer"
        ));
    }
    if named(REFUSES).as_deref() == Some(*path) {
        refuse(&format!(
            "Error: pass refused to write {path}, for a reason of its own"
        ));
    }

    let mut body = Vec::new();
    if std::io::stdin().lock().read_to_end(&mut body).is_err() {
        refuse("Error: pass could not read the record from standard input");
    }

    let writes = spool().join("writes");
    let seen = std::fs::read_to_string(&writes)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let _ = std::fs::write(&writes, format!("{}", seen.saturating_add(1)));

    let (directory, file) = store_path(path);
    let _ = std::fs::create_dir_all(directory);
    let _ = std::fs::write(file, encoded(&body));
    place_marker(path);
    std::process::exit(0);
}

/// What `pass ls` would render, in the shape the runtime is documented not to
/// parse.
///
/// The box-drawing glyphs are deliberate. A runtime that started reading this
/// instead of the `*.gpg` names would be reading `tree(1)` output, and the
/// recorded argv is how a test sees that it did.
fn list() -> ! {
    println!("Password Store");
    for name in names() {
        println!("\u{251c}\u{2500}\u{2500} {name}");
    }
    std::process::exit(0);
}

/// `pass git`, which safix never runs.
fn forwarded() -> ! {
    refuse("Error: safix does not run `pass git`; the store's history is the store's");
}

/// The marker file the store's own layout would carry at this path.
///
/// A real `pass` write creates `<root>/<path>.gpg`, and the runtime's listing
/// reads exactly those names — so the stub creates one too, or a second run
/// would not find an entry the first one wrote. What it does *not* contain is
/// the body: the marker says only that the name exists. A runtime that read the
/// tool's own file instead of asking the tool finds this sentence where a value
/// should be, which is what makes the cheat visible rather than green.
fn place_marker(path: &str) {
    let Some(root) = named(STORE_DIR) else {
        return;
    };
    let file = std::path::PathBuf::from(root).join(format!("{path}.gpg"));
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        file,
        "this is the stub's name marker and never a record body\n",
    );
}

/// Every entry name the stub's own store holds, sorted.
fn names() -> Vec<String> {
    let mut held: Vec<String> = std::fs::read_dir(spool().join("store"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .map(|name| name.replace('%', "/"))
        .collect();
    held.sort();
    held
}

/// The directory and file one entry is stored under, in the stub's own layout.
///
/// Deliberately not the store's layout: one flat directory of percent-escaped
/// names, with no `.gpg` suffix and no `.gpg-id` of its own. Nothing in the
/// runtime may read a file the store placed, so a stub that reproduced the
/// store's directory scheme would make a runtime that cheated pass — the cheat
/// would find the record where it expected it.
fn store_path(path: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let directory = spool().join("store");
    let file = directory.join(path.replace('/', "%"));
    (directory, file)
}

fn stored(path: &str) -> Option<Vec<u8>> {
    decoded(&std::fs::read_to_string(store_path(path).1).ok()?)
}

/// Every body the stub holds, for the argv check below.
fn bodies() -> Vec<Vec<u8>> {
    names().iter().filter_map(|name| stored(name)).collect()
}

/// Refuse any invocation carrying a value in its argument vector.
///
/// The promise this enforces is the transport's own: no value and no field
/// travels an argument vector, because every process on the host can read one.
/// Checked against what the stub holds rather than against a pattern, so it is
/// the actual bodies a test seeded that may not appear — the same shape
/// `op-stub.rs` refuses an `=` assignment with, made specific to what this
/// store carries.
fn refuse_a_value_in_argv(arguments: &[String]) {
    let held = bodies();
    for word in arguments {
        for body in &held {
            let text = String::from_utf8_lossy(body);
            for line in text.lines() {
                if line.len() >= 4 && word.contains(line) {
                    refuse(&format!(
                        "Error: the argument vector carries a value this store holds: {word}"
                    ));
                }
            }
        }
    }
}

/// The whole environment, one `KEY=VALUE` line per variable, sorted.
///
/// Whole rather than the one variable the transport is supposed to set: a test
/// asserting "the only variable safix added is the store's location" needs to
/// see everything, and a recorder that filtered would be deciding the answer.
fn record_environment() {
    let mut lines: Vec<String> = std::env::vars()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    lines.sort();
    record("env", &lines.join("\n"));
}

/// The store holds an encoding of the record rather than the record.
///
/// Hex, which is not encryption and is not pretending to be. What it reproduces
/// is the one property of the real store that matters to the readings in
/// `syscall_proof.rs`: a real `pass` writes ciphertext, so the plaintext bytes
/// never appear in a write to a regular file.
fn encoded(body: &[u8]) -> String {
    use std::fmt::Write as _;
    body.iter().fold(String::new(), |mut out, byte| {
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
