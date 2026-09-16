//! The operator's Bitwarden vault, and the whole of how `sync` reaches it.
//!
//! One transport: the vault's own client, driven as a child process, with the
//! master password read once per run and the session key it returns carried to
//! every later child. Every value travels standard input in and a pipe out;
//! what travels an argument vector is an item's own identifier, a folder's, and
//! the search term — never a value, never the master password, and never the
//! payload of a write.
//!
//! # What the client's own behaviour requires, measured rather than assumed
//!
//! Read out of `bitwarden-cli` 2026.8.0 — this flake's pin — on 2026-09-16,
//! against a client whose `BITWARDENCLI_APPDATA_DIR` was a scratch directory:
//!
//! - `bw --nointeraction status` prints one JSON object,
//!   `{"serverUrl":…,"lastSync":…,"status":…}`, and `status` is
//!   `unauthenticated` for a client that was never logged in. The client also
//!   writes `Could not find dir, …; creating it instead.` lines around it, so
//!   [`Bitwarden::preflight`] reads the JSON from the first `{` of standard
//!   output rather than assuming the whole stream is the object.
//! - `--passwordfile <path>` is read by the client's own `NodeUtils.readFirstLine`,
//!   which is `fs.createReadStream(path)` behind `readline` — a stream read,
//!   never a `stat`. Measured twice: the bundled implementation at
//!   `lib/node_modules/@bitwarden/clients/build/bw.js`'s `readFirstLine`, and
//!   that same mechanism run against `/dev/stdin` with the password on a pipe,
//!   which returned the first line. So the transport passes `/dev/stdin` and
//!   writes the password there; the fallback task 4.3 named — a file inside the
//!   run's own memory-backed staging root — is not taken, and `--passwordenv`
//!   is not the fallback under any outcome, because it would move the
//!   credential that outlives the run into an environment.
//!   The same measurement fixes one detail the design did not state: the client
//!   reads the *first line* only, so the password is written followed by a
//!   newline and a master password carrying one cannot be given at all — which
//!   is the client's own limit rather than safix's.
//! - `create item` and `edit item` read base64 JSON from standard input when no
//!   positional payload is given (`CliUtils.readStdin`, taken when
//!   `requestJson == null || requestJson === ""`). The documented positional
//!   `<encodedJson>` form is therefore never needed, and it is never used: a
//!   payload carrying a value in an argument vector is what this transport's
//!   own rule forbids. The base64 is computed here, not by a `bw encode`
//!   child, because the encoding is a pure function of bytes safix already
//!   holds and a second child is a second process the plaintext passes through.
//!
//! # `--nointeraction` on every invocation
//!
//! A client that would otherwise prompt fails instead of hanging a check.
//! safix asks for the master password itself, on a terminal, for the reason
//! [`crate::store`] asks for the database's: the child's standard input is
//! where safix writes, and a stream cannot be both a pipe and a keyboard.
//!
//! # The session key is in the child's environment and nowhere else
//!
//! The suite asserts, over every other transport, that no credential travels a
//! child's argument vector *or* its environment. This transport narrows that
//! claim deliberately, and the narrowing is exact: no master password and no
//! mapped value in any argument vector or any environment, on any invocation;
//! the session key in the environment of the invocations that need it, and in
//! no argument, no file safix writes, and no output path.
//!
//! The client offers no third channel — `BW_SESSION` in the environment or
//! `--session <key>` in argv, and nothing else. An argument vector is readable
//! by every process on the host through `/proc/*/cmdline`, where an environment
//! is readable by the same uid and root alone, so the key is placed in the
//! lesser exposure rather than in none, which this transport does not offer.
//! `openspec/changes/add-bitwarden-bridge/design.md`'s B3 records the decision,
//! and [`tests::no_argument_vector_can_carry_a_value`] is what holds it.
//!
//! # Every write is a read-modify-write
//!
//! `bw edit item` replaces the whole item. So a write fetches the item, sets
//! the value, the declared fields and — on a two-way mapping — the memory field
//! on that object, and sends the whole object back. A write that constructed a
//! fresh payload would delete every field of the item the declaration does not
//! govern, which is what [`tests::an_edit_carries_the_rest_of_the_item_forward`]
//! holds.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{Map, Value};
use zeroize::Zeroizing;

use crate::endpoint::{
    self, Capabilities, Channel, Endpoint, Existing, Field, FieldName, Record, ResolvedFields,
    Verdict,
};
use crate::enroll::custody::DatabasePassword;
use crate::error::{Error, Result};
use crate::model::{Bitwarden as Declared, BitwardenMapping, BitwardenMode};
use crate::progress::{Progress, log};
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::sync::{self, Outcome, Tally};
use crate::workspace::Workspace;
use crate::{bridge, enroll, scratch};

/// The environment variable naming the client, so a hermetic check can point
/// safix at a stub without a real `bw` on the host.
///
/// The shape [`crate::enroll::custody::KEEPASSXC_OVERRIDE`] has, read the same
/// way: one override mechanism per program, spelled the same everywhere.
pub const PROGRAM_OVERRIDE: &str = "SAFIX_BW";

/// The environment variable the client takes its session key in.
pub const SESSION_VARIABLE: &str = "BW_SESSION";

/// The custom field a two-way mapping records its last agreement in, on the
/// mapped item itself.
///
/// A field of the item rather than a companion item, which is the one place
/// this target's memory differs from `crate::store`'s: the client can write a
/// hidden custom field, which `keepassxc-cli` cannot, so this target needs no
/// reserved name suffix, no companion object, and no declaration can collide
/// with the memory.
pub const STATE_FIELD: &str = "safix-sync-state";

/// Bitwarden's own field type for a hidden custom field.
///
/// Hidden rather than text so that the memory is not shown beside the item's
/// own fields in a client, and so that it reads as bookkeeping rather than as
/// something a person should edit.
const HIDDEN: u8 = 1;

/// The path the master password is handed to the client as.
///
/// `/dev/stdin` and not a file safix writes: the module header records the
/// measurement that the client reads the path as a stream.
const PASSWORD_FILE: &str = "/dev/stdin";

/// What this transport can carry beside a value, and whether its value channel
/// takes more than one line.
///
/// The other half of `modules/flake/safix/fields.nix`'s `channels.bitwarden`,
/// which is what the nix half refuses a declaration against;
/// `modules/flake/checks/bitwarden.nix` holds that module's own table equal to
/// the shared one. `tags` is unsupported because this vault has no tag concept
/// at all — a folder is a placement and a collection is a permission boundary,
/// and neither is a label. `multiline` is true because the value crosses inside
/// a JSON payload on standard input rather than on a command line, so
/// [`Error::ValueSpansLines`] is a property of the keepassxc transport and not
/// one this target inherits.
pub const CAPABILITIES: Capabilities = Capabilities {
    username: Channel::Stdin,
    url: Channel::Stdin,
    notes: Channel::Stdin,
    tags: Channel::Unsupported,
    multiline: true,
};

/// Where the one master-password prompt comes from.
///
/// [`DatabasePassword`] rather than a trait of this target's own, and the
/// reuse is deliberate: that trait is "the password that unlocks the thing at
/// this address", its one implementation's prompt names the thing and
/// deliberately says nothing about which verb is asking
/// (`crates/safix/src/prompt.rs`'s own note on it), and the address this
/// target names is the server the client reports reaching. A second trait
/// would be a second prompt path to keep in step with the first, and a second
/// parameter on `audit`'s and `sync`'s own entry points for one object that
/// cannot be borrowed mutably twice.
type Prompt<'a> = dyn DatabasePassword + 'a;

/// What one declared mapping's address resolves against.
struct Claim {
    /// The mapping's own declared name, which is what the ambiguity refusal
    /// names.
    mapping: String,
    /// The declared folder, or none for the vault's root.
    folder: Option<String>,
    /// The item's declared name.
    item: String,
}

/// One client, unlocked for the length of one run.
///
/// No `Debug`: the session key is held here, and deriving one would print it
/// through the field.
pub struct Bitwarden {
    program: PathBuf,
    /// The declared server, or none for whichever one the client is configured
    /// against.
    server: Option<String>,
    /// The session key, once the run has unlocked or found an unlocked client.
    session: Option<Zeroizing<String>>,
    /// Whether this run is what unlocked the client, which is what decides
    /// whether it locks it again.
    unlocked_by_us: bool,
    /// Every declared address, with what it resolves against and which mapping
    /// declared it.
    claims: BTreeMap<String, Claim>,
    /// Every folder a declaration names, for [`Endpoint::list`].
    declared_folders: Vec<Option<String>>,
    /// The vault's folders, name to identifier, read once per run.
    folders: RefCell<Option<BTreeMap<String, String>>>,
    /// Each address's resolution, memoized for the run: the item's identifier,
    /// or none where the vault holds no such item.
    resolved: RefCell<BTreeMap<String, Option<String>>>,
}

impl Bitwarden {
    /// A client for one declaration, before anything has been asked of it.
    #[must_use]
    pub fn new(declared: &Declared) -> Self {
        let mut claims = BTreeMap::new();
        let mut declared_folders: Vec<Option<String>> = Vec::new();
        for mapping in &declared.mappings {
            claims.insert(
                declared.address_of(mapping),
                Claim {
                    mapping: mapping.id.clone(),
                    folder: mapping.bitwarden.folder.clone(),
                    item: mapping.bitwarden.item.clone(),
                },
            );
            let folder = mapping.bitwarden.folder.clone();
            if !declared_folders.contains(&folder) {
                declared_folders.push(folder);
            }
        }
        Self {
            program: program(),
            server: declared.server.clone(),
            session: None,
            unlocked_by_us: false,
            claims,
            declared_folders,
            folders: RefCell::new(None),
            resolved: RefCell::new(BTreeMap::new()),
        }
    }

    /// The server a report names: the declared one, or what the client
    /// reported reaching once it was asked.
    #[must_use]
    pub fn server(&self) -> String {
        self.server.clone().unwrap_or_default()
    }

    /// Ask the client about itself, unlock it where a terminal allows, and
    /// refresh its local copy — in that order, once per run, before any
    /// mapping is read.
    ///
    /// # Errors
    ///
    /// [`Error::BitwardenUnavailable`] when the client cannot be run,
    /// [`Error::BitwardenLocked`] for a client that is not logged in or is
    /// locked with no terminal to ask on, [`Error::BitwardenServerMismatch`]
    /// when a declared server is not the one reached,
    /// [`Error::BitwardenCommandFailed`] when the client refuses an
    /// invocation, and [`Error::BitwardenStale`] when its refresh fails.
    pub fn preflight(&mut self, password: &mut Prompt<'_>) -> Result<()> {
        let printed = self.invoke(&status_arguments(), None)?;
        let status = object_of(&printed)?;
        let state = text_at(&status, "status").unwrap_or_default();
        let reached = text_at(&status, "serverUrl").unwrap_or_default();

        if state == "unauthenticated" {
            return Err(Error::BitwardenLocked {
                state: "unauthenticated",
            });
        }
        // Before the unlock rather than after it: a run that prompted for a
        // master password and then refused over the server would have asked a
        // person for a credential it had already decided not to use.
        if let Some(declared) = self.server.clone()
            && !same_server(&declared, &reached)
        {
            return Err(Error::BitwardenServerMismatch { declared, reached });
        }

        if state == "locked" {
            if !enroll::terminal_present() {
                return Err(Error::BitwardenLocked { state: "locked" });
            }
            let named = if reached.is_empty() {
                String::from("the vault")
            } else {
                reached.clone()
            };
            let master = password.database_password(std::path::Path::new(&named))?;
            self.unlock(&master)?;
            self.unlocked_by_us = true;
        }

        // One refresh for the whole run, and a failed one refuses every
        // mapping: the client's reads answer from a local copy that may predate
        // another device's change, so a comparison against a stale copy would
        // report agreement that is not there.
        self.invoke(&sync_arguments(), None)
            .map_err(|reason| Error::BitwardenStale {
                output: reason.to_string(),
            })?;
        Ok(())
    }

    /// Lock the client again, when and only when this run is what unlocked it.
    ///
    /// A run that found the client already unlocked leaves it as it found it:
    /// the session was the operator's before safix ran, and invalidating it
    /// would end whatever else they were doing with it.
    ///
    /// Nothing is raised. The run's own outcome is what the caller reports, and
    /// a client that will not lock has already done everything the run asked of
    /// it.
    pub fn lock(&mut self) {
        if !self.unlocked_by_us {
            return;
        }
        drop(self.invoke(&lock_arguments(), None));
        self.session = None;
        self.unlocked_by_us = false;
    }

    /// The identifier of the item one declared address names, or none where
    /// the vault holds no such item.
    ///
    /// Memoized for the run: the same address is resolved by the read and then
    /// by the write, and two answers would be two items.
    ///
    /// # Errors
    ///
    /// [`Error::BitwardenItemAmbiguous`] when more than one item answers to
    /// the address, and whatever the client refused the search with.
    pub fn resolve(&self, address: &str) -> Result<Option<String>> {
        if let Some(known) = self.resolved.borrow().get(address) {
            return Ok(known.clone());
        }
        let Some(claim) = self.claims.get(address) else {
            // An address no declaration carries is one the caller invented,
            // which is a defect here rather than a state of the vault.
            return Err(Error::BitwardenCommandFailed {
                address: address.to_owned(),
                arguments: String::from("<resolve>"),
                output: String::from("no declared mapping names this address"),
            });
        };
        let folder = self.folder_id(claim.folder.as_deref())?;
        let printed = self.invoke(&search_arguments(&claim.item), None)?;
        let found = array_of(&printed)?;
        let matched: Vec<&Value> = found
            .iter()
            .filter(|item| text_at_value(item, "name").as_deref() == Some(claim.item.as_str()))
            .filter(|item| text_at_value(item, "folderId") == folder)
            .collect();

        // Nothing picks between two items with one name: a vault legitimately
        // holds both, and a mirror that chose would converge a person's secret
        // with whichever one happened to sort first.
        if matched.len() > 1 {
            return Err(Error::BitwardenItemAmbiguous {
                mapping: claim.mapping.clone(),
                address: address.to_owned(),
                matched: matched.len(),
            });
        }
        let identifier = matched
            .first()
            .and_then(|item| text_at_value(item, "id"))
            .clone();
        self.resolved
            .borrow_mut()
            .insert(address.to_owned(), identifier.clone());
        Ok(identifier)
    }

    /// Which mapping declared one address, for a refusal that names it.
    #[must_use]
    pub fn mapping_of(&self, address: &str) -> Option<&str> {
        self.claims.get(address).map(|claim| claim.mapping.as_str())
    }

    /// The agreement the item's own hidden field remembers, as the bytes it
    /// holds.
    ///
    /// An unreadable or absent memory is treated as absent rather than as a
    /// refusal, exactly as `crate::bridge`'s own memory is: it is safix's
    /// bookkeeping, and a run that stopped over it would refuse a mapping whose
    /// two sides it can see perfectly well.
    #[must_use]
    pub fn recorded(&self, address: &str) -> Option<Secret> {
        let identifier = self.resolve(address).ok().flatten()?;
        let item = self.read_item(&identifier).ok()?;
        let line = custom_field(&item, STATE_FIELD)?;
        Secret::read_from(&mut line.as_bytes()).ok()
    }

    /// Write one item: the value, the declared fields, and the agreement when
    /// the mode records one — all in one invocation.
    ///
    /// One write rather than two, which is what makes the ordering
    /// `crate::sync::remembered_after` insists on unnecessary here: the memory
    /// and the value it is about land together, so there is no window in which
    /// the memory describes a value that was not written.
    ///
    /// # Errors
    ///
    /// [`Error::BitwardenItemAmbiguous`] from the address's own resolution,
    /// and whatever the client refused the write with.
    pub fn write_item(
        &self,
        address: &str,
        value: &Secret,
        fields: &ResolvedFields,
        memory: Option<&str>,
        existing: Existing,
    ) -> Result<()> {
        let identifier = self.resolve(address)?;
        let claim = self.claims.get(address);
        let folder = match claim {
            Some(claim) => self.folder_id(claim.folder.as_deref())?,
            None => None,
        };
        let name = claim.map_or(address, |claim| claim.item.as_str());

        let base = match (existing, identifier.as_deref()) {
            (Existing::Present, Some(identifier)) => Some(self.read_item(identifier)?),
            _ => None,
        };
        let payload = payload_of(
            base.as_ref(),
            name,
            folder.as_deref(),
            value,
            fields,
            memory,
        )?;
        let encoded = Zeroizing::new(base64(&payload));

        match identifier.as_deref() {
            Some(identifier) => self.invoke(&edit_arguments(identifier), Some(encoded.as_bytes())),
            None => self.invoke(&create_arguments(), Some(encoded.as_bytes())),
        }?;
        // The item's identifier is new after a create, and stale after an edit
        // that moved it, so the memo is dropped rather than guessed at.
        self.resolved.borrow_mut().remove(address);
        Ok(())
    }

    /// One item, as the client prints it.
    ///
    /// # Errors
    ///
    /// Whatever the client refused the read with, and
    /// [`Error::BitwardenCommandFailed`] when what it printed is not an
    /// object.
    pub fn read_item(&self, identifier: &str) -> Result<Value> {
        let printed = self.invoke(&get_arguments(identifier), None)?;
        Ok(Value::Object(object_of(&printed)?))
    }

    /// The vault's folders, name to identifier, read once per run.
    fn folder_id(&self, folder: Option<&str>) -> Result<Option<String>> {
        let Some(folder) = folder else {
            // The vault's root, which the client reports as a folder whose
            // identifier is null — so an item there carries no folder rather
            // than one named for the root.
            return Ok(None);
        };
        if self.folders.borrow().is_none() {
            let printed = self.invoke(&folders_arguments(), None)?;
            let listed = array_of(&printed)?;
            let mut known = BTreeMap::new();
            for entry in &listed {
                if let (Some(name), Some(identifier)) =
                    (text_at_value(entry, "name"), text_at_value(entry, "id"))
                {
                    known.insert(name, identifier);
                }
            }
            *self.folders.borrow_mut() = Some(known);
        }
        Ok(self
            .folders
            .borrow()
            .as_ref()
            .and_then(|known| known.get(folder).cloned()))
    }

    /// One invocation, with `--nointeraction` first, the session key in the
    /// child's environment when there is one, and the payload — never a
    /// positional — on standard input.
    ///
    /// # Errors
    ///
    /// [`Error::BitwardenUnavailable`] when the client cannot be run, and
    /// [`Error::BitwardenCommandFailed`] carrying the argument vector as
    /// printed and the client's own standard error verbatim when it exits
    /// non-zero.
    fn invoke(&self, arguments: &[String], stdin: Option<&[u8]>) -> Result<Vec<u8>> {
        let mut command = Command::new(&self.program);
        command.arg("--nointeraction");
        command.args(arguments);
        for (name, value) in session_environment(self.session.as_ref().map(|key| key.as_str())) {
            command.env(name, value);
        }
        command
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|cause| Error::BitwardenUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if let Some(payload) = stdin {
            let mut pipe = child.stdin.take().ok_or(Error::StorePipeMissing)?;
            pipe.write_all(payload)
                .and_then(|()| pipe.flush())
                .map_err(|cause| Error::SecretRead { cause })?;
        }
        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::BitwardenUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if !finished.status.success() {
            return Err(Error::BitwardenCommandFailed {
                address: address_in(arguments),
                arguments: arguments.join(" "),
                output: String::from_utf8_lossy(&finished.stderr)
                    .trim_end_matches('\n')
                    .to_owned(),
            });
        }
        Ok(finished.stdout)
    }

    /// The unlock, with the master password on the child's standard input and
    /// the session key read off its standard output.
    fn unlock(&mut self, master: &Secret) -> Result<()> {
        let mut child = Command::new(&self.program)
            .arg("--nointeraction")
            .args(unlock_arguments())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|cause| Error::BitwardenUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        {
            let mut pipe = child.stdin.take().ok_or(Error::StorePipeMissing)?;
            // The newline is what ends the line the client reads, and the
            // module header records the measurement that it reads exactly one.
            master
                .write_to(&mut pipe)
                .and_then(|()| pipe.write_all(b"\n"))
                .and_then(|()| pipe.flush())
                .map_err(|cause| Error::SecretRead { cause })?;
        }
        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::BitwardenUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if !finished.status.success() {
            return Err(Error::BitwardenCommandFailed {
                address: String::from("<the vault>"),
                arguments: unlock_arguments().join(" "),
                output: String::from_utf8_lossy(&finished.stderr)
                    .trim_end_matches('\n')
                    .to_owned(),
            });
        }
        let printed = Zeroizing::new(String::from_utf8_lossy(&finished.stdout).into_owned());
        self.session = Some(Zeroizing::new(printed.trim().to_owned()));
        Ok(())
    }
}

impl Endpoint for Bitwarden {
    /// Nothing, and the absence is deliberate.
    ///
    /// This transport's unlock is [`Bitwarden::preflight`], which takes the one
    /// thing a trait method cannot be handed — the prompt the master password is
    /// asked on — and additionally performs the refresh and the two refusals
    /// that have to precede the first read of a run. The trait's own `unlock`
    /// would be a second, weaker entry point to the same contract, which is the
    /// shape `crate::store`'s own `unlock` refuses for the same reason.
    fn unlock(&mut self) -> Result<()> {
        Ok(())
    }

    /// The value at this address and, when the declaration names a field, the
    /// fields beside it.
    ///
    /// One invocation for both, where the keepassxc transport needs two: the
    /// client prints the whole item as JSON, so the value and the fields arrive
    /// together and there is no second pipe for either to travel.
    fn read(&self, id: &str, declared: &ResolvedFields) -> Result<Option<Record>> {
        let Some(identifier) = self.resolve(id)? else {
            return Ok(None);
        };
        let item = self.read_item(&identifier)?;
        let value = match value_of(&item) {
            Some(value) => Some(Secret::read_from(&mut value.as_bytes())?),
            None => None,
        };
        Ok(Some(Record {
            value,
            fields: held_fields(&item, declared)?,
        }))
    }

    /// The value and its declared fields, in one write, with no memory.
    ///
    /// A two-way mapping's memory goes in the same write, which is what
    /// [`Bitwarden::write_item`] takes and this does not: the trait has no
    /// parameter for it, and a second write to add it would be the window the
    /// single write exists to close.
    fn write(&mut self, id: &str, record: &Record, existing: Existing) -> Result<()> {
        let Some(value) = record.value.as_ref() else {
            return Err(Error::NoValueRead);
        };
        self.write_item(id, value, &record.fields, None, existing)
    }

    /// Every item under the folders the declarations name.
    ///
    /// The addresses, in the shape a report names them, so that a lingering
    /// item reads the same way a declared one does.
    fn list(&self) -> Result<Vec<String>> {
        let mut held = Vec::new();
        for folder in &self.declared_folders {
            let wanted = self.folder_id(folder.as_deref())?;
            let printed = self.invoke(&list_arguments(), None)?;
            for item in array_of(&printed)? {
                if text_at_value(&item, "folderId") != wanted {
                    continue;
                }
                let Some(name) = text_at_value(&item, "name") else {
                    continue;
                };
                let address = match folder {
                    Some(folder) => format!("{folder}/{name}"),
                    None => name,
                };
                if !held.contains(&address) {
                    held.push(address);
                }
            }
        }
        held.sort();
        Ok(held)
    }

    fn capabilities(&self) -> Capabilities {
        CAPABILITIES
    }
}

/// The client [`PROGRAM_OVERRIDE`] names, or `bw`.
///
/// The shape `crate::enroll::custody`'s own `named` has, read the same way and
/// with the same empty-value rule: an override set to nothing is no override.
#[must_use]
pub fn program() -> PathBuf {
    std::env::var_os(PROGRAM_OVERRIDE)
        .filter(|value| !value.is_empty())
        .map_or_else(|| PathBuf::from("bw"), PathBuf::from)
}

/// The environment one invocation carries, which is the session key and
/// nothing else.
///
/// A function rather than a line inside [`Bitwarden::invoke`] so that the one
/// place the key may travel is a thing a test can enumerate; the narrowing B3
/// records is asserted over this.
#[must_use]
pub fn session_environment(session: Option<&str>) -> Vec<(&'static str, String)> {
    match session {
        Some(key) => vec![(SESSION_VARIABLE, key.to_owned())],
        None => Vec::new(),
    }
}

/// The argument vector the client's own state is read through.
#[must_use]
pub fn status_arguments() -> Vec<String> {
    vec![String::from("status")]
}

/// The argument vector the unlock is performed through.
///
/// `--passwordfile /dev/stdin` and never `--passwordenv`: the master password
/// is the credential that outlives the run, and an environment is not where it
/// goes. The module header records the measurement that the client reads the
/// path as a stream.
#[must_use]
pub fn unlock_arguments() -> Vec<String> {
    vec![
        String::from("--raw"),
        String::from("unlock"),
        String::from("--passwordfile"),
        String::from(PASSWORD_FILE),
    ]
}

/// The argument vector the client's local copy is refreshed through.
#[must_use]
pub fn sync_arguments() -> Vec<String> {
    vec![String::from("sync")]
}

/// The argument vector the client is locked through.
#[must_use]
pub fn lock_arguments() -> Vec<String> {
    vec![String::from("lock")]
}

/// The argument vector the vault's folders are listed through.
#[must_use]
pub fn folders_arguments() -> Vec<String> {
    vec![String::from("list"), String::from("folders")]
}

/// The argument vector every item is listed through, for the lingering report.
#[must_use]
pub fn list_arguments() -> Vec<String> {
    vec![String::from("list"), String::from("items")]
}

/// The argument vector one declared item name is searched for through.
///
/// The name is a declared string out of a world-readable store, never a value:
/// what a declaration writes may travel an argument vector, and what an
/// encrypted entry holds may not.
#[must_use]
pub fn search_arguments(item: &str) -> Vec<String> {
    vec![
        String::from("list"),
        String::from("items"),
        String::from("--search"),
        item.to_owned(),
    ]
}

/// The argument vector one item is read through.
#[must_use]
pub fn get_arguments(identifier: &str) -> Vec<String> {
    vec![
        String::from("get"),
        String::from("item"),
        identifier.to_owned(),
    ]
}

/// The argument vector one item is created through.
///
/// No positional payload: the documented `<encodedJson>` form would put the
/// value in an argument vector, and the client reads the payload from standard
/// input when the positional is absent — which the module header records as
/// measured.
#[must_use]
pub fn create_arguments() -> Vec<String> {
    vec![String::from("create"), String::from("item")]
}

/// The argument vector one item is edited through, for [`create_arguments`]'s
/// reason.
#[must_use]
pub fn edit_arguments(identifier: &str) -> Vec<String> {
    vec![
        String::from("edit"),
        String::from("item"),
        identifier.to_owned(),
    ]
}

/// Whether a declared server and a reached one are the same server.
///
/// Compared without a trailing slash, because `https://vault.example.org` and
/// `https://vault.example.org/` are one server and refusing over the byte would
/// be a refusal about punctuation.
fn same_server(declared: &str, reached: &str) -> bool {
    declared.trim_end_matches('/') == reached.trim_end_matches('/')
}

/// The item identifier an argument vector names, for a refusal that says what
/// it refused over.
fn address_in(arguments: &[String]) -> String {
    arguments
        .last()
        .filter(|_| arguments.len() > 2)
        .cloned()
        .unwrap_or_else(|| String::from("<the vault>"))
}

/// The JSON object a client's answer carries, read from its first `{`.
///
/// From the first brace rather than from byte zero, because the client writes
/// its own `Could not find dir, …` notices around the answer — measured, and
/// recorded in the module header.
fn object_of(printed: &[u8]) -> Result<Map<String, Value>> {
    let text = String::from_utf8_lossy(printed);
    let at = text.find('{').unwrap_or(0);
    let body = text.get(at..).unwrap_or_default();
    serde_json::from_str::<Map<String, Value>>(body).map_err(|cause| {
        Error::BitwardenCommandFailed {
            address: String::from("<the vault>"),
            arguments: String::from("<a json answer>"),
            output: cause.to_string(),
        }
    })
}

/// The JSON array a client's answer carries, read from its first `[`.
fn array_of(printed: &[u8]) -> Result<Vec<Value>> {
    let text = String::from_utf8_lossy(printed);
    let at = text.find('[').unwrap_or(0);
    let body = text.get(at..).unwrap_or_default();
    serde_json::from_str::<Vec<Value>>(body).map_err(|cause| Error::BitwardenCommandFailed {
        address: String::from("<the vault>"),
        arguments: String::from("<a json answer>"),
        output: cause.to_string(),
    })
}

/// One string member of an object, or none where it is absent or null.
fn text_at(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|text| !text.is_empty())
}

/// One string member of a value that may not be an object at all.
fn text_at_value(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|text| !text.is_empty())
}

/// The value an item carries, as the client prints it: `login.password`.
fn value_of(item: &Value) -> Option<String> {
    item.get("login")
        .and_then(|login| login.get("password"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// One named custom field of an item, or none where it carries no such field.
fn custom_field(item: &Value, name: &str) -> Option<String> {
    item.get("fields")?
        .as_array()?
        .iter()
        .find(|field| text_at_value(field, "name").as_deref() == Some(name))
        .and_then(|field| text_at_value(field, "value"))
}

/// The fields an item carries, read for the declared ones alone.
///
/// A declaration that names no field reads nothing, which is
/// [`ResolvedFields::wanted`]'s own contract: safix declares nothing about a
/// field the mapping does not name, and an item's own username is the
/// operator's until a declaration takes it.
///
/// Read as [`Field::Resolved`] whatever the declaration's own shape, because
/// what the vault holds is a value out of the vault; [`Field::equals`] is what
/// compares it against a declared literal.
fn held_fields(item: &Value, declared: &ResolvedFields) -> Result<ResolvedFields> {
    let mut held = ResolvedFields::default();
    for field in declared.wanted() {
        let carried = match field {
            FieldName::Username => item
                .get("login")
                .and_then(|login| login.get("username"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            FieldName::Url => item
                .get("login")
                .and_then(|login| login.get("uris"))
                .and_then(Value::as_array)
                .and_then(|uris| uris.first())
                .and_then(|uri| text_at_value(uri, "uri")),
            FieldName::Notes => item
                .get("notes")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .filter(|notes| !notes.is_empty()),
            // Never carried, so never read: a declared tag is refused at
            // evaluation and cannot reach here.
            FieldName::Tags => None,
        };
        let Some(carried) = carried else {
            continue;
        };
        let carried = Field::Resolved(Secret::read_from(&mut carried.as_bytes())?);
        match field {
            FieldName::Username => held.username = Some(carried),
            FieldName::Url => held.url = Some(carried),
            FieldName::Notes => held.notes = Some(carried),
            FieldName::Tags => held.tags.push(carried),
        }
    }
    Ok(held)
}

/// One JSON member of a payload, with a secret leaf that has no other egress.
///
/// The reason this exists rather than a [`Value`] tree: a value may not be put
/// in a `Value::String`, because that is a plaintext `String` with no zeroizing
/// owner, reachable for as long as the tree is. Each leaf that carries a secret
/// stays a [`Secret`] until the serializer writes it, and the whole serialized
/// payload is held in a zeroizing buffer by the one caller.
enum Piece<'a> {
    /// A member of the fetched item the declaration does not govern, carried
    /// forward exactly as the client printed it.
    Kept(&'a Value),
    /// A secret leaf: the value, or a field resolved out of another entry.
    Secret(&'a Secret),
    /// A string safix wrote, which came out of a declaration and is therefore
    /// not a secret.
    Text(String),
    /// Bitwarden's own field type, which is a number.
    Number(u8),
    /// An object safix assembles.
    Object(Vec<(String, Piece<'a>)>),
    /// An array safix assembles.
    Array(Vec<Piece<'a>>),
    /// A member that is explicitly nothing, which is not the same as one that
    /// is absent.
    Null,
}

impl serde::Serialize for Piece<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Kept(value) => value.serialize(serializer),
            Self::Secret(secret) => {
                let mut exposed = Zeroizing::new(Vec::new());
                secret
                    .write_to(&mut *exposed)
                    .map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&String::from_utf8_lossy(&exposed))
            }
            Self::Text(text) => serializer.serialize_str(text),
            Self::Number(number) => serializer.serialize_u8(*number),
            Self::Object(members) => {
                use serde::ser::SerializeMap as _;
                let mut map = serializer.serialize_map(Some(members.len()))?;
                for (key, value) in members {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Array(items) => {
                use serde::ser::SerializeSeq as _;
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(item)?;
                }
                sequence.end()
            }
            Self::Null => serializer.serialize_none(),
        }
    }
}

/// The payload one write sends: the fetched item with the value, the declared
/// fields and the memory set on it, or a fresh login item where the vault holds
/// none.
///
/// Read-modify-write rather than a fresh payload for an existing item, because
/// `bw edit item` replaces the whole item: every member the declaration does
/// not govern is carried forward from what the client printed, which is what
/// keeps a `totp`, a second URI or somebody's own custom field from being
/// deleted by a value repair.
fn payload_of(
    base: Option<&Value>,
    name: &str,
    folder: Option<&str>,
    value: &Secret,
    fields: &ResolvedFields,
    memory: Option<&str>,
) -> Result<Zeroizing<Vec<u8>>> {
    let empty = Map::new();
    let held = base.and_then(Value::as_object).unwrap_or(&empty);

    let mut members: Vec<(String, Piece<'_>)> = Vec::new();
    for (key, carried) in held {
        // The three members below are assembled rather than kept, because the
        // declaration governs them.
        if matches!(key.as_str(), "login" | "notes" | "fields") {
            continue;
        }
        members.push((key.clone(), Piece::Kept(carried)));
    }
    if !held.contains_key("object") {
        members.push((String::from("object"), Piece::Text(String::from("item"))));
    }
    if !held.contains_key("type") {
        // Bitwarden's own type 1, a login item, which is the one shape that
        // carries a password at all.
        members.push((String::from("type"), Piece::Number(1)));
    }
    if !held.contains_key("name") {
        members.push((String::from("name"), Piece::Text(name.to_owned())));
    }
    if !held.contains_key("folderId") {
        members.push((
            String::from("folderId"),
            match folder {
                Some(folder) => Piece::Text(folder.to_owned()),
                None => Piece::Null,
            },
        ));
    }

    members.push((String::from("login"), login_of(held, value, fields)));
    members.push((String::from("notes"), notes_of(held, fields)));
    members.push((String::from("fields"), custom_fields_of(held, memory)));

    let serialized = serde_json::to_vec(&Piece::Object(members)).map_err(|cause| {
        Error::BitwardenCommandFailed {
            address: name.to_owned(),
            arguments: String::from("<the item payload>"),
            output: cause.to_string(),
        }
    })?;
    Ok(Zeroizing::new(serialized))
}

/// The item's `login`, with the value as its password and the declared
/// username and url beside it.
///
/// Every other member of an existing `login` — a `totp`, a second URI past the
/// first — is carried forward: the declaration governs three fields and says
/// nothing about the rest of the item.
fn login_of<'a>(
    held: &'a Map<String, Value>,
    value: &'a Secret,
    fields: &'a ResolvedFields,
) -> Piece<'a> {
    let login = held.get("login").and_then(Value::as_object);

    let mut members: Vec<(String, Piece<'a>)> = Vec::new();
    for (key, carried) in login.into_iter().flatten() {
        if matches!(key.as_str(), "password" | "username" | "uris") {
            continue;
        }
        members.push((key.clone(), Piece::Kept(carried)));
    }
    members.push((String::from("password"), Piece::Secret(value)));
    members.push((
        String::from("username"),
        match fields.username.as_ref() {
            Some(username) => piece_of(username),
            None => login
                .and_then(|login| login.get("username"))
                .map_or(Piece::Null, Piece::Kept),
        },
    ));
    members.push((String::from("uris"), uris_of(login, fields)));
    Piece::Object(members)
}

/// The item's `login.uris`, with the declared url as the first entry.
///
/// A declared url replaces the first URI and leaves every later one where it
/// was, because the declaration names one address and says nothing about the
/// others the person put there.
fn uris_of<'a>(login: Option<&'a Map<String, Value>>, fields: &'a ResolvedFields) -> Piece<'a> {
    let held: Vec<&Value> = login
        .and_then(|login| login.get("uris"))
        .and_then(Value::as_array)
        .map(|uris| uris.iter().collect())
        .unwrap_or_default();

    let Some(url) = fields.url.as_ref() else {
        return Piece::Array(held.into_iter().map(Piece::Kept).collect());
    };
    let mut items = vec![Piece::Object(vec![
        (String::from("match"), Piece::Null),
        (String::from("uri"), piece_of(url)),
    ])];
    items.extend(held.into_iter().skip(1).map(Piece::Kept));
    Piece::Array(items)
}

/// The item's `notes`, which the declaration governs when it names them.
fn notes_of<'a>(held: &'a Map<String, Value>, fields: &'a ResolvedFields) -> Piece<'a> {
    match fields.notes.as_ref() {
        Some(notes) => piece_of(notes),
        None => held.get("notes").map_or(Piece::Null, Piece::Kept),
    }
}

/// The item's custom fields, with the memory among them when the mode records
/// one.
///
/// Every custom field the person put there is carried forward, and the
/// memory's own entry is replaced rather than appended, so that a two-way
/// mapping converged twice leaves one memory rather than two.
fn custom_fields_of<'a>(held: &'a Map<String, Value>, memory: Option<&str>) -> Piece<'a> {
    let carried: Vec<&Value> = held
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().collect())
        .unwrap_or_default();

    let mut items: Vec<Piece<'a>> = Vec::new();
    for field in carried {
        if text_at_value(field, "name").as_deref() == Some(STATE_FIELD) {
            continue;
        }
        items.push(Piece::Kept(field));
    }
    if let Some(line) = memory {
        items.push(Piece::Object(vec![
            (String::from("name"), Piece::Text(String::from(STATE_FIELD))),
            (String::from("value"), Piece::Text(line.to_owned())),
            (String::from("type"), Piece::Number(HIDDEN)),
        ]));
    }
    Piece::Array(items)
}

/// One resolved field as the payload carries it: a literal stays a string, and
/// a value read out of another entry stays a secret until it is written.
fn piece_of(field: &Field) -> Piece<'_> {
    match field {
        Field::Literal(text) => Piece::Text(text.clone()),
        Field::Resolved(secret) => Piece::Secret(secret),
    }
}

/// The base64 of one payload, computed here rather than by a `bw encode`
/// child.
///
/// The encoding is a pure function of bytes safix already holds, so a second
/// process would only be a second process the plaintext passes through.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk.first().copied().unwrap_or(0);
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let triple = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        let sextet = |shift: u32| {
            let at = usize::try_from((triple >> shift) & 0x3F).unwrap_or(0);
            ALPHABET.get(at).copied().map_or('=', char::from)
        };
        encoded.push(sextet(18));
        encoded.push(sextet(12));
        encoded.push(if chunk.len() > 1 { sextet(6) } else { '=' });
        encoded.push(if chunk.len() > 2 { sextet(0) } else { '=' });
    }
    encoded
}

/// One mapping's line in a run's report.
pub struct Converged {
    /// The mapping's declared name.
    pub mapping: String,
    /// How it converges.
    pub mode: BitwardenMode,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// The vault endpoint, as the address the declaration names.
    pub address: String,
    /// What happened.
    pub outcome: Outcome,
}

/// Everything a run judged, and what it found beside it.
pub struct Report {
    /// The server the run converged against, as the declaration names it or as
    /// the empty string where none was declared.
    pub server: String,
    /// One entry per declared mapping, in declaration order. Every declared
    /// mapping is here, whatever happened to it.
    pub converged: Vec<Converged>,
    /// Items under a declared folder that no declared mapping names.
    ///
    /// Information rather than a finding: no mode deletes an item, so a mapping
    /// that was removed leaves its last value behind on purpose, and only a
    /// person removes it.
    pub lingering: Vec<String>,
}

impl Report {
    /// Whether every mapping converged without a conflict, a refusal or an
    /// unjudgeable side.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        !self
            .converged
            .iter()
            .any(|entry| entry.outcome.is_failure())
    }

    /// How many mappings ended in each outcome.
    #[must_use]
    pub fn tally(&self) -> Tally {
        let count = |wanted: &str| {
            self.converged
                .iter()
                .filter(|entry| entry.outcome.as_str() == wanted)
                .count()
        };
        Tally {
            unchanged: count("unchanged"),
            updated: count("updated"),
            pulled: count("pulled"),
            fields_updated: count("fields updated"),
            fields_diverged: count("fields diverged"),
            conflict: count("conflict"),
            refused: count("refused"),
            not_judged: count("not judged"),
        }
    }
}

/// A value already in hand, for the write path that expects to read one.
///
/// The same seam `crate::sync` uses and for the same reason: a mirrored value
/// was read once, from the vault, and there is nothing to compare it against,
/// so this hands it over and the rest of the write path is unchanged. Its own
/// copy rather than that module's, because that one is private to the target it
/// was written for and this target drives its own convergence.
struct Held(Option<Secret>);

impl ValueSource for Held {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        self.0.take().ok_or(Error::NoValueRead)
    }
}

/// What a mapping's two sides came back as, and what is to be done about it.
///
/// No `Debug`, and every value here is zeroed when the run returns.
enum Decision {
    /// Nothing to do.
    Settled(Outcome),
    /// Write the vault's side.
    Push(Pushing),
    /// Write this value into safix, and record the agreement when the mode
    /// remembers one.
    Pull { value: Secret, remember: bool },
}

/// One vault write: the value, the declared fields beside it, whether the item
/// is already there, and whether the agreement is recorded in the same write.
struct Pushing {
    /// The value the item will hold, which is the one it already holds when
    /// only the fields moved.
    value: Secret,
    /// Whether this mode records the agreement, in this same write.
    remember: bool,
    /// The declared fields, resolved, that ride along.
    resolved: ResolvedFields,
    /// Whether the vault already holds the item.
    existing: Existing,
    /// The declared fields the item did not match, when the value itself
    /// agreed; empty when the value is what moved.
    drifted: Vec<&'static str>,
}

impl Pushing {
    /// What a completed write of this push is reported as.
    fn outcome(self) -> Outcome {
        if self.drifted.is_empty() {
            return Outcome::Updated;
        }
        Outcome::FieldsUpdated(self.drifted)
    }
}

/// Converge every declared mapping, or the ones named.
///
/// # No write burst, and that asymmetry is a decision
///
/// `crate::sync` reads every mapping, decides, and only then writes, because a
/// kdbx save rewrites and re-uploads the whole database — 292 MB on the fleet
/// it was written for (`crate::sync`'s own header, its "Two phases" section).
/// This target writes one item per call over a network, so deferring the writes
/// would buy nothing and would delay every safix-side commit to the end of the
/// run. Each mapping is decided and acted on in turn.
///
/// # Errors
///
/// [`Error::UnknownSyncMapping`] when a named mapping is not one that is
/// declared, and every refusal [`Bitwarden::preflight`] raises — an
/// unavailable client, a locked or unauthenticated one, a declared server that
/// is not the one reached, and a failed refresh. Each of those stops the whole
/// run and is raised before the first mapping is read, for the reason
/// `crate::sync` raises its own there: a run that discovered them partway
/// through would already have said "unchanged" about mappings it never looked
/// at. Anything about one mapping is that mapping's outcome rather than the
/// run's.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    password: &mut Prompt<'_>,
    only: &[String],
) -> Result<Report> {
    scratch::set_floor(workspace.vault_root());
    let _guard = scratch::Guard;

    let declared = workspace.bitwarden()?;
    let selected = selected(declared, only)?;
    if selected.is_empty() {
        return Ok(Report {
            server: declared.server.clone().unwrap_or_default(),
            converged: Vec::new(),
            lingering: Vec::new(),
        });
    }

    let mut vault = Bitwarden::new(declared);
    if let Err(reason) = vault.preflight(password) {
        vault.lock();
        return Err(reason);
    }

    let converged = converge(workspace, progress, &vault, declared, &selected);
    let lingering = lingering(&vault, declared);
    vault.lock();

    Ok(Report {
        server: vault.server(),
        converged,
        lingering,
    })
}

/// Every selected mapping decided and acted on, in declaration order.
fn converge(
    workspace: &Workspace,
    progress: &dyn Progress,
    vault: &Bitwarden,
    declared: &Declared,
    selected: &[&BitwardenMapping],
) -> Vec<Converged> {
    let mut converged = Vec::with_capacity(selected.len());
    for mapping in selected {
        let address = declared.address_of(mapping);
        let outcome = match decide(workspace, vault, mapping, &address) {
            Decision::Settled(outcome) => outcome,
            Decision::Push(pushing) => push(progress, vault, mapping, &address, pushing),
            Decision::Pull { value, remember } => pull(
                workspace, progress, vault, mapping, &address, value, remember,
            ),
        };
        converged.push(Converged {
            mapping: mapping.id.clone(),
            mode: mapping.mode,
            safix: format!("{}.{}", mapping.safix.user, mapping.safix.name),
            address,
            outcome,
        });
    }
    converged
}

/// The mappings one run acts on, refusing before any of them is touched.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s bitwarden target
/// reuses this exact selection, so that scoping a comparison and scoping a
/// write cannot answer "which mappings" differently.
pub(crate) fn selected<'a>(
    declared: &'a Declared,
    only: &[String],
) -> Result<Vec<&'a BitwardenMapping>> {
    if only.is_empty() {
        return Ok(declared.mappings.iter().collect());
    }
    let mut mappings = Vec::with_capacity(only.len());
    for id in only {
        let mapping = declared
            .named(id)
            .ok_or_else(|| Error::UnknownSyncMapping {
                mapping: id.clone(),
                declared: declared.declared(),
            })?;
        mappings.push(mapping);
    }
    Ok(mappings)
}

/// Items under a declared folder that no declared mapping accounts for.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s bitwarden target
/// reports the identical list rather than a second computation of it.
///
/// A client that will not enumerate yields nothing rather than a refusal: the
/// lingering list is information beside the report, and a run whose mappings
/// all converged is not made a failure by an item nobody declared.
pub(crate) fn lingering(vault: &Bitwarden, declared: &Declared) -> Vec<String> {
    let claimed: Vec<String> = declared
        .mappings
        .iter()
        .map(|mapping| declared.address_of(mapping))
        .collect();
    vault
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter(|address| !claimed.contains(address))
        .collect()
}

/// Read both sides of one mapping and decide what its mode says to do.
///
/// The value verdict is decided first and a field divergence never displaces
/// it, for the reason `crate::sync`'s own `decide` gives: the same write that
/// repairs the value carries the fields, so reporting the lesser fact would
/// bury the greater one.
fn decide(
    workspace: &Workspace,
    vault: &Bitwarden,
    mapping: &BitwardenMapping,
    address: &str,
) -> Decision {
    // Before either side is read: a declaration this target cannot honour is
    // refused rather than partly written, and the refusal costs no read.
    let declared = match endpoint::resolve_fields(
        workspace,
        "bitwarden",
        &mapping.safix.user,
        &mapping.bitwarden.fields,
        &CAPABILITIES,
    ) {
        Ok(declared) => declared,
        Err(reason) => return Decision::Settled(Outcome::Refused(reason)),
    };

    let held = match vault.read(address, &declared) {
        Ok(held) => held,
        // An ambiguous address is a refusal of the mapping rather than an
        // unjudgeable side: nothing about the vault is unknown, and what the
        // operator has to do is rename one of two items.
        Err(reason @ Error::BitwardenItemAmbiguous { .. }) => {
            return Decision::Settled(Outcome::Refused(reason));
        }
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
    };
    let existing = if held.is_some() {
        Existing::Present
    } else {
        Existing::Absent
    };
    let (theirs, held_fields) = match held {
        Some(record) => (record.value, record.fields),
        None => (None, ResolvedFields::default()),
    };
    let ours = match bridge::held_by_safix(
        workspace,
        &mapping.id,
        &mapping.safix.user,
        &mapping.safix.name,
    ) {
        Ok(held) => held,
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
    };

    let drifted = declared.diff(&held_fields);

    match mapping.mode {
        BitwardenMode::SafixToBitwarden => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                repairing_fields(ours, declared, drifted, existing, false)
            }
            (Some(ours), _) => Decision::Push(Pushing {
                value: ours,
                remember: false,
                resolved: declared,
                existing,
                drifted: Vec::new(),
            }),
        },

        BitwardenMode::BitwardenToSafix => match (ours, theirs) {
            (_, None) => Decision::Settled(Outcome::Refused(Error::BitwardenItemAbsent {
                mapping: mapping.id.clone(),
                address: address.to_owned(),
                mode: mapping.mode.as_str(),
            })),
            // The declaration is the author of a field, and this mode writes
            // safix rather than the vault, so a field difference is reported
            // and nothing is written.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            (_, Some(theirs)) => Decision::Pull {
                value: theirs,
                remember: false,
            },
        },

        BitwardenMode::Backup => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            // Absence is the one case `backup` writes, so the declared fields
            // ride along with the value that creates the item.
            (Some(ours), None) => Decision::Push(Pushing {
                value: ours,
                remember: false,
                resolved: declared,
                existing,
                drifted: Vec::new(),
            }),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            // The whole of what `backup` is: an existing differing value is
            // reported and never overwritten.
            (Some(_), Some(_)) => Decision::Settled(Outcome::Conflict),
        },

        BitwardenMode::TwoWay => two_way(vault, address, ours, theirs, declared, drifted, existing),
    }
}

/// A two-way mapping's decision: the shared judge's verdict over the two
/// values and the agreement the item's own hidden field remembers.
///
/// The memory is consulted only where both sides hold a value and they differ,
/// which is the only case the judge asks about it, and an absent or unreadable
/// memory is an absence rather than a refusal.
fn two_way(
    vault: &Bitwarden,
    address: &str,
    ours: Option<Secret>,
    theirs: Option<Secret>,
    declared: ResolvedFields,
    drifted: Vec<&'static str>,
    existing: Existing,
) -> Decision {
    let differ = match (ours.as_ref(), theirs.as_ref()) {
        (Some(ours), Some(theirs)) => !ours.equals(theirs),
        _ => false,
    };
    let remembered = if differ {
        vault.recorded(address)
    } else {
        None
    };
    let agreement = |remembered: &Secret, value: &Secret| agrees(remembered, value);
    let verdict = endpoint::judge(
        ours.as_ref(),
        theirs.as_ref(),
        remembered.as_ref(),
        &agreement,
    );
    match verdict {
        Verdict::Unchanged => match ours {
            // A field has one author, so a two-way mapping's fields converge
            // toward the declaration — and no agreement is recorded for one,
            // because there is nothing for a memory to arbitrate.
            Some(ours) => repairing_fields(ours, declared, drifted, existing, false),
            None => Decision::Settled(Outcome::Unchanged),
        },
        Verdict::Conflict => Decision::Settled(Outcome::Conflict),
        Verdict::PushValue { remember } => match ours {
            Some(value) => Decision::Push(Pushing {
                value,
                remember,
                resolved: declared,
                existing,
                drifted: Vec::new(),
            }),
            None => Decision::Settled(Outcome::Conflict),
        },
        Verdict::PullValue { remember } => match theirs {
            Some(value) => Decision::Pull { value, remember },
            None => Decision::Settled(Outcome::Conflict),
        },
    }
}

/// The two sides' values agree: either there is nothing to do, or the declared
/// fields moved and this mode writes them.
fn repairing_fields(
    ours: Secret,
    resolved: ResolvedFields,
    drifted: Vec<&'static str>,
    existing: Existing,
    remember: bool,
) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Push(Pushing {
        value: ours,
        remember,
        resolved,
        existing,
        drifted,
    })
}

/// The two sides' values agree and this mode does not write the vault's side,
/// so a field difference is the finding.
fn diverging_fields(drifted: Vec<&'static str>) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Settled(Outcome::FieldsDiverged(drifted))
}

/// Whether the remembered agreement is the one this value would have recorded.
///
/// Compared as bytes against the line a converging write would have written,
/// which is what makes a memory written under a tag this version does not know
/// behave as no memory at all.
fn agrees(remembered: &Secret, value: &Secret) -> bool {
    let line = memory_of(value);
    Secret::read_from(&mut line.as_bytes()).is_ok_and(|written| written.equals(remembered))
}

/// The line a converging two-way write records the agreement as.
///
/// `crate::sync::FORMAT` rather than a tag of this target's own: one reader
/// understands both targets' memories, which is what
/// `openspec/changes/add-bitwarden-bridge/design.md`'s B5 asks for.
#[must_use]
pub fn memory_of(value: &Secret) -> String {
    sync::memory_of(value)
}

/// safix holds nothing for this mapping, and its mode makes safix the source.
///
/// [`Error::SyncSourceEmpty`] rather than a variant of this target's own: the
/// refusal is about safix's own half and says nothing about a vault.
fn empty_source(workspace: &Workspace, mapping: &BitwardenMapping) -> Error {
    let (file, generated) = workspace
        .resolve(&mapping.safix.user, &mapping.safix.name)
        .map_or_else(
            |_| (String::new(), false),
            |placement| (placement.file.clone(), placement.generator.is_some()),
        );
    Error::SyncSourceEmpty {
        mapping: mapping.id.clone(),
        user: mapping.safix.user.clone(),
        name: mapping.safix.name.clone(),
        file,
        generated,
    }
}

/// Write the vault's side — the value, the declared fields and the agreement —
/// in one invocation.
fn push(
    progress: &dyn Progress,
    vault: &Bitwarden,
    mapping: &BitwardenMapping,
    address: &str,
    pushing: Pushing,
) -> Outcome {
    log(
        progress,
        &format!(
            "safix: flake.safix.users.{}.{} -> {address}",
            mapping.safix.user, mapping.safix.name,
        ),
    );
    let memory = if pushing.remember {
        Some(memory_of(&pushing.value))
    } else {
        None
    };
    if let Err(reason) = vault.write_item(
        address,
        &pushing.value,
        &pushing.resolved,
        memory.as_deref(),
        pushing.existing,
    ) {
        return Outcome::Refused(reason);
    }
    pushing.outcome()
}

/// Write safix's side through the ordinary write path, and the agreement when
/// the mode remembers one.
///
/// Only the value crosses. A field is a declaration and safix's side has
/// nowhere to hold one, so this function is unchanged by the field axis except
/// for the memory's own write, which carries the fields the declaration names
/// because it is the same item write.
fn pull(
    workspace: &Workspace,
    progress: &dyn Progress,
    vault: &Bitwarden,
    mapping: &BitwardenMapping,
    address: &str,
    value: Secret,
    remember: bool,
) -> Outcome {
    log(
        progress,
        &format!(
            "safix: {address} -> flake.safix.users.{}.{}",
            mapping.safix.user, mapping.safix.name,
        ),
    );

    // The memory is derived before the write path below consumes the value.
    let memory = memory_of(&value);
    let status = set::run_committing(
        workspace,
        progress,
        &mut Held(Some(value)),
        &mapping.safix.user,
        &mapping.safix.name,
        &commit_subject(mapping),
    );
    match status {
        Err(reason) => return Outcome::Refused(reason),
        // sops refused and has said why on its own standard error, which is
        // inherited. Reporting it as this mapping's refusal keeps the rest of
        // the run going and the report honest about which mapping it was.
        Ok(status) if status != 0 => {
            return Outcome::Refused(Error::BitwardenCommandFailed {
                address: address.to_owned(),
                arguments: String::from("<the safix write path>"),
                output: format!("sops exited {status}; its own message is above"),
            });
        }
        Ok(_) => {}
    }

    if remember {
        // The memory lands beside the value the vault already holds, in the
        // one write this transport makes: the item is read, the memory set on
        // it, and the whole item sent back.
        let held = match vault.read(address, &ResolvedFields::default()) {
            Ok(Some(record)) => record,
            Ok(None) => return Outcome::Pulled,
            Err(reason) => return Outcome::Refused(reason),
        };
        let Some(carried) = held.value else {
            return Outcome::Pulled;
        };
        if let Err(reason) = vault.write_item(
            address,
            &carried,
            &ResolvedFields::default(),
            Some(&memory),
            Existing::Present,
        ) {
            return Outcome::Refused(reason);
        }
    }
    Outcome::Pulled
}

/// The commit subject a mirrored value lands under.
#[must_use]
pub fn commit_subject(mapping: &BitwardenMapping) -> String {
    format!(
        "chore(safix): sync {} for {}",
        mapping.id, mapping.safix.user
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(text: &str) -> Secret {
        Secret::read_from(&mut text.as_bytes()).expect("a fixture value reads")
    }

    fn declared_fields() -> ResolvedFields {
        ResolvedFields {
            username: Some(Field::Literal(String::from("alice@example.com"))),
            url: Some(Field::Literal(String::from(
                "https://grafana.example.invalid",
            ))),
            notes: Some(Field::Literal(String::from("minted by safix"))),
            tags: Vec::new(),
        }
    }

    /// The namesake of `crate::store`'s own argv test, over this transport's
    /// three credentials: the mapped value, the master password, and the
    /// session key.
    ///
    /// Every invocation this module constructs is built here and asserted
    /// against the fixture strings, and the session key is asserted to appear
    /// in exactly one environment entry. Drill 4.13 is moving the key into a
    /// `--session <key>` argument and watching this turn red.
    #[test]
    fn no_argument_vector_can_carry_a_value() {
        let value = "fixture-value";
        let master = "fixture-master-password";
        let session = "fixture-session-key";

        let every = [
            status_arguments(),
            unlock_arguments(),
            sync_arguments(),
            lock_arguments(),
            folders_arguments(),
            list_arguments(),
            search_arguments("grafana"),
            get_arguments("3a1c"),
            create_arguments(),
            edit_arguments("3a1c"),
        ];
        for arguments in &every {
            for argument in arguments {
                assert!(!argument.contains(value), "{argument} carries the value");
                assert!(
                    !argument.contains(master),
                    "{argument} carries the master password"
                );
                assert!(
                    !argument.contains(session),
                    "{argument} carries the session key"
                );
            }
        }

        let environment = session_environment(Some(session));
        assert_eq!(environment.len(), 1);
        assert_eq!(
            environment.first().map(|(name, _)| *name),
            Some(SESSION_VARIABLE)
        );
        assert_eq!(
            environment.first().map(|(_, key)| key.as_str()),
            Some(session)
        );
        assert!(session_environment(None).is_empty());
    }

    /// The payload the unlock's password travels is standard input, and the
    /// path it names is the stream rather than a file safix writes.
    #[test]
    fn the_master_password_travels_a_stream_and_never_an_environment() {
        let arguments = unlock_arguments();
        assert!(arguments.iter().any(|argument| argument == PASSWORD_FILE));
        assert!(!arguments.iter().any(|argument| argument == "--passwordenv"));
        assert_eq!(PASSWORD_FILE, "/dev/stdin");
    }

    /// B6, held by an instrument: every member of the fetched item the
    /// declaration does not govern survives the write. Drill 4.12 is building
    /// a fresh payload instead of mutating the fetched one, which turns this
    /// red on all three.
    #[test]
    fn an_edit_carries_the_rest_of_the_item_forward() {
        let held: Value = serde_json::from_str(
            r#"{
              "object": "item",
              "id": "3a1c",
              "type": 1,
              "name": "grafana",
              "folderId": "f1",
              "notes": "the operator's note",
              "favorite": true,
              "login": {
                "username": "someone-else",
                "password": "the-old-value",
                "totp": "otpauth://totp/fixture",
                "uris": [
                  { "match": null, "uri": "https://first.example.invalid" },
                  { "match": null, "uri": "https://second.example.invalid" }
                ]
              },
              "fields": [ { "name": "operator-field", "value": "kept", "type": 0 } ]
            }"#,
        )
        .expect("the fixture item parses");

        let payload = payload_of(
            Some(&held),
            "grafana",
            Some("f1"),
            &secret("the-new-value"),
            &declared_fields(),
            None,
        )
        .expect("the payload builds");
        let sent: Value =
            serde_json::from_slice(&payload).expect("the payload is the JSON the client reads");

        assert_eq!(sent.get("favorite").and_then(Value::as_bool), Some(true));
        assert_eq!(
            sent.pointer("/login/totp").and_then(Value::as_str),
            Some("otpauth://totp/fixture")
        );
        assert_eq!(
            sent.pointer("/login/uris/1/uri").and_then(Value::as_str),
            Some("https://second.example.invalid")
        );
        assert_eq!(
            sent.pointer("/fields/0/name").and_then(Value::as_str),
            Some("operator-field")
        );

        // And the three the declaration does govern moved.
        assert_eq!(
            sent.pointer("/login/password").and_then(Value::as_str),
            Some("the-new-value")
        );
        assert_eq!(
            sent.pointer("/login/username").and_then(Value::as_str),
            Some("alice@example.com")
        );
        assert_eq!(
            sent.pointer("/login/uris/0/uri").and_then(Value::as_str),
            Some("https://grafana.example.invalid")
        );
        assert_eq!(
            sent.get("notes").and_then(Value::as_str),
            Some("minted by safix")
        );
    }

    /// The memory is a hidden custom field of the item itself, in the format
    /// the keepassxc target's own memory uses — asserted against the literal,
    /// because the other side of that agreement is `crate::sync::FORMAT` and a
    /// drift between them would make one reader of two memories impossible.
    #[test]
    fn the_memory_is_a_hidden_field_in_the_shared_format() {
        let line = memory_of(&secret("the-value"));
        assert!(line.starts_with("safix-sync-v1 "));
        assert_eq!(line, sync::memory_of(&secret("the-value")));

        let payload = payload_of(
            None,
            "grafana",
            None,
            &secret("the-value"),
            &ResolvedFields::default(),
            Some(&line),
        )
        .expect("the payload builds");
        let sent: Value = serde_json::from_slice(&payload).expect("the payload is JSON");

        assert_eq!(
            sent.pointer("/fields/0/name").and_then(Value::as_str),
            Some(STATE_FIELD)
        );
        assert_eq!(
            sent.pointer("/fields/0/type").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            sent.pointer("/fields/0/value").and_then(Value::as_str),
            Some(line.as_str())
        );
        assert_eq!(STATE_FIELD, "safix-sync-state");
    }

    /// A second converge leaves one memory rather than two, which is what
    /// makes the field a memory rather than a log.
    #[test]
    fn a_second_agreement_replaces_the_first_rather_than_appending() {
        let held: Value = serde_json::from_str(
            r#"{ "id": "3a1c", "fields": [
                 { "name": "safix-sync-state", "value": "safix-sync-v1 old", "type": 1 },
                 { "name": "operator-field", "value": "kept", "type": 0 }
               ] }"#,
        )
        .expect("the fixture item parses");
        let payload = payload_of(
            Some(&held),
            "grafana",
            None,
            &secret("the-value"),
            &ResolvedFields::default(),
            Some("safix-sync-v1 new"),
        )
        .expect("the payload builds");
        let sent: Value = serde_json::from_slice(&payload).expect("the payload is JSON");
        let fields = sent
            .get("fields")
            .and_then(Value::as_array)
            .expect("the payload carries fields");
        let memories = fields
            .iter()
            .filter(|field| text_at_value(field, "name").as_deref() == Some(STATE_FIELD))
            .count();
        assert_eq!(memories, 1);
        assert_eq!(fields.len(), 2);
    }

    /// The three carried fields, each round-tripping through the payload
    /// builder and the item reader.
    #[test]
    fn each_carried_field_round_trips_through_the_payload_and_the_reader() {
        let payload = payload_of(
            None,
            "grafana",
            None,
            &secret("the-value"),
            &declared_fields(),
            None,
        )
        .expect("the payload builds");
        let sent: Value = serde_json::from_slice(&payload).expect("the payload is JSON");

        let declared = declared_fields();
        let held = held_fields(&sent, &declared).expect("the fields read back");
        assert!(
            declared.diff(&held).is_empty(),
            "a field did not round trip"
        );
    }

    /// A value carrying newlines crosses whole: this transport's value channel
    /// is a JSON payload on standard input rather than a command line, so the
    /// refusal the keepassxc target makes is not one this target inherits.
    #[test]
    fn a_multi_line_value_crosses_whole() {
        let value = "first\nsecond\nthird";
        let payload = payload_of(
            None,
            "grafana",
            None,
            &secret(value),
            &ResolvedFields::default(),
            None,
        )
        .expect("the payload builds");
        let sent: Value = serde_json::from_slice(&payload).expect("the payload is JSON");
        assert_eq!(
            sent.pointer("/login/password").and_then(Value::as_str),
            Some(value)
        );
        assert!(CAPABILITIES.multiline);
    }

    /// The tags row of the shared capability table, which is what the nix
    /// half's own refusal is judged against.
    #[test]
    fn tags_are_not_a_field_this_target_carries() {
        assert_eq!(CAPABILITIES.channel(FieldName::Tags), Channel::Unsupported);
        assert_eq!(CAPABILITIES.channel(FieldName::Username), Channel::Stdin);
        assert_eq!(CAPABILITIES.channel(FieldName::Url), Channel::Stdin);
        assert_eq!(CAPABILITIES.channel(FieldName::Notes), Channel::Stdin);
    }

    /// The encoding the payload crosses as, against the published vectors —
    /// which is the independent oracle a hand-written encoder needs.
    #[test]
    fn the_base64_vectors_come_out() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn a_declared_server_is_compared_without_its_trailing_slash() {
        assert!(same_server(
            "https://vault.example.org/",
            "https://vault.example.org"
        ));
        assert!(!same_server(
            "https://vault.example.org",
            "https://vault.bitwarden.com"
        ));
    }

    #[test]
    fn a_client_answer_is_read_from_its_first_brace() {
        let printed = b"Could not find dir, \"/x\"; creating it instead.\n{\"status\":\"locked\"}";
        let object = object_of(printed).expect("the answer parses");
        assert_eq!(text_at(&object, "status").as_deref(), Some("locked"));
    }
}
