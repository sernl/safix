//! A Bitwarden client whose behaviour is asserted rather than assumed.
//!
//! # Why stubbing this boundary is permitted where stubbing sops is not
//!
//! sops is what safix's claims are *about*. "The value was encrypted to this
//! audience" is a statement about a sops document, so a suite that stubbed sops
//! would be asserting that safix said the right things to a program that agreed
//! with it, which is not the claim.
//!
//! The vault's client is a boundary safix delegates across. The claims are
//! about the delegation: that the master password travels standard input and
//! nothing else, that the session key travels one environment variable and no
//! argument, that a payload is never a positional argument, that an edit
//! carries the whole item forward, and that the client's own refusals reach the
//! operator as the client's words. Each of those is a statement about what
//! safix does at the boundary, and a stub that records what it was handed is a
//! better instrument for them than a real client, because it can be asked what
//! it saw.
//!
//! **What this stub cannot establish**, stated as plainly as
//! `clan-stub.rs` states its own limit: that the argument vector means to the
//! real client what safix thinks it means. No check in this repository drives a
//! real `bw` — it cannot authenticate without a network and a nix build has
//! none, which `modules/flake/checks/bitwarden.nix` records — so nothing here
//! holds that. What holds it is review of the client's own behaviour against
//! the contract below, and the citations are there so a reader can repeat it.
//!
//! # The contract this stands in for
//!
//! Read out of `bitwarden-cli` 2026.8.0, this flake's pin, on 2026-09-16 —
//! out of the shipped bundle rather than out of its documentation:
//!
//! - `bw --nointeraction status` prints one JSON object,
//!   `{"serverUrl":…,"lastSync":…,"userEmail":…,"userId":…,"status":…}`, and
//!   `status` is `unauthenticated`, `locked` or `unlocked`.
//! - `bw --nointeraction --raw unlock --passwordfile <path>` reads the first
//!   line of `<path>` through `NodeUtils.readFirstLine`, which is
//!   `fs.createReadStream` behind `readline` — a stream read, so `/dev/stdin`
//!   works with the password on a pipe — and prints the session key on standard
//!   output.
//! - Every later invocation takes the session key in `BW_SESSION` or in
//!   `--session <key>`, and in no other channel.
//! - `bw create item` and `bw edit item <id>` read base64 JSON from standard
//!   input when the positional `[encodedJson]` is absent (`CliUtils.readStdin`,
//!   taken when `requestJson == null || requestJson === ""`), and `bw edit item`
//!   replaces the whole item.
//! - `bw list items --search <term>`, `bw list folders` and `bw get item <id>`
//!   print JSON on standard output.
//! - `bw sync` pulls the vault into the client's local copy, which is what its
//!   read commands answer from.
//!
//! # What it records
//!
//! Every invocation's argument vector, its whole environment, and whatever
//! arrived on its standard input — the three channels a credential could
//! travel — one file per invocation in invocation order, so a test can assert
//! the order of a run as well as its content.
//!
//! # What it asserts itself
//!
//! Five things, each failing the invocation rather than recording it silently,
//! because a stub that quietly accepted them would make every test that greps
//! for them pass: `--nointeraction` present; no fixture value and no fixture
//! master password in the argument vector or the environment; the session key
//! in `BW_SESSION` and in no argument; a positional that is not valid base64
//! refused, which is what catches a payload handed as `<encodedJson>`; and an
//! `edit` whose decoded payload is not a complete item refused.
//!
//! # The store is deliberately not a client's data directory
//!
//! Items are JSON files under the spool, keyed by a generated id, so a `get`
//! reads what a prior `create` or `edit` wrote. The layout is unlike a real
//! client's `data.json` on purpose, for the reason `clan-stub.rs` gives about
//! its own: a runtime that cheated by reading the tool's own files would find
//! nothing here.

use std::io::Read as _;

/// Where the item store, the invocation spool and the counter live.
const SPOOL: &str = "SAFIX_BW_STUB_SPOOL";

/// What `status` reports: `unlocked` (the default), `locked`, or
/// `unauthenticated`.
const STATE: &str = "SAFIX_BW_STUB_STATE";

/// The `serverUrl` `status` reports.
const SERVER: &str = "SAFIX_BW_STUB_SERVER";

/// Set to make `sync` exit non-zero, which is the stale-copy drill.
const SYNC_FAILS: &str = "SAFIX_BW_STUB_SYNC_FAILS";

/// Set to make every search answer with two items of the declared name, which
/// is the ambiguity drill.
const AMBIGUOUS: &str = "SAFIX_BW_STUB_AMBIGUOUS";

/// A subcommand word this invocation refuses, for the refusal drills.
const REFUSES: &str = "SAFIX_BW_STUB_REFUSES";

/// Every fixture value and password that may not appear in an argument vector
/// or an environment, space separated.
const FORBIDDEN: &str = "SAFIX_BW_STUB_FORBIDDEN";

/// The session key the unlock prints, and therefore the one every later
/// invocation must carry in `BW_SESSION` and nowhere else.
const SESSION_KEY: &str = "fixture-session-key";

/// The server `status` reports when no fixture named one.
const DEFAULT_SERVER: &str = "http://127.0.0.1:8222";

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let environ: Vec<(String, String)> = std::env::vars().collect();
    let stdin = read_stdin();

    record_invocation(&arguments, &environ, &stdin);
    assert_channels(&arguments, &environ);

    let words: Vec<&str> = arguments
        .iter()
        .map(String::as_str)
        .filter(|word| !word.starts_with("--"))
        .collect();
    let subcommand = words.first().copied().unwrap_or_default();

    if let Some(word) = named(REFUSES)
        && subcommand == word
    {
        refuse(&format!(
            "Error: the vault's client refused {word}, for a reason of its own"
        ));
    }

    match subcommand {
        "status" => status(),
        "config" => config(),
        "sync" => sync(),
        "unlock" => unlock(&arguments),
        "list" => list(&arguments),
        "get" => get(&words),
        "create" => create(&stdin),
        "edit" => edit(&words, &stdin),
        "lock" => lock(),
        other => refuse(&format!("no bw-stub role answers `{other}`")),
    }
}

/// The client's own state, as one JSON object.
fn status() -> ! {
    let state = named(STATE).unwrap_or_else(|| String::from("unlocked"));
    let server = named(SERVER).unwrap_or_else(|| String::from(DEFAULT_SERVER));
    let reported = if state == "unauthenticated" {
        format!(r#"{{"serverUrl":"{server}","lastSync":null,"status":"unauthenticated"}}"#)
    } else {
        format!(
            r#"{{"serverUrl":"{server}","lastSync":"2026-09-16T00:00:00.000Z","userEmail":"alice@example.com","userId":"fixture-user","status":"{state}"}}"#
        )
    };
    println!("{reported}");
    std::process::exit(0)
}

/// safix never runs this, and the role exists to say so: a run that configured
/// the operator's client would be caught by the spool carrying a `config`
/// invocation rather than by review.
fn config() -> ! {
    refuse("safix does not configure the client; pinning a server is the operator's own act")
}

/// The one refresh a run performs, and the switch that fails it.
fn sync() -> ! {
    if named(SYNC_FAILS).is_some() {
        refuse("Failed to sync: connect ECONNREFUSED 127.0.0.1:8222");
    }
    println!("Syncing complete.");
    std::process::exit(0)
}

/// The unlock: the password arrives on standard input through the path named by
/// `--passwordfile`, and the session key goes out on standard output.
fn unlock(arguments: &[String]) -> ! {
    if arguments.iter().any(|word| word == "--passwordenv") {
        refuse("--passwordenv puts the master password in an environment, and safix must not");
    }
    let Some(at) = arguments.iter().position(|word| word == "--passwordfile") else {
        refuse("no --passwordfile was given, so the master password had no channel");
    };
    let path = arguments
        .get(at.saturating_add(1))
        .cloned()
        .unwrap_or_default();
    if path != "/dev/stdin" {
        refuse(&format!(
            "--passwordfile named {path}, and this run expects the stream"
        ));
    }
    // The password reached standard input, which `main` already recorded, and
    // the first line of it is what the real client reads. An empty read is a
    // password that never arrived.
    let password = record_of("stdin-last").unwrap_or_default();
    if password.trim().is_empty() {
        refuse("the master password did not arrive on standard input");
    }
    if !arguments.iter().any(|word| word == "--raw") {
        refuse("--raw is what makes the session key the whole of standard output");
    }
    println!("{SESSION_KEY}");
    std::process::exit(0)
}

/// `list folders` and `list items [--search <term>]`.
fn list(arguments: &[String]) -> ! {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    if words.contains(&"folders") {
        let mut folders: Vec<String> = Vec::new();
        for item in stored() {
            let Some(folder) = text_at(&item, "folder") else {
                continue;
            };
            let entry = format!(r#"{{"object":"folder","id":"{folder}-id","name":"{folder}"}}"#);
            if !folders.contains(&entry) {
                folders.push(entry);
            }
        }
        println!("[{}]", folders.join(","));
        std::process::exit(0);
    }

    let search = words
        .iter()
        .position(|word| *word == "--search")
        .and_then(|at| words.get(at.saturating_add(1)))
        .copied();
    let mut answered: Vec<String> = Vec::new();
    for item in stored() {
        let name = text_at(&item, "name").unwrap_or_default();
        if let Some(search) = search
            && !name.contains(search)
        {
            continue;
        }
        answered.push(item.clone());
        // Two items of one name, which is what the ambiguity refusal is about.
        // A second copy under a second id, so nothing can tell them apart by
        // anything but their identifiers.
        if named(AMBIGUOUS).is_some() && search.is_some() {
            answered.push(item.replace(r#""id":""#, r#""id":"second-"#));
        }
    }
    println!("[{}]", answered.join(","));
    std::process::exit(0)
}

/// One item by its identifier.
fn get(words: &[&str]) -> ! {
    let identifier = words.get(2).copied().unwrap_or_default();
    for item in stored() {
        if text_at(&item, "id").as_deref() == Some(identifier) {
            println!("{item}");
            std::process::exit(0);
        }
    }
    refuse("Not found.")
}

/// A new item, from the base64 payload on standard input.
fn create(stdin: &str) -> ! {
    let payload = decoded(stdin);
    let name = text_at(&payload, "name")
        .unwrap_or_else(|| refuse("the payload names no item, so nothing could be created"));
    let identifier = format!("{}-id", slug(&name));
    write_item(&identifier, &payload);
    println!("{}", stored_item(&identifier));
    std::process::exit(0)
}

/// One whole item, replaced from the base64 payload on standard input.
fn edit(words: &[&str], stdin: &str) -> ! {
    let identifier = words
        .get(2)
        .copied()
        .unwrap_or_else(|| refuse("no item identifier was given to edit"));
    let payload = decoded(stdin);
    // The real client replaces the whole item, so a payload that is not a
    // complete one is refused here rather than silently merged — which is what
    // makes the read-modify-write claim an assertion instead of a hope.
    if text_at(&payload, "name").is_none() || !payload.contains(r#""login""#) {
        refuse("the payload is not a complete item, and an edit replaces the whole item");
    }
    if !stored().iter().any(|item| {
        text_at(item, "id")
            .as_deref()
            .is_some_and(|held| held == identifier)
    }) {
        refuse("Not found.");
    }
    write_item(identifier, &payload);
    println!("{}", stored_item(identifier));
    std::process::exit(0)
}

/// Locking, which only a run that unlocked performs.
fn lock() -> ! {
    println!("Your vault is locked.");
    std::process::exit(0)
}

/// The five assertions this stub makes on every invocation.
fn assert_channels(arguments: &[String], environ: &[(String, String)]) {
    if !arguments.iter().any(|word| word == "--nointeraction") {
        refuse("--nointeraction was absent, and a client that may prompt hangs a check");
    }

    // Every environment entry except this stub's own switches. A switch is the
    // instrument's configuration rather than a channel safix used, and
    // `SAFIX_BW_STUB_FORBIDDEN` carries the forbidden words themselves — so
    // scanning it would make every invocation refuse itself over the list it was
    // handed.
    let environment: Vec<String> = environ
        .iter()
        .filter(|(name, _)| !name.starts_with("SAFIX_BW_STUB_"))
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    for forbidden in named(FORBIDDEN).unwrap_or_default().split_whitespace() {
        if arguments.iter().any(|word| word.contains(forbidden)) {
            refuse(&format!(
                "a fixture value or master password reached an argument vector: {forbidden}"
            ));
        }
        if environment.iter().any(|entry| entry.contains(forbidden)) {
            refuse(&format!(
                "a fixture value or master password reached an environment: {forbidden}"
            ));
        }
    }

    if arguments.iter().any(|word| word.contains(SESSION_KEY)) {
        refuse("the session key reached an argument vector, where every process can read it");
    }
    if arguments.iter().any(|word| word == "--session") {
        refuse("--session puts the session key in an argument vector");
    }
    for (name, value) in environ {
        if value.contains(SESSION_KEY) && name != "BW_SESSION" {
            refuse(&format!(
                "the session key reached the environment as {name}, and BW_SESSION is the one \
                 place it may travel"
            ));
        }
    }

    // A positional is an item identifier, a folder word, a search term or a
    // subcommand — never a payload. Valid base64 of something decoding to JSON
    // is what a payload handed as `<encodedJson>` looks like, so it is refused.
    for word in arguments.iter().skip(1) {
        if word.starts_with("--") || word.len() < 16 {
            continue;
        }
        if let Some(decoded) = from_base64(word)
            && decoded.trim_start().starts_with('{')
        {
            refuse(
                "a positional argument decoded to a JSON payload, and a payload travels standard \
                 input alone",
            );
        }
    }
}

/// One file per invocation, in invocation order, carrying all three channels.
fn record_invocation(arguments: &[String], environ: &[(String, String)], stdin: &str) {
    let spool = spool();
    let _ = std::fs::create_dir_all(&spool);
    let at = next_index();
    let environment: Vec<String> = environ
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    let record = format!(
        "argv: {}\nenv: {}\nstdin: {}\n",
        arguments.join(" "),
        environment.join(" "),
        stdin.replace('\n', "\\n"),
    );
    let _ = std::fs::write(spool.join(format!("invocation-{at:04}")), record);
    let _ = std::fs::write(spool.join("stdin-last"), stdin);
}

/// The next invocation's own number, so the spool reads in order.
fn next_index() -> usize {
    let path = spool().join("counter");
    let seen: usize = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0);
    let next = seen.saturating_add(1);
    let _ = std::fs::write(path, format!("{next}"));
    next
}

/// Every item the store holds, as the JSON each was written as.
fn stored() -> Vec<String> {
    let mut items = Vec::new();
    let Ok(entries) = std::fs::read_dir(spool().join("items")) else {
        return items;
    };
    let mut paths: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if let Ok(text) = std::fs::read_to_string(path) {
            items.push(text.trim().to_owned());
        }
    }
    items
}

/// One stored item by its identifier, as it is held.
fn stored_item(identifier: &str) -> String {
    std::fs::read_to_string(spool().join("items").join(identifier))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Store one item under its identifier, with that identifier and its folder's
/// name recorded on it so a later `list` can answer both.
fn write_item(identifier: &str, payload: &str) {
    let items = spool().join("items");
    let _ = std::fs::create_dir_all(&items);
    let folder = text_at(payload, "folderId")
        .map(|folder| folder.trim_end_matches("-id").to_owned())
        .unwrap_or_default();
    let mut held = payload.trim().to_owned();
    // The identifier and the folder's own name, spliced in as the client's own
    // answer would carry them. Textual rather than parsed because this file
    // takes no dependency: a stub that pulled in a JSON crate would be a stub
    // whose build is the suite's problem.
    held = splice(&held, "id", identifier);
    if !folder.is_empty() {
        held = splice(&held, "folder", &folder);
    }
    let _ = std::fs::write(items.join(identifier), held);
}

/// One member set on a JSON object's front, replacing any existing one.
fn splice(object: &str, key: &str, value: &str) -> String {
    let pattern = format!(r#""{key}":"#);
    let without = match object.find(&pattern) {
        Some(at) => {
            let head = object.get(..at).unwrap_or_default();
            let tail = object.get(at..).unwrap_or_default();
            let end = tail
                .find(',')
                .map_or(tail.len(), |comma| comma.saturating_add(1));
            format!("{head}{}", tail.get(end..).unwrap_or_default())
        }
        None => object.to_owned(),
    };
    let body = without.trim_start_matches('{');
    format!(r#"{{"{key}":"{value}",{body}"#)
}

/// One string member of a JSON object, read textually.
fn text_at(object: &str, key: &str) -> Option<String> {
    let pattern = format!(r#""{key}":"#);
    let at = object.find(&pattern)?;
    let tail = object.get(at.saturating_add(pattern.len())..)?.trim_start();
    let quoted = tail.strip_prefix('"')?;
    let end = quoted.find('"')?;
    quoted.get(..end).map(str::to_owned)
}

/// The payload one write carried, decoded.
fn decoded(stdin: &str) -> String {
    from_base64(stdin.trim())
        .unwrap_or_else(|| refuse("the payload on standard input is not valid base64"))
}

/// An item identifier a name can be recovered from by eye, for a spool a person
/// reads.
fn slug(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Standard input, when there is any.
fn read_stdin() -> String {
    let mut text = String::new();
    let _ = std::io::stdin().read_to_string(&mut text);
    text
}

/// One recorded file's content, for the unlock's own look at what arrived.
fn record_of(name: &str) -> Option<String> {
    std::fs::read_to_string(spool().join(name)).ok()
}

/// Decode base64, or nothing when the text is not base64 at all.
fn from_base64(text: &str) -> Option<String> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.len().is_multiple_of(4) {
        return None;
    }
    let mut bytes: Vec<u8> = Vec::new();
    for chunk in trimmed.as_bytes().chunks(4) {
        let mut sextets = [0u32; 4];
        let mut carried = 0usize;
        for (at, byte) in chunk.iter().enumerate() {
            if *byte == b'=' {
                break;
            }
            let found = ALPHABET.iter().position(|letter| letter == byte)?;
            let slot = sextets.get_mut(at)?;
            *slot = u32::try_from(found).ok()?;
            carried = carried.saturating_add(1);
        }
        let triple = (sextets.first().copied().unwrap_or(0) << 18)
            | (sextets.get(1).copied().unwrap_or(0) << 12)
            | (sextets.get(2).copied().unwrap_or(0) << 6)
            | sextets.get(3).copied().unwrap_or(0);
        let octets = [
            u8::try_from((triple >> 16) & 0xFF).ok()?,
            u8::try_from((triple >> 8) & 0xFF).ok()?,
            u8::try_from(triple & 0xFF).ok()?,
        ];
        let wanted = carried.saturating_sub(1);
        for octet in octets.iter().take(wanted) {
            bytes.push(*octet);
        }
    }
    String::from_utf8(bytes).ok()
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

/// Refuse loudly: a stub that failed quietly would make a drill pass by not
/// running.
fn refuse(reason: &str) -> ! {
    eprintln!("{reason}");
    std::process::exit(1)
}
