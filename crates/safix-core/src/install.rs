//! The installer: one manifest in, one generation of a secret store out.
//!
//! This is the half of safix that runs on the host being activated rather than
//! at an operator's terminal. Nobody types `safix install`; a NixOS activation
//! script or a home-manager activation entry does, against a manifest the nix
//! half built in the store. The manifest is the whole input — which is why
//! `userMode` is a field of it rather than a flag, and why the two check modes
//! validate the artifact rather than an argument vector.
//!
//! # The schema is here, and only here
//!
//! [`Manifest`] is the single definition of the shape both scopes' modules
//! emit. It denies unknown fields and carries a [`MANIFEST_VERSION`], because
//! the decoder this replaces ignored what it did not understand: a schema
//! change then becomes a wrong installation rather than a message, and a
//! version field is what turns it back into a message.
//!
//! # The sequence
//!
//! Validate paths and ownership, prepare the runtime filesystem and identities,
//! then decrypt and render into a private generation. Prepare reversible
//! external links before committing the generation symlink. Prune old
//! generations, then propagate changed values through checked service hooks.
//! A dry run removes its private staging and changes neither live links nor
//! services. Early-user and normal outputs use separate manifests and stores.
//!
//! # Values
//!
//! Upstream age or SOPS writes plaintext into zeroizing buffers. Structured
//! key extraction decodes the byte envelope; whole-document reads preserve
//! the decrypted bytes. Templates substitute those bytes only at runtime.
//! No value reaches an argument vector or an environment variable.
//!
//! # Privilege without `unsafe`
//!
//! `mount(2)` and the filesystem-magic read are the two syscalls here the
//! standard library does not expose, and both are reached through `rustix`'s
//! safe wrappers. The workspace forbids unsafe code with no per-module
//! exception, and a new syscall is exactly the occasion on which such a rule
//! acquires its first one.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{DirBuilder, File, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _, PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rustix::fs::{Gid, Uid};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::ciphertext::{self, Format, Identities, Source};
use crate::error::{Error, Result};
use crate::progress::{Progress, log};
use crate::secret::Secret;

/// The manifest schema version this binary reads.
pub const MANIFEST_VERSION: u32 = 1;

/// The environment variable naming the `ssh-to-age` binary, overriding the
/// default.
///
/// The same shape `SAFIX_SOPS` and `SAFIX_AGE_KEYGEN` have, and here it is what
/// lets a check assert that an ssh key was converted at all: the conversion is
/// a subprocess precisely so that no age implementation is linked into this
/// crate, and a subprocess nothing can redirect is a subprocess no hermetic
/// check can observe.
pub const SSH_TO_AGE_VARIABLE: &str = "SAFIX_SSH_TO_AGE";

/// The environment variable an activation sets to say which activation it is.
///
/// Read here rather than translated by the activation script, so that the
/// module's `supportsDryActivation` claim is true of the program.
pub const ACTIVATION_VARIABLE: &str = "NIXOS_ACTION";

/// The value of [`ACTIVATION_VARIABLE`] that means "perform everything but the
/// change".
pub const DRY_ACTIVATION: &str = "dry-activate";

/// The environment variable that selects `systemctl` over the activation
/// lists.
///
/// The unit sets it and the activation script does not, which is the whole
/// mechanism by which the two registration paths propagate a restart
/// differently.
pub const RESTART_VIA_SYSTEMCTL: &str = "SOPS_RESTART_UNITS_VIA_SYSTEMCTL";

/// Where an activation script expects to find the units to restart.
const RESTART_LIST: &str = "/run/nixos/activation-restart-list";

/// Where an activation script expects to find the units to reload.
const RELOAD_LIST: &str = "/run/nixos/activation-reload-list";

/// `statfs`'s answer for tmpfs, from `linux/magic.h`.
const TMPFS_MAGIC: i128 = 0x0102_1994;

/// `statfs`'s answer for ramfs, from `linux/magic.h`.
const RAMFS_MAGIC: i128 = 0x8584_58f6;

/// The mode every directory of the store is made at.
///
/// `0751`: traversable by anyone who already knows the name of the file they
/// are allowed to read, and listable by nobody but root.
const DIRECTORY_MODE: u32 = 0o751;

/// The mode the assembled identity file is made at.
const IDENTITY_MODE: u32 = 0o600;

/// The group the store's directories belong to where the host has one.
const KEYS_GROUP: &str = "keys";

/// The group a host without a `keys` group falls back to.
const FALLBACK_GROUP: &str = "nogroup";

/// One entry of the resolved set, as the manifest carries it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestSecret {
    /// The entry's name, which is also its file name inside the store.
    pub name: String,
    /// The `/`-nested key holding the value inside the document.
    pub key: String,
    /// Where the entry is readable from, which is inside the store unless the
    /// declaration moved it.
    pub path: String,
    /// The user the file belongs to, or `null` for [`ManifestSecret::uid`].
    pub owner: Option<String>,
    /// The group the file belongs to, or `null` for [`ManifestSecret::gid`].
    pub group: Option<String>,
    /// The numeric owner, used when no name is given.
    pub uid: u32,
    /// The numeric group, used when no name is given.
    pub gid: u32,
    /// The encrypted document the value is read from.
    pub sops_file: PathBuf,
    /// The document's format.
    pub format: Format,
    /// The file's mode, as an octal string.
    pub mode: String,
    /// Units to restart when this entry is new or has changed.
    pub restart_units: Vec<String>,
    /// Units to reload when this entry is new or has changed.
    pub reload_units: Vec<String>,
    /// Install in the separate root-only store before users are created.
    #[serde(default)]
    pub needed_for_users: bool,
}

/// Public template text and the deployment contract of its rendered output.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestTemplate {
    /// The output's name inside the generation.
    pub name: String,
    /// Public text containing `<safix:NAME>` placeholders.
    pub content: String,
    /// Installed path of the rendered file.
    pub path: String,
    /// Owner name, or null for the numeric owner.
    pub owner: Option<String>,
    /// Group name, or null for the numeric group.
    pub group: Option<String>,
    /// Numeric owner.
    pub uid: u32,
    /// Numeric group.
    pub gid: u32,
    /// Permission bits in octal.
    pub mode: String,
    /// Units restarted after a changed output is published.
    pub restart_units: Vec<String>,
    /// Units reloaded after a changed output is published.
    pub reload_units: Vec<String>,
}

impl ManifestTemplate {
    fn into_parts(self) -> (ManifestSecret, String) {
        let output = ManifestSecret {
            name: self.name,
            path: self.path,
            owner: self.owner,
            group: self.group,
            uid: self.uid,
            gid: self.gid,
            mode: self.mode,
            restart_units: self.restart_units,
            reload_units: self.reload_units,
            // Rendering supplies the value; this deployment record is never decrypted.
            key: String::new(),
            sops_file: PathBuf::new(),
            format: Format::Binary,
            needed_for_users: false,
        };
        (output, self.content)
    }
}

/// What the installer says while it works.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestLogging {
    /// Report each identity imported.
    pub key_import: bool,
    /// Report each entry that is new or has changed.
    pub secret_changes: bool,
}

/// The whole input of one installation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    /// The schema version, which this binary refuses when it is not
    /// [`MANIFEST_VERSION`].
    pub version: u32,
    /// The resolved set.
    pub secrets: Vec<ManifestSecret>,
    /// Public templates expanded only in private runtime memory.
    #[serde(default)]
    pub templates: Vec<ManifestTemplate>,
    /// Explicit `GnuPG` identity directory, outside the Nix store.
    #[serde(default)]
    pub gnupg_home: Option<PathBuf>,
    /// The directory the generations are made under.
    pub secrets_mount_point: String,
    /// The symlink that names the current generation.
    pub symlink_path: String,
    /// How many generations survive a run.
    pub keep_generations: usize,
    /// The age identity file, appended to the assembled identity.
    pub age_key_file: Option<String>,
    /// The ssh private keys converted into the assembled identity.
    pub age_ssh_key_paths: Vec<String>,
    /// Mount a tmpfs rather than a ramfs.
    pub use_tmpfs: bool,
    /// A user-scope install: no mount or chown, user-manager hooks, and
    /// `%r` expanded against the platform's runtime directory.
    pub user_mode: bool,
    /// What to report.
    pub logging: ManifestLogging,
    /// A hash over the documents this manifest names, or `null` when the
    /// consumer turned validation off.
    ///
    /// Read by nobody here, and deliberately so: it exists to make the
    /// manifest derivation a function of the ciphertext, so that editing a
    /// document rebuilds the manifest. Deserializing it is what keeps
    /// `deny_unknown_fields` from refusing the field that carries that
    /// property.
    pub manifest_input_hash: Option<String>,
}

impl Manifest {
    /// Refuse a manifest whose schema version this binary does not know.
    ///
    /// # Errors
    ///
    /// [`Error::ManifestVersionUnknown`], naming both numbers.
    pub const fn validate_version(&self) -> Result<()> {
        if self.version == MANIFEST_VERSION {
            Ok(())
        } else {
            Err(Error::ManifestVersionUnknown {
                found: self.version,
                supported: MANIFEST_VERSION,
            })
        }
    }
}

/// How much of the manifest is checked before anything is done with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckMode {
    /// Install: validate as far as each step needs and perform them.
    #[default]
    Off,
    /// Validate the schema, the version, every mode and every owner and group,
    /// and stop. Opens no document at all.
    Manifest,
    /// Everything [`CheckMode::Manifest`] does, and additionally open each
    /// distinct document and verify that every declared key is in it.
    ///
    /// Still decrypts nothing: the check reads the document's cleartext
    /// structure, since sops enciphers leaf values and leaves mapping keys in
    /// the clear. That is what makes this the mode the manifest derivation's
    /// own check phase runs in, inside a build sandbox holding no identity.
    Document,
}

impl CheckMode {
    /// The mode this word selects, if it selects one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "off" => Some(Self::Off),
            "manifest" => Some(Self::Manifest),
            "document" => Some(Self::Document),
            _ => None,
        }
    }
}

/// How one installation is run.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// How much to check before doing anything.
    pub check_mode: CheckMode,
    /// Skip every user, group and `keys`-group lookup and force ownership to
    /// `0`.
    pub ignore_passwd: bool,
    /// Perform every step except the atomic symlink swap.
    pub dry_run: bool,
}

/// Whether the environment says this is a dry activation.
///
/// The program reads it rather than the activation script translating it,
/// which is what makes the module's `supportsDryActivation` a claim about the
/// program.
#[must_use]
pub fn dry_activation_requested() -> bool {
    std::env::var_os(ACTIVATION_VARIABLE).is_some_and(|action| action == DRY_ACTIVATION)
}

/// Read and parse one manifest.
///
/// # Errors
///
/// [`Error::ManifestUnreadable`] when the file cannot be read and
/// [`Error::ManifestUnparsable`] when it is not this schema.
pub fn load(path: &Path) -> Result<Manifest> {
    let text = std::fs::read_to_string(path).map_err(|cause| Error::ManifestUnreadable {
        path: path.display().to_string(),
        cause,
    })?;
    serde_json::from_str(&text).map_err(|cause| Error::ManifestUnparsable {
        path: path.display().to_string(),
        cause: cause.to_string(),
    })
}

/// What validation resolved for one entry, once, so that no later step repeats
/// a lookup or a parse that could have failed.
#[derive(Debug, Clone, Copy)]
struct Resolved {
    /// The file's mode.
    mode: u32,
    /// The owning user.
    uid: u32,
    /// The owning group.
    gid: u32,
}

/// Install one manifest.
///
/// # Errors
///
/// Every refusal this module raises: a manifest that does not read or parse, a
/// version it does not know, a mode, owner, group or key it cannot resolve, an
/// identity file it cannot read, a document that does not decrypt, a mount it
/// cannot make, and any filesystem step that fails.
pub fn run(manifest_path: &Path, options: &Options, progress: &dyn Progress) -> Result<()> {
    let mut manifest = load(manifest_path)?;
    manifest.validate_version()?;
    if manifest.user_mode && options.check_mode == CheckMode::Off {
        expand_user_paths(&mut manifest, &runtime_directory()?);
    }
    let templates: Vec<_> = std::mem::take(&mut manifest.templates)
        .into_iter()
        .map(ManifestTemplate::into_parts)
        .collect();
    let entries = || {
        manifest
            .secrets
            .iter()
            .chain(templates.iter().map(|(entry, _)| entry))
    };
    validate_layout(&manifest, entries())?;
    validate_template_references(&manifest, &templates)?;
    let mut resolved = validate(&manifest, options.ignore_passwd)?;
    resolved.extend(validate_entries(
        templates.iter().map(|(entry, _)| entry),
        options.ignore_passwd,
    )?);
    if options.check_mode == CheckMode::Document {
        check_documents(&manifest)?;
    }
    if options.check_mode != CheckMode::Off {
        return Ok(());
    }

    let keys_gid = keys_group(options.ignore_passwd);
    let mount_point = PathBuf::from(&manifest.secrets_mount_point);
    let symlink_path = PathBuf::from(&manifest.symlink_path);
    mount_store(&manifest, &mount_point, keys_gid)?;
    let identity = assemble_identity(&manifest, &mount_point, progress)?;
    let documents = decrypt_documents(&manifest, Some(&identity))?;
    let mut values = BTreeMap::new();
    for entry in &manifest.secrets {
        let document = documents
            .get(&(entry.sops_file.clone(), entry.format, entry.key.is_empty()))
            .ok_or_else(|| missing_key(entry))?;
        values.insert(entry.name.as_str(), extract_key(document, entry)?);
    }
    drop(documents);
    for (entry, content) in &templates {
        let value = render_template(content, &values)?;
        values.insert(entry.name.as_str(), value);
    }

    let generation = next_generation(&symlink_path)?;
    let directory = mount_point.join(generation.to_string());
    if directory.exists() {
        std::fs::remove_dir_all(&directory).map_err(|cause| unwritable(&directory, cause))?;
    }
    let mut pending = PendingGeneration {
        path: &directory,
        published: false,
    };
    make_directory(&directory, 0o700)?;
    if !manifest.user_mode {
        own(&directory, 0, keys_gid)?;
    }
    let mut changed = Vec::new();
    for (entry, resolved) in entries().zip(&resolved) {
        let value = values
            .get(entry.name.as_str())
            .ok_or_else(|| missing_key(entry))?;
        let destination = directory.join(&entry.name);
        if let Some(parent) = destination.parent().filter(|parent| *parent != directory) {
            make_directory(parent, DIRECTORY_MODE)?;
            if !manifest.user_mode {
                own(parent, 0, keys_gid)?;
            }
        }
        write_entry(&destination, value, resolved.mode)?;
        if !manifest.user_mode {
            own(&destination, resolved.uid, resolved.gid)?;
        }
        if differs(&symlink_path.join(&entry.name), value) {
            changed.push(entry);
            if manifest.logging.secret_changes {
                log(progress, &format!("safix: {} changed", entry.name));
            }
        }
    }
    if options.dry_run {
        return Ok(());
    }
    let (restart, reload) = units_of(&changed, symlink_path.exists());
    publish_generation(&directory, &symlink_path, entries())?;
    pending.published = true;
    prune(
        &mount_point,
        manifest.keep_generations,
        &symlink_path,
        generation,
    )?;
    propagate(&restart, &reload, manifest.user_mode)
}

fn publish_generation<'a>(
    directory: &Path,
    symlink_path: &Path,
    entries: impl Iterator<Item = &'a ManifestSecret>,
) -> Result<()> {
    let mut links = Vec::new();
    let mut root_ordinal = 0;
    for (ordinal, entry) in entries.enumerate() {
        root_ordinal = ordinal
            .checked_add(1)
            .ok_or_else(|| layout_error("too many installation outputs"))?;
        let declared = Path::new(&entry.path);
        let inside = symlink_path.join(&entry.name);
        if declared != inside
            && let Some(link) = PreparedLink::prepare(&inside, declared, ordinal)?
        {
            links.push(link);
        }
    }
    let mut current = PreparedLink::prepare(directory, symlink_path, root_ordinal)?;
    for link in &mut links {
        link.publish()?;
    }
    std::fs::set_permissions(directory, Permissions::from_mode(DIRECTORY_MODE))
        .map_err(|cause| unwritable(directory, cause))?;
    if let Some(current) = &mut current {
        current.publish()?;
        current.committed = true;
    }
    for link in &mut links {
        link.committed = true;
    }
    Ok(())
}

fn layout_error(reason: &str) -> Error {
    Error::DocumentOperation {
        operation: "validate installation",
        path: "<manifest>".into(),
        cause: reason.into(),
    }
}

fn validate_entry_layout(
    entry: &ManifestSecret,
    mount: &Path,
    root: &Path,
    user_mode: bool,
) -> Result<()> {
    let name = Path::new(&entry.name);
    if entry.name.is_empty()
        || name
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(layout_error(
            "output names must be relative paths without traversal",
        ));
    }
    let path = Path::new(&entry.path);
    if path.starts_with(mount)
        || mount.starts_with(path)
        || root.starts_with(path)
        || (path.starts_with(root) && path != root.join(&entry.name))
    {
        return Err(layout_error(
            "output path collides with another location inside the secret stores",
        ));
    }
    if !(path.is_absolute() || user_mode && entry.path.starts_with("%r/"))
        || path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(layout_error(
            "installation paths must be absolute and without traversal",
        ));
    }
    if user_mode
        && (entry.owner.is_some()
            || entry.group.is_some()
            || entry.uid != 0
            || entry.gid != 0
            || entry.needed_for_users)
    {
        return Err(layout_error(
            "user installations cannot override ownership or request early-user secrets",
        ));
    }
    if entry.needed_for_users
        && (entry.uid != 0
            || entry.gid != 0
            || entry.owner.as_deref().is_some_and(|name| name != "root")
            || entry.group.as_deref().is_some_and(|name| name != "root")
            || u32::from_str_radix(&entry.mode, 8).is_ok_and(|mode| mode & 0o7077 != 0))
    {
        return Err(layout_error(
            "early-user secrets must be root-owned with owner-only permission bits",
        ));
    }
    if !entry.sops_file.as_os_str().is_empty()
        && matches!(entry.format, Format::Age | Format::Binary)
        && !entry.key.is_empty()
    {
        return Err(layout_error("age and binary sources require an empty key"));
    }
    if entry
        .restart_units
        .iter()
        .chain(&entry.reload_units)
        .any(|unit| {
            unit.is_empty() || unit.starts_with('-') || unit.chars().any(char::is_whitespace)
        })
    {
        return Err(layout_error(
            "service unit names must be nonempty arguments without whitespace",
        ));
    }
    Ok(())
}

fn validate_layout<'a>(
    manifest: &Manifest,
    entries: impl Iterator<Item = &'a ManifestSecret>,
) -> Result<()> {
    let mount = Path::new(&manifest.secrets_mount_point);
    let root = Path::new(&manifest.symlink_path);
    for path in [mount, root] {
        let runtime_relative =
            manifest.user_mode && path.to_str().is_some_and(|path| path.starts_with("%r/"));
        if (!path.is_absolute() && !runtime_relative)
            || path.file_name().is_none()
            || path.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            return Err(layout_error(
                "secret store roots must be absolute, normalized directories",
            ));
        }
    }
    if mount.starts_with(root) || root.starts_with(mount) {
        return Err(layout_error(
            "the mount point and current-generation link must be disjoint",
        ));
    }
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut formats = BTreeMap::new();
    let mut early = None;
    for entry in entries {
        validate_entry_layout(entry, mount, root, manifest.user_mode)?;
        if !names.insert(Path::new(&entry.name)) || !paths.insert(Path::new(&entry.path)) {
            return Err(layout_error("duplicate output name or installation path"));
        }
        if early.is_some_and(|previous| previous != entry.needed_for_users) {
            return Err(layout_error(
                "early and normal outputs require separate installation manifests",
            ));
        }
        early = Some(entry.needed_for_users);
        if !entry.sops_file.as_os_str().is_empty()
            && formats
                .insert(&entry.sops_file, entry.format)
                .is_some_and(|format| format != entry.format)
        {
            return Err(layout_error(
                "one source file cannot have conflicting formats",
            ));
        }
    }
    for name in &names {
        if name
            .ancestors()
            .skip(1)
            .any(|parent| names.contains(parent))
        {
            return Err(layout_error(
                "an output name cannot be a parent of another output",
            ));
        }
    }
    for path in &paths {
        if path
            .ancestors()
            .skip(1)
            .any(|parent| paths.contains(parent))
        {
            return Err(layout_error(
                "an installed file cannot be a parent of another output",
            ));
        }
    }
    Ok(())
}

fn visit_template<'a>(
    content: &'a str,
    mut visit: impl FnMut(bool, &'a str) -> Result<()>,
) -> Result<()> {
    let mut remaining = content;
    while let Some((literal, reference)) = remaining.split_once("<safix:") {
        visit(false, literal)?;
        let (name, rest) = reference
            .split_once('>')
            .ok_or_else(|| layout_error("unterminated template placeholder"))?;
        visit(true, name)?;
        remaining = rest;
    }
    visit(false, remaining)
}

fn validate_template_references(
    manifest: &Manifest,
    templates: &[(ManifestSecret, String)],
) -> Result<()> {
    for (_, content) in templates {
        visit_template(content, |reference, text| {
            if reference
                && !manifest
                    .secrets
                    .iter()
                    .any(|entry| entry.name == text && !entry.needed_for_users)
            {
                return Err(layout_error(
                    "template references an unknown or early-user secret",
                ));
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn render_template(content: &str, values: &BTreeMap<&str, Secret>) -> Result<Secret> {
    let mut length = 0usize;
    visit_template(content, |reference, text| {
        let bytes = if reference {
            values
                .get(text)
                .ok_or_else(|| layout_error("unknown template reference"))?
                .len()
        } else {
            text.len()
        };
        length = length
            .checked_add(bytes)
            .ok_or_else(|| layout_error("rendered template is too large"))?;
        Ok(())
    })?;
    let mut rendered = Zeroizing::new(Vec::with_capacity(length));
    visit_template(content, |reference, text| {
        if reference {
            values
                .get(text)
                .ok_or_else(|| layout_error("unknown template reference"))?
                .write_to(&mut *rendered)
                .map_err(|cause| Error::SecretRead { cause })?;
        } else {
            rendered.extend_from_slice(text.as_bytes());
        }
        Ok(())
    })?;
    Secret::read_from(&mut rendered.as_slice())
}

/// Validate every entry's mode, owner and group, in the manifest's own order.
///
/// Performed before anything is mounted or written, because a manifest naming
/// a user that does not exist is a configuration error and a store half
/// written under one is an incident.
///
/// # Errors
///
/// [`Error::ManifestModeUnparsable`], [`Error::ManifestOwnerUnknown`] and
/// [`Error::ManifestGroupUnknown`], each naming the entry.
fn validate(manifest: &Manifest, ignore_passwd: bool) -> Result<Vec<Resolved>> {
    validate_entries(manifest.secrets.iter(), ignore_passwd)
}

fn validate_entries<'a>(
    entries: impl Iterator<Item = &'a ManifestSecret>,
    ignore_passwd: bool,
) -> Result<Vec<Resolved>> {
    entries
        .map(|entry| {
            let mode =
                u32::from_str_radix(&entry.mode, 8).map_err(|_| Error::ManifestModeUnparsable {
                    name: entry.name.clone(),
                    mode: entry.mode.clone(),
                })?;
            if mode > 0o7777 {
                return Err(Error::ManifestModeUnparsable {
                    name: entry.name.clone(),
                    mode: entry.mode.clone(),
                });
            }
            if ignore_passwd {
                return Ok(Resolved {
                    mode,
                    uid: 0,
                    gid: 0,
                });
            }
            let uid = match &entry.owner {
                None => entry.uid,
                Some(owner) => identifier_of("/etc/passwd", owner).ok_or_else(|| {
                    Error::ManifestOwnerUnknown {
                        name: entry.name.clone(),
                        owner: owner.clone(),
                    }
                })?,
            };
            let gid = match &entry.group {
                None => entry.gid,
                Some(group) => identifier_of("/etc/group", group).ok_or_else(|| {
                    Error::ManifestGroupUnknown {
                        name: entry.name.clone(),
                        group: group.clone(),
                    }
                })?,
            };
            Ok(Resolved { mode, uid, gid })
        })
        .collect()
}

/// The numeric identifier a name has in one of the two colon-separated
/// databases, or the number the name already is.
///
/// The files are read directly rather than through the name-service switch,
/// which is a deliberate limit and not an oversight: this runs during
/// activation on a host whose users the same configuration declares, where the
/// switch's other sources — a directory service reached over a network that is
/// not up yet — are exactly the ones that cannot be consulted. The numeric
/// fallback is what a manifest naming an identifier rather than a name gets,
/// and it is checked first so that a host with no databases at all still
/// installs entries whose ownership is stated numerically.
fn identifier_of(database: &str, name: &str) -> Option<u32> {
    if let Ok(numeric) = name.parse::<u32>() {
        return Some(numeric);
    }
    let text = std::fs::read_to_string(database).ok()?;
    text.lines().find_map(|line| {
        let mut fields = line.split(':');
        (fields.next()? == name).then(|| fields.nth(1)?.parse().ok())?
    })
}

/// The gid of the group the store's directories belong to.
///
/// `keys` where the host declares one, `nogroup` where it does not, and `0`
/// under `--ignore-passwd` — which is what a nix build and the sandboxed
/// coexistence check run as, neither having any group at all.
fn keys_group(ignore_passwd: bool) -> u32 {
    if ignore_passwd {
        return 0;
    }
    identifier_of("/etc/group", KEYS_GROUP)
        .or_else(|| identifier_of("/etc/group", FALLBACK_GROUP))
        .unwrap_or(0)
}

/// Expand `%r` in the two store roots and in every entry's path.
///
/// Only under `userMode`: a system-scope store is an absolute path and a `%r`
/// in one is a literal directory named `%r`, which is what the substitution
/// this mirrors does too.
///
/// The runtime directory is resolved once and handed in, so that one
/// installation cannot expand two of its own paths against two answers.
fn expand_user_paths(manifest: &mut Manifest, runtime: &str) {
    manifest.secrets_mount_point = expand_runtime_dir(&manifest.secrets_mount_point, runtime);
    manifest.symlink_path = expand_runtime_dir(&manifest.symlink_path, runtime);
    for entry in &mut manifest.secrets {
        entry.path = expand_runtime_dir(&entry.path, runtime);
    }
    for template in &mut manifest.templates {
        template.path = expand_runtime_dir(&template.path, runtime);
    }
}

/// `%r` replaced by `runtime`, and `%%` by one literal `%`.
///
/// Split on `%%` first and substitute inside each piece, so that a path
/// spelling `%%r` means a literal `%r` rather than an escaped percent followed
/// by an expansion.
///
/// Pure, and takes the directory rather than looking it up, because the lookup
/// is the platform-dependent half ([`runtime_directory`]) and the
/// substitution is the half a test can state a literal expectation about on
/// either platform.
#[must_use]
pub fn expand_runtime_dir(template: &str, runtime: &str) -> String {
    if !template.contains('%') {
        return template.to_owned();
    }
    template
        .split("%%")
        .map(|piece| piece.replace("%r", runtime))
        .collect::<Vec<_>>()
        .join("%")
}

/// The session's own runtime directory.
///
/// `XDG_RUNTIME_DIR` on linux, and darwin's per-user temporary directory
/// elsewhere — asked of `getconf`, which is where the platform states it,
/// rather than assembled from a uid. safix supports a darwin platform, so the
/// second arm is not optional: a user scope that only works on linux is a
/// scope that only half exists.
///
/// # Errors
///
/// [`Error::InstallRuntimeDirUnknown`] when the platform names none.
pub fn runtime_directory() -> Result<String> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Error::InstallRuntimeDirUnknown {
                asked: "XDG_RUNTIME_DIR".to_owned(),
            })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let named = "getconf DARWIN_USER_TEMP_DIR";
        let answer = Command::new("getconf")
            .arg("DARWIN_USER_TEMP_DIR")
            .stdin(Stdio::null())
            .output()
            .map_err(|_| Error::InstallRuntimeDirUnknown {
                asked: named.to_owned(),
            })?;
        let text = String::from_utf8_lossy(&answer.stdout).trim().to_owned();
        if !answer.status.success() || text.is_empty() {
            return Err(Error::InstallRuntimeDirUnknown {
                asked: named.to_owned(),
            });
        }
        Ok(text.trim_end_matches('/').to_owned())
    }
}

/// Convert one ssh private key to its age form.
///
/// A subprocess, and the one genuinely cryptography-adjacent step of the
/// installation: the conversion is an ed25519-to-X25519 birational map plus an
/// OpenSSH private-key parser, and doing it here would be the first
/// cryptographic implementation in a crate whose stated design is to have
/// none. The binary is upstream's own, and [`SSH_TO_AGE_VARIABLE`] is what
/// points a check at a script instead.
///
/// # Errors
///
/// [`Error::InstallSshKeyUnconvertible`] when the binary cannot be run, exits
/// non-zero, or produces nothing. The installer prints that refusal as one
/// line and skips the key rather than propagating it — see
/// [`assemble_identity`].
pub fn ssh_to_age(key: &Path) -> Result<Secret> {
    convert_ssh_key(&ssh_to_age_binary(), key)
}

/// The binary [`SSH_TO_AGE_VARIABLE`] names, or `ssh-to-age`.
fn ssh_to_age_binary() -> PathBuf {
    std::env::var_os(SSH_TO_AGE_VARIABLE).map_or_else(|| PathBuf::from("ssh-to-age"), PathBuf::from)
}

/// One conversion, through a named binary.
///
/// The binary is a parameter and not a lookup so that the identity assembly
/// resolves it once for a whole run, and so that a test can drive the
/// conversion against a script without setting a variable in its own process
/// — `std::env::set_var` is `unsafe` under the 2024 edition and this
/// workspace forbids unsafe code.
fn convert_ssh_key(program: &Path, key: &Path) -> Result<Secret> {
    let mut child = Command::new(program)
        .arg("-private-key")
        .arg("-i")
        .arg(key)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|cause| Error::InstallSshKeyUnconvertible {
            path: key.display().to_string(),
            reason: format!("could not run {}: {cause}", program.display()),
        })?;

    let converted = {
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::InstallSshKeyUnconvertible {
                path: key.display().to_string(),
                reason: "its output could not be read".to_owned(),
            })?;
        Secret::read_from(&mut stdout)?
    };

    let status = child
        .wait()
        .map_err(|cause| Error::InstallSshKeyUnconvertible {
            path: key.display().to_string(),
            reason: format!("could not wait for {}: {cause}", program.display()),
        })?;

    if !status.success() {
        return Err(Error::InstallSshKeyUnconvertible {
            path: key.display().to_string(),
            reason: format!(
                "{} exited {}",
                program.display(),
                status.code().unwrap_or(1)
            ),
        });
    }
    if converted.is_empty() {
        return Err(Error::InstallSshKeyUnconvertible {
            path: key.display().to_string(),
            reason: format!("{} produced nothing", program.display()),
        });
    }
    Ok(converted)
}

/// Write `<secretsMountPoint>/age-keys.txt` and return its path.
///
/// The ssh keys first, in the order the manifest names them, then the age key
/// file's own contents. The asymmetry between the two sources is the
/// behaviour, not an accident of the implementation: an unreadable age key
/// file is fatal and names the path, where an ssh key that is absent or does
/// not convert is one line on standard error and a skip, so a host with three
/// ssh keys of which one is unconvertible still decrypts.
///
/// # Errors
///
/// [`Error::IdentityKeyFileUnreadable`] for the age key file, and
/// [`Error::FileUnwritable`] for the assembled file itself.
pub fn assemble_identity(
    manifest: &Manifest,
    mount_point: &Path,
    progress: &dyn Progress,
) -> Result<PathBuf> {
    assemble_identity_with(manifest, mount_point, progress, &ssh_to_age_binary())
}

/// [`assemble_identity`], against a named converter.
fn assemble_identity_with(
    manifest: &Manifest,
    mount_point: &Path,
    progress: &dyn Progress,
    converter: &Path,
) -> Result<PathBuf> {
    let path = mount_point.join("age-keys.txt");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(IDENTITY_MODE)
        .open(&path)
        .map_err(|cause| unwritable(&path, cause))?;
    std::fs::set_permissions(&path, Permissions::from_mode(IDENTITY_MODE))
        .map_err(|cause| unwritable(&path, cause))?;

    for key in &manifest.age_ssh_key_paths {
        let key = Path::new(key);
        if !key.exists() {
            log(
                progress,
                &format!("safix: {} is not there; skipping it", key.display()),
            );
            continue;
        }
        match convert_ssh_key(converter, key) {
            Ok(identity) => {
                identity
                    .write_to(&mut file)
                    .and_then(|()| io::Write::write_all(&mut file, b"\n"))
                    .map_err(|cause| unwritable(&path, cause))?;
                if manifest.logging.key_import {
                    log(progress, &format!("safix: imported {}", key.display()));
                }
            }
            Err(refusal) => log(progress, &format!("safix: {refusal}")),
        }
    }

    if let Some(named) = &manifest.age_key_file {
        let named = Path::new(named);
        let mut source = File::open(named).map_err(|cause| Error::IdentityKeyFileUnreadable {
            path: named.display().to_string(),
            cause,
        })?;
        let identity = Secret::read_from(&mut source)?;
        identity
            .write_to(&mut file)
            .and_then(|()| io::Write::write_all(&mut file, b"\n"))
            .map_err(|cause| unwritable(&path, cause))?;
        if manifest.logging.key_import {
            log(progress, &format!("safix: imported {}", named.display()));
        }
    }

    io::Write::flush(&mut file).map_err(|cause| unwritable(&path, cause))?;
    Ok(path)
}

/// Verify each entry's key exists in the document that is to hold it, without
/// decrypting anything.
///
/// What `--check-mode=document` performs, and it reads no ciphertext: sops
/// enciphers leaf values and leaves the mapping keys in the clear, so whether
/// a declared key is in its document is answerable from the bytes. That is
/// what makes this runnable as the manifest derivation's own check phase,
/// inside a build sandbox holding no identity — a check mode that needed a key
/// would be a check mode no build could run, which is the mode this replaces
/// and the reason it read structure rather than plaintext.
///
/// Each distinct document is read once, for the reason
/// [`decrypt_documents`] runs once per document rather than once per entry.
///
/// # Errors
///
/// [`Error::FileUnreadable`] when a named document is absent or unreadable,
/// [`Error::SopsDocumentUnreadable`] when it is not YAML, and
/// [`Error::ManifestKeyMissing`] when a declared key is not in it.
fn check_documents(manifest: &Manifest) -> Result<()> {
    let distinct: BTreeSet<_> = manifest
        .secrets
        .iter()
        .map(|entry| &entry.sops_file)
        .collect();
    let mut documents = BTreeMap::new();
    for path in distinct {
        let bytes = std::fs::read(path).map_err(|cause| Error::FileUnreadable {
            path: path.display().to_string(),
            cause,
        })?;
        documents.insert(path, bytes);
    }
    for entry in &manifest.secrets {
        let bytes = documents
            .get(&entry.sops_file)
            .ok_or_else(|| missing_key(entry))?;
        if entry.format == Format::Age {
            if !bytes.starts_with(b"age-encryption.org/v1\n")
                && !bytes.starts_with(b"-----BEGIN AGE ENCRYPTED FILE-----")
            {
                return Err(layout_error("raw age source has no age container header"));
            }
        } else {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| layout_error("SOPS ciphertext must be a text container"))?;
            if !crate::sops::document::holds_key(text, &entry.key)? {
                return Err(missing_key(entry));
            }
        }
    }
    Ok(())
}

/// Decrypt each distinct document the manifest names, once.
///
/// Per document rather than per entry: the document set is small by
/// construction — one audience gets one file — and an entry count is not a
/// subprocess count.
///
/// `identity` names the assembled age identity file. Empty assemblies are
/// omitted so native SSH and `GnuPG` identities can decrypt on their own.
/// Only identities from this manifest are used; ambient keys are excluded.
///
/// # Errors
///
/// [`Error::DocumentOperation`] when the selected cryptographic backend refuses.
/// Backend diagnostics are discarded because some tools include private input.
pub fn decrypt_documents(
    manifest: &Manifest,
    identity: Option<&Path>,
) -> Result<BTreeMap<(PathBuf, Format, bool), Secret>> {
    let age_key_file = match identity {
        Some(path) => {
            let metadata =
                std::fs::metadata(path).map_err(|cause| Error::IdentityKeyFileUnreadable {
                    path: path.display().to_string(),
                    cause,
                })?;
            (metadata.len() != 0).then(|| path.to_path_buf())
        }
        None => manifest.age_key_file.as_ref().map(PathBuf::from),
    };
    let identities = Identities {
        age_key_file,
        age_ssh_key_paths: manifest
            .age_ssh_key_paths
            .iter()
            .map(PathBuf::from)
            .collect(),
        gnupg_home: manifest.gnupg_home.clone(),
        inherit_environment: false,
    };
    let distinct: BTreeSet<_> = manifest
        .secrets
        .iter()
        .map(|entry| (&entry.sops_file, entry.format, entry.key.is_empty()))
        .collect();
    let mut documents = BTreeMap::new();
    for (path, format, whole) in distinct {
        let source = Source {
            path: path.clone(),
            format,
            key: String::new(),
        };
        let value = if whole {
            ciphertext::read(&source, &identities)?
        } else {
            ciphertext::read_document_json(&source, &identities)?
        };
        documents.insert((path.clone(), format, whole), value);
    }
    Ok(documents)
}

/// Resolve one entry's `/`-nested key inside its decrypted document.
///
/// The document is JSON because sops was asked for JSON, and the walk is the
/// recursion the program this replaces performs: each `/`-separated segment
/// selects a member of an object, and the leaf is a scalar.
///
/// The value comes back as a [`Secret`] rather than a `String`, which is the
/// discipline rather than a preference: the parse happens over a zeroizing
/// buffer inside this function and nothing from it is returned, so the only
/// long-lived holder of a decrypted byte is the type that zeroes itself.
///
/// # Errors
///
/// [`Error::InstallDocumentUnparsable`] when the decrypted document is not
/// JSON, and [`Error::ManifestKeyMissing`] when a segment is absent, a segment
/// is not an object, or the leaf is not a scalar.
pub fn extract_key(document: &Secret, entry: &ManifestSecret) -> Result<Secret> {
    let mut buffer = Zeroizing::new(Vec::new());
    document
        .write_to(&mut *buffer)
        .map_err(|cause| Error::SecretRead { cause })?;
    if entry.key.is_empty() {
        return Secret::read_from(&mut buffer.as_slice());
    }
    ciphertext::extract_json(
        &buffer,
        &Source {
            path: entry.sops_file.clone(),
            format: entry.format,
            key: entry.key.clone(),
        },
    )
    .map_err(|_| missing_key(entry))
}

/// The refusal an entry whose key does not resolve carries.
fn missing_key(entry: &ManifestSecret) -> Error {
    Error::ManifestKeyMissing {
        name: entry.name.clone(),
        document: entry.sops_file.display().to_string(),
        key: entry.key.clone(),
    }
}

/// Make the store's mount point and mount a memory-backed filesystem on it.
///
/// A no-op under `userMode`, which mounts nothing: a user-scope install has
/// none of the privileges the three steps here need, and the installation it
/// replaces did not have them either.
///
/// `ramfs` by default and `tmpfs` when the consumer asked for one, with
/// `noswap` where the kernel takes it — the mount options are the point of the
/// whole step, since a secret store that can be swapped out is a secret store
/// on a disk.
///
/// # Errors
///
/// [`Error::InstallMountFailed`] when the mount cannot be made, and
/// [`Error::FileUnwritable`] when the directory cannot be made or owned.
fn mount_store(manifest: &Manifest, mount_point: &Path, keys_gid: u32) -> Result<()> {
    match std::fs::symlink_metadata(mount_point) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(layout_error(
                "the secret mount point must be a real directory, not a link",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(unwritable(mount_point, error)),
    }
    make_directory(mount_point, DIRECTORY_MODE)?;
    if manifest.user_mode {
        return Ok(());
    }

    let (filesystem, magic) = if manifest.use_tmpfs {
        ("tmpfs", TMPFS_MAGIC)
    } else {
        ("ramfs", RAMFS_MAGIC)
    };

    if filesystem_magic(mount_point) != Some(magic) {
        mount_memory_backed(mount_point, filesystem, manifest.use_tmpfs)?;
    }

    own(mount_point, 0, keys_gid)
}

/// The magic number of the filesystem mounted at this path.
fn filesystem_magic(path: &Path) -> Option<i128> {
    rustix::fs::statfs(path).ok().map(|answer| {
        let kind: i128 = answer.f_type.into();
        kind
    })
}

/// `mount(2)`, through a safe wrapper.
///
/// linux only, because the two filesystems are: a darwin host reaching here
/// has no memory-backed mount to make and is told so rather than silently
/// installing onto a disk.
#[cfg(target_os = "linux")]
fn mount_memory_backed(mount_point: &Path, filesystem: &str, use_tmpfs: bool) -> Result<()> {
    use rustix::mount::{MountFlags, mount};

    let flags = MountFlags::NODEV | MountFlags::NOSUID | MountFlags::NOEXEC;
    let failed = |cause: rustix::io::Errno| Error::InstallMountFailed {
        path: mount_point.display().to_string(),
        filesystem: filesystem.to_owned(),
        cause: io::Error::from(cause),
    };

    // The mount data is a C string because that is what the syscall takes;
    // both spellings are literals, so neither is assembled at runtime.
    if !use_tmpfs {
        return mount(filesystem, mount_point, filesystem, flags, c"mode=0751").map_err(failed);
    }

    // `noswap` is what makes a tmpfs as good as a ramfs for this, and it is
    // also younger than the kernels this may run on: a kernel that does not
    // know the option rejects the whole mount with EINVAL, and the fallback is
    // the same mount without it rather than no store at all.
    match mount(
        filesystem,
        mount_point,
        filesystem,
        flags,
        c"noswap,mode=0751",
    ) {
        Ok(()) => Ok(()),
        Err(rustix::io::Errno::INVAL) => {
            mount(filesystem, mount_point, filesystem, flags, c"mode=0751").map_err(failed)
        }
        Err(cause) => Err(failed(cause)),
    }
}

/// The platforms with no memory-backed mount of either kind.
#[cfg(not(target_os = "linux"))]
fn mount_memory_backed(mount_point: &Path, filesystem: &str, _use_tmpfs: bool) -> Result<()> {
    Err(Error::InstallMountFailed {
        path: mount_point.display().to_string(),
        filesystem: filesystem.to_owned(),
        cause: io::Error::from(io::ErrorKind::Unsupported),
    })
}

/// The number the next generation directory is named.
///
/// Start at one without a recognized counter; never reuse an exhausted counter.
fn next_generation(symlink_path: &Path) -> Result<u64> {
    let previous = match std::fs::read_link(symlink_path) {
        Ok(target) => generation_of(&target).unwrap_or(0),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(unwritable(symlink_path, error)),
    };
    previous
        .checked_add(1)
        .ok_or_else(|| layout_error("the generation counter is exhausted"))
}

/// The generation number a symlink target names, if it names one.
fn generation_of(target: &Path) -> Option<u64> {
    target.file_name()?.to_str()?.parse().ok()
}

/// A directory at exactly this mode, whatever the umask is.
fn make_directory(path: &Path, mode: u32) -> Result<()> {
    if !path.is_dir() {
        DirBuilder::new()
            .recursive(true)
            .mode(mode)
            .create(path)
            .map_err(|cause| unwritable(path, cause))?;
    }
    std::fs::set_permissions(path, Permissions::from_mode(mode))
        .map_err(|cause| unwritable(path, cause))
}

/// One entry's file, at exactly its declared mode.
fn write_entry(destination: &Path, value: &Secret, mode: u32) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(destination)
        .map_err(|cause| unwritable(destination, cause))?;
    value
        .write_to(&mut file)
        .and_then(|()| io::Write::flush(&mut file))
        .map_err(|cause| unwritable(destination, cause))?;
    std::fs::set_permissions(destination, Permissions::from_mode(mode))
        .map_err(|cause| unwritable(destination, cause))
}

/// `chown`, through a safe wrapper.
fn own(path: &Path, uid: u32, gid: u32) -> Result<()> {
    rustix::fs::chown(path, Some(Uid::from_raw(uid)), Some(Gid::from_raw(gid)))
        .map_err(|cause| unwritable(path, io::Error::from(cause)))
}

/// Whether the previous generation held something other than this value.
///
/// A path with nothing at it is a new entry, which counts as changed; a value
/// that reads back identically is not, and neither restarts nor reloads
/// anything.
fn differs(previous: &Path, value: &Secret) -> bool {
    let Ok(mut file) = File::open(previous) else {
        return true;
    };
    Secret::read_from(&mut file).map_or(true, |held| !held.equals(value))
}

/// The units the changed entries name, or nothing at all when there is no
/// previous generation to have changed from.
///
/// `previous_exists` is whether the store's symlink is there, and the early
/// return on it is the stage-2-init case: on a first install every entry is
/// new, so every entry reads as changed, and a run that propagated from that
/// would restart every unit the manifest names on a machine that is still
/// coming up. There is nothing to restart because nothing was running.
fn units_of(
    changed: &[&ManifestSecret],
    previous_exists: bool,
) -> (BTreeSet<String>, BTreeSet<String>) {
    if !previous_exists {
        return (BTreeSet::new(), BTreeSet::new());
    }
    let collect = |units: fn(&ManifestSecret) -> &Vec<String>| -> BTreeSet<String> {
        changed
            .iter()
            .flat_map(|entry| units(entry).iter().cloned())
            .collect()
    };
    let restart = collect(|entry| &entry.restart_units);
    let mut reload = collect(|entry| &entry.reload_units);
    reload.retain(|unit| !restart.contains(unit));
    (restart, reload)
}

/// Restart and reload the units named.
///
/// Two mechanisms, selected by [`RESTART_VIA_SYSTEMCTL`] and nothing else: the
/// unit sets it and the activation script does not, so the same code serves
/// both registration paths.
fn propagate(restart: &BTreeSet<String>, reload: &BTreeSet<String>, user_mode: bool) -> Result<()> {
    if restart.is_empty() && reload.is_empty() {
        return Ok(());
    }
    if user_mode || std::env::var_os(RESTART_VIA_SYSTEMCTL).is_some() {
        for (verb, units) in [("restart", restart), ("reload", reload)] {
            if units.is_empty() {
                continue;
            }
            let mut command = Command::new(
                std::env::var_os("SAFIX_SYSTEMCTL").unwrap_or_else(|| "systemctl".into()),
            );
            if user_mode {
                command.arg("--user");
            }
            let status = command
                .arg("--no-block")
                .arg(verb)
                .arg("--")
                .args(units)
                .stdin(Stdio::null())
                .status()
                .map_err(|cause| {
                    layout_error(&format!("service manager could not run: {cause}"))
                })?;
            if !status.success() {
                return Err(layout_error(
                    "service manager refused a post-publication hook",
                ));
            }
        }
        return Ok(());
    }

    for (list, units) in [(RESTART_LIST, restart), (RELOAD_LIST, reload)] {
        if units.is_empty() {
            continue;
        }
        let path = Path::new(list);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|cause| unwritable(parent, cause))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|cause| unwritable(path, cause))?;
        for unit in units {
            io::Write::write_all(&mut file, format!("{unit}\n").as_bytes())
                .map_err(|cause| unwritable(path, cause))?;
        }
    }
    Ok(())
}

/// Roll back unpublished generations and external symlink changes on ordinary errors.
struct PendingGeneration<'a> {
    path: &'a Path,
    published: bool,
}

impl Drop for PendingGeneration<'_> {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_dir_all(self.path);
        }
    }
}

struct PreparedLink<'a> {
    directory: PathBuf,
    destination: &'a Path,
    previous: bool,
    published: bool,
    committed: bool,
}

impl<'a> PreparedLink<'a> {
    fn prepare(target: &Path, destination: &'a Path, ordinal: usize) -> Result<Option<Self>> {
        let previous = match std::fs::symlink_metadata(destination) {
            Ok(metadata) if metadata.is_symlink() => Some(
                std::fs::read_link(destination).map_err(|cause| unwritable(destination, cause))?,
            ),
            Ok(_) => {
                return Err(layout_error(
                    "installation refuses to replace a non-symlink destination",
                ));
            }
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => None,
            Err(cause) => return Err(unwritable(destination, cause)),
        };
        if previous.as_deref() == Some(target) {
            return Ok(None);
        }
        let parent = destination
            .parent()
            .ok_or_else(|| layout_error("installation path has no parent"))?;
        std::fs::create_dir_all(parent).map_err(|cause| unwritable(parent, cause))?;
        let directory = parent.join(format!(".safix-link-{}-{ordinal}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder
            .create(&directory)
            .map_err(|cause| unwritable(&directory, cause))?;
        let prepared = Self {
            directory,
            destination,
            previous: previous.is_some(),
            published: false,
            committed: false,
        };
        if let Some(previous) = previous {
            symlink(previous, prepared.directory.join("previous"))
                .map_err(|cause| unwritable(destination, cause))?;
        }
        symlink(target, prepared.directory.join("candidate"))
            .map_err(|cause| unwritable(destination, cause))?;
        Ok(Some(prepared))
    }

    fn publish(&mut self) -> Result<()> {
        std::fs::rename(self.directory.join("candidate"), self.destination)
            .map_err(|cause| unwritable(self.destination, cause))?;
        self.published = true;
        Ok(())
    }
}

impl Drop for PreparedLink<'_> {
    fn drop(&mut self) {
        if self.published && !self.committed {
            let restored = if self.previous {
                std::fs::rename(self.directory.join("previous"), self.destination)
            } else {
                std::fs::remove_file(self.destination)
            };
            if restored.is_err() {
                // Keep the private backup if the filesystem also refuses rollback.
                return;
            }
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// Remove every generation directory but the last `keep`, and never the one
/// the store's symlink names.
///
/// `keep` counts from the generation just written, which is the newest by
/// construction: a run that wrote generation 7 with `keepGenerations = 2`
/// leaves 6 and 7.
///
/// The symlink's own target is kept whatever the arithmetic says, and that is
/// a guard rather than a redundancy: the generation a reader is reading
/// through right now is the one file set the store cannot lose, and any
/// future path that writes a generation without swapping onto it — a dry run
/// did exactly this — turns a count into a deletion of the live store.
fn prune(mount_point: &Path, keep: usize, symlink_path: &Path, newest: u64) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(mount_point) else {
        return Ok(());
    };
    let live = std::fs::read_link(symlink_path)
        .ok()
        .and_then(|target| generation_of(&target));
    let oldest_kept = newest.saturating_sub(keep.try_into().unwrap_or(u64::MAX).saturating_sub(1));
    for found in entries.flatten() {
        let path = found.path();
        let Some(generation) = generation_of(&path) else {
            continue;
        };
        if generation < oldest_kept && Some(generation) != live {
            std::fs::remove_dir_all(&path).map_err(|cause| unwritable(&path, cause))?;
        }
    }
    Ok(())
}

/// The refusal a filesystem step that failed carries.
fn unwritable(path: &Path, cause: io::Error) -> Error {
    Error::FileUnwritable {
        path: path.display().to_string(),
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::Recorded;

    /// The manifest every test here mutates one field of.
    ///
    /// A real one: the fleet is `alice` and `bob`, the paths are the store's
    /// own defaults, and the document names are the ones the resolver mints.
    fn manifest_json() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "secrets": [ secret_json() ],
            "secretsMountPoint": "/run/safix.d",
            "symlinkPath": "/run/safix",
            "keepGenerations": 1,
            "ageKeyFile": "/var/lib/safix/keys.txt",
            "ageSshKeyPaths": [ "/etc/ssh/ssh_host_ed25519_key" ],
            "useTmpfs": false,
            "userMode": false,
            "logging": { "keyImport": false, "secretChanges": false },
            "manifestInputHash": "0000000000000000000000000000000000000000000000000000000000000000"
        })
    }

    fn secret_json() -> serde_json::Value {
        serde_json::json!({
            "name": "alice-alone",
            "key": "alice-alone",
            "path": "/run/safix/alice-alone",
            "owner": null,
            "group": null,
            "uid": 0,
            "gid": 0,
            "sopsFile": "/nix/store/xxx-secrets.yaml",
            "format": "yaml",
            "mode": "0400",
            "restartUnits": [],
            "reloadUnits": []
        })
    }

    /// One field of a fixture, set by JSON pointer, whether or not the
    /// fixture already carries it.
    ///
    /// A pointer rather than `json["secrets"][0]["mode"]`, because indexing a
    /// `serde_json::Value` panics on a path that is not there and this
    /// workspace denies the construction that can. Adding a field the fixture
    /// does not have is deliberate rather than accidental here: several tests
    /// are about exactly that — a manifest carrying `templates`, an entry
    /// carrying `neededForUsers` — so a missing pointer means the parent
    /// object, and only a missing parent is the fixture having drifted.
    fn set(json: &mut serde_json::Value, pointer: &str, value: serde_json::Value) {
        if let Some(slot) = json.pointer_mut(pointer) {
            *slot = value;
            return;
        }
        let (parent, field) = pointer.rsplit_once('/').unwrap_or(("", pointer));
        match json
            .pointer_mut(parent)
            .and_then(serde_json::Value::as_object_mut)
        {
            Some(object) => {
                object.insert(field.to_owned(), value);
            }
            None => unreachable!("the fixture has no object at '{parent}'"),
        }
    }

    fn entry_named(name: &str, key: &str) -> ManifestSecret {
        let mut json = secret_json();
        set(
            &mut json,
            "/name",
            serde_json::Value::String(name.to_owned()),
        );
        set(&mut json, "/key", serde_json::Value::String(key.to_owned()));
        serde_json::from_value(json).expect("the fixture entry parses")
    }

    fn document_of(json: &serde_json::Value) -> Secret {
        let text = json.to_string();
        Secret::read_from(&mut text.as_bytes()).expect("a document reads")
    }

    fn rendered(value: &Secret) -> String {
        let mut bytes = Vec::new();
        value.write_to(&mut bytes).expect("a value writes");
        String::from_utf8(bytes).expect("the fixture value is text")
    }

    #[test]
    fn a_version_this_binary_does_not_know_is_refused_naming_both() {
        let mut json = manifest_json();
        set(&mut json, "/version", serde_json::json!(2));
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");
        let refusal = manifest
            .validate_version()
            .expect_err("version 2 is not read");
        assert_eq!(refusal.code().as_str(), "safix::manifest_version_unknown");
        assert!(refusal.to_string().contains("version 2"));
        assert!(refusal.to_string().contains("version 1"));
    }

    #[test]
    fn an_unknown_entry_field_is_refused_rather_than_ignored() {
        let mut json = manifest_json();
        set(
            &mut json,
            "/secrets/0/neededForUser",
            serde_json::json!(true),
        );
        let refusal = serde_json::from_value::<Manifest>(json)
            .expect_err("an entry field this runtime does not declare is refused");
        assert!(refusal.to_string().contains("neededForUser"));
    }

    #[test]
    fn an_unknown_top_level_field_is_refused_rather_than_ignored() {
        let mut json = manifest_json();
        set(&mut json, "/template", serde_json::json!([]));
        let refusal = serde_json::from_value::<Manifest>(json)
            .expect_err("a top-level field this runtime does not declare is refused");
        assert!(refusal.to_string().contains("template"));
    }

    #[test]
    fn a_format_outside_the_enum_is_refused() {
        let mut json = manifest_json();
        set(&mut json, "/secrets/0/format", serde_json::json!("toml"));
        assert!(serde_json::from_value::<Manifest>(json).is_err());
    }

    #[test]
    fn a_manifest_that_is_not_there_is_refused_naming_the_path() {
        let refusal = load(Path::new("/nonexistent/safix-manifest.json"))
            .expect_err("nothing is there to read");
        assert_eq!(refusal.code().as_str(), "safix::manifest_unreadable");
        assert!(
            refusal
                .to_string()
                .contains("/nonexistent/safix-manifest.json")
        );
    }

    #[test]
    fn a_manifest_that_is_not_json_is_refused_naming_the_path() {
        let directory = scratch("unparsable");
        let path = directory.join("manifest.json");
        std::fs::write(&path, "not json at all").expect("the fixture writes");
        let refusal = load(&path).expect_err("that is not a manifest");
        assert_eq!(refusal.code().as_str(), "safix::manifest_unparsable");
        assert!(refusal.to_string().contains("manifest.json"));
    }

    /// A sops document as sops writes one: the mapping keys in the clear, the
    /// leaf values enciphered. Nothing here is a real ciphertext and nothing
    /// decrypts it — the point is that the key names are readable without an
    /// identity, which is what the document check mode rests on.
    fn ciphertext_document(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(
            &path,
            "alice-alone: ENC[AES256_GCM,data:ZmFrZQ==,iv:aXY=,tag:dGFn,type:str]\n\
             services:\n\
             \x20   nginx:\n\
             \x20       token: ENC[AES256_GCM,data:ZGVlcA==,iv:aXY=,tag:dGFn,type:str]\n\
             sops:\n\
             \x20   age: []\n",
        )
        .expect("the fixture writes");
        path
    }

    #[test]
    fn the_document_check_resolves_every_key_without_decrypting_anything() {
        let directory = scratch("document-check");
        let document = ciphertext_document(&directory, "secrets.yaml");

        let mut json = manifest_json();
        set(
            &mut json,
            "/secrets/0/sopsFile",
            serde_json::json!(document.display().to_string()),
        );
        set(
            &mut json,
            "/secrets/0/key",
            serde_json::json!("services/nginx/token"),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        // No identity is named anywhere and no sops is reachable: a check that
        // decrypted could not pass this.
        check_documents(&manifest).expect("the key is in the document's cleartext structure");
    }

    #[test]
    fn the_document_check_refuses_a_key_the_document_does_not_hold() {
        let directory = scratch("document-absent-key");
        let document = ciphertext_document(&directory, "secrets.yaml");

        let mut json = manifest_json();
        set(
            &mut json,
            "/secrets/0/sopsFile",
            serde_json::json!(document.display().to_string()),
        );
        set(
            &mut json,
            "/secrets/0/key",
            serde_json::json!("services/nginx/absent"),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        let refusal = check_documents(&manifest).expect_err("that key is not in it");
        assert_eq!(refusal.code().as_str(), "safix::manifest_key_missing");
        assert!(refusal.to_string().contains("services/nginx/absent"));
        assert!(refusal.to_string().contains("secrets.yaml"));
    }

    #[test]
    fn the_document_check_refuses_a_document_that_is_not_there_naming_it() {
        let mut json = manifest_json();
        set(
            &mut json,
            "/secrets/0/sopsFile",
            serde_json::json!("/nowhere/at/all.yaml"),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        let refusal = check_documents(&manifest).expect_err("nothing is there to open");
        assert_eq!(refusal.code().as_str(), "safix::file_unreadable");
        assert!(refusal.to_string().contains("/nowhere/at/all.yaml"));
    }

    #[test]
    fn a_user_mode_manifest_checks_without_a_runtime_directory_to_expand_against() {
        let directory = scratch("check-user-mode");
        let document = ciphertext_document(&directory, "secrets.yaml");

        let mut json = manifest_json();
        set(&mut json, "/userMode", serde_json::json!(true));
        set(
            &mut json,
            "/secretsMountPoint",
            serde_json::json!("%r/safix.d"),
        );
        set(&mut json, "/symlinkPath", serde_json::json!("%r/safix"));
        set(
            &mut json,
            "/secrets/0/path",
            serde_json::json!("%r/safix/alice-alone"),
        );
        set(
            &mut json,
            "/secrets/0/sopsFile",
            serde_json::json!(document.display().to_string()),
        );
        let path = directory.join("manifest.json");
        std::fs::write(&path, json.to_string()).expect("the fixture writes");

        // `%r` is a session's runtime directory and a check mode runs where
        // there is no session: the manifest derivation's own check phase is a
        // nix build. Expansion is step 1 of the installation and of nothing
        // else, so neither mode may reach it — a user-scope manifest that
        // refused here would refuse in every sandbox.
        for mode in [CheckMode::Manifest, CheckMode::Document] {
            let options = Options {
                check_mode: mode,
                ignore_passwd: true,
                dry_run: false,
            };
            run(&path, &options, &Recorded::default())
                .expect("a check mode validates the manifest as written");
        }
    }

    #[test]
    fn a_mode_that_is_not_octal_is_refused_naming_the_entry() {
        let mut json = manifest_json();
        set(&mut json, "/secrets/0/mode", serde_json::json!("0o400"));
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");
        let refusal = validate(&manifest, false).expect_err("that is not a mode");
        assert_eq!(refusal.code().as_str(), "safix::manifest_mode_unparsable");
        assert!(refusal.to_string().contains("alice-alone"));
    }

    #[test]
    fn an_owner_this_host_does_not_declare_is_refused_naming_the_entry() {
        let mut json = manifest_json();
        set(
            &mut json,
            "/secrets/0/owner",
            serde_json::json!("nobody-by-that-name"),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");
        let refusal = validate(&manifest, false).expect_err("no such user");
        assert_eq!(refusal.code().as_str(), "safix::manifest_owner_unknown");
        assert!(refusal.to_string().contains("nobody-by-that-name"));
    }

    #[test]
    fn a_group_this_host_does_not_declare_is_refused_naming_the_entry() {
        let mut json = manifest_json();
        set(
            &mut json,
            "/secrets/0/group",
            serde_json::json!("no-such-group-here"),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");
        let refusal = validate(&manifest, false).expect_err("no such group");
        assert_eq!(refusal.code().as_str(), "safix::manifest_group_unknown");
        assert!(refusal.to_string().contains("no-such-group-here"));
    }

    #[test]
    fn a_numeric_owner_needs_no_database_and_ignore_passwd_forces_zero() {
        let mut json = manifest_json();
        set(&mut json, "/secrets/0/owner", serde_json::json!("4242"));
        set(&mut json, "/secrets/0/group", serde_json::json!("4243"));
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        let resolved = validate(&manifest, false).expect("a numeric owner resolves to itself");
        assert_eq!(
            resolved.first().map(|one| (one.uid, one.gid)),
            Some((4242, 4243))
        );

        let forced = validate(&manifest, true).expect("ignore-passwd looks nothing up");
        assert_eq!(forced.first().map(|one| (one.uid, one.gid)), Some((0, 0)));
        assert_eq!(forced.first().map(|one| one.mode), Some(0o400));
    }

    #[test]
    fn a_flat_key_resolves() {
        let document = document_of(&serde_json::json!({ "alice-alone": "a value" }));
        let value = extract_key(&document, &entry_named("alice-alone", "alice-alone"))
            .expect("the key is there");
        assert_eq!(rendered(&value), "a value");
    }

    #[test]
    fn a_nested_key_resolves_through_every_segment() {
        let document = document_of(&serde_json::json!({
            "services": { "nginx": { "token": "deep" } }
        }));
        let value = extract_key(&document, &entry_named("token", "services/nginx/token"))
            .expect("the nested key is there");
        assert_eq!(rendered(&value), "deep");
    }

    /// An extraction that must refuse, unwrapped.
    ///
    /// A `let ... else` rather than `expect_err`, which would need the
    /// success type to implement `Debug`: [`Secret`] deliberately does not,
    /// and `secret.rs` asserts that absence at compile time.
    macro_rules! refusal_of {
        ($outcome:expr, $why:literal) => {
            match $outcome {
                Ok(_) => unreachable!($why),
                Err(refusal) => refusal,
            }
        };
    }

    #[test]
    fn a_missing_leaf_is_refused_naming_the_key_and_the_document() {
        let document = document_of(&serde_json::json!({ "services": { "nginx": {} } }));
        let refusal = refusal_of!(
            extract_key(&document, &entry_named("token", "services/nginx/token")),
            "the leaf is absent"
        );
        assert_eq!(refusal.code().as_str(), "safix::manifest_key_missing");
        assert!(refusal.to_string().contains("services/nginx/token"));
        assert!(refusal.to_string().contains("secrets.yaml"));
    }

    #[test]
    fn a_segment_that_is_not_an_object_is_refused_rather_than_walked_into() {
        let document = document_of(&serde_json::json!({ "services": "a string" }));
        let refusal = refusal_of!(
            extract_key(&document, &entry_named("token", "services/nginx/token")),
            "a string has no members"
        );
        assert_eq!(refusal.code().as_str(), "safix::manifest_key_missing");
    }

    #[test]
    fn a_leaf_that_is_not_a_scalar_is_refused() {
        let document = document_of(&serde_json::json!({ "block": { "inner": "x" } }));
        let refusal = refusal_of!(
            extract_key(&document, &entry_named("block", "block")),
            "an object is not a value"
        );
        assert_eq!(refusal.code().as_str(), "safix::manifest_key_missing");
    }

    /// Task 5.3's claim, as a compile-time fact rather than a sentence: the
    /// extraction's result is the crate's own zeroizing type, which has no
    /// `Display`, no `ToString` and no accessor returning its bytes, so a
    /// caller cannot hold the plaintext as a `String` even carelessly.
    #[test]
    fn the_extraction_returns_the_zeroizing_type_rather_than_a_string() {
        let document = document_of(&serde_json::json!({ "alice-alone": "a value" }));
        let value: Secret = extract_key(&document, &entry_named("alice-alone", "alice-alone"))
            .expect("the key is there");
        let mut sink = Vec::new();
        value
            .write_to(&mut sink)
            .expect("its only egress is a write");
        assert_eq!(sink, b"a value");
    }

    #[test]
    fn a_number_and_a_boolean_leaf_render_as_they_are_written() {
        let document = document_of(&serde_json::json!({ "port": 8080, "on": true }));
        let port = extract_key(&document, &entry_named("port", "port")).expect("a number resolves");
        assert_eq!(rendered(&port), "8080");
        let flag = extract_key(&document, &entry_named("on", "on")).expect("a boolean resolves");
        assert_eq!(rendered(&flag), "true");
    }

    #[test]
    fn the_runtime_directory_is_substituted_and_doubling_is_a_literal_percent() {
        assert_eq!(
            expand_runtime_dir("%r/safix.d", "/run/user/1000"),
            "/run/user/1000/safix.d"
        );
        assert_eq!(
            expand_runtime_dir("100%% sure", "/run/user/1000"),
            "100% sure"
        );
        assert_eq!(
            expand_runtime_dir("%%r/literal", "/run/user/1000"),
            "%r/literal",
            "a doubled percent is a literal one, so %%r is not an expansion"
        );
        assert_eq!(
            expand_runtime_dir("/run/safix", "/run/user/1000"),
            "/run/safix"
        );
    }

    #[test]
    fn expansion_reaches_both_roots_and_every_entry_path() {
        let mut json = manifest_json();
        set(&mut json, "/userMode", serde_json::json!(true));
        set(
            &mut json,
            "/secretsMountPoint",
            serde_json::json!("%r/safix.d"),
        );
        set(&mut json, "/symlinkPath", serde_json::json!("%r/safix"));
        set(
            &mut json,
            "/secrets/0/path",
            serde_json::json!("%r/safix/alice-alone"),
        );
        let mut manifest: Manifest = serde_json::from_value(json).expect("it parses");
        expand_user_paths(&mut manifest, "/run/user/1000");

        assert_eq!(manifest.secrets_mount_point, "/run/user/1000/safix.d");
        assert_eq!(manifest.symlink_path, "/run/user/1000/safix");
        assert_eq!(
            manifest.secrets.first().map(|one| one.path.clone()),
            Some("/run/user/1000/safix/alice-alone".to_owned())
        );
    }

    #[test]
    fn the_generation_number_is_the_symlink_target_s_basename_incremented() {
        let directory = scratch("generations");
        let link = directory.join("safix");

        assert_eq!(
            next_generation(&link).expect("first generation"),
            1,
            "no store yet is generation 1"
        );

        symlink(directory.join("7"), &link).expect("the fixture links");
        assert_eq!(next_generation(&link).expect("next generation"), 8);

        std::fs::remove_file(&link).expect("the fixture unlinks");
        symlink(directory.join("not-a-number"), &link).expect("the fixture links");
        assert_eq!(
            next_generation(&link).expect("unrecognized counter"),
            1,
            "a target that names no generation is not a counter to continue from"
        );

        assert_eq!(generation_of(Path::new("/run/safix.d/12")), Some(12));
        assert_eq!(generation_of(Path::new("/run/safix.d/-1")), None);
    }

    #[test]
    fn pruning_never_removes_the_generation_the_symlink_names() {
        let mount_point = scratch("prune-live");
        let link = scratch("prune-live-store").join("safix");
        for generation in 1..=3_u64 {
            std::fs::create_dir_all(mount_point.join(generation.to_string()))
                .expect("the fixture makes a generation");
        }
        symlink(mount_point.join("1"), &link).expect("the fixture links");

        // keepGenerations = 1 against a newest of 3 would leave 3 alone, and
        // the live store is 1: an arithmetic prune deletes what a reader is
        // reading. It is not a hypothetical — a dry run writes a generation
        // it does not swap onto, which is exactly this shape.
        prune(&mount_point, 1, &link, 3).expect("the prune");

        assert!(
            mount_point.join("1").is_dir(),
            "the generation the symlink names survives whatever the count says"
        );
        assert!(
            !mount_point.join("2").is_dir(),
            "and the one nothing names does not"
        );
        assert!(mount_point.join("3").is_dir(), "nor does the newest go");
    }

    #[test]
    fn a_first_install_restarts_nothing_because_nothing_was_running() {
        let mut named = entry_named("alice-alone", "alice-alone");
        named.restart_units = vec!["nginx.service".to_owned()];
        named.reload_units = vec!["sshd.service".to_owned()];
        let changed = vec![&named];

        let (restart, reload) = units_of(&changed, false);
        assert!(
            restart.is_empty() && reload.is_empty(),
            "with no previous generation every entry reads as changed, and a \
             machine still coming up has nothing to restart"
        );

        let (restart, reload) = units_of(&changed, true);
        assert!(restart.contains("nginx.service"));
        assert!(reload.contains("sshd.service"));
    }

    #[test]
    fn an_absent_age_key_file_is_a_refusal_naming_the_path() {
        let directory = scratch("identity-absent");
        let mut json = manifest_json();
        set(&mut json, "/ageSshKeyPaths", serde_json::json!([]));
        set(
            &mut json,
            "/ageKeyFile",
            serde_json::json!(directory.join("nowhere.txt").display().to_string()),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        let refusal = assemble_identity(&manifest, &directory, &Recorded::default())
            .expect_err("a named key file that is not there is fatal");
        assert_eq!(
            refusal.code().as_str(),
            "safix::identity_key_file_unreadable"
        );
        assert!(refusal.to_string().contains("nowhere.txt"));
    }

    #[test]
    fn a_present_key_file_is_appended_and_the_assembled_file_is_private() {
        let directory = scratch("identity-present");
        let key_file = directory.join("keys.txt");
        std::fs::write(&key_file, "AGE-SECRET-KEY-FIXTURE\n").expect("the fixture writes");

        let mut json = manifest_json();
        set(&mut json, "/ageSshKeyPaths", serde_json::json!([]));
        set(
            &mut json,
            "/ageKeyFile",
            serde_json::json!(key_file.display().to_string()),
        );
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        let assembled = assemble_identity(&manifest, &directory, &Recorded::default())
            .expect("the key file is readable");
        let held = std::fs::read_to_string(&assembled).expect("the assembled file reads");
        assert!(held.contains("AGE-SECRET-KEY-FIXTURE"));

        let mode = std::fs::metadata(&assembled)
            .expect("it is there")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, IDENTITY_MODE);
    }

    #[test]
    fn three_ssh_keys_of_which_one_does_not_convert_leave_two_lines_and_one_complaint() {
        let directory = scratch("identity-ssh");
        let converter = directory.join("ssh-to-age");
        std::fs::write(
            &converter,
            "#!/bin/sh\n\
             case \"$3\" in\n\
             *bad*) echo 'not an ssh key' >&2; exit 1 ;;\n\
             *) echo \"AGE-SECRET-KEY-$(basename \"$3\")\" ;;\n\
             esac\n",
        )
        .expect("the fixture writes");
        std::fs::set_permissions(&converter, Permissions::from_mode(0o755))
            .expect("the fixture chmods");

        let mut named = Vec::new();
        for name in ["good-one", "bad-one", "good-two"] {
            let path = directory.join(name);
            std::fs::write(&path, "ssh key material").expect("the fixture writes");
            named.push(path.display().to_string());
        }
        named.push(directory.join("absent-one").display().to_string());

        let mut json = manifest_json();
        set(&mut json, "/ageKeyFile", serde_json::Value::Null);
        set(&mut json, "/ageSshKeyPaths", serde_json::json!(named));
        let manifest: Manifest = serde_json::from_value(json).expect("it parses");

        let recorded = Recorded::default();
        let assembled = assemble_identity_with(&manifest, &directory, &recorded, &converter)
            .expect("an unconvertible key is not fatal");

        let held = std::fs::read_to_string(&assembled).expect("the assembled file reads");
        let lines: Vec<&str> = held.lines().filter(|line| !line.is_empty()).collect();
        assert_eq!(lines.len(), 2, "two of the three converted: {held}");
        assert!(held.contains("AGE-SECRET-KEY-good-one"));
        assert!(held.contains("AGE-SECRET-KEY-good-two"));

        let said = recorded.written();
        assert!(said.contains("bad-one"), "the skip is reported: {said}");
        assert!(
            said.contains("absent-one"),
            "the absence is reported: {said}"
        );
    }

    /// A scratch directory of this module's own, removed by nothing: the tests
    /// write no plaintext and the harness's tmpfs discipline is for the values
    /// a run decrypts, which none of these do.
    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("safix-install-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the fixture directory is made");
        directory
    }
}
