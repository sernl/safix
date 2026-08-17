//! The card surface the enrollment suite is driven against.
//!
//! # Why stubbing these is permitted where stubbing sops is not
//!
//! sops is what safix's claims are *about*: "the value was encrypted to this
//! audience" is a statement about a sops document, so a stubbed sops would
//! establish that safix said the right things to a program that agreed with it.
//!
//! `ykman`, `age-plugin-yubikey` and the two password stores are boundaries safix
//! delegates across, and the claims are about the delegation: that the PIN and
//! the PUK safix generated are distinct and reach `ykman` as flags, that the
//! management key is generated on the card and named nowhere, that the generator
//! is answered on a terminal with exactly one attempt, that a credential reaches
//! a store on standard input and never in argv, and that no path issues an OTP
//! command. Every one of those is a statement about what safix does at the
//! boundary, and a stub that records what it was handed answers it better than
//! the real tool, because it can be asked what it saw.
//!
//! There is a second reason here that the clan stub does not have, and it is the
//! whole reason this file is one binary with four roles rather than a set of
//! wrappers around the real tools. The real tools act on hardware, and the two
//! cards this fleet has hold the master identities for everything it owns: a
//! `ykman piv reset` or a write to OTP slot 2 is an irreversible loss of the
//! fleet's secrets and of the password database that opens them. A suite that
//! drove the real `ykman` would be one bad argument away from that, whatever it
//! intended. So no check in this repository ever runs it, and the card the suite
//! knows is this file.
//!
//! What a stub cannot establish is that the arguments mean to `ykman` and to the
//! plugin what safix thinks they mean, or that a real card accepts them. That is
//! the operator's own first enrollment, and it is deliberately not automated.
//!
//! # The contracts these stand in for
//!
//! Read out of the tools rather than out of their documentation, at the revisions
//! `openspec/changes/enroll-hardware-custody/design.md` records:
//!
//! - `ykman list --serials` writes one serial per line to standard output.
//! - `ykman --device <serial> piv info` reports, among other lines, whether the
//!   management key is held on the card under the PIN. That line's presence is
//!   the provisioned-or-factory answer, and asking costs no PIN retry.
//! - `ykman --device <serial> piv access change-pin|change-puk` take the current
//!   and new values as flags; `change-management-key --protect --generate --pin`
//!   puts a random key on the card under the PIN.
//! - `age-plugin-yubikey --generate` writes its chatter and its PIN prompt to
//!   standard error, turning the terminal's echo off around the read
//!   (`dialoguer::Password`, via `console`), instructs the operator to touch the
//!   card, and writes the identity block to standard output — six metadata
//!   comments, one of which names the recipient, then the stub line
//!   (`src/util.rs`, `i18n/en-US/age_plugin_yubikey.ftl`).
//! - `secret-tool store --label=<l> <attr> <value>` reads the secret from
//!   standard input; `keepassxc-cli add --password-prompt <db> <entry>` reads the
//!   database password and then the entry's from standard input.
//!
//! # How a role is chosen
//!
//! By the shape of the argument vector, not by a variable. All four tools are
//! this one binary and safix invokes them with one environment, so a variable
//! naming the role could only ever name one of them; and the shapes are disjoint
//! anyway, on the word each vector opens with: `list` and `--device` are
//! `ykman`'s, `--generate` is the plugin's, `store` and `lookup` are the secret
//! service's, and `add` is the store's. The word rather than its presence
//! anywhere, because `ykman piv access change-management-key --protect
//! --generate` carries `--generate` too and is not the plugin.
//!
//! # What each role records
//!
//! Every invocation's argument vector, so a test can assert what a credential
//! flag was given and that no vector ever named the OTP applet. Every
//! invocation's environment, which is the other half of that claim: a credential
//! that reached a store must have arrived on standard input, and neither of the
//! two channels a process listing can read. Every value that arrived on standard
//! input, so a test can assert what reached a store and by which channel. And,
//! for the generator, whether its three streams were terminals — which is the one
//! thing the prompt claim cannot be made without.

use std::io::{Read as _, Write as _};

/// Where every role records what it saw.
const SPOOL: &str = "SAFIX_CARD_STUB_SPOOL";

/// The serials `ykman list --serials` answers with, space-separated.
const SERIALS: &str = "SAFIX_CARD_STUB_SERIALS";

/// `provisioned` to report a card whose management key is PIN-protected.
const STATE: &str = "SAFIX_CARD_STUB_STATE";

/// Set to make `ykman list` report no smartcard service at all.
const NO_PCSCD: &str = "SAFIX_CARD_STUB_NO_PCSCD";

/// A subcommand word this invocation refuses, for the refusal drills.
const REFUSES: &str = "SAFIX_CARD_STUB_REFUSES";

/// The recipient the generator prints, which the fixture chose.
const RECIPIENT: &str = "SAFIX_CARD_STUB_RECIPIENT";

/// The PIN the generator accepts, when a test names one.
///
/// Unset means the first answer is accepted, which is the ordinary case: safix
/// generates the PIN, so a test cannot know it in advance and the stub cannot be
/// told it. Set to something safix cannot have generated, it makes the generator
/// prompt again — which is what a card that refused the PIN looks like from
/// outside, and is how the one-attempt claim is drilled.
const EXPECTED_PIN: &str = "SAFIX_CARD_STUB_EXPECTED_PIN";

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    record("argv", &arguments.join(" "));
    // Read out of this process rather than out of `/proc`, so the claim is made
    // the same way on every platform. One line per invocation, with the
    // separators spelled, so a test can scope a reading to one tool's own run.
    let environ: Vec<String> = std::env::vars()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    record(
        "environ",
        &format!("[{}] {}", arguments.join(" "), environ.join(" ")),
    );

    // Asserted here rather than in a test, because a stub that quietly performed
    // an OTP command would make every test that greps for one pass. There is no
    // role under which this file writes an OTP slot.
    if arguments.iter().any(|word| word == "otp") {
        refuse("the OTP applet was named, and no path may name it");
    }

    match arguments.first().map(String::as_str).unwrap_or_default() {
        "--generate" => plugin(&arguments),
        "list" | "--device" => ykman(&arguments),
        "store" | "lookup" => secret_tool(&arguments),
        "add" => keepassxc(&arguments),
        other => refuse(&format!("no card-stub role answers `{other}`")),
    }
}

/// Enumerate cards, report one's state, or set its access.
fn ykman(arguments: &[String]) -> ! {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    if let Some(word) = named(REFUSES)
        && words.contains(&word.as_str())
    {
        refuse(&format!(
            "Error: ykman refused {word}, for a reason of its own"
        ));
    }

    match words.as_slice() {
        ["list", "--serials"] => {
            if named(NO_PCSCD).is_some() {
                refuse("Error: PCSC not available. Make sure pcscd is running.");
            }
            for serial in named(SERIALS).unwrap_or_default().split_whitespace() {
                println!("{serial}");
            }
            std::process::exit(0);
        }
        ["--device", serial, "piv", "info"] => {
            println!("PIV version:              5.4.3");
            println!("PIN tries remaining:      3/3");
            println!("PUK tries remaining:      3/3");
            println!("Management key algorithm: TDES");
            if named(STATE).as_deref() == Some("provisioned") {
                println!("Management key is stored on the YubiKey, protected by PIN.");
            }
            println!("CHUID:  no data available");
            record("state-asked", serial);
            std::process::exit(0);
        }
        [
            "--device",
            serial,
            "piv",
            "access",
            "change-pin",
            "-P",
            current,
            "-n",
            new,
        ] => {
            record("pin", &format!("{serial} {current} -> {new}"));
            std::process::exit(0);
        }
        [
            "--device",
            serial,
            "piv",
            "access",
            "change-puk",
            "-p",
            current,
            "-n",
            new,
        ] => {
            record("puk", &format!("{serial} {current} -> {new}"));
            std::process::exit(0);
        }
        [
            "--device",
            serial,
            "piv",
            "access",
            "change-management-key",
            "--protect",
            "--generate",
            "--pin",
            pin,
            "-f",
        ] => {
            record("management-key", &format!("{serial} protected under {pin}"));
            std::process::exit(0);
        }
        other => refuse(&format!(
            "ykman: unrecognized arguments: {}",
            other.join(" ")
        )),
    }
}

/// Generate an identity, asking for the PIN on the terminal the way the real
/// plugin does.
fn plugin(arguments: &[String]) -> ! {
    if !arguments.iter().any(|word| word == "--generate") {
        refuse(&format!(
            "age-plugin-yubikey: unrecognized arguments: {}",
            arguments.join(" ")
        ));
    }
    record(
        "streams",
        &format!(
            "stdin={} stdout={} stderr={}",
            kind(std::io::stdin()),
            kind(std::io::stdout()),
            kind(std::io::stderr())
        ),
    );

    let serial = following(arguments, "--serial").unwrap_or_default();
    let name = following(arguments, "--name").unwrap_or_default();
    let pin_policy = following(arguments, "--pin-policy").unwrap_or_default();
    let touch_policy = following(arguments, "--touch-policy").unwrap_or_default();

    eprintln!("🎲 Generating key...");
    let expected = named(EXPECTED_PIN);
    let mut attempts = 0_u32;
    loop {
        attempts = attempts.saturating_add(1);
        let asked = read_unechoed(&format!(
            "Enter PIN for YubiKey with serial {serial} (default is 123456): "
        ));
        record("pin-attempt", &asked);
        match &expected {
            None => break,
            Some(wanted) if *wanted == asked => break,
            Some(_) => (),
        }
        if attempts >= 3 {
            refuse("Error: PIN is blocked");
        }
        eprintln!(
            "Error: incorrect PIN, {} tries left",
            3_u32.saturating_sub(attempts)
        );
    }

    eprintln!();
    eprintln!("🔏 Generating certificate...");
    if touch_policy != "never" {
        eprintln!("👆 Please touch the YubiKey");
    }

    let recipient = named(RECIPIENT).unwrap_or_default();
    // Standard output is a pipe under the wrapper, which is what makes the real
    // plugin echo the recipient here as well; both are reproduced so a runtime
    // reading either finds it.
    eprintln!("Recipient: {recipient}");
    println!("#       Serial: {serial}, Slot: 1");
    println!("#         Name: {name}");
    println!("#      Created: Mon, 17 Aug 2026 00:00:00 +0000");
    println!("#   PIN policy: {pin_policy}");
    println!("# Touch policy: {touch_policy}");
    println!("#    Recipient: {recipient}");
    println!(
        "AGE-PLUGIN-YUBIKEY-1{}",
        serial.trim_start_matches('0').to_uppercase()
    );
    std::process::exit(0);
}

/// A secret service that records the entry it was handed.
fn secret_tool(arguments: &[String]) -> ! {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["store", label, attribute, value] => {
            let secret = drained();
            record("service-entry", &format!("{label} {attribute} {value}"));
            store(&format!("service-{value}"), &secret);
            std::process::exit(0);
        }
        // The reachability probe, and the lookup a round-trip reads back with. An
        // entry that is not there exits non-zero with nothing on standard error,
        // which is what the probe distinguishes an unreachable service from.
        ["lookup", attribute, value] => {
            record("service-lookup", &format!("{attribute} {value}"));
            match retrieve(&format!("service-{value}")) {
                Some(secret) => {
                    let mut out = std::io::stdout().lock();
                    let _ = out.write_all(&secret);
                    let _ = out.flush();
                    std::process::exit(0);
                }
                None => std::process::exit(1),
            }
        }
        other => {
            eprintln!("secret-tool: unrecognized arguments: {}", other.join(" "));
            std::process::exit(2);
        }
    }
}

/// A password store that records the two values it read from standard input.
fn keepassxc(arguments: &[String]) -> ! {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["add", "--password-prompt", database, entry] => {
            let fed = drained();
            let mut lines = fed.split(|byte| *byte == b'\n');
            let unlock = lines.next().unwrap_or_default().to_vec();
            let password = lines.next().unwrap_or_default().to_vec();
            record("store-entry", &format!("{database} {entry}"));
            store(&format!("unlock-{entry}"), &unlock);
            store(&format!("store-{entry}"), &password);
            std::process::exit(0);
        }
        other => {
            eprintln!("keepassxc-cli: unrecognized arguments: {}", other.join(" "));
            std::process::exit(2);
        }
    }
}

/// One prompt on standard error with the terminal's echo off around the read,
/// which is the shape the wrapper answers on.
///
/// `stty` rather than a termios call of this file's own: the shape being
/// reproduced is what `console` does, and spelling it with the tool every unix
/// has keeps this stub from asserting anything about how the real one is written.
fn read_unechoed(prompt: &str) -> String {
    let _ = std::process::Command::new("stty").arg("-echo").status();
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    let read = std::io::stdin().read_line(&mut line);
    let _ = std::process::Command::new("stty").arg("echo").status();
    eprintln!();
    if read.is_err() {
        refuse("Error: could not read the PIN");
    }
    line.trim_end_matches(['\r', '\n']).to_owned()
}

/// Whether one of this process's streams is a terminal.
fn kind<F: std::os::fd::AsFd>(stream: F) -> &'static str {
    if rustix::termios::isatty(stream) {
        "terminal"
    } else {
        "pipe"
    }
}

/// The value following a flag in an argument vector.
fn following(arguments: &[String], flag: &str) -> Option<String> {
    arguments
        .iter()
        .position(|word| word == flag)
        .and_then(|at| arguments.get(at.saturating_add(1)))
        .cloned()
}

/// Everything on standard input.
fn drained() -> Vec<u8> {
    let mut fed = Vec::new();
    if std::io::stdin().lock().read_to_end(&mut fed).is_err() {
        refuse("could not read standard input");
    }
    fed
}

/// One stored value, hex-encoded.
///
/// Hex, which is not encryption and is not pretending to be, for the reason the
/// clan stub encodes its store: a plaintext credential written to a regular file
/// would make the value-pipe reading fail on this stub's behaviour rather than on
/// safix's.
fn store(name: &str, value: &[u8]) {
    use std::fmt::Write as _;
    let encoded = value.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    });
    let _ = std::fs::write(spool().join(name), encoded);
}

fn retrieve(name: &str) -> Option<Vec<u8>> {
    let text = std::fs::read_to_string(spool().join(name)).ok()?;
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

/// Refuse loudly: a stub that failed quietly would make a drill pass by not
/// running.
fn refuse(reason: &str) -> ! {
    eprintln!("{reason}");
    std::process::exit(1)
}
