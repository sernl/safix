//! The operator's `pass` store, and the whole of how `sync` reaches it.
//!
//! One transport: the store's own command. Four invocations and no others,
//! which is the whole argument surface this module has:
//!
//! - read: `pass show <path>`, the whole record on standard output.
//! - write: `pass insert --multiline --force <path>`, the whole record body on
//!   standard input until EOF. `--multiline` is what makes the read go to EOF;
//!   `--force` is what suppresses the overwrite prompt, so a run never blocks
//!   on a question a non-interactive check cannot answer.
//! - list: the `*.gpg` filenames under the store root, with the suffix removed.
//! - preflight: the store root is a directory carrying a `.gpg-id`.
//!
//! `pass rm`, `pass init`, `pass generate`, `pass otp` and `pass git` are never
//! run: no mode deletes an entry, a store's `.gpg-id` is its own audience
//! declaration and writing one would be safix deciding who can read somebody
//! else's store, and safix mints its own values through its own generators.
//!
//! The store root reaches every child as `PASSWORD_STORE_DIR` in its
//! environment, and that is the only variable this module sets. A root is a
//! *location* rather than a value, which is why it may travel there at all: the
//! rule is that no value and no field travels an argument vector or an
//! environment variable, and where a store lives is neither.
//!
//! # The record layout, which is safix's own
//!
//! `pass` fixes no metadata schema. Its own convention is that the value is the
//! first line and further information follows it, and the machine-readable
//! spellings `login:`, `url:` and `notes:` are browserpass's rather than the
//! tool's. So unlike every other target, safix *defines* the layout here, and
//! `openspec/changes/add-pass-bridge/design.md` D2 is where the decision is
//! recorded.
//!
//! A write emits the value's bytes, then — only when at least one field is
//! declared — one blank line, then one `<key>: <value>` line per set field in
//! [`FIELD_KEYS`] order, omitting a field that is unset and the `tags` line
//! when the list is empty. `tags` is joined with `", "`.
//!
//! A read is the inverse: the field block is the maximal suffix of lines every
//! one of which matches `^(login|url|notes|tags): `, one immediately preceding
//! empty line is consumed as the separator when it is there, and the value is
//! every byte before that, verbatim. A body with no such suffix is a value and
//! nothing else, which is what keeps a record safix wrote with no field
//! indistinguishable from one a person wrote by hand.
//!
//! Three properties follow, and they are why this layout rather than another. A
//! single-line value with no declared field round-trips as exactly its own
//! bytes. `pass show -c` and browserpass keep working, because the value is
//! still the first line and the field spellings are the ones they already read.
//! A multi-line value keeps its own last line, because the blank separator is
//! what tells the value's end from the field block's start.
//!
//! The one ambiguity this leaves is stated rather than engineered away: a
//! *value* whose own trailing lines all look like `url: …` reads back as
//! fields. Refusing such a value would be a safix-invented limitation on a
//! store that carries it fine, and a private framing marker would make the
//! record unreadable by the tools above — `pass -c` would copy a brace and
//! browserpass would find no password. The blank separator is what keeps
//! safix's own writes out of that case: a value is always written newline
//! terminated and followed by an empty line, so the parse strips exactly what
//! the write added and the round trip is byte-exact for every value, a trailing
//! newline included.
//!
//! # What the tool's own behaviour requires, measured rather than assumed
//!
//! Read out of `pass` against scratch stores minted by
//! `crates/safix/tests/pass_cli.rs`, which is the one place a store is created
//! and the only place these claims are made against the tool rather than
//! against a model:
//!
//! - `pass insert --multiline --force` reads the body to EOF and stores it, and
//!   replaces an existing entry without a prompt.
//! - `pass show` prints that body back byte for byte. It adds nothing, which is
//!   why [`Secret::without_one_trailing_newline`] is *not* applied here:
//!   `crates/safix-core/src/store.rs:47-50` records that keepassxc needs it
//!   because `show -s -a Password` appends a newline of its own, and that is a
//!   property of that command rather than of secrets.
//! - `--multiline` preserves a body's final newline byte for byte: a body
//!   ending in `\n` comes back ending in `\n`, and one ending without comes back
//!   without. That was `design.md`'s open question and this is its measured
//!   answer; `safix-pass-cli` is the check that keeps it answered.
//! - `pass show` on an absent entry exits non-zero, which is why absence is
//!   answered from the name listing rather than from a status: a declined
//!   decrypt exits non-zero too, and a `backup` mapping that read the two the
//!   same way would write over an entry it merely could not read.
//! - a `.gpg` file appears under the store root at the declared path, which is
//!   the listing's own coupling.
//!
//! [`Error::ValueSpansLines`] is not raised here either, and for the same shape
//! of reason: `crates/safix-core/src/store.rs:51-54` records that
//! `add --password-prompt` reads an entry's value as one line, so that refusal
//! is a keepassxc limitation rather than a truth about values. This transport
//! declares `multiline: true` and inherits no rule that was never about it.
//!
//! # The `*.gpg` listing is a measured coupling, and the alternative is worse
//!
//! [`Store::new`] walks the store root for `*.gpg` files and strips the suffix,
//! never opening one. That couples this module to the store's on-disk layout —
//! in one direction only, names and never content — and it is stated here the
//! way `store.rs:29-51` states its own measurements rather than left implicit.
//!
//! The alternative is parsing `pass ls`, which is `tree(1)` output: box-drawing
//! glyphs and colour. `store.rs:39-43` was willing to parse `ls -R -f` for
//! keepassxc precisely because that is a flat line-per-object format. A listing
//! is used for one thing, the lingering report, and building it on a rendering
//! meant for a human is how a report becomes wrong after somebody's `tree`
//! changes.
//!
//! # What is per-target here, and why each thing is
//!
//! Inside [`Endpoint`]: `unlock`, `read`, `write`, `list`, `capabilities`.
//! Outside it and deliberately so: the trailer parse and the blank separator,
//! because the layout is this target's; the companion entry and its suffix,
//! because it is a second object in the store's own namespace where keepassxc's
//! is a kdbx entry and clan's a safix entry; the `*.gpg` listing, for the
//! reason above; and the absence of a write burst, because a `pass` write is one
//! file and the deferred-pulls discipline at `crate::sync`'s own header exists
//! for a 292 MB whole-file rewrite that has no analogue here.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use zeroize::Zeroizing;

use crate::endpoint::{
    self, Capabilities, Channel, Endpoint, Existing, Field, FieldName, Record, ResolvedFields,
    Verdict,
};
use crate::enroll::custody::named;
use crate::error::{Error, Result};
use crate::model::{Pass, PassMapping, PassMode};
use crate::progress::{Progress, log};
use crate::secret::Secret;
use crate::set::{self, ValueSource};
use crate::workspace::Workspace;
use crate::{bridge, scratch};

/// The environment variable that names the store's own command, for a consumer
/// whose `pass` is not on the path and for every check that drives the stub.
///
/// The shape `crate::enroll::custody::KEEPASSXC_OVERRIDE` has, resolved through
/// the same helper, so that "what an override spells" has one answer.
pub const PASS_OVERRIDE: &str = "SAFIX_PASS";

/// The store's own command [`PASS_OVERRIDE`] names, or `pass`.
#[must_use]
pub fn pass_cli() -> PathBuf {
    named(PASS_OVERRIDE, "pass")
}

/// The tag the recorded agreement carries.
///
/// The same `crate::sync::FORMAT` both existing store mechanisms record under:
/// a memory whose tag this version does not write is read as no memory at all,
/// which takes the mapping to bootstrap semantics rather than to a conflict on
/// every entry.
pub const FORMAT: &str = crate::sync::FORMAT;

/// The suffix safix reserves for the entry a two-way mapping records its last
/// agreement in.
///
/// The same string `modules/flake/safix/pass.nix` reserves, and the evaluation
/// refusal there is what keeps a declaration from naming one: the companion of
/// a declared path is that path plus this suffix, and a declared path carrying
/// the suffix is refused, so the two name spaces cannot overlap.
pub const STATE_SUFFIX: &str = ".safix-sync-state";

/// The entry a mapping's last agreement is recorded in, beside the entry
/// itself.
#[must_use]
pub fn companion_of(entry: &str) -> String {
    format!("{entry}{STATE_SUFFIX}")
}

/// Whether this entry path is one safix reserves rather than one a mapping may
/// name.
#[must_use]
pub fn is_companion(entry: &str) -> bool {
    entry.ends_with(STATE_SUFFIX)
}

/// The field spellings a record's trailing block carries, in the order a write
/// emits them and a read asks for them.
///
/// `login` rather than `username`: the ecosystem's own spelling is `login:`,
/// and the mapping between it and the declaration's name lives here rather than
/// in the report or in the declaration. The other three are spelled the same on
/// both sides.
pub const FIELD_KEYS: [&str; 4] = ["login", "url", "notes", "tags"];

/// The key one field is written under.
///
/// The literals rather than an index into [`FIELD_KEYS`]: one table is read
/// forwards by the parse and one is read per field by the write, and both name
/// the same four strings.
const fn key_of(field: FieldName) -> &'static str {
    match field {
        FieldName::Username => "login",
        FieldName::Url => "url",
        FieldName::Notes => "notes",
        FieldName::Tags => "tags",
    }
}

/// The field one key names, or none where the line is not a field line.
fn field_of(key: &str) -> Option<FieldName> {
    FieldName::ALL
        .into_iter()
        .find(|field| key_of(*field) == key)
}

/// What a store's own tools separate a tag list's elements with.
const TAG_SEPARATOR: &str = ", ";

/// The bytes one field's value contributes to a body.
///
/// A [`Field::Resolved`] is written through [`Secret::write_to`] rather than
/// read out as a slice, which is the type's own discipline: the plaintext does
/// not leave it.
fn append_field(body: &mut Vec<u8>, field: &Field) {
    match field {
        Field::Literal(text) => body.extend_from_slice(text.as_bytes()),
        // Into a `Vec` the caller holds inside a `Zeroizing`, which is what
        // makes this the one place a resolved field's bytes exist in the clear
        // and makes them go away when the body does.
        Field::Resolved(secret) => {
            let mut written = Vec::new();
            if secret.write_to(&mut written).is_ok() {
                body.extend_from_slice(&written);
            }
            written.fill(0);
        }
    }
}

/// The record body one write sends on standard input.
///
/// Zeroizing rather than a bare `Vec`, because this is the one buffer in which
/// a value and a resolved field exist as plaintext bytes: the body is what
/// crosses the pipe, so it has to be assembled, and it goes away when the write
/// returns.
fn body_of(record: &Record) -> Zeroizing<Vec<u8>> {
    let mut body: Vec<u8> = Vec::new();
    if let Some(value) = record.value.as_ref() {
        let mut written = Vec::new();
        if value.write_to(&mut written).is_ok() {
            body.extend_from_slice(&written);
        }
        written.fill(0);
    }
    if !record.fields.declares() {
        return Zeroizing::new(body);
    }

    // The separator: the newline that terminates the value's own last line, and
    // then the empty line. Both are written unconditionally, which is what makes
    // the parse strip exactly what this added and the round trip byte-exact for
    // a value that ends in a newline of its own.
    body.extend_from_slice(b"\n\n");

    if let Some(username) = record.fields.username.as_ref() {
        append_line(&mut body, FieldName::Username, username);
    }
    if let Some(url) = record.fields.url.as_ref() {
        append_line(&mut body, FieldName::Url, url);
    }
    if let Some(notes) = record.fields.notes.as_ref() {
        append_line(&mut body, FieldName::Notes, notes);
    }
    if !record.fields.tags.is_empty() {
        body.extend_from_slice(key_of(FieldName::Tags).as_bytes());
        body.extend_from_slice(b": ");
        for (at, tag) in record.fields.tags.iter().enumerate() {
            if at > 0 {
                body.extend_from_slice(TAG_SEPARATOR.as_bytes());
            }
            append_field(&mut body, tag);
        }
        body.push(b'\n');
    }
    Zeroizing::new(body)
}

/// One `<key>: <value>` line, terminated.
fn append_line(body: &mut Vec<u8>, field: FieldName, value: &Field) {
    body.extend_from_slice(key_of(field).as_bytes());
    body.extend_from_slice(b": ");
    append_field(body, value);
    body.push(b'\n');
}

/// Where a body's own field block begins, or its length where there is none.
///
/// The maximal suffix of complete lines each of which is a field line. Computed
/// over bytes rather than over a `String`, because a value is arbitrary bytes
/// and a lossy conversion of it would be a value this module corrupted.
fn field_block_at(body: &[u8]) -> usize {
    // Every line's own byte range, terminator excluded. Built once and scanned
    // backwards, rather than walked by arithmetic over two indices at a time:
    // the maximal suffix is the whole question, and one pass each way is how it
    // is answered without an off-by-one to reason about.
    let mut lines: Vec<(usize, usize)> = Vec::new();
    let mut at = 0;
    while at < body.len() {
        let rest = body.get(at..).unwrap_or_default();
        if let Some(offset) = rest.iter().position(|byte| *byte == b'\n') {
            let end = at.saturating_add(offset);
            lines.push((at, end));
            at = end.saturating_add(1);
        } else {
            lines.push((at, body.len()));
            at = body.len();
        }
    }

    let mut start = body.len();
    for (begins, ends) in lines.into_iter().rev() {
        let line = body.get(begins..ends).unwrap_or_default();
        if !is_field_line(line) {
            break;
        }
        start = begins;
    }
    start
}

/// Whether one line, without its terminator, is a field line.
fn is_field_line(line: &[u8]) -> bool {
    FIELD_KEYS.into_iter().any(|key| {
        let mut prefix = key.as_bytes().to_vec();
        prefix.extend_from_slice(b": ");
        line.starts_with(&prefix)
    })
}

/// What one body holds: the value, and the fields the trailing block carries.
///
/// The inverse of [`body_of`]. A value of zero bytes is `None`, because a
/// record whose body is nothing but a field block holds no value.
fn record_from(bytes: &[u8]) -> Result<Record> {
    let start = field_block_at(bytes);
    let mut fields = ResolvedFields::default();

    if start < bytes.len() {
        let block = bytes.get(start..).unwrap_or_default();
        for line in block.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let text = String::from_utf8_lossy(line).into_owned();
            let Some((key, rest)) = text.split_once(": ") else {
                continue;
            };
            let Some(field) = field_of(key) else {
                continue;
            };
            let carried = |text: &str| -> Result<Field> {
                Ok(Field::Resolved(Secret::read_from(&mut text.as_bytes())?))
            };
            match field {
                FieldName::Username => fields.username = Some(carried(rest)?),
                FieldName::Url => fields.url = Some(carried(rest)?),
                FieldName::Notes => fields.notes = Some(carried(rest)?),
                FieldName::Tags => {
                    for tag in rest.split(TAG_SEPARATOR) {
                        fields.tags.push(carried(tag)?);
                    }
                }
            }
        }
    }

    // The separator, consumed: the newline that terminated the value's own last
    // line and the empty line after it, and only where a field block exists. A
    // body that is a value and nothing else is taken verbatim, which is what
    // keeps a trailing newline the operator stored.
    let head = bytes.get(..start).unwrap_or_default();
    let value_bytes = if start == bytes.len() {
        head
    } else if let Some(rest) = head.strip_suffix(b"\n\n") {
        rest
    } else if let Some(rest) = head.strip_suffix(b"\n") {
        rest
    } else {
        head
    };

    let value = if value_bytes.is_empty() {
        None
    } else {
        let mut source = value_bytes;
        Some(Secret::read_from(&mut source)?)
    };
    Ok(Record { value, fields })
}

/// One `pass` store, for the length of one run.
///
/// No `Debug`: nothing here is a value, but a listing of an operator's entry
/// names is not something a stray formatting call should be able to print.
pub struct Store {
    program: PathBuf,
    root: PathBuf,
    names: BTreeSet<String>,
}

impl Store {
    /// The declared store, checked to be one, with its own names read.
    ///
    /// The check happens here rather than in [`Endpoint::unlock`] and before
    /// any mapping is read, which is what makes it a refusal that costs the
    /// operator nothing: a run that treated an absent store as an empty one
    /// would report every mapping as one-sided and, in `backup` mode, write.
    ///
    /// # Errors
    ///
    /// [`Error::NoPassStore`] when the expanded root is not a directory or
    /// carries no `.gpg-id` anywhere in the declared tree. `mappings` is how
    /// many are declared against it, which the refusal names so the operator
    /// knows how much of the run this stopped.
    pub fn new(store: &str, mappings: usize) -> Result<Self> {
        let root = expanded(store);
        if !root.is_dir() || !holds_recipients(&root) {
            return Err(Error::NoPassStore {
                store: root.display().to_string(),
                mappings,
            });
        }
        let mut names = BTreeSet::new();
        collect_names(&root, &root, &mut names);
        Ok(Self {
            program: pass_cli(),
            root,
            names,
        })
    }

    /// The store this run is converging against, for a report that names it.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the store holds this entry.
    ///
    /// Answered from the listing taken when the store was read rather than by
    /// asking about the entry, because `pass show` exits non-zero both for an
    /// entry that is not there and for a decrypt the operator's agent declined,
    /// and a `backup` mapping that read the two the same way would write over
    /// an entry it merely could not read.
    #[must_use]
    pub fn holds(&self, entry: &str) -> bool {
        self.names.contains(entry)
    }

    /// Every entry the store holds, in path order.
    ///
    /// What the report's information line about entries no mapping declares is
    /// computed from, companion entries included: a companion whose own entry
    /// is gone is exactly as much of a leftover as the entry would have been.
    pub fn entries(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }

    /// What this transport can carry beside a value, and whether its value
    /// channel takes more than one line.
    ///
    /// The other half of `modules/flake/safix/fields.nix`'s `channels.pass`,
    /// which is what the nix half refuses a declaration against;
    /// `modules/flake/checks/pass.nix` holds that module's own table equal to
    /// the shared one. All four are `Stdin` because the whole body crosses on
    /// one pipe, and `multiline` is true because `pass insert --multiline`
    /// reads to EOF — so [`Error::FieldUnsupported`],
    /// [`Error::FieldSourceInArgv`] and [`Error::ValueSpansLines`] are all
    /// unreachable for this target.
    #[must_use]
    pub const fn capabilities(&self) -> Capabilities {
        Capabilities {
            username: Channel::Stdin,
            url: Channel::Stdin,
            notes: Channel::Stdin,
            tags: Channel::Stdin,
            multiline: true,
        }
    }

    /// What the store holds for one entry, or nothing where it holds no such
    /// entry.
    ///
    /// # Errors
    ///
    /// [`Error::PassUnavailable`] when the command cannot be run,
    /// [`Error::PassLocked`] when the operator's own agent declined the
    /// decrypt, and [`Error::PassCommandFailed`] for every other non-zero exit.
    pub fn read_record(&self, entry: &str) -> Result<Option<Record>> {
        if !self.holds(entry) {
            return Ok(None);
        }
        let arguments = vec![String::from("show"), entry.to_owned()];
        let finished =
            self.command(&arguments)
                .output()
                .map_err(|cause| Error::PassUnavailable {
                    program: self.program.display().to_string(),
                    cause,
                })?;
        if !finished.status.success() {
            let output = trimmed(&String::from_utf8_lossy(&finished.stderr));
            if declined(&output) {
                return Err(Error::PassLocked {
                    entry: entry.to_owned(),
                    output,
                });
            }
            return Err(Error::PassCommandFailed {
                entry: entry.to_owned(),
                arguments: arguments.join(" "),
                output,
            });
        }
        let body = Zeroizing::new(finished.stdout);
        record_from(&body).map(Some)
    }

    /// Put one record in the store, value and declared fields in one body.
    ///
    /// # Errors
    ///
    /// [`Error::PassUnavailable`] when the command cannot be run,
    /// [`Error::PassCommandFailed`] when its standard input was not there to
    /// write or when it refuses.
    pub fn write_record(&mut self, entry: &str, record: &Record) -> Result<()> {
        let arguments = vec![
            String::from("insert"),
            String::from("--multiline"),
            String::from("--force"),
            entry.to_owned(),
        ];
        let body = body_of(record);
        let mut child = self
            .command(&arguments)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|cause| Error::PassUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        {
            // The analogue of `Error::StorePipeMissing`, raised as this target's
            // own refusal: a command started without the pipe its whole record
            // travels wrote nothing, and saying so names this entry.
            let Some(mut stdin) = child.stdin.take() else {
                return Err(Error::PassCommandFailed {
                    entry: entry.to_owned(),
                    arguments: arguments.join(" "),
                    output: String::from(
                        "the store's own command was started without the pipe its record travels",
                    ),
                });
            };
            stdin
                .write_all(&body)
                .and_then(|()| stdin.flush())
                .map_err(|cause| Error::SecretRead { cause })?;
        }
        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::PassUnavailable {
                program: self.program.display().to_string(),
                cause,
            })?;
        if !finished.status.success() {
            return Err(Error::PassCommandFailed {
                entry: entry.to_owned(),
                arguments: arguments.join(" "),
                output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
            });
        }
        self.names.insert(entry.to_owned());
        Ok(())
    }

    /// One invocation, with the store root in its environment and nothing else.
    ///
    /// `PASSWORD_STORE_DIR` and no other variable: a root is a location, and
    /// every value and every field crosses on standard input. Standard error is
    /// captured because a refusal's own words — gpg's included — are what the
    /// two refusals below carry.
    fn command(&self, arguments: &[String]) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(arguments)
            .env("PASSWORD_STORE_DIR", &self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

impl Endpoint for Store {
    /// Nothing, and the absence is the point.
    ///
    /// The store check happens in [`Store::new`], before any mapping is read,
    /// so an endpoint that existed before it could be read from is not a state
    /// this transport has. And there is no passphrase prompt at all: `pass`
    /// shells to gpg, and the unlock belongs to the ambient `gpg-agent`, which
    /// may answer from its own cache, from a pinentry on the operator's own
    /// terminal, or not at all. Interposing would defeat a hardware-backed key
    /// and would mean holding a passphrase safix has no use for.
    fn unlock(&mut self) -> Result<()> {
        Ok(())
    }

    /// The whole record at this address, or nothing where the store holds no
    /// entry there.
    ///
    /// `declared` is unread, and the asymmetry with keepassxc is the layout's:
    /// there, the value and the fields are two invocations and the second only
    /// happens for a mapping that declares a field. Here one `show` returns the
    /// whole body, so there is no second invocation to decide about and nothing
    /// a declaration could narrow.
    fn read(&self, id: &str, _declared: &ResolvedFields) -> Result<Option<Record>> {
        self.read_record(id)
    }

    /// The value and its declared fields, in one body.
    ///
    /// `existing` is unread: `--force` makes the create and the overwrite one
    /// argument vector, so there is no `add`/`edit` decision for this target.
    /// Whether an existing entry may be overwritten at all is the converge
    /// loop's question — that is what `backup` is — and answering it here would
    /// be a second answer free to disagree with the first.
    fn write(&mut self, id: &str, record: &Record, _existing: Existing) -> Result<()> {
        if record.value.is_none() {
            return Err(Error::NoValueRead);
        }
        self.write_record(id, record)
    }

    /// Every entry the store holds, from its own `*.gpg` names.
    fn list(&self) -> Result<Vec<String>> {
        Ok(self.entries().map(str::to_owned).collect())
    }

    fn capabilities(&self) -> Capabilities {
        Store::capabilities(self)
    }
}

/// The declared root with a leading `~/` expanded against `HOME`.
///
/// Expanded by the runtime rather than by nix, because evaluation has no home
/// to expand against — which is what `modules/flake/safix/options.nix` states
/// about this option.
fn expanded(store: &str) -> PathBuf {
    let Some(rest) = store.strip_prefix("~/") else {
        return PathBuf::from(store);
    };
    match std::env::var_os("HOME") {
        Some(home) if !home.is_empty() => PathBuf::from(home).join(rest),
        _ => PathBuf::from(store),
    }
}

/// Whether the tree carries a `.gpg-id` anywhere under it.
///
/// A store's `.gpg-id` is its own audience declaration, and a per-subtree one
/// is how `pass` lets a subtree have its own recipients — so a root whose own
/// level carries none but whose children do is still a store. safix reads
/// whether one exists and never writes one.
fn holds_recipients(root: &Path) -> bool {
    if root.join(".gpg-id").is_file() {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && holds_recipients(&path) {
            return true;
        }
    }
    false
}

/// Every `*.gpg` name under the root, with the suffix removed.
///
/// Names and never content: no file here is opened. That is the coupling the
/// module doc records, and it is one direction wide.
fn collect_names(root: &Path, here: &Path, names: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(here) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_dir() {
            // `.git` and `.gpg-id` are the store's own bookkeeping rather than
            // entries, and a dotted directory is never an entry's parent in a
            // store `pass` itself wrote.
            if name.starts_with('.') {
                continue;
            }
            collect_names(root, &path, names);
            continue;
        }
        let Some(stem) = name.strip_suffix(".gpg") else {
            continue;
        };
        let Ok(relative) = path.parent().unwrap_or(root).strip_prefix(root) else {
            continue;
        };
        let mut id = String::new();
        for segment in relative.components() {
            let Some(segment) = segment.as_os_str().to_str() else {
                continue;
            };
            id.push_str(segment);
            id.push('/');
        }
        id.push_str(stem);
        names.insert(id);
    }
}

/// Whether a non-zero `pass show` is the operator's own agent declining rather
/// than any other failure.
///
/// gpg's own words about a passphrase, a secret key or the agent. Read out of
/// the message rather than out of the status because both cases exit non-zero
/// and the status does not distinguish them; the alternative — treating every
/// non-zero exit as a declined decrypt — would report a wrong invocation as the
/// operator's key problem.
fn declined(output: &str) -> bool {
    let said = output.to_lowercase();
    said.contains("passphrase") || said.contains("secret key") || said.contains("agent")
}

fn trimmed(complaint: &str) -> String {
    complaint.trim_end_matches('\n').to_owned()
}

// ── converging one declared mapping with one store entry ──
//
// Beside the transport rather than in `crate::sync`, which is the keepassxc
// target's converge: that module's `selected`, `lingering`, `decide` and two
// phases are all typed on `Keepassxc`, `SyncMapping` and `Database`, and its
// two-phase shape exists for a 292 MB whole-file save this target does not
// have. What is shared is what `crate::endpoint` exists to share — the judge,
// the field resolution and the `Endpoint` questions — and that is reused here
// rather than copied.

/// What happened to one mapping.
///
/// No `Debug`, for the reason `crate::sync::Outcome` has none: nothing here
/// holds a value, and keeping it that way is easier than proving each future
/// variant does.
pub enum Outcome {
    /// Both sides already held the same bytes and the same declared fields.
    Unchanged,
    /// The store now holds what safix holds.
    Updated,
    /// safix now holds what the store holds, through the ordinary write path.
    Pulled,
    /// The values already agreed and the declared fields were written, in one
    /// body carrying the value the store already held.
    ///
    /// The names of the fields written, never their contents: a note may itself
    /// be sensitive, so the report is a `&'static str` per field and no
    /// run-time string can reach it.
    FieldsUpdated(Vec<&'static str>),
    /// The declared fields differ from the record's and this mode does not
    /// write them, so nothing was written.
    FieldsDiverged(Vec<&'static str>),
    /// The two sides disagree and the mode does not say who wins.
    Conflict,
    /// This mapping was refused, and this is why.
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
    /// The reason `crate::sync::Outcome::is_failure` gives, unchanged: a
    /// declared field that is not there is a declaration that is not true, and
    /// answering whether the declarations are true is the whole purpose of
    /// reporting.
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
    pub mode: PassMode,
    /// The safix endpoint, as `<user>.<name>`.
    pub safix: String,
    /// The store endpoint, as the entry path inside the store.
    pub entry: String,
    /// What happened.
    pub outcome: Outcome,
}

/// Everything a run judged, and what it found beside it.
pub struct Report {
    /// The store root the run converged against, expanded.
    pub store: String,
    /// One entry per declared mapping, in declaration order.
    pub converged: Vec<Converged>,
    /// Entries in the store that no declared mapping names, including the
    /// companions of mappings that are gone.
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
    ///
    /// [`crate::sync::Tally`] rather than a second struct with the same eight
    /// fields: the two store targets report the same set of outcomes, so one
    /// closing line's shape serves both and a divergence between them would be
    /// a difference with nothing behind it.
    #[must_use]
    pub fn tally(&self) -> crate::sync::Tally {
        let count = |wanted: &str| {
            self.converged
                .iter()
                .filter(|entry| entry.outcome.as_str() == wanted)
                .count()
        };
        crate::sync::Tally {
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
struct Held(Option<Secret>);

impl ValueSource for Held {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        self.0.take().ok_or(Error::NoValueRead)
    }
}

/// What a mapping's two sides came back as, and what is to be done about it.
enum Decision {
    /// Nothing to do.
    Settled(Outcome),
    /// Write the store's side.
    Push(Pushing),
    /// Write this value into safix, and record the agreement when the mode
    /// remembers one.
    Pull { value: Secret, remember: bool },
}

/// One store write: the value, the declared fields beside it, and whether the
/// agreement is recorded afterwards.
struct Pushing {
    /// The value the record will hold, which is the one it already holds when
    /// only the fields moved.
    value: Secret,
    /// Whether this mode records the agreement afterwards.
    remember: bool,
    /// The declared fields, resolved, that ride along in the same body.
    resolved: ResolvedFields,
    /// The declared fields the record did not match, when the value itself
    /// agreed; empty when the value is what moved.
    drifted: Vec<&'static str>,
}

/// Converge every declared mapping, or the ones named.
///
/// # Errors
///
/// [`Error::UnknownSyncMapping`] when a named mapping is not one that is
/// declared, [`Error::NoPassStore`] when the declared store is not one, and
/// whatever evaluating the declarations failed with. Each stops the whole run
/// and is raised before the first mapping is read, for the reason
/// [`crate::sync::run`] raises its own there: a run that discovered them partway
/// through would already have said "unchanged" about mappings it never looked
/// at.
pub fn run(workspace: &Workspace, progress: &dyn Progress, only: &[String]) -> Result<Report> {
    scratch::set_floor(workspace.vault_root());
    let _guard = scratch::Guard;

    let declared = workspace.pass()?;
    let selected = selected(declared, only)?;
    if selected.is_empty() {
        return Ok(Report {
            store: declared.store.clone(),
            converged: Vec::new(),
            lingering: Vec::new(),
        });
    }

    let mut store = Store::new(&declared.store, declared.mappings.len())?;
    store.unlock()?;

    let mut converged: Vec<Converged> = Vec::with_capacity(selected.len());
    for mapping in &selected {
        let outcome = match decide(workspace, &store, mapping) {
            Decision::Settled(outcome) => outcome,
            Decision::Push(pushing) => push(progress, &mut store, mapping, pushing),
            Decision::Pull { value, remember } => {
                pull(workspace, progress, &mut store, mapping, value, remember)
            }
        };
        converged.push(Converged {
            mapping: mapping.id.clone(),
            mode: mapping.mode,
            safix: format!("{}.{}", mapping.safix.user, mapping.safix.name),
            entry: declared.entry_of(mapping),
            outcome,
        });
    }

    Ok(Report {
        store: store.root().display().to_string(),
        lingering: lingering(&store, declared),
        converged,
    })
}

/// The mappings one run acts on, refusing before any of them is touched.
///
/// `pub(crate)` rather than private: [`crate::audit`]'s pass target reuses this
/// exact selection so that scoping a comparison and scoping a write cannot
/// answer "which mappings" differently.
pub(crate) fn selected<'a>(declared: &'a Pass, only: &[String]) -> Result<Vec<&'a PassMapping>> {
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

/// Entries in the store that no declared mapping accounts for.
///
/// Every mapping accounts for its own entry *and* for the companion beside it,
/// so a companion whose mapping is gone lingers exactly as its entry does —
/// which is the point of computing this from the listing rather than from the
/// mappings. Without the companion claim every two-way mapping would report its
/// own memory as an unclaimed entry forever.
pub(crate) fn lingering(store: &Store, declared: &Pass) -> Vec<String> {
    let mut claimed: Vec<String> = Vec::new();
    for mapping in &declared.mappings {
        let entry = declared.entry_of(mapping);
        claimed.push(companion_of(&entry));
        claimed.push(entry);
    }
    store
        .entries()
        .filter(|entry| !claimed.iter().any(|held| held == entry))
        .map(str::to_owned)
        .collect()
}

/// Read both sides of one mapping and decide what its mode says to do.
///
/// The value verdict is decided first and a field divergence never displaces
/// it, which is `crate::sync::decide`'s stated precedence: a mapping whose
/// value and whose declared fields both differ is `updated` or `conflict`,
/// never `fields diverged`, because the same body that repairs the value
/// carries the fields.
fn decide(workspace: &Workspace, store: &Store, mapping: &PassMapping) -> Decision {
    let entry = mapping.pass.path.clone();

    // Before either side is read: a declaration this target cannot honour is
    // refused rather than partly written. For this target that cannot happen —
    // every channel is a pipe — but an `{ entry = … }` source is resolved here,
    // and a source naming an entry the person does not hold is refused here
    // rather than with a body half written.
    let declared = match endpoint::resolve_fields(
        workspace,
        "pass",
        &mapping.safix.user,
        &mapping.pass.fields,
        &store.capabilities(),
    ) {
        Ok(declared) => declared,
        Err(reason) => return Decision::Settled(Outcome::Refused(reason)),
    };

    let held = match store.read_record(&entry) {
        Ok(held) => held,
        Err(reason) => return Decision::Settled(Outcome::NotJudged(reason)),
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
        PassMode::SafixToPass => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => {
                repairing_fields(ours, declared, drifted, false)
            }
            (Some(ours), _) => Decision::Push(Pushing {
                value: ours,
                remember: false,
                resolved: declared,
                drifted: Vec::new(),
            }),
        },

        PassMode::PassToSafix => match (ours, theirs) {
            (_, None) => Decision::Settled(Outcome::Refused(Error::PassEntryAbsent {
                mapping: mapping.id.clone(),
                entry,
                mode: mapping.mode.as_str(),
            })),
            // The declaration is the author of a field, and this mode writes
            // safix rather than the store, so a field difference is reported
            // and nothing is written.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            (_, Some(theirs)) => Decision::Pull {
                value: theirs,
                remember: false,
            },
        },

        PassMode::Backup => match (ours, theirs) {
            (None, _) => Decision::Settled(Outcome::Refused(empty_source(workspace, mapping))),
            // Absence is the one case `backup` writes, so the declared fields
            // ride along with the value that creates the entry.
            (Some(ours), None) => Decision::Push(Pushing {
                value: ours,
                remember: false,
                resolved: declared,
                drifted: Vec::new(),
            }),
            // `backup` never overwrites an existing entry, and writing its
            // fields while refusing its value would make `backup` half a mode.
            (Some(ours), Some(theirs)) if ours.equals(&theirs) => diverging_fields(drifted),
            // The whole of what `backup` is: an existing differing value is
            // reported and never overwritten.
            (Some(_), Some(_)) => Decision::Settled(Outcome::Conflict),
        },

        PassMode::TwoWay => two_way(store, &entry, ours, theirs, declared, drifted),
    }
}

/// A two-way mapping's decision: the shared judge's verdict over the two values
/// and the agreement the companion entry remembers, worded as this target's
/// own.
fn two_way(
    store: &Store,
    entry: &str,
    ours: Option<Secret>,
    theirs: Option<Secret>,
    declared: ResolvedFields,
    drifted: Vec<&'static str>,
) -> Decision {
    let remembered = recorded(store, entry);
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
            Some(ours) => repairing_fields(ours, declared, drifted, false),
            None => Decision::Settled(Outcome::Unchanged),
        },
        Verdict::Conflict => Decision::Settled(Outcome::Conflict),
        Verdict::PushValue { remember } => match ours {
            Some(value) => Decision::Push(Pushing {
                value,
                remember,
                resolved: declared,
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
    remember: bool,
) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Push(Pushing {
        value: ours,
        remember,
        resolved,
        drifted,
    })
}

/// The two sides' values agree and this mode does not write the store's side,
/// so a field difference is the finding.
fn diverging_fields(drifted: Vec<&'static str>) -> Decision {
    if drifted.is_empty() {
        return Decision::Settled(Outcome::Unchanged);
    }
    Decision::Settled(Outcome::FieldsDiverged(drifted))
}

/// The agreement the companion entry remembers, as the bytes it holds.
///
/// A companion that will not read is treated as absent rather than as a
/// refusal: the memory is safix's own bookkeeping, and a run that stopped over
/// it would refuse a mapping whose two sides it can see perfectly well.
fn recorded(store: &Store, entry: &str) -> Option<Secret> {
    store
        .read_record(&companion_of(entry))
        .ok()
        .flatten()
        .and_then(|record| record.value)
}

/// Whether the remembered agreement is the one this value would have recorded.
fn agrees(remembered: &Secret, value: &Secret) -> bool {
    let line = memory_of(value);
    Secret::read_from(&mut line.as_bytes()).is_ok_and(|written| written.equals(remembered))
}

/// The line a converging two-way write records the agreement as.
///
/// `pub(crate)` rather than public: it is a derivative of a value, and the one
/// place it may land is the encrypted store.
pub(crate) fn memory_of(value: &Secret) -> String {
    format!("{FORMAT} {}", value.fingerprint())
}

/// safix holds nothing for this mapping, and its mode makes safix the source.
fn empty_source(workspace: &Workspace, mapping: &PassMapping) -> Error {
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

/// Write the store's side — the value and the declared fields in one body — and
/// the agreement when the mode remembers one.
fn push(
    progress: &dyn Progress,
    store: &mut Store,
    mapping: &PassMapping,
    pushing: Pushing,
) -> Outcome {
    let entry = mapping.pass.path.clone();
    log(
        progress,
        &format!(
            "safix: flake.safix.users.{}.{} -> {entry}",
            mapping.safix.user, mapping.safix.name,
        ),
    );
    let Pushing {
        value,
        remember,
        resolved,
        drifted,
    } = pushing;
    let record = Record {
        value: Some(value),
        fields: resolved,
    };
    if let Err(reason) = store.write_record(&entry, &record) {
        return Outcome::Refused(reason);
    }
    if let Err(reason) = remembered_after(store, &entry, record.value.as_ref(), remember) {
        return Outcome::Refused(reason);
    }
    // The whole of the precedence between a value and a field, in one place: an
    // empty `drifted` is a value that moved, and a non-empty one is a value
    // that agreed and fields that did not.
    if drifted.is_empty() {
        return Outcome::Updated;
    }
    Outcome::FieldsUpdated(drifted)
}

/// Write safix's side through the ordinary write path, and the agreement when
/// the mode remembers one.
///
/// Only the value crosses. A field is a declaration and safix's side has
/// nowhere to hold one — a safix entry is a file, a key inside it and an
/// audience — so a pull writes the value alone.
fn pull(
    workspace: &Workspace,
    progress: &dyn Progress,
    store: &mut Store,
    mapping: &PassMapping,
    value: Secret,
    remember: bool,
) -> Outcome {
    let entry = mapping.pass.path.clone();
    log(
        progress,
        &format!(
            "safix: {entry} -> flake.safix.users.{}.{}",
            mapping.safix.user, mapping.safix.name,
        ),
    );

    // The memory is derived from a second copy, because the write path below
    // consumes the value it is handed.
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
        Ok(status) if status != 0 => {
            return Outcome::Refused(Error::PassCommandFailed {
                entry: entry.clone(),
                arguments: String::from("<the safix write path>"),
                output: format!("sops exited {status}; its own message is above"),
            });
        }
        Ok(_) => {}
    }

    if remember {
        let recorded = match Secret::read_from(&mut memory.as_bytes()) {
            Ok(recorded) => recorded,
            Err(reason) => return Outcome::Refused(reason),
        };
        let record = Record {
            value: Some(recorded),
            fields: ResolvedFields::default(),
        };
        if let Err(reason) = store.write_record(&companion_of(&entry), &record) {
            return Outcome::Refused(reason);
        }
    }
    Outcome::Pulled
}

/// Record the agreement, after the value it is about has landed.
///
/// This order is load-bearing and the other one loses data, which is the
/// reasoning `crate::sync::remembered_after` records: a memory written first
/// and then not followed by its value would say the two sides agreed on a value
/// only one of them holds, and the next run would read that as "the side
/// holding the new value never changed" and converge the other way —
/// overwriting the new value with the old one.
///
/// The companion carries no declared field, because it is safix's own
/// bookkeeping rather than an entry a mapping mirrors.
fn remembered_after(
    store: &mut Store,
    entry: &str,
    value: Option<&Secret>,
    remember: bool,
) -> Result<()> {
    if !remember {
        return Ok(());
    }
    let Some(value) = value else {
        return Ok(());
    };
    let line = memory_of(value);
    let recorded = Secret::read_from(&mut line.as_bytes())?;
    store.write_record(
        &companion_of(entry),
        &Record {
            value: Some(recorded),
            fields: ResolvedFields::default(),
        },
    )
}

/// The commit subject a mirrored value lands under.
#[must_use]
pub fn commit_subject(mapping: &PassMapping) -> String {
    format!(
        "chore(safix): sync {} for {}",
        mapping.id, mapping.safix.user
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal(text: &str) -> Field {
        Field::Literal(text.to_owned())
    }

    fn value_of(text: &str) -> Secret {
        Secret::read_from(&mut text.as_bytes()).unwrap()
    }

    fn record(value: Option<&str>, fields: ResolvedFields) -> Record {
        Record {
            value: value.map(value_of),
            fields,
        }
    }

    fn body(record: &Record) -> Vec<u8> {
        body_of(record).to_vec()
    }

    #[test]
    fn the_reserved_suffix_is_the_one_the_declaration_refuses() {
        assert_eq!(STATE_SUFFIX, ".safix-sync-state");
        assert_eq!(
            companion_of("alice/grafana"),
            "alice/grafana.safix-sync-state"
        );
        assert!(is_companion("alice/grafana.safix-sync-state"));
        assert!(!is_companion("alice/grafana"));
    }

    /// Equal on purpose: both name an entry *inside a store*, where clan's
    /// hyphenated `-safix-bridge-sync-state` names a safix entry and is
    /// therefore a different name space — `crate::bridge`'s own comment on that
    /// form is where the distinction is recorded.
    #[test]
    fn the_two_store_suffixes_agree() {
        assert_eq!(STATE_SUFFIX, crate::store::STATE_SUFFIX);
    }

    #[test]
    fn a_single_line_value_with_no_field_is_exactly_its_own_bytes() {
        let written = record(Some("hunter2"), ResolvedFields::default());
        assert_eq!(body(&written), b"hunter2");

        let read = record_from(b"hunter2").unwrap();
        assert!(read.value.unwrap().equals(&value_of("hunter2")));
        assert!(!read.fields.declares());
    }

    #[test]
    fn a_value_ending_in_a_newline_keeps_the_byte_when_no_field_is_declared() {
        let read = record_from(b"hunter2\n").unwrap();
        assert!(read.value.unwrap().equals(&value_of("hunter2\n")));
    }

    #[test]
    fn a_value_and_one_field_round_trip_both() {
        let fields = ResolvedFields {
            username: Some(literal("alice@example.invalid")),
            ..Default::default()
        };
        let written = record(Some("hunter2"), fields);
        assert_eq!(body(&written), b"hunter2\n\nlogin: alice@example.invalid\n");

        let read = record_from(&body(&written)).unwrap();
        assert!(read.value.unwrap().equals(&value_of("hunter2")));
        assert!(
            read.fields
                .username
                .unwrap()
                .equals(&literal("alice@example.invalid"))
        );
    }

    #[test]
    fn a_multi_line_value_plus_fields_keeps_its_final_line() {
        let fields = ResolvedFields {
            url: Some(literal("https://grafana.example.invalid")),
            ..Default::default()
        };
        let written = record(Some("-----BEGIN-----\nline two\nlast line"), fields);
        assert_eq!(
            body(&written),
            b"-----BEGIN-----\nline two\nlast line\n\nurl: https://grafana.example.invalid\n"
        );

        let read = record_from(&body(&written)).unwrap();
        assert!(
            read.value
                .unwrap()
                .equals(&value_of("-----BEGIN-----\nline two\nlast line"))
        );
    }

    #[test]
    fn a_multi_line_value_ending_in_a_newline_round_trips_with_fields() {
        let fields = ResolvedFields {
            notes: Some(literal("minted by safix")),
            ..Default::default()
        };
        let written = record(Some("first\nsecond\n"), fields);
        let read = record_from(&body(&written)).unwrap();
        assert!(read.value.unwrap().equals(&value_of("first\nsecond\n")));
        assert!(
            read.fields
                .notes
                .unwrap()
                .equals(&literal("minted by safix"))
        );
    }

    /// browserpass's own convention writes the field block with no blank line
    /// before it. Read as optional, written always: both layouts parse and only
    /// the unambiguous one is produced.
    #[test]
    fn a_body_written_without_the_blank_separator_parses_as_value_plus_fields() {
        let read = record_from(b"hunter2\nlogin: alice\nurl: https://x.invalid\n").unwrap();
        assert!(read.value.unwrap().equals(&value_of("hunter2")));
        assert!(read.fields.username.unwrap().equals(&literal("alice")));
        assert!(
            read.fields
                .url
                .unwrap()
                .equals(&literal("https://x.invalid"))
        );
    }

    /// The layout's one ambiguity, asserted as documented behaviour rather than
    /// reported as a bug: a *value* whose own trailing lines are spelled like
    /// fields reads back as fields. safix's own writes never produce it, because
    /// they emit the blank separator and the value is written above it.
    #[test]
    fn a_value_whose_trailing_lines_look_like_fields_is_read_as_fields() {
        let read = record_from(b"hunter2\nurl: not-a-field-but-part-of-the-value").unwrap();
        assert!(read.value.unwrap().equals(&value_of("hunter2")));
        assert!(
            read.fields
                .url
                .unwrap()
                .equals(&literal("not-a-field-but-part-of-the-value"))
        );
    }

    #[test]
    fn tags_round_trip_as_a_comma_separated_list() {
        let fields = ResolvedFields {
            tags: vec![literal("work"), literal("fleet")],
            ..Default::default()
        };
        let written = record(Some("hunter2"), fields);
        assert_eq!(body(&written), b"hunter2\n\ntags: work, fleet\n");

        let read = record_from(&body(&written)).unwrap();
        let tags = &read.fields.tags;
        assert_eq!(tags.len(), 2);
        assert!(tags.first().is_some_and(|tag| tag.equals(&literal("work"))));
        assert!(tags.get(1).is_some_and(|tag| tag.equals(&literal("fleet"))));
    }

    #[test]
    fn all_four_fields_are_written_in_the_declared_order() {
        let fields = ResolvedFields {
            username: Some(literal("alice")),
            url: Some(literal("https://x.invalid")),
            notes: Some(literal("a note")),
            tags: vec![literal("work")],
        };
        let written = record(Some("hunter2"), fields);
        assert_eq!(
            body(&written),
            b"hunter2\n\nlogin: alice\nurl: https://x.invalid\nnotes: a note\ntags: work\n"
        );
    }

    #[test]
    fn a_body_that_is_nothing_but_a_field_block_holds_no_value() {
        let read = record_from(b"\n\nlogin: alice\n").unwrap();
        assert!(read.value.is_none());
        assert!(read.fields.username.is_some());
    }

    /// Held by a check rather than by a sentence: all four fields are carried,
    /// which is what makes `Error::FieldUnsupported` and
    /// `Error::FieldSourceInArgv` unreachable for this target.
    #[test]
    fn capabilities_carry_all_four_fields_on_a_pipe() {
        let store = Store {
            program: PathBuf::from("pass"),
            root: PathBuf::from("/nowhere"),
            names: BTreeSet::new(),
        };
        let carried = Store::capabilities(&store);
        for field in FieldName::ALL {
            assert_eq!(
                carried.channel(field),
                Channel::Stdin,
                "{} is carried on a pipe",
                field.as_str()
            );
        }
        assert!(carried.multiline);
    }

    /// The environment rule, held: the store's location and nothing else, and no
    /// argument vector carrying a value.
    #[test]
    fn the_only_variable_a_child_is_given_is_the_store_root() {
        let store = Store {
            program: PathBuf::from("pass"),
            root: PathBuf::from("/tmp/fixture-store"),
            names: BTreeSet::new(),
        };
        let arguments = vec![
            String::from("insert"),
            String::from("--multiline"),
            String::from("--force"),
            String::from("alice/grafana"),
        ];
        let command = store.command(&arguments);

        let given: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect();
        assert_eq!(
            given,
            vec![(
                String::from("PASSWORD_STORE_DIR"),
                Some(String::from("/tmp/fixture-store"))
            )]
        );

        let vector: Vec<String> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(vector, arguments);
        assert!(
            !vector.iter().any(|argument| argument.contains("hunter2")),
            "no value is in an argument vector"
        );
    }

    /// No `set_var` here: this crate forbids `unsafe`, and the safe form of
    /// that call does not exist. The two claims that matter are testable
    /// without one — an absolute declaration is passed through untouched, and a
    /// tilde declaration does not reach a child still spelled with a tilde,
    /// which is what a store root reaching `PASSWORD_STORE_DIR` as `~/…` would
    /// mean.
    #[test]
    fn a_leading_tilde_is_expanded_and_an_absolute_root_is_not_touched() {
        assert_eq!(
            expanded("/absolute/store"),
            PathBuf::from("/absolute/store")
        );

        let home = std::env::var_os("HOME").unwrap_or_default();
        let resolved = expanded("~/.password-store");
        if home.is_empty() {
            assert_eq!(resolved, PathBuf::from("~/.password-store"));
        } else {
            assert_eq!(resolved, PathBuf::from(home).join(".password-store"));
            assert!(!resolved.display().to_string().starts_with('~'));
        }
    }

    #[test]
    fn an_absent_store_is_refused_rather_than_read_as_empty() {
        let refused = Store::new("/nowhere/at/all/for/this/test", 3);
        assert!(matches!(
            refused,
            Err(Error::NoPassStore { mappings: 3, .. })
        ));
    }
}
