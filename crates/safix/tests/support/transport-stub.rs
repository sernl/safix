//! The ssh-adjacent tools `safix upload` shells out to, driven against
//! synthetic identity material rather than real cryptography.
//!
//! # Why stubbing these is permitted where stubbing sops is not
//!
//! sops is what safix's claims are *about*: a stubbed sops would be
//! asserting that safix said the right things to a program that agreed with
//! it, which is not the claim. `ssh-keygen`, `ssh-to-age`, `ssh-keyscan` and
//! `ssh` are boundaries `upload` delegates across, and the claims are about
//! the delegation: that a probe reads only what the target offers and opens
//! no write-capable session, that a write streams the built tarball over
//! `ssh` and reaches no other channel, and that no key material ever reaches
//! an argument vector. Every one of those is a statement about what the
//! runtime does at the boundary, and a stub that records what it was handed
//! is a better instrument for it than the real tools, because it can be
//! asked what it saw and needs no network and no real ed25519 keys.
//!
//! What a stub cannot establish is that a real target's ssh daemon accepts
//! the same argument vector, or that a real `ssh-to-age` converts a real key
//! the way this one converts a synthetic one. Neither claim has a check in
//! this repository, matching every other stub here.
//!
//! # How a role is chosen
//!
//! By the shape of the argument vector, not by a variable — the same
//! discipline `card-stubs.rs` states for its own four roles. `-y -f <path>`
//! is `ssh-keygen`'s; `-i -` is `ssh-to-age`'s; `-t ed25519 <address>` is
//! `ssh-keyscan`'s; and `-o <opt> -o <opt> -o <opt> <target> <script>` is
//! `ssh`'s. The four shapes are disjoint on their first one or two words.
//!
//! # The conversion this stub performs, and why it is not real cryptography
//!
//! `ssh-keygen -y -f <path>` reads the file's own bytes, trimmed, and prints
//! `ssh-ed25519 <bytes>`. `ssh-to-age`, fed that on standard input, strips
//! the `ssh-ed25519 ` prefix and prints `age1<bytes>`. The composition is
//! injective and needs no real key material, so a fixture can write any
//! string as an "identity" and predict exactly what this stub derives from
//! it — which is what lets a test construct a matching identity, a
//! mismatched one, and an unrelated presented key without minting a single
//! real ed25519 key.
//!
//! # What it records
//!
//! Every role played, one word per line, in the order it was played — what
//! [`crates/safix/tests/harness/mod.rs`]'s `transport_invocations` reads,
//! and what task 2.5's and 3.6's claims are asserted against: a
//! `--directory` run's list holds no `ssh-keyscan` and no `ssh`. `ssh`'s own
//! argument vector and the bytes it read from standard input — the tarball
//! `Host::write` streamed — go to their own files, so a test can read the
//! archive back and assert its mode bits without trusting the writer.

use std::io::Read as _;

/// Where the spool lives.
const SPOOL: &str = "SAFIX_TRANSPORT_STUB_SPOOL";

/// The content `ssh-keyscan`'s stub presents, or unset for no key offered.
const PRESENTED: &str = "SAFIX_TRANSPORT_STUB_PRESENTED";

/// Set to make the `ssh` role refuse, for the simulated-transport-failure
/// drill task 4.5 names.
const SSH_REFUSES: &str = "SAFIX_TRANSPORT_STUB_SSH_REFUSES";

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["-y", "-f", path] => ssh_keygen(path),
        ["-i", "-"] => ssh_to_age(),
        ["-t", "ed25519", address] => ssh_keyscan(address),
        [first, ..] if *first == "-o" => ssh(&arguments),
        _ => refuse(&format!("unexpected invocation: {}", arguments.join(" "))),
    }
}

/// `ssh-keygen -y -f <path>`: the public half of a synthetic identity.
fn ssh_keygen(path: &str) -> ! {
    record("roles", "ssh-keygen");
    match std::fs::read_to_string(path) {
        Ok(content) => {
            println!("ssh-ed25519 {}", content.trim());
            std::process::exit(0);
        }
        Err(cause) => refuse(&format!("{path}: {cause}")),
    }
}

/// `ssh-to-age -i -`: the age recipient of a public key read from standard
/// input.
fn ssh_to_age() -> ! {
    record("roles", "ssh-to-age");
    let mut fed = String::new();
    if std::io::stdin().read_to_string(&mut fed).is_err() || fed.trim().is_empty() {
        refuse("no public key given on standard input");
    }
    let body = fed.trim().strip_prefix("ssh-ed25519 ").unwrap_or(fed.trim());
    println!("age1{body}");
    std::process::exit(0);
}

/// `ssh-keyscan -t ed25519 <address>`: the key the target presents,
/// unauthenticated — silent when [`PRESENTED`] names none, matching a real
/// `ssh-keyscan`'s silence when nothing answers or nothing is offered.
fn ssh_keyscan(address: &str) -> ! {
    record("roles", "ssh-keyscan");
    if let Some(content) = named(PRESENTED) {
        println!("{address} ssh-ed25519 {content}");
    }
    std::process::exit(0);
}

/// The wipe-then-extract write, over the connection D2 describes: the
/// argument vector and the streamed tarball are both recorded, so a test can
/// assert the options this run passed and read the archive it sent.
fn ssh(arguments: &[String]) -> ! {
    record("roles", "ssh");
    let _ = std::fs::write(spool().join("ssh-argv"), arguments.join("\n"));

    let mut tarball = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut tarball);
    let _ = std::fs::write(spool().join("ssh-stdin.tar.gz"), &tarball);

    if named(SSH_REFUSES).is_some() {
        refuse("simulated transport failure");
    }
    std::process::exit(0);
}

/// One environment variable this stub was given, or `None`.
fn named(variable: &str) -> Option<String> {
    std::env::var(variable).ok().filter(|value| !value.is_empty())
}

/// The spool directory, made if it is not there yet.
fn spool() -> std::path::PathBuf {
    let path = std::path::PathBuf::from(
        std::env::var(SPOOL).unwrap_or_else(|_| refuse(&format!("{SPOOL} is unset"))),
    );
    let _ = std::fs::create_dir_all(&path);
    path
}

/// Append one line to a spool file, creating it if it is not there yet — a
/// whole-file read, append, and rewrite, mirroring `clan-stub.rs`'s own
/// `record`: this stub runs once per invocation and never concurrently with
/// itself, so the read-modify-write carries no race to guard against.
fn record(name: &str, line: &str) {
    let path = spool().join(name);
    let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
    existing.push_str(line);
    existing.push('\n');
    let _ = std::fs::write(path, existing);
}

/// Refuse, on standard error, at status 1 — the shape every subprocess
/// refusal in this suite takes.
fn refuse(reason: &str) -> ! {
    eprintln!("{reason}");
    std::process::exit(1);
}
