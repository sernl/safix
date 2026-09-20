//! The 1Password service, and the whole of how `sync` and `audit` reach it.
//!
//! One transport: the service's own command, under whatever session the
//! operator's own environment already carries. Every value and every declared
//! field travels standard input as one JSON object; what travels an argument
//! vector is the vault name, the item title, the account shorthand and `op`'s
//! own flags. No `field=value` assignment statement is ever spelled — 1Password's
//! own documentation states that such a statement is recorded in shell history
//! and can be visible to other processes — and no session token is ever placed
//! in an argument vector, because a process's arguments are readable by every
//! user on the machine while its environment is not.
//!
//! # These facts are documented rather than measured, which is the difference
//! from [`crate::store`]
//!
//! `crate::store`'s header records `keepassxc-cli` 2.7.12 behaviour read off
//! scratch databases. Nothing here was measured, and nothing here can be: no
//! check of this repository may run the real program (see below). Every claim
//! below comes from 1Password's published command-line reference, and the
//! stand-in the suite drives emits the documented shape rather than a shape
//! chosen for convenience:
//!
//! - an item prints as `{ id, title, category, tags: [..], urls: [{ label,
//!   primary, href }], fields: [{ id, type, purpose, label, value }] }`, with
//!   further members this version does not name.
//! - `--reveal` is what makes a concealed field's value appear in that JSON
//!   instead of a placeholder.
//! - `--format json` is the machine-readable form of every read.
//! - `op item create -` and `op item edit -` take the whole item as one JSON
//!   object on standard input, which is the only write form safix uses.
//! - the item's `urls[]` entry is the autofill website; the `url` *field type*
//!   is explicitly not used for autofill, so a declared url goes to `urls[]`
//!   and a declared url written anywhere else would be a field safix wrote and
//!   nobody sees.
//! - an item edited from a JSON *template* loses the passkeys on it. [`Endpoint::write`]
//!   against [`Existing::Present`] therefore starts from the item's own current
//!   JSON, including every member this version does not understand
//!   ([`Item::rest`]), and replaces only the value, the declared fields and the
//!   recorded agreement. safix cannot reach that outcome, because it never sends
//!   a template — which is why no refusal about passkey-bearing items exists and
//!   an ordinary login item carrying one stays mappable.
//!
//! A contributor holding a licensed `op` should re-measure exactly those six
//! facts, in that order, and the wording this module classifies a refusal by:
//! [`NOT_A_VAULT`] and [`NOT_AN_ITEM`].
//!
//! # No check of this repository drives a real `op`, and that is permanent
//!
//! Three grounds, each sufficient alone: `_1password-cli` at this flake's pin is
//! unfree, so naming it from a check, a package or a development shell makes
//! evaluation fail for every consumer who has not allowed unfree packages; there
//! is no self-hostable 1Password server to point a sandboxed node at; and every
//! authentication path needs the network, which no `nix build` and no hermetic
//! VM node has. None of the three is a condition that may later be satisfied.
//! The claim is written here, in `modules/flake/safix/onepassword.nix` and in
//! `openspec/specs/onepassword-sync/`, rather than inferred from a missing file,
//! because an unstated absence is a claim nobody decided to stop making.
//!
//! # The refusals this transport does not have
//!
//! There is no analogue of [`Error::ValueSpansLines`], and the absence is
//! deliberate: that refusal is a `keepassxc-cli` limitation — `--password-prompt`
//! reads one line — rather than a truth about secrets, and this transport's
//! standard-input payload carries a multi-line value whole. [`CAPABILITIES`]
//! states that positively, as `multiline: true`, so nobody adds the refusal back
//! by analogy.
//!
//! There is no analogue of [`Error::FieldUnsupported`] or
//! [`Error::FieldSourceInArgv`] either, and for the same kind of reason: all four
//! fields cross on the one channel the value crosses, so both refusals are
//! unreachable for this target. They are reachable for keepassxc, which is why
//! the capability table exists rather than a shared default.
//!
//! # The memory is a concealed field of the mapped item
//!
//! A `two-way` mapping's last agreement is [`STATE_FIELD`], a `CONCEALED` custom
//! field on the item itself, carrying the shared `safix-sync-v1 <fingerprint>`
//! line [`crate::sync::FORMAT`] names. So this target reserves no item name and
//! has no reserved-name refusal: keepassxc's companion entry exists only because
//! `keepassxc-cli` cannot write a custom attribute on any verb, and `op` can.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde::Deserialize;
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use crate::endpoint::{
    self, Capabilities, Channel, Endpoint, Existing, Field, Record, ResolvedFields, Verdict,
};
use crate::enroll::custody;
use crate::error::{Error, Result};
use crate::model::{OnePassword as Mirror, OnePasswordMapping, OnePasswordMode};
use crate::progress::{Progress, log};
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::sync;
use crate::workspace::Workspace;
use crate::{bridge, scratch};

/// The environment variable that replaces the 1Password command, for checks.
///
/// The seam [`crate::enroll::custody`] establishes for every program safix
/// shells to, resolved through that module's own rule rather than a second copy
/// of it. It is the only way a check reaches this transport at all, because the
/// real program is one no check may run.
pub const OP_OVERRIDE: &str = "SAFIX_OP";

/// The custom field a `two-way` mapping records its last agreement in.
///
/// On the mapped item rather than beside it, which is what makes this target
/// reserve no name: a consumer may declare any item they like, because safix
/// claims a field of it rather than a sibling of it.
pub const STATE_FIELD: &str = "safix-sync-state";

/// What this transport can carry beside a value, and what shape of value its
/// channel accepts.
///
/// The other half of `modules/flake/safix/fields.nix`'s `channels.onepassword`,
/// which is what the nix half refuses a declaration against. Every row is
/// [`Channel::Stdin`] and `multiline` is true because the whole item is one JSON
/// object on standard input: there is one channel, it carries everything, and it
/// carries a newline.
pub const CAPABILITIES: Capabilities = Capabilities {
    username: Channel::Stdin,
    url: Channel::Stdin,
    notes: Channel::Stdin,
    tags: Channel::Stdin,
    multiline: true,
};

/// The wordings `op` refuses an unreachable vault with.
///
/// Classified rather than guessed at from an exit status, because every refusal
/// this command makes exits one: an absent item and an unreachable vault are
/// different faults with different remedies, and the only thing that tells them
/// apart is the program's own sentence. Checked before [`NOT_AN_ITEM`], because
/// a vault-absent message may carry the word "found" too.
pub const NOT_A_VAULT: [&str; 3] = ["isn't a vault", "no vault matches", "vault not found"];

/// The wordings `op` reports an absent item with, which is an ordinary absence
/// rather than a refusal.
pub const NOT_AN_ITEM: [&str; 3] = ["isn't an item", "no item matches", "item not found"];

/// The category a created item is given.
///
/// A login is what a mapped credential is: it is the category whose built-in
/// fields are the password, the username and the note this target declares, and
/// whose `urls[]` entry the browser fills from.
const CATEGORY: &str = "LOGIN";

/// The service, reached through its own command for the length of one run.
///
/// No `Debug`: the parsed items cached here carry values, and deriving one would
/// print them through the field.
pub struct OnePassword<'a> {
    /// The program, as [`OP_OVERRIDE`] named it or as `op`.
    program: PathBuf,
    /// The account shorthand every invocation names, or none to let the program
    /// resolve its own.
    account: Option<String>,
    /// The vaults the declaration names, in declaration order and without
    /// repeats: what [`Endpoint::list`] enumerates, since the trait asks what
    /// the far side holds without being told where to look.
    vaults: Vec<String>,
    /// The run's own commentary channel.
    progress: &'a dyn Progress,
    /// The item JSON last read per address, which is what the edit leg starts
    /// from.
    ///
    /// Interior mutability because [`Endpoint::read`] takes `&self` and this is
    /// its by-product rather than its answer: the alternative is a second `item
    /// get` inside the write, which is a second answer the two could disagree
    /// over — and a round trip through an item nobody re-read is the whole of
    /// how a passkey survives.
    items: RefCell<BTreeMap<String, Item>>,
    /// The mapping id each declared address belongs to.
    ///
    /// A refusal about a vault names the mapping, and the trait hands this
    /// transport an address rather than a mapping: the declaration knows both,
    /// so the correspondence is taken from it once here rather than threaded
    /// through every read and write.
    declared: BTreeMap<String, String>,
}

impl<'a> OnePassword<'a> {
    /// The service the declaration names, not yet asked anything.
    #[must_use]
    pub fn new(mirror: &Mirror, progress: &'a dyn Progress) -> Self {
        let mut vaults: Vec<String> = Vec::new();
        for mapping in &mirror.mappings {
            if !vaults.iter().any(|held| held == &mapping.onepassword.vault) {
                vaults.push(mapping.onepassword.vault.clone());
            }
        }
        let mut declared: BTreeMap<String, String> = BTreeMap::new();
        for mapping in &mirror.mappings {
            declared.insert(mirror.item_of(mapping), mapping.id.clone());
        }
        Self {
            program: op_binary(),
            account: mirror.account.clone(),
            vaults,
            declared,
            progress,
            items: RefCell::new(BTreeMap::new()),
        }
    }

    /// What the item at one address holds, and the agreement it remembers.
    ///
    /// The one read of this transport: the value, the four fields and the memory
    /// all arrive in the same JSON object, so there is no second invocation for
    /// fields the way [`crate::store`] has one.
    ///
    /// # Errors
    ///
    /// [`Error::OnePasswordUnavailable`] when the command cannot be run,
    /// [`Error::OnePasswordVaultAbsent`] when the session cannot see the
    /// declared vault, and [`Error::OnePasswordCommandFailed`] carrying the
    /// program's own message for anything else.
    pub fn read_address(&self, id: &str) -> Result<Option<Reading>> {
        let (vault, title) = address(id);
        let arguments = read_arguments(self.account.as_deref(), vault, title);
        let finished = self.run(&arguments, None)?;
        if !finished.status.success() {
            let complaint = trimmed(&String::from_utf8_lossy(&finished.stderr));
            return match self.classify(id, vault, &arguments, &complaint) {
                Some(refusal) => Err(refusal),
                None => Ok(None),
            };
        }

        // Zeroized when this returns, and parsed twice out of it rather than
        // cloned: a `Secret` has no clone by construction, and the second pass
        // is over a buffer that is wiped either way. One parse answers the
        // comparison and the other is what the edit leg round-trips through.
        let printed = Zeroizing::new(finished.stdout);
        let held: Item = parse(&printed, id, &arguments)?;
        let retained: Item = parse(&printed, id, &arguments)?;
        self.items.borrow_mut().insert(id.to_owned(), retained);
        Ok(Some(Reading::of(held)))
    }

    /// Write the value, the declared fields and — where the mode records one —
    /// the agreement, in one write.
    ///
    /// Against [`Existing::Present`] this is a round trip: the payload is the
    /// item's own JSON with three kinds of member replaced, so every member the
    /// declaration does not name survives, a passkey among them. Against
    /// [`Existing::Absent`] it is a created login item.
    ///
    /// The memory is a parameter rather than a member of [`Record`], because
    /// whether an agreement is recorded is the mode's answer rather than the far
    /// side's: [`Endpoint::write`] passes none, and the converging run that
    /// knows the mode passes one.
    ///
    /// # Errors
    ///
    /// [`Error::NoValueRead`] when the record carries no value,
    /// [`Error::OnePasswordUnavailable`] when the command cannot be run,
    /// [`Error::OnePasswordVaultAbsent`] when the session cannot see the vault,
    /// and [`Error::OnePasswordCommandFailed`] carrying the program's own
    /// message for anything else.
    pub fn write_address(
        &mut self,
        id: &str,
        record: &Record,
        existing: Existing,
        memory: Option<&Secret>,
    ) -> Result<()> {
        let Some(value) = record.value.as_ref() else {
            return Err(Error::NoValueRead);
        };
        let (vault, title) = address(id);
        let arguments = match existing {
            Existing::Absent => create_arguments(self.account.as_deref(), vault, title),
            Existing::Present => edit_arguments(self.account.as_deref(), vault, title),
        };

        // Only an edit round-trips, and only through an item this run read: a
        // create has nothing to preserve, and a base fetched inside the write
        // would be a second answer to a question the caller already answered.
        let base = match existing {
            Existing::Absent => None,
            Existing::Present => self.items.borrow_mut().remove(id),
        };
        let payload = Payload {
            title,
            base: base.as_ref(),
            value,
            fields: &record.fields,
            memory,
        };
        let finished = self.run(&arguments, Some(&payload))?;
        // The cached copy is now a description of an item that no longer holds
        // what it says, so it is gone rather than stale.
        self.items.borrow_mut().remove(id);
        if finished.status.success() {
            return Ok(());
        }
        let complaint = trimmed(&String::from_utf8_lossy(&finished.stderr));
        Err(self
            .classify(id, vault, &arguments, &complaint)
            .unwrap_or_else(|| Error::OnePasswordCommandFailed {
                item: id.to_owned(),
                arguments: arguments.join(" "),
                output: complaint,
            }))
    }

    /// Every item the declared vaults hold, as `<vault>/<title>`.
    ///
    /// Titles only, which is the whole of what the lingering report reads: no
    /// field of an unmapped item is ever requested, so an item no mapping
    /// declares contributes its name and nothing else.
    ///
    /// # Errors
    ///
    /// The program's own refusal to enumerate one declared vault.
    pub fn items_under(&self, vault: &str) -> Result<Vec<String>> {
        let arguments = list_arguments(self.account.as_deref(), vault);
        let finished = self.run(&arguments, None)?;
        if !finished.status.success() {
            let complaint = trimmed(&String::from_utf8_lossy(&finished.stderr));
            return Err(self
                .classify(vault, vault, &arguments, &complaint)
                .unwrap_or(Error::OnePasswordCommandFailed {
                    item: vault.to_owned(),
                    arguments: arguments.join(" "),
                    output: complaint,
                }));
        }
        let listed: Vec<Listed> = serde_json::from_slice(&finished.stdout).map_err(|cause| {
            Error::OnePasswordCommandFailed {
                item: vault.to_owned(),
                arguments: arguments.join(" "),
                output: format!("its listing did not parse as the documented shape: {cause}"),
            }
        })?;
        Ok(listed
            .into_iter()
            .map(|item| format!("{vault}/{}", item.title))
            .collect())
    }

    /// Which refusal one non-zero exit is, or none where it is an ordinary
    /// absence.
    ///
    /// The vault is judged first for the reason [`NOT_A_VAULT`] records.
    fn classify(
        &self,
        id: &str,
        vault: &str,
        arguments: &[String],
        complaint: &str,
    ) -> Option<Error> {
        let lowered = complaint.to_lowercase();
        if NOT_A_VAULT.iter().any(|wording| lowered.contains(wording)) {
            return Some(Error::OnePasswordVaultAbsent {
                mapping: self
                    .declared
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.to_owned()),
                vault: vault.to_owned(),
                output: complaint.to_owned(),
            });
        }
        if NOT_AN_ITEM.iter().any(|wording| lowered.contains(wording)) {
            return None;
        }
        Some(Error::OnePasswordCommandFailed {
            item: id.to_owned(),
            arguments: arguments.join(" "),
            output: complaint.to_owned(),
        })
    }

    /// One invocation, with all three descriptors set explicitly and the payload
    /// — when there is one — written to standard input before anything is read
    /// back.
    ///
    /// Standard input is a pipe for a write and closed for a read, rather than
    /// inherited in either case: a command that inherited this process's own
    /// standard input could read an operator's terminal, which is not a channel
    /// safix hands to a child.
    fn run(&self, arguments: &[String], payload: Option<&Payload<'_>>) -> Result<Output> {
        let encoded = payload
            .map(|payload| {
                let mut bytes = Zeroizing::new(Vec::new());
                payload
                    .write_to(&mut *bytes)
                    .map_err(|cause| Error::DocumentOperation {
                        operation: "encode 1Password value",
                        path: "1Password item".into(),
                        cause: cause.to_string(),
                    })?;
                Ok::<_, Error>(bytes)
            })
            .transpose()?;
        let mut child = Command::new(&self.program)
            .args(arguments)
            .stdin(if payload.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|cause| Error::OnePasswordUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;

        let written = if let Some(encoded) = &encoded {
            match child.stdin.take() {
                Some(mut stdin) => stdin.write_all(encoded).and_then(|()| stdin.flush()),
                None => Err(io::Error::other("1Password input pipe is unavailable")),
            }
        } else {
            Ok(())
        };

        let output = child
            .wait_with_output()
            .map_err(|cause| Error::OnePasswordUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        written.map_err(|cause| Error::SecretRead { cause })?;
        Ok(output)
    }
}

impl Endpoint for OnePassword<'_> {
    /// The session preflight, once per run and before any side is read.
    ///
    /// The position [`crate::sync::run`] gives `crate::enroll::terminal_present`
    /// and for the same reason: a run that discovered a signed-out session
    /// partway through would already have decrypted safix's side of every
    /// mapping it had reached.
    fn unlock(&mut self) -> Result<()> {
        let arguments = whoami_arguments(self.account.as_deref());
        log(self.progress, &format!("safix: op {}", arguments.join(" ")));
        let finished = self.run(&arguments, None)?;
        if finished.status.success() {
            return Ok(());
        }
        Err(Error::OnePasswordSignedOut {
            account: self.account.clone(),
            output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
        })
    }

    /// The value and the fields beside it, in one read.
    ///
    /// `declared` decides nothing here, and the asymmetry against
    /// [`crate::store`] is the transport's own: one `item get` answers the
    /// value, all four fields and the memory together, so there is no second
    /// invocation for a mapping that declares a field and none to skip for a
    /// mapping that declares none.
    fn read(&self, id: &str, _declared: &ResolvedFields) -> Result<Option<Record>> {
        Ok(self.read_address(id)?.map(|reading| reading.record))
    }

    /// The value and its declared fields, in one write, recording no agreement.
    ///
    /// The memory is the mode's business, so the trait's write never records
    /// one; [`OnePassword::write_address`] is what a converging run calls.
    fn write(&mut self, id: &str, record: &Record, existing: Existing) -> Result<()> {
        self.write_address(id, record, existing, None)
    }

    /// Every item the declared vaults hold, as `<vault>/<title>`.
    fn list(&self) -> Result<Vec<String>> {
        let mut found = Vec::new();
        for vault in &self.vaults {
            found.extend(self.items_under(vault)?);
        }
        Ok(found)
    }

    fn capabilities(&self) -> Capabilities {
        CAPABILITIES.clone()
    }
}

/// What one item was found holding: the comparison's answer, and the agreement
/// the item remembers.
///
/// Two things out of one read rather than two reads, because they are two
/// members of one JSON object.
pub struct Reading {
    /// The value and the fields, as every target reports them.
    pub record: Record,
    /// The line [`STATE_FIELD`] carries, or none where the item carries no such
    /// field or an unreadable one.
    pub memory: Option<Secret>,
}

impl Reading {
    /// The record and the memory, taken out of the parsed item.
    fn of(item: Item) -> Self {
        let mut value = None;
        let mut fields = ResolvedFields::default();
        let mut memory = None;

        for field in item.fields {
            let named_state = field.id.as_deref() == Some(STATE_FIELD)
                || field.label.as_deref() == Some(STATE_FIELD);
            let Some(held) = field.value else {
                continue;
            };
            match field.purpose.as_deref() {
                Some("PASSWORD") => value = Some(held.0),
                Some("USERNAME") => fields.username = Some(Field::Resolved(held.0)),
                Some("NOTES") => fields.notes = Some(Field::Resolved(held.0)),
                _ if named_state => memory = Some(held.0),
                _ => {}
            }
        }

        // The autofill website, which is where a declared url is written and
        // therefore where one is compared from.
        if let Some(first) = item.urls.first()
            && let Ok(href) = Secret::read_from(&mut first.href.as_bytes())
        {
            fields.url = Some(Field::Resolved(href));
        }
        for tag in &item.tags {
            if let Ok(carried) = Secret::read_from(&mut tag.as_bytes()) {
                fields.tags.push(Field::Resolved(carried));
            }
        }

        Self {
            record: Record { value, fields },
            memory,
        }
    }
}

/// The 1Password command [`OP_OVERRIDE`] names, or `op`.
#[must_use]
pub fn op_binary() -> PathBuf {
    custody::named(OP_OVERRIDE, "op")
}

/// The vault and the item title one address names.
///
/// The address is `<vault>/<title>`, built by [`Mirror::item_of`] and split back
/// apart here at the first separator. The vault is what may not carry one, and
/// that is the service's own constraint rather than safix's: an `op://` secret
/// reference is `op://<vault>/<item>/<field>`, so 1Password's own addressing
/// already reserves the separator in a vault name. An item title may carry as
/// many as it likes.
#[must_use]
fn address(id: &str) -> (&str, &str) {
    id.split_once('/').unwrap_or((id, ""))
}

/// The argument vector the session preflight is asked through.
#[must_use]
pub fn whoami_arguments(account: Option<&str>) -> Vec<String> {
    let mut arguments = account_arguments(account);
    arguments.push(String::from("whoami"));
    arguments.push(String::from("--format"));
    arguments.push(String::from("json"));
    arguments
}

/// The argument vector one item is read through.
///
/// `--reveal` because the value is concealed and a placeholder is not a value;
/// `--vault` because a service account's `item get` requires it, and because an
/// unqualified title would resolve against whatever vault the session happens to
/// reach.
#[must_use]
pub fn read_arguments(account: Option<&str>, vault: &str, item: &str) -> Vec<String> {
    let mut arguments = account_arguments(account);
    arguments.extend([
        String::from("item"),
        String::from("get"),
        item.to_owned(),
        String::from("--vault"),
        vault.to_owned(),
        String::from("--format"),
        String::from("json"),
        String::from("--reveal"),
    ]);
    arguments
}

/// The argument vector a new item is created through.
///
/// The trailing `-` is what makes the whole item arrive on standard input. No
/// `field=value` assignment appears here or anywhere else in this module.
#[must_use]
pub fn create_arguments(account: Option<&str>, vault: &str, item: &str) -> Vec<String> {
    let mut arguments = account_arguments(account);
    arguments.extend([
        String::from("item"),
        String::from("create"),
        String::from("--vault"),
        vault.to_owned(),
        String::from("--category"),
        String::from("login"),
        String::from("--title"),
        item.to_owned(),
        String::from("-"),
    ]);
    arguments
}

/// The argument vector an existing item is edited through.
#[must_use]
pub fn edit_arguments(account: Option<&str>, vault: &str, item: &str) -> Vec<String> {
    let mut arguments = account_arguments(account);
    arguments.extend([
        String::from("item"),
        String::from("edit"),
        item.to_owned(),
        String::from("--vault"),
        vault.to_owned(),
        String::from("-"),
    ]);
    arguments
}

/// The argument vector one vault's items are enumerated through.
#[must_use]
pub fn list_arguments(account: Option<&str>, vault: &str) -> Vec<String> {
    let mut arguments = account_arguments(account);
    arguments.extend([
        String::from("item"),
        String::from("list"),
        String::from("--vault"),
        vault.to_owned(),
        String::from("--format"),
        String::from("json"),
    ]);
    arguments
}

/// The account prefix every invocation carries when one is declared, and
/// nothing when none is.
///
/// A shorthand is not a credential: it names which account, where the session
/// material that proves it lives in the environment.
#[must_use]
fn account_arguments(account: Option<&str>) -> Vec<String> {
    match account {
        Some(account) => vec![String::from("--account"), account.to_owned()],
        None => Vec::new(),
    }
}

/// One item, as the documented JSON prints it, with everything this version does
/// not understand retained.
///
/// No `Debug`: [`ItemField::value`] carries values.
#[derive(Deserialize)]
struct Item {
    /// The item's own identifier, which safix neither chooses nor changes.
    #[serde(default)]
    id: Option<String>,
    /// The title, which is the half of the address the declaration names.
    #[serde(default)]
    title: Option<String>,
    /// The category, which safix sets on a created item and never changes on an
    /// existing one.
    #[serde(default)]
    category: Option<String>,
    /// The item's own tags, which is where a declared `tags` field lives.
    #[serde(default)]
    tags: Vec<String>,
    /// The autofill websites, of which the first is where a declared `url`
    /// lives.
    #[serde(default)]
    urls: Vec<Website>,
    /// Every field, built-in and custom alike.
    #[serde(default)]
    fields: Vec<ItemField>,
    /// Every member of the object this version does not name — a passkey among
    /// them.
    ///
    /// What makes the edit leg a round trip rather than a template: it is
    /// written back verbatim, so a member safix has never heard of survives a
    /// write that safix made.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

/// One autofill website of an item.
#[derive(Deserialize)]
struct Website {
    /// The label 1Password shows beside it.
    #[serde(default)]
    label: Option<String>,
    /// Whether this is the entry autofill prefers.
    #[serde(default)]
    primary: Option<bool>,
    /// The address itself.
    #[serde(default)]
    href: String,
    /// Every other member, retained for the round trip.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

/// One field of an item.
///
/// No `Debug`: [`ItemField::value`] carries a value, including a one-time
/// password of an item no declaration named.
#[derive(Deserialize)]
struct ItemField {
    /// The field's identifier — `password`, `username`, `notesPlain` for the
    /// built-in three, and whatever a custom field was created under.
    #[serde(default)]
    id: Option<String>,
    /// The field's type: `CONCEALED`, `STRING`, `OTP` and the rest.
    #[serde(rename = "type", default)]
    kind: Option<String>,
    /// What the field is for, which is how the built-in three are recognised
    /// rather than by their identifiers.
    #[serde(default)]
    purpose: Option<String>,
    /// The label 1Password shows.
    #[serde(default)]
    label: Option<String>,
    /// The field's own value.
    #[serde(default)]
    value: Option<Concealed>,
    /// Every other member, retained for the round trip.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

/// One item as a listing prints it: the title, and nothing else safix reads.
#[derive(Deserialize)]
struct Listed {
    /// The title, which is the half of the address a mapping declares.
    title: String,
}

/// A value inside an item's JSON, held as a [`Secret`] from the moment it is
/// parsed.
///
/// Deliberately not `Serialize`, which [`Secret`]'s own compile-time probes
/// forbid anyway: the payload is streamed through [`Secret::write_json_to`], so
/// a value is encoded where it is written rather than materialised as a
/// `String` some caller could keep.
struct Concealed(Secret);

impl<'de> Deserialize<'de> for Concealed {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ConcealedVisitor)
    }
}

/// The visitor that turns one JSON string into a [`Concealed`] without an owned
/// `String` in between.
struct ConcealedVisitor;

impl serde::de::Visitor<'_> for ConcealedVisitor {
    type Value = Concealed;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a field value as a JSON string")
    }

    fn visit_str<E>(self, held: &str) -> std::result::Result<Concealed, E>
    where
        E: serde::de::Error,
    {
        Secret::read_from(&mut held.as_bytes())
            .map(Concealed)
            .map_err(E::custom)
    }
}

/// Where one member of the payload's value comes from.
///
/// Two shapes because provenance is in the type on this transport too: a
/// declared literal is a public string out of a world-readable store, and
/// everything else is a [`Secret`] with no egress that yields bytes.
enum Source<'a> {
    /// A string out of the declaration, or out of the item's own public
    /// metadata.
    Text(&'a str),
    /// A value.
    Held(&'a Secret),
}

impl Source<'_> {
    /// Write this value as a JSON string.
    fn write_to<W: io::Write>(&self, sink: &mut W) -> io::Result<()> {
        match self {
            Self::Text(text) => serde_json::to_writer(sink, text).map_err(io::Error::other),
            Self::Held(secret) => secret.write_json_to(sink),
        }
    }

    /// The source one resolved field is, which is the provenance the declaration
    /// gave it.
    fn of(field: &Field) -> Source<'_> {
        match field {
            Field::Literal(text) => Source::Text(text),
            Field::Resolved(secret) => Source::Held(secret),
        }
    }
}

/// One field of the payload being written: the object the item already carries,
/// when it carries one, and the value safix puts in it, when safix names one.
struct Carried<'a> {
    /// The field as the item already holds it, whose identifier, type, purpose,
    /// label and unrecognised members are written back.
    base: Option<&'a ItemField>,
    /// The identifier, type, purpose and label to write when there is no base
    /// to take them from.
    shape: Shape<'a>,
    /// The value safix writes, or none to write the base's own.
    value: Option<Source<'a>>,
}

/// What a field safix creates says about itself.
struct Shape<'a> {
    /// The field's identifier.
    id: &'a str,
    /// The field's type.
    kind: &'a str,
    /// What the field is for, for the built-in three.
    purpose: Option<&'a str>,
}

/// The whole item, as one JSON object on standard input.
///
/// Streamed rather than serialized: a `Serialize` implementation would have to
/// hand serde a `&str` per value, and [`Secret`] has no such egress by
/// construction. So the object is emitted member by member and every value goes
/// through [`Secret::write_json_to`], which encodes into the sink and zeroes its
/// own intermediate.
struct Payload<'a> {
    /// The item's title, which a create names and an edit keeps.
    title: &'a str,
    /// The item as it is now, for the round trip, or none for a create.
    base: Option<&'a Item>,
    /// The value to write.
    value: &'a Secret,
    /// The declared fields, resolved.
    fields: &'a ResolvedFields,
    /// The agreement to record, or none where this mode records none.
    memory: Option<&'a Secret>,
}

impl Payload<'_> {
    /// Write the whole object.
    fn write_to<W: io::Write>(&self, sink: &mut W) -> io::Result<()> {
        let mut members = Members::open(sink)?;
        if let Some(base) = self.base {
            for (key, held) in &base.rest {
                members.json(key, held)?;
            }
            if let Some(id) = &base.id {
                members.json("id", id)?;
            }
        }
        members.json(
            "title",
            self.base
                .and_then(|base| base.title.as_deref())
                .unwrap_or(self.title),
        )?;
        members.json(
            "category",
            self.base
                .and_then(|base| base.category.as_deref())
                .unwrap_or(CATEGORY),
        )?;
        self.write_tags(&mut members)?;
        self.write_urls(&mut members)?;
        self.write_fields(&mut members)?;
        members.close()
    }

    /// The item's tags: the declared ones where the declaration names any, and
    /// the item's own where it names none.
    ///
    /// An undeclared field is not a claim, which is what keeps a write from
    /// deleting a tag nobody declared.
    fn write_tags<W: io::Write>(&self, members: &mut Members<'_, W>) -> io::Result<()> {
        if !self.fields.tags.is_empty() {
            members.begin("tags")?;
            members.sink.write_all(b"[")?;
            for (written, tag) in self.fields.tags.iter().enumerate() {
                if written > 0 {
                    members.sink.write_all(b",")?;
                }
                Source::of(tag).write_to(members.sink)?;
            }
            return members.sink.write_all(b"]");
        }
        match self.base {
            Some(base) if !base.tags.is_empty() => members.json("tags", &base.tags),
            _ => Ok(()),
        }
    }

    /// The item's autofill websites: the declared url in the first entry, and
    /// every other entry untouched.
    ///
    /// The `urls[]` entry rather than a `url`-typed field, because 1Password's
    /// documentation is explicit that the field type is not what autofill reads.
    fn write_urls<W: io::Write>(&self, members: &mut Members<'_, W>) -> io::Result<()> {
        let held = self.base.map_or(&[][..], |base| base.urls.as_slice());
        let Some(declared) = self.fields.url.as_ref() else {
            if held.is_empty() {
                return Ok(());
            }
            members.begin("urls")?;
            return write_array(members.sink, held, |sink, site| {
                write_site(sink, Some(site), None)
            });
        };
        members.begin("urls")?;
        let declared = Source::of(declared);
        if held.is_empty() {
            return write_array(
                members.sink,
                std::slice::from_ref(&declared),
                |sink, href| write_site(sink, None, Some(href)),
            );
        }
        members.sink.write_all(b"[")?;
        for (written, site) in held.iter().enumerate() {
            if written > 0 {
                members.sink.write_all(b",")?;
            }
            write_site(
                members.sink,
                Some(site),
                if written == 0 { Some(&declared) } else { None },
            )?;
        }
        members.sink.write_all(b"]")
    }

    /// The item's fields: the value, the two declared string fields, the
    /// agreement, and every field the declaration does not name written back
    /// exactly as it was.
    fn write_fields<W: io::Write>(&self, members: &mut Members<'_, W>) -> io::Result<()> {
        let mut carried: Vec<Carried<'_>> = Vec::new();
        let mut wrote_value = false;
        let mut wrote_username = false;
        let mut wrote_notes = false;
        let mut wrote_memory = false;

        for field in self.base.map_or(&[][..], |base| base.fields.as_slice()) {
            let names_state = field.id.as_deref() == Some(STATE_FIELD)
                || field.label.as_deref() == Some(STATE_FIELD);
            match field.purpose.as_deref() {
                Some("PASSWORD") => {
                    wrote_value = true;
                    carried.push(Carried {
                        base: Some(field),
                        shape: VALUE_SHAPE,
                        value: Some(Source::Held(self.value)),
                    });
                }
                Some("USERNAME") if self.fields.username.is_some() => {
                    wrote_username = true;
                    carried.push(Carried {
                        base: Some(field),
                        shape: USERNAME_SHAPE,
                        value: self.fields.username.as_ref().map(Source::of),
                    });
                }
                Some("NOTES") if self.fields.notes.is_some() => {
                    wrote_notes = true;
                    carried.push(Carried {
                        base: Some(field),
                        shape: NOTES_SHAPE,
                        value: self.fields.notes.as_ref().map(Source::of),
                    });
                }
                _ if names_state && self.memory.is_some() => {
                    wrote_memory = true;
                    carried.push(Carried {
                        base: Some(field),
                        shape: STATE_SHAPE,
                        value: self.memory.map(Source::Held),
                    });
                }
                // Every other field — a one-time password, a custom section's
                // own entry, a field of a kind this version has never heard of
                // — written back with its value and its unrecognised members
                // exactly as they arrived.
                _ => carried.push(Carried {
                    base: Some(field),
                    shape: VALUE_SHAPE,
                    value: None,
                }),
            }
        }

        if !wrote_value {
            carried.push(Carried {
                base: None,
                shape: VALUE_SHAPE,
                value: Some(Source::Held(self.value)),
            });
        }
        if !wrote_username && self.fields.username.is_some() {
            carried.push(Carried {
                base: None,
                shape: USERNAME_SHAPE,
                value: self.fields.username.as_ref().map(Source::of),
            });
        }
        if !wrote_notes && self.fields.notes.is_some() {
            carried.push(Carried {
                base: None,
                shape: NOTES_SHAPE,
                value: self.fields.notes.as_ref().map(Source::of),
            });
        }
        if !wrote_memory && self.memory.is_some() {
            carried.push(Carried {
                base: None,
                shape: STATE_SHAPE,
                value: self.memory.map(Source::Held),
            });
        }

        members.begin("fields")?;
        write_array(members.sink, &carried, write_field)
    }
}

/// What the value field of a created item says about itself.
const VALUE_SHAPE: Shape<'static> = Shape {
    id: "password",
    kind: "CONCEALED",
    purpose: Some("PASSWORD"),
};

/// What the username field of a created item says about itself.
const USERNAME_SHAPE: Shape<'static> = Shape {
    id: "username",
    kind: "STRING",
    purpose: Some("USERNAME"),
};

/// What the note field of a created item says about itself.
///
/// `notesPlain` with `purpose: "NOTES"` is the item's own note rather than a
/// custom field that happens to be called one.
const NOTES_SHAPE: Shape<'static> = Shape {
    id: "notesPlain",
    kind: "STRING",
    purpose: Some("NOTES"),
};

/// What the agreement field says about itself.
///
/// `CONCEALED`, because it is a derivative of a value and a person reading their
/// own item has no use for it; and a custom field with no purpose, because it is
/// not one of the item's built-in three.
const STATE_SHAPE: Shape<'static> = Shape {
    id: STATE_FIELD,
    kind: "CONCEALED",
    purpose: None,
};

/// A JSON object being written member by member.
///
/// Held rather than built because the values it carries are secrets: a
/// `serde_json::Value` tree of the payload would be the whole item in owned
/// `String`s.
struct Members<'a, W: io::Write> {
    /// Where the object is written.
    sink: &'a mut W,
    /// Whether any member has been written, which is what decides the comma.
    started: bool,
}

impl<'a, W: io::Write> Members<'a, W> {
    /// Open an object.
    fn open(sink: &'a mut W) -> io::Result<Self> {
        sink.write_all(b"{")?;
        Ok(Self {
            sink,
            started: false,
        })
    }

    /// Start one member, leaving its value to the caller.
    fn begin(&mut self, key: &str) -> io::Result<()> {
        if self.started {
            self.sink.write_all(b",")?;
        }
        self.started = true;
        serde_json::to_writer(&mut *self.sink, key).map_err(io::Error::other)?;
        self.sink.write_all(b":")
    }

    /// One member whose value carries nothing secret.
    fn json<T: serde::Serialize + ?Sized>(&mut self, key: &str, value: &T) -> io::Result<()> {
        self.begin(key)?;
        serde_json::to_writer(&mut *self.sink, value).map_err(io::Error::other)
    }

    /// One member whose value is a secret.
    fn held(&mut self, key: &str, value: &Source<'_>) -> io::Result<()> {
        self.begin(key)?;
        value.write_to(self.sink)
    }

    /// Close the object.
    fn close(self) -> io::Result<()> {
        self.sink.write_all(b"}")
    }
}

/// Write one JSON array, element by element.
fn write_array<W, T>(
    sink: &mut W,
    elements: &[T],
    mut one: impl FnMut(&mut W, &T) -> io::Result<()>,
) -> io::Result<()>
where
    W: io::Write,
{
    sink.write_all(b"[")?;
    for (at, element) in elements.iter().enumerate() {
        if at > 0 {
            sink.write_all(b",")?;
        }
        one(sink, element)?;
    }
    sink.write_all(b"]")
}

/// Write one autofill website: the entry the item already carries, with its
/// address replaced where a declaration names one.
fn write_site<W: io::Write>(
    sink: &mut W,
    base: Option<&Website>,
    href: Option<&Source<'_>>,
) -> io::Result<()> {
    let mut members = Members::open(sink)?;
    if let Some(base) = base {
        for (key, held) in &base.rest {
            members.json(key, held)?;
        }
        match &base.label {
            Some(label) => members.json("label", label)?,
            None => members.json("label", "website")?,
        }
        members.json("primary", &base.primary.unwrap_or(true))?;
    } else {
        members.json("label", "website")?;
        members.json("primary", &true)?;
    }
    match href {
        Some(href) => members.held("href", href)?,
        None => {
            if let Some(base) = base {
                members.json("href", &base.href)?;
            }
        }
    }
    members.close()
}

/// Write one field of the payload.
fn write_field<W: io::Write>(sink: &mut W, carried: &Carried<'_>) -> io::Result<()> {
    let mut members = Members::open(sink)?;
    if let Some(base) = carried.base {
        for (key, held) in &base.rest {
            members.json(key, held)?;
        }
        match &base.id {
            Some(id) => members.json("id", id)?,
            None => members.json("id", carried.shape.id)?,
        }
        match &base.kind {
            Some(kind) => members.json("type", kind)?,
            None => members.json("type", carried.shape.kind)?,
        }
        if let Some(purpose) = base.purpose.as_deref().or(carried.shape.purpose) {
            members.json("purpose", purpose)?;
        }
        if let Some(label) = &base.label {
            members.json("label", label)?;
        }
    } else {
        members.json("id", carried.shape.id)?;
        members.json("type", carried.shape.kind)?;
        if let Some(purpose) = carried.shape.purpose {
            members.json("purpose", purpose)?;
        }
        members.json("label", carried.shape.id)?;
    }
    match &carried.value {
        Some(value) => members.held("value", value)?,
        None => {
            if let Some(held) = carried.base.and_then(|base| base.value.as_ref()) {
                members.held("value", &Source::Held(&held.0))?;
            }
        }
    }
    members.close()
}

/// One item's JSON, parsed, or the program's answer refused for not being the
/// documented shape.
fn parse(printed: &[u8], id: &str, arguments: &[String]) -> Result<Item> {
    serde_json::from_slice(printed).map_err(|cause| Error::OnePasswordCommandFailed {
        item: id.to_owned(),
        arguments: arguments.join(" "),
        output: format!("its answer did not parse as the documented item shape: {cause}"),
    })
}

/// A complaint without the newline the command's own prints end with.
fn trimmed(complaint: &str) -> String {
    complaint.trim_end_matches('\n').to_owned()
}

/// What happened to one mapping.
///
/// No `Debug`, for the reason [`crate::sync::Outcome`] has none: nothing here
/// holds a value, and keeping it that way is easier than proving each future
/// variant does.
pub enum Outcome {
    /// Both sides already held the same bytes. Nothing written anywhere.
    Unchanged,
    /// The item now holds what safix holds.
    Updated,
    /// safix now holds what the item holds, through the ordinary write path.
    Pulled,
    /// The two values already agreed and the declared fields were written, in
    /// one write carrying the value the item already held.
    ///
    /// The names of the fields, never their contents: a note body and a field
    /// sourced from another entry are themselves secrets, so the report is a
    /// `&'static str` per field and no run-time string can reach it.
    FieldsUpdated(Vec<&'static str>),
    /// The declared fields differ and this mode does not write them.
    FieldsDiverged(Vec<&'static str>),
    /// The two sides disagree and the mode does not say who wins, so nothing was
    /// written.
    Conflict,
    /// This mapping was refused, and this is why.
    ///
    /// A failure against the service lands here and the run goes on: the report
    /// is complete over the declarations whatever happened.
    Refused(Error),
    /// This mapping could not be judged, and this is why.
    NotJudged(Error),
}

impl Outcome {
    /// The word a report prints for this outcome.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Updated => "updated",
            Self::Pulled => "pulled",
            Self::FieldsUpdated(_) => "fields updated",
            Self::FieldsDiverged(_) => "fields diverged",
            Self::Conflict => "conflict",
            Self::Refused(_) => "refused",
            Self::NotJudged(_) => "not judged",
        }
    }

    /// Whether this outcome makes the run a failure.
    ///
    /// The same answer [`crate::sync::Outcome::is_failure`] gives, for its
    /// reasons: a conflict is a state the operator has to resolve, an unjudged
    /// mapping is something the run does not know, and a declared field that is
    /// not there is a declaration that is not true.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Conflict | Self::Refused(_) | Self::NotJudged(_) | Self::FieldsDiverged(_)
        )
    }
}

/// One mapping's line in a run's report.
pub struct Converged {
    /// The mapping's declared name.
    pub mapping: String,
    /// How it converges.
    pub mode: OnePasswordMode,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// The 1Password endpoint, as `<vault>/<item>`.
    pub item: String,
    /// What happened.
    pub outcome: Outcome,
}

/// Everything a run judged, and what it found beside it.
pub struct Report {
    /// The account the run named, or none where the declaration names none.
    pub account: Option<String>,
    /// One entry per declared mapping, in declaration order. Every declared
    /// mapping is here, whatever happened to it.
    pub converged: Vec<Converged>,
    /// Items in a declared vault that no declared mapping names.
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
}

/// A value already in hand, for the write path that expects to read one.
///
/// The same seam [`crate::sync`] and [`crate::bridge`] each hold: a mirrored
/// value was read once, from the item, and there is nothing to compare it
/// against, so this hands it over and the rest of the write path is unchanged.
struct Held(Option<Secret>);

impl ValueSource for Held {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        self.0.take().ok_or(Error::NoValueRead)
    }
}

/// What one mapping's two sides came back as, and what is to be done about it.
///
/// No `Debug`, and every value here is zeroed when the run returns.
enum Decision {
    /// Nothing to do.
    Settled(Outcome),
    /// Write the item's side.
    Push {
        /// The value the item will hold, which is the one it already holds when
        /// only the fields moved.
        value: Secret,
        /// Whether this mode records the agreement in the same write.
        remember: bool,
        /// The declared fields, resolved, that ride along.
        resolved: ResolvedFields,
        /// The declared fields the item did not match, when the value agreed;
        /// empty when the value is what moved.
        drifted: Vec<&'static str>,
        /// Whether the item is already there, which decides create against
        /// edit.
        existing: Existing,
    },
    /// Write this value into safix, recording the agreement when the mode
    /// remembers one.
    Pull {
        /// The value the item holds.
        value: Secret,
        /// Whether the agreement is recorded afterwards.
        remember: bool,
        /// The item as it is, so the memory's write is a round trip like every
        /// other write on this transport.
        record: Record,
    },
}

/// Converge every declared mapping of this target, or the ones named.
///
/// One phase rather than two, which is where this loop differs from
/// [`crate::sync::run`]: a kdbx save rewrites and re-uploads the whole file, so
/// that target batches its writes, where an `op` write is one item over the
/// network. Batching here would buy nothing and would hold a decision from
/// before the write against a far side that may have moved since.
///
/// A failure against the service on one mapping is that mapping's own refusal
/// and the loop goes on, so the report is complete over the declarations
/// whatever happened and the run exits non-zero.
///
/// # Errors
///
/// [`Error::UnknownSyncMapping`] when a named mapping is not one that is
/// declared, [`Error::OnePasswordUnavailable`] when the command cannot be run at
/// all, [`Error::OnePasswordSignedOut`] when the session does not answer, and
/// whatever evaluating the declarations failed with. Each of those stops the
/// whole run and is raised before the first mapping is read, for the reason
/// [`crate::sync::run`] raises its own there.
pub fn run(workspace: &Workspace, progress: &dyn Progress, only: &[String]) -> Result<Report> {
    scratch::set_floor(workspace.vault_root());
    let _guard = scratch::Guard;

    let mirror = workspace.onepassword()?;
    let selected = selected(mirror, only)?;
    if selected.is_empty() {
        // A consumer declaring no mapping of this target reaches the service not
        // at all, not even for the preflight.
        return Ok(Report {
            account: mirror.account.clone(),
            converged: Vec::new(),
            lingering: Vec::new(),
        });
    }

    let mut service = OnePassword::new(mirror, progress);
    service.unlock()?;

    let mut converged: Vec<Converged> = Vec::with_capacity(selected.len());
    for mapping in &selected {
        let item = mirror.item_of(mapping);
        let outcome = match decide(workspace, &service, mapping, &item) {
            Decision::Settled(outcome) => outcome,
            Decision::Push {
                value,
                remember,
                resolved,
                drifted,
                existing,
            } => push(
                progress,
                &mut service,
                mapping,
                &item,
                PushingItem {
                    value,
                    remember,
                    resolved,
                    drifted,
                    existing,
                },
            ),
            Decision::Pull {
                value,
                remember,
                record,
            } => pull(
                workspace,
                progress,
                &mut service,
                mapping,
                &item,
                value,
                remember,
                record,
            ),
        };
        converged.push(Converged {
            mapping: mapping.id.clone(),
            mode: mapping.mode,
            safix: format!("{}.{}", mapping.safix.user, mapping.safix.name),
            item,
            outcome,
        });
    }

    Ok(Report {
        account: mirror.account.clone(),
        lingering: lingering(&service, mirror),
        converged,
    })
}

/// The mappings one run acts on, refusing before any of them is touched.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s 1password target reuses
/// this exact selection, so that scoping a comparison and scoping a write cannot
/// answer "which mappings" differently.
pub(crate) fn selected<'a>(
    mirror: &'a Mirror,
    only: &[String],
) -> Result<Vec<&'a OnePasswordMapping>> {
    if only.is_empty() {
        return Ok(mirror.mappings.iter().collect());
    }
    let mut mappings = Vec::with_capacity(only.len());
    for id in only {
        let mapping = mirror.named(id).ok_or_else(|| Error::UnknownSyncMapping {
            mapping: id.clone(),
            declared: mirror.declared(),
        })?;
        mappings.push(mapping);
    }
    Ok(mappings)
}

/// Items in a declared vault that no declared mapping accounts for.
///
/// Computed from the service's own listing rather than from the mappings, and
/// silent where the listing itself refuses: a vault that will not enumerate is
/// not a finding about any declared mapping, and every mapping's own outcome has
/// already been decided by the time this runs.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s 1password target reports
/// the identical list rather than a second computation of it.
pub(crate) fn lingering(service: &OnePassword<'_>, mirror: &Mirror) -> Vec<String> {
    let claimed: Vec<String> = mirror
        .mappings
        .iter()
        .map(|mapping| mirror.item_of(mapping))
        .collect();
    let mut found: Vec<String> = service
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter(|item| !claimed.iter().any(|held| held == item))
        .collect();
    found.sort();
    found
}

/// One item write: the value, the declared fields beside it, and whether the
/// agreement is recorded in the same write.
struct PushingItem {
    /// The value the item will hold.
    value: Secret,
    /// Whether this mode records the agreement.
    remember: bool,
    /// The declared fields, resolved.
    resolved: ResolvedFields,
    /// The declared fields the item did not match, when the value agreed.
    drifted: Vec<&'static str>,
    /// Whether the item is already there.
    existing: Existing,
}

impl PushingItem {
    /// What a completed write of this push is reported as.
    ///
    /// The whole of the precedence between a value and a field, in one place: an
    /// empty `drifted` is a value that moved, and a non-empty one is a value
    /// that agreed and fields that did not.
    fn outcome(drifted: Vec<&'static str>) -> Outcome {
        if drifted.is_empty() {
            return Outcome::Updated;
        }
        Outcome::FieldsUpdated(drifted)
    }
}

/// Read both sides of one mapping and decide what its mode says to do.
///
/// # The order, which is a precedence rather than a sequence
///
/// The value verdict is decided first and a field divergence never displaces it:
/// a mapping whose value and whose declared fields both differ is `updated` or
/// `conflict`, never `fields diverged`, because the same write that repairs the
/// value carries the fields and reporting the lesser fact would bury the greater
/// one. That is held in the code by where `drifted` is filled in: only the arms
/// whose two values already agree carry it.
fn decide(
    workspace: &Workspace,
    service: &OnePassword<'_>,
    mapping: &OnePasswordMapping,
    item: &str,
) -> Decision {
    // Before either side is read: a declaration this target cannot honour is
    // refused rather than partly written. Every channel here is standard input,
    // so what this can still raise is an `{ entry = … }` source naming an entry
    // its own person does not hold.
    let declared = match endpoint::resolve_fields(
        workspace,
        "1password",
        &mapping.safix.user,
        &mapping.onepassword.fields,
        &CAPABILITIES,
    ) {
        Ok(declared) => declared,
        Err(reason) => return Decision::Settled(Outcome::Refused(reason)),
    };

    let reading = match service.read_address(item) {
        Ok(reading) => reading,
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
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

    let existing = if reading.is_some() {
        Existing::Present
    } else {
        Existing::Absent
    };
    let (theirs, held_fields, remembered) = match reading {
        Some(reading) => (reading.record.value, reading.record.fields, reading.memory),
        None => (None, ResolvedFields::default(), None),
    };
    let drifted = declared.diff(&held_fields);

    match mapping.mode {
        OnePasswordMode::SafixToOnePassword => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                repairing_fields(ours, declared, drifted, false, existing)
            }
            (Some(ours), _) => Decision::Push {
                value: ours,
                remember: false,
                resolved: declared,
                drifted: Vec::new(),
                existing,
            },
        },

        OnePasswordMode::OnePasswordToSafix => match (ours, theirs) {
            (_, None) => Decision::Settled(Outcome::Refused(Error::OnePasswordItemAbsent {
                mapping: mapping.id.clone(),
                vault: mapping.onepassword.vault.clone(),
                item: mapping.onepassword.item.clone(),
                mode: mapping.mode.as_str(),
            })),
            // The declaration is the author of a field, and this mode writes
            // safix rather than the item, so a field difference is reported and
            // nothing is written.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            (_, Some(theirs)) => Decision::Pull {
                value: theirs,
                remember: false,
                record: Record {
                    value: None,
                    fields: declared,
                },
            },
        },

        OnePasswordMode::Backup => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            // Absence is the one case `backup` writes, so the declared fields
            // ride along with the value that creates the item.
            (Some(ours), None) => Decision::Push {
                value: ours,
                remember: false,
                resolved: declared,
                drifted: Vec::new(),
                existing,
            },
            // `backup` never overwrites an existing value, and writing its
            // fields while refusing its value would make `backup` half a mode.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                let _ = ours;
                let _ = theirs;
                diverging_fields(drifted)
            }
            // The whole of what `backup` is: an existing differing value is
            // reported and never overwritten.
            (Some(_), Some(_)) => Decision::Settled(Outcome::Conflict),
        },

        OnePasswordMode::TwoWay => two_way(
            ours,
            theirs,
            remembered.as_ref(),
            declared,
            drifted,
            existing,
        ),
    }
}

/// A two-way mapping's decision: the shared judge's verdict over the two values
/// and the agreement the item's own concealed field remembers, worded as this
/// target's own.
///
/// `agrees` is passed into the judge rather than derived inside it, because the
/// memory carries [`crate::sync::FORMAT`] and the bridge target's carries its
/// own, deliberately distinct tag.
fn two_way(
    ours: Option<Secret>,
    theirs: Option<Secret>,
    remembered: Option<&Secret>,
    declared: ResolvedFields,
    drifted: Vec<&'static str>,
    existing: Existing,
) -> Decision {
    let agreement = |remembered: &Secret, value: &Secret| agrees(remembered, value);
    let verdict = endpoint::judge(ours.as_ref(), theirs.as_ref(), remembered, &agreement);
    match verdict {
        Verdict::Unchanged => match ours {
            // A field has one author, so a two-way mapping's fields converge
            // toward the declaration — and no agreement is recorded for one,
            // because there is nothing for a memory to arbitrate.
            Some(ours) => repairing_fields(ours, declared, drifted, false, existing),
            None => Decision::Settled(Outcome::Unchanged),
        },
        Verdict::Conflict => Decision::Settled(Outcome::Conflict),
        Verdict::PushValue { remember } => match ours {
            Some(value) => Decision::Push {
                value,
                remember,
                resolved: declared,
                drifted: Vec::new(),
                existing,
            },
            // The judge asks for a push only where safix holds a value, so
            // this is the absence the judge already ruled out rather than a
            // case this target decides differently.
            None => Decision::Settled(Outcome::Conflict),
        },
        Verdict::PullValue { remember } => match theirs {
            Some(value) => Decision::Pull {
                value,
                remember,
                record: Record {
                    value: None,
                    fields: declared,
                },
            },
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
    remember: bool,
    existing: Existing,
) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Push {
        value: ours,
        remember,
        resolved,
        drifted,
        existing,
    }
}

/// The two sides' values agree and this mode does not write the item's side, so
/// a field difference is the finding.
fn diverging_fields(drifted: Vec<&'static str>) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Settled(Outcome::FieldsDiverged(drifted))
}

/// Whether the remembered agreement is the one this value would have recorded.
///
/// Compared as bytes against the line a converging write would have written,
/// through [`crate::sync::memory_of`] so that both mechanisms record the same
/// tag. That is what makes a memory a person edited, or one written under a tag
/// this version does not know, behave as no memory: it matches neither side, and
/// the judge then reports a conflict rather than guessing.
fn agrees(remembered: &Secret, value: &Secret) -> bool {
    let line = sync::memory_of(value);
    Secret::read_from(&mut line.as_bytes()).is_ok_and(|written| written.equals(remembered))
}

/// safix holds nothing for this mapping, and its mode makes safix the source.
fn empty_source(workspace: &Workspace, mapping: &OnePasswordMapping) -> Error {
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

/// Write the item's side — the value, the declared fields and, where the mode
/// records one, the agreement — in one write.
///
/// One write rather than a value write followed by a memory write, which is the
/// other difference from [`crate::sync`]: the memory is a field of this item, so
/// recording it is not a second object to write and cannot be interrupted
/// between the two.
///
/// The log line names the two endpoints and no field, because the log is about
/// which sides a value moved between; which fields were written is the report's
/// own sentence, under the mapping's line.
fn push(
    progress: &dyn Progress,
    service: &mut OnePassword<'_>,
    mapping: &OnePasswordMapping,
    item: &str,
    pushing: PushingItem,
) -> Outcome {
    log(
        progress,
        &format!(
            "safix: flake.safix.users.{}.{} -> {item}",
            mapping.safix.user, mapping.safix.name,
        ),
    );
    let memory = match remembered(&pushing.value, pushing.remember) {
        Ok(memory) => memory,
        Err(reason) => return Outcome::Refused(reason),
    };
    let record = Record {
        value: Some(pushing.value),
        fields: pushing.resolved,
    };
    if let Err(reason) = service.write_address(item, &record, pushing.existing, memory.as_ref()) {
        return Outcome::Refused(reason);
    }
    PushingItem::outcome(pushing.drifted)
}

/// Write safix's side through the ordinary write path, and the agreement when
/// the mode remembers one.
///
/// Only the value crosses toward safix. A field is a declaration and safix's
/// side has nowhere to hold one — a safix entry is a file, a key inside it and
/// an audience — so this function is unchanged by the field axis except for the
/// memory's own write, which rides on the item the value came from.
#[expect(
    clippy::too_many_arguments,
    reason = "every one is a distinct thing this write needs: the two endpoints, \
              the value, whether the agreement is recorded, and the item the \
              memory rides back on"
)]
fn pull(
    workspace: &Workspace,
    progress: &dyn Progress,
    service: &mut OnePassword<'_>,
    mapping: &OnePasswordMapping,
    item: &str,
    value: Secret,
    remember: bool,
    record: Record,
) -> Outcome {
    log(
        progress,
        &format!(
            "safix: {item} -> flake.safix.users.{}.{}",
            mapping.safix.user, mapping.safix.name,
        ),
    );

    // The memory is derived from a second copy, because the write path below
    // consumes the value it is handed.
    let memory = match remembered(&value, remember) {
        Ok(memory) => memory,
        Err(reason) => return Outcome::Refused(reason),
    };
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
        // inherited. Reporting it as this mapping's refusal is what keeps the
        // rest of the run going and the report honest about which mapping it
        // was.
        Ok(status) if status != 0 => {
            return Outcome::Refused(Error::OnePasswordCommandFailed {
                item: item.to_owned(),
                arguments: String::from("<the safix write path>"),
                output: format!("sops exited {status}; its own message is above"),
            });
        }
        Ok(_) => {}
    }

    // After the value it is about has landed, which is load-bearing and the
    // other order loses data: a memory written first and then not followed by
    // its value would say the two sides agreed on a value only one of them
    // holds, and the next run would converge the other way.
    if let Some(memory) = memory.as_ref() {
        let carried = Record {
            value: record.value.or_else(|| read_back(service, item)),
            fields: record.fields,
        };
        if carried.value.is_none() {
            return Outcome::Refused(Error::NoValueRead);
        }
        if let Err(reason) = service.write_address(item, &carried, Existing::Present, Some(memory))
        {
            return Outcome::Refused(reason);
        }
    }
    Outcome::Pulled
}

/// The item's own value, read again so the memory's write carries it.
///
/// The memory is a field of the mapped item, so recording it is an edit of that
/// item, and an edit carries the value the item is to hold. The value that
/// crossed toward safix was consumed by the write path, so this reads the side
/// the agreement is about rather than keeping a second copy of it alive for the
/// length of a commit.
fn read_back(service: &OnePassword<'_>, item: &str) -> Option<Secret> {
    service
        .read_address(item)
        .ok()
        .flatten()
        .and_then(|reading| reading.record.value)
}

/// The agreement this write records, or none where the mode records none.
fn remembered(value: &Secret, remember: bool) -> Result<Option<Secret>> {
    if !remember {
        return Ok(None);
    }
    let line = sync::memory_of(value);
    Secret::read_from(&mut line.as_bytes()).map(Some)
}

/// The commit subject a mirrored value lands under, for a caller that wants to
/// assert it without running a sync.
///
/// The shape [`crate::sync::commit_subject`] has, because a pulled value is not
/// "set by hand" either and a commit saying it was would be the one sentence in
/// the history that is wrong about where the value came from.
#[must_use]
pub fn commit_subject(mapping: &OnePasswordMapping) -> String {
    format!(
        "chore(safix): sync {} for {}",
        mapping.id, mapping.safix.user
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::FieldName;
    use crate::progress::Silent;

    /// Every vector this transport produces, over an account, a vault, an item
    /// and the five operations.
    fn every_vector() -> Vec<Vec<String>> {
        let mut produced = Vec::new();
        for account in [None, Some("fixture.example.com")] {
            produced.push(whoami_arguments(account));
            produced.push(read_arguments(account, "fixture-vault", "grafana"));
            produced.push(create_arguments(account, "fixture-vault", "grafana"));
            produced.push(edit_arguments(account, "fixture-vault", "grafana"));
            produced.push(list_arguments(account, "fixture-vault"));
        }
        produced
    }

    /// The rule the stand-in enforces, held here as well so that a vector that
    /// grew an assignment is caught without a process.
    #[test]
    fn no_argument_vector_this_transport_builds_carries_an_assignment() {
        for arguments in every_vector() {
            for word in &arguments {
                assert!(
                    !word.contains('='),
                    "a vector carried the assignment '{word}': {arguments:?}"
                );
            }
        }
    }

    /// Every item command names the vault, which is service-account semantics:
    /// a command that omitted it would resolve against whatever vault the
    /// session happened to reach.
    #[test]
    fn every_item_command_names_its_vault() {
        for arguments in every_vector() {
            if !arguments.iter().any(|word| word == "item") {
                continue;
            }
            assert!(
                arguments.iter().any(|word| word == "--vault"),
                "an item command named no vault: {arguments:?}"
            );
        }
    }

    #[test]
    fn the_capability_is_all_four_fields_on_standard_input() {
        for field in FieldName::ALL {
            assert_eq!(
                CAPABILITIES.channel(field),
                Channel::Stdin,
                "{} is not on standard input",
                field.as_str()
            );
        }
        // Stated positively, so the value-shape refusal is not inherited by
        // analogy from a transport whose channel reads one line.
        assert!(CAPABILITIES.multiline);
    }

    #[test]
    fn an_address_splits_into_the_vault_and_the_title() {
        assert_eq!(
            address("fixture-vault/grafana"),
            ("fixture-vault", "grafana")
        );
        // An item title may carry the separator; a vault name may not, because
        // 1Password's own `op://` reference spelling already reserves it.
        assert_eq!(
            address("fixture-vault/alice/grafana"),
            ("fixture-vault", "alice/grafana")
        );
    }

    /// The item shape this transport reads, as the published reference prints
    /// it, including a member this version has never heard of.
    const ITEM: &str = r#"{
      "id": "abc123",
      "title": "grafana",
      "category": "LOGIN",
      "tags": ["work", "fleet"],
      "urls": [{ "label": "website", "primary": true, "href": "https://grafana.example.invalid" }],
      "fields": [
        { "id": "password", "type": "CONCEALED", "purpose": "PASSWORD", "label": "password", "value": "hunter2" },
        { "id": "username", "type": "STRING", "purpose": "USERNAME", "label": "username", "value": "alice@example.com" },
        { "id": "notesPlain", "type": "STRING", "purpose": "NOTES", "label": "notesPlain", "value": "minted by safix" },
        { "id": "safix-sync-state", "type": "CONCEALED", "label": "safix-sync-state", "value": "safix-sync-v1 deadbeef" },
        { "id": "one-time", "type": "OTP", "label": "one-time password", "value": "otpauth://totp/x" }
      ],
      "passkey": { "credentialId": "kept" },
      "sections": [{ "id": "custom", "label": "notes from a person" }]
    }"#;

    /// One member of a written payload, by JSON pointer.
    ///
    /// A pointer rather than `back["urls"][0]["href"]`, because indexing a
    /// `serde_json::Value` panics on a path that is not there and this
    /// workspace denies the construction that can. A missing pointer is the
    /// assertion's own failure: the payload did not carry the member.
    fn at(json: &serde_json::Value, pointer: &str) -> serde_json::Value {
        json.pointer(pointer)
            .expect("the payload carries the member the assertion names")
            .clone()
    }

    /// One field of a written payload, by its identifier, because a field's
    /// position in `fields[]` is not part of what the payload promises.
    fn field_by_id(json: &serde_json::Value, id: &str) -> serde_json::Value {
        at(json, "/fields")
            .as_array()
            .expect("the payload carries fields[] as an array")
            .iter()
            .find(|field| field.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .expect("the payload carries the field the assertion names")
            .clone()
    }

    #[test]
    fn a_read_takes_the_value_the_four_fields_and_the_memory_out_of_one_object() {
        let item: Item = serde_json::from_str(ITEM).unwrap();
        let reading = Reading::of(item);

        let expect = |text: &str| Secret::read_from(&mut text.as_bytes()).unwrap();
        assert!(reading.record.value.unwrap().equals(&expect("hunter2")));
        assert!(
            reading
                .memory
                .unwrap()
                .equals(&expect("safix-sync-v1 deadbeef"))
        );

        let held = reading.record.fields;
        assert!(
            held.username
                .unwrap()
                .equals(&Field::Literal(String::from("alice@example.com")))
        );
        assert!(
            held.notes
                .unwrap()
                .equals(&Field::Literal(String::from("minted by safix")))
        );
        // The autofill website rather than a `url`-typed field, which is where
        // 1Password's documentation says a person's browser looks.
        assert!(held.url.unwrap().equals(&Field::Literal(String::from(
            "https://grafana.example.invalid"
        ))));
        assert_eq!(held.tags.len(), 2);
    }

    #[test]
    fn an_edit_writes_back_every_member_the_declaration_does_not_name() {
        let base: Item = serde_json::from_str(ITEM).unwrap();
        let value = Secret::read_from(&mut "rotated".as_bytes()).unwrap();
        let memory = Secret::read_from(&mut "safix-sync-v1 cafe".as_bytes()).unwrap();
        let fields = ResolvedFields {
            username: Some(Field::Literal(String::from("bob@example.com"))),
            ..ResolvedFields::default()
        };

        let mut written: Vec<u8> = Vec::new();
        Payload {
            title: "grafana",
            base: Some(&base),
            value: &value,
            fields: &fields,
            memory: Some(&memory),
        }
        .write_to(&mut written)
        .unwrap();

        let back: serde_json::Value = serde_json::from_slice(&written).unwrap();
        // The three members no declaration names, byte-identical.
        assert_eq!(at(&back, "/passkey/credentialId"), "kept");
        assert_eq!(at(&back, "/sections/0/label"), "notes from a person");
        assert_eq!(at(&back, "/id"), "abc123");

        let by_id = |id: &str| field_by_id(&back, id);
        assert_eq!(at(&by_id("password"), "/value"), "rotated");
        assert_eq!(at(&by_id("username"), "/value"), "bob@example.com");
        // Undeclared, so untouched rather than emptied.
        assert_eq!(at(&by_id("notesPlain"), "/value"), "minted by safix");
        assert_eq!(at(&by_id("one-time"), "/value"), "otpauth://totp/x");
        assert_eq!(at(&by_id("one-time"), "/type"), "OTP");
        assert_eq!(
            at(&by_id("safix-sync-state"), "/value"),
            "safix-sync-v1 cafe"
        );
        // Undeclared tags and url stay the item's own.
        assert_eq!(at(&back, "/tags/0"), "work");
        assert_eq!(at(&back, "/urls/0/href"), "https://grafana.example.invalid");
    }

    #[test]
    fn a_created_item_carries_the_value_the_declared_fields_and_nothing_else() {
        let value = Secret::read_from(&mut "first\nsecond\n".as_bytes()).unwrap();
        let fields = ResolvedFields {
            username: Some(Field::Literal(String::from("alice@example.com"))),
            url: Some(Field::Literal(String::from("https://x.example.invalid"))),
            notes: Some(Field::Literal(String::from("minted by safix"))),
            tags: vec![Field::Literal(String::from("safix"))],
        };
        let mut written: Vec<u8> = Vec::new();
        Payload {
            title: "grafana",
            base: None,
            value: &value,
            fields: &fields,
            memory: None,
        }
        .write_to(&mut written)
        .unwrap();

        let back: serde_json::Value = serde_json::from_slice(&written).unwrap();
        assert_eq!(at(&back, "/title"), "grafana");
        assert_eq!(at(&back, "/category"), CATEGORY);
        assert_eq!(at(&back, "/tags/0"), "safix");
        assert_eq!(at(&back, "/urls/0/href"), "https://x.example.invalid");
        assert_eq!(at(&back, "/urls/0/primary"), true);
        let by_id = |id: &str| field_by_id(&back, id);
        // A multi-line value crosses whole: the channel is one JSON string on
        // standard input, so there is nothing to refuse about its shape.
        assert_eq!(at(&by_id("password"), "/value"), "first\nsecond\n");
        assert_eq!(at(&by_id("password"), "/purpose"), "PASSWORD");
        assert_eq!(at(&by_id("username"), "/value"), "alice@example.com");
        assert_eq!(at(&by_id("notesPlain"), "/purpose"), "NOTES");
        assert!(
            at(&back, "/fields")
                .as_array()
                .unwrap()
                .iter()
                .all(|field| field.get("id").and_then(serde_json::Value::as_str)
                    != Some(STATE_FIELD))
        );
    }

    /// The vault is judged before the item, which is what keeps two faults with
    /// two remedies from collapsing into one sentence.
    #[test]
    fn a_non_zero_exit_is_classified_by_the_programs_own_words() {
        let mirror: Mirror = serde_json::from_str(r#"{"account": null, "mappings": []}"#).unwrap();
        let quiet = Silent;
        let service = OnePassword::new(&mirror, &quiet);
        let arguments = read_arguments(None, "fixture-vault", "grafana");

        assert!(matches!(
            service.classify(
                "fixture-vault/grafana",
                "fixture-vault",
                &arguments,
                "[ERROR] \"fixture-vault\" isn't a vault in this account"
            ),
            Some(Error::OnePasswordVaultAbsent { .. })
        ));
        assert!(
            service
                .classify(
                    "fixture-vault/grafana",
                    "fixture-vault",
                    &arguments,
                    "[ERROR] \"grafana\" isn't an item in any vault"
                )
                .is_none()
        );
        assert!(matches!(
            service.classify(
                "fixture-vault/grafana",
                "fixture-vault",
                &arguments,
                "[ERROR] the service is unavailable"
            ),
            Some(Error::OnePasswordCommandFailed { .. })
        ));
    }
}
