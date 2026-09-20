//! Source-preserving migration with independent decryption before publication.

use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{self, DirBuilder, File, OpenOptions},
    io::Write,
    os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    Error, Progress, Result,
    ciphertext::{self, Format, Identities, Source},
    digest::sha256_hex,
    scratch,
};

static NEXT_CANDIDATE: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Plan {
    version: u32,
    #[serde(default)]
    source_identities: IdentitySpec,
    #[serde(default)]
    target_identities: IdentitySpec,
    entries: Vec<Entry>,
    #[serde(default)]
    templates: Vec<Template>,
    receipt: PathBuf,
    deployment_output: PathBuf,
    deployment_target: DeploymentTarget,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum DeploymentTarget {
    #[serde(rename = "agenix")]
    Agenix,
    #[serde(rename = "sops-nix")]
    SopsNix,
    #[serde(rename = "safix")]
    Safix,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IdentitySpec {
    #[serde(default)]
    age_key_file: Option<PathBuf>,
    #[serde(default)]
    age_ssh_key_paths: Vec<PathBuf>,
    #[serde(default)]
    gnupg_home: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceSpec {
    path: PathBuf,
    format: Format,
    key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    source: SourceSpec,
    destination: SourceSpec,
    recipients: Vec<String>,
    deployment: Deployment,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Template {
    name: String,
    content: String,
    deployment: Deployment,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Deployment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    restart_units: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reload_units: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    needed_for_users: Option<bool>,
}

struct PreparedEntry {
    name: String,
    source: Source,
    destination: Source,
    recipients: Vec<String>,
    deployment: Deployment,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt<'a> {
    version: u32,
    verification: &'static str,
    source_retained: bool,
    deployment_target: DeploymentTarget,
    deployment_output: &'a Path,
    source_identities: &'a Identities,
    target_identities: &'a Identities,
    entries: Vec<ReceiptEntry<'a>>,
    templates: &'a [Template],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptEntry<'a> {
    name: &'a str,
    source: &'a Source,
    destination: &'a Source,
    recipients: &'a [String],
    deployment: &'a Deployment,
    verification: &'static str,
}

/// The version a journal this release writes and reads carries.
const JOURNAL_VERSION: u32 = 1;

/// The suffix the journal's name adds to the receipt's.
const JOURNAL_SUFFIX: &str = ".journal";

/// The prefix every private staging directory a migration creates is named
/// with, and the only prefix an abandonment will remove a directory under.
const STAGING_PREFIX: &str = ".safix-migrate-";

/// Which of a migration's three kinds of output a journal record names.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum OutputKind {
    Ciphertext,
    Declarations,
    Receipt,
}

/// One published output, as the journal proves it.
///
/// The identity and the digest are both recorded because neither alone is the
/// claim: the identity says this is the file the run created, and the digest
/// says its bytes are the ones the run verified.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JournalOutput {
    path: PathBuf,
    kind: OutputKind,
    device: u64,
    inode: u64,
    sha256: String,
}

/// What an interrupted migration left, written beside the receipt and removed
/// as the run's last step.
///
/// Its presence is the whole meaning of "interrupted": a completed migration
/// has none, and a rerun that finds one resumes the run that wrote it.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Journal {
    version: u32,
    /// The plan's content and canonical directory, so the same JSON at another
    /// path is a different plan.
    plan_digest: String,
    plan: PathBuf,
    staging: Vec<PathBuf>,
    outputs: Vec<JournalOutput>,
}

fn refused(reason: impl Into<String>) -> Error {
    Error::MigrationRefused {
        reason: reason.into(),
    }
}

fn io_result<T>(result: std::io::Result<T>, operation: &'static str) -> Result<T> {
    result.map_err(|_| refused(operation))
}

fn absolute(base: &Path, path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(refused("empty migration path"));
    }
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    })
}

fn lexical(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(refused("migration path is not absolute"));
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => result.push(component.as_os_str()),
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return Err(refused("migration path escapes its filesystem root"));
                }
            }
            Component::Prefix(_) => {
                return Err(refused("unsupported migration path prefix"));
            }
        }
    }
    Ok(result)
}

fn existing_file(path: &Path, operation: &'static str) -> Result<PathBuf> {
    let canonical = io_result(fs::canonicalize(path), operation)?;
    let metadata = io_result(fs::metadata(&canonical), operation)?;
    if !metadata.is_file() {
        return Err(refused(operation));
    }
    Ok(canonical)
}

fn require_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(refused(
            "migration output already exists; overwriting is forbidden",
        )),
        Err(_) => Err(refused(
            "cannot determine whether a migration output exists",
        )),
    }
}

/// Where an output may be resolved, and the canonical form it resolves to.
///
/// Split out of [`OutputNames::reserve`] because the journal's own name is
/// derived from the receipt's canonical path before any reservation happens,
/// and deriving it any other way would let the two disagree.
fn canonical_output(base: &Path, requested: &Path) -> Result<(PathBuf, PathBuf)> {
    let absolute = absolute(base, requested)?;
    let file_name = absolute
        .file_name()
        .ok_or_else(|| refused("migration output must name a file"))?
        .to_owned();
    let parent = absolute
        .parent()
        .ok_or_else(|| refused("migration output has no parent directory"))?;
    let parent = io_result(
        fs::canonicalize(parent),
        "migration output parent directory does not exist or cannot be resolved",
    )?;
    if !io_result(
        fs::metadata(&parent),
        "cannot inspect migration output parent",
    )?
    .is_dir()
    {
        return Err(refused("migration output parent is not a directory"));
    }
    Ok((absolute, parent.join(file_name)))
}

struct OutputNames {
    protected: HashSet<PathBuf>,
    outputs: HashSet<PathBuf>,
    /// Paths an earlier interrupted run of this plan published, which exist and
    /// are therefore exempt from the absence rule — and from nothing else.
    resumed: HashSet<PathBuf>,
}

impl OutputNames {
    fn new(resumed: HashSet<PathBuf>) -> Self {
        Self {
            protected: HashSet::new(),
            outputs: HashSet::new(),
            resumed,
        }
    }

    fn protect(&mut self, original: &Path, canonical: &Path) -> Result<()> {
        self.protected.insert(lexical(original)?);
        self.protected.insert(canonical.to_owned());
        Ok(())
    }

    fn reserve(&mut self, base: &Path, requested: &Path) -> Result<PathBuf> {
        let (absolute, canonical) = canonical_output(base, requested)?;
        let lexical = lexical(&absolute)?;

        for name in [&lexical, &canonical] {
            if self.protected.contains(name) {
                return Err(refused(
                    "migration output aliases a source, identity file, or migration plan",
                ));
            }
            if self.outputs.contains(name) {
                return Err(refused(
                    "duplicate or aliased migration output; multiple keys in one destination document are not supported",
                ));
            }
        }

        if !self.resumed.contains(&canonical) {
            require_absent(&absolute)?;
            require_absent(&canonical)?;
        }
        self.outputs.insert(lexical);
        self.outputs.insert(canonical.clone());
        Ok(canonical)
    }
}

fn prepare_identities(
    spec: IdentitySpec,
    base: &Path,
    names: &mut OutputNames,
) -> Result<Identities> {
    let mut file = |path: PathBuf| -> Result<PathBuf> {
        let original = absolute(base, &path)?;
        let canonical = existing_file(&original, "cannot resolve migration identity file")?;
        names.protect(&original, &canonical)?;
        Ok(canonical)
    };

    let age_key_file = spec.age_key_file.map(&mut file).transpose()?;
    let age_ssh_key_paths = spec
        .age_ssh_key_paths
        .into_iter()
        .map(&mut file)
        .collect::<Result<Vec<_>>>()?;

    let gnupg_home = spec
        .gnupg_home
        .map(|path| {
            let original = absolute(base, &path)?;
            let canonical = io_result(
                fs::canonicalize(&original),
                "cannot resolve migration GnuPG home",
            )?;
            if !io_result(
                fs::metadata(&canonical),
                "cannot inspect migration GnuPG home",
            )?
            .is_dir()
            {
                return Err(refused("migration GnuPG home is not a directory"));
            }
            names.protect(&original, &canonical)?;
            Ok(canonical)
        })
        .transpose()?;

    Ok(Identities {
        age_key_file,
        age_ssh_key_paths,
        gnupg_home,
        inherit_environment: false,
    })
}

fn validate_source_shape(source: &SourceSpec) -> Result<()> {
    if matches!(source.format, Format::Age | Format::Binary) && !source.key.is_empty() {
        return Err(refused(
            "age and binary migration documents require an empty key",
        ));
    }
    Ok(())
}

fn validate_recipients(recipients: &[String], format: Format) -> Result<()> {
    if recipients.is_empty() {
        return Err(refused(
            "migration requires explicit nonempty target recipients",
        ));
    }
    let mut seen = HashSet::new();
    for recipient in recipients {
        if !seen.insert(recipient) {
            return Err(refused("duplicate migration recipient"));
        }

        if let Some(fingerprint) = recipient.strip_prefix("pgp:") {
            if matches!(format, Format::Age) {
                return Err(refused("raw age destinations cannot use PGP recipients"));
            }
            if !matches!(fingerprint.len(), 40 | 64)
                || !fingerprint
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            {
                return Err(refused(
                    "PGP recipients require an uppercase full fingerprint",
                ));
            }
            continue;
        }

        if recipient.starts_with("age1")
            && recipient.len() > 4
            && recipient
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        {
            continue;
        }

        if ciphertext::is_ssh_recipient(recipient)
            && (format == Format::Age || !recipient.contains(','))
        {
            continue;
        }

        return Err(refused(
            "unsupported or malformed public migration recipient",
        ));
    }
    Ok(())
}

fn nix_string(value: &str) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(refused(
            "deployment strings must not contain control characters",
        ));
    }
    let quoted =
        serde_json::to_string(value).map_err(|_| refused("cannot encode a deployment string"))?;
    Ok(quoted.replace("${", "\\${"))
}

fn path_string(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| refused("deployment paths must be valid UTF-8"))
}

fn validate_deployment(
    deployment: &mut Deployment,
    target: DeploymentTarget,
    destination: &Source,
    base: &Path,
) -> Result<()> {
    if let Some(path) = &deployment.path {
        let resolved = lexical(&absolute(base, path)?)?;
        if resolved == Path::new("/") {
            return Err(refused("deployment path must name a file"));
        }
        nix_string(path_string(&resolved)?)?;
        deployment.path = Some(resolved);
    }

    if let Some(mode) = &deployment.mode
        && (mode.len() != 4
            || !mode.starts_with('0')
            || !mode.bytes().all(|byte| (b'0'..=b'7').contains(&byte)))
    {
        return Err(refused(
            "deployment mode must be four octal digits beginning with 0",
        ));
    }

    for value in [&deployment.owner, &deployment.group].into_iter().flatten() {
        if value.is_empty() {
            return Err(refused("deployment owner and group must not be empty"));
        }
        nix_string(value)?;
    }

    for units in [&deployment.restart_units, &deployment.reload_units]
        .into_iter()
        .flatten()
    {
        let mut seen = HashSet::new();
        for unit in units {
            if unit.is_empty() || unit.starts_with('-') || !seen.insert(unit) {
                return Err(refused(
                    "deployment hook units must be nonempty, unique unit names",
                ));
            }
            nix_string(unit)?;
        }
    }

    if deployment.owner.is_some() && deployment.uid.is_some_and(|uid| uid != 0) {
        return Err(refused("specify deployment owner or uid, not both"));
    }
    if deployment.group.is_some() && deployment.gid.is_some_and(|gid| gid != 0) {
        return Err(refused("specify deployment group or gid, not both"));
    }

    match target {
        DeploymentTarget::Agenix => {
            if !matches!(destination.format, Format::Age) || !destination.key.is_empty() {
                return Err(refused(
                    "agenix deployment requires whole-document raw age destinations",
                ));
            }
            if deployment
                .restart_units
                .as_ref()
                .is_some_and(|units| !units.is_empty())
                || deployment
                    .reload_units
                    .as_ref()
                    .is_some_and(|units| !units.is_empty())
                || deployment.needed_for_users == Some(true)
            {
                return Err(refused(
                    "agenix deployment does not support restartUnits, reloadUnits, or neededForUsers",
                ));
            }
        }
        DeploymentTarget::SopsNix => {
            if matches!(destination.format, Format::Age) {
                return Err(refused(
                    "sops-nix deployment does not support raw age files",
                ));
            }
        }
        DeploymentTarget::Safix => {}
    }
    if deployment.needed_for_users == Some(true)
        && (deployment
            .owner
            .as_deref()
            .is_some_and(|owner| owner != "root")
            || deployment
                .group
                .as_deref()
                .is_some_and(|group| group != "root")
            || deployment.uid.is_some_and(|uid| uid != 0)
            || deployment.gid.is_some_and(|gid| gid != 0))
    {
        return Err(refused("early-user secrets must be root-owned"));
    }
    if matches!(target, DeploymentTarget::Safix)
        && deployment.needed_for_users == Some(true)
        && deployment
            .mode
            .as_ref()
            .is_some_and(|mode| !mode.ends_with("00"))
    {
        return Err(refused(
            "safix early-user secrets require owner-only permissions",
        ));
    }
    Ok(())
}

fn string_field(output: &mut String, name: &str, value: &str) -> Result<()> {
    output.push_str("      ");
    output.push_str(name);
    output.push_str(" = ");
    output.push_str(&nix_string(value)?);
    output.push_str(";\n");
    Ok(())
}

fn list_field(output: &mut String, name: &str, values: &[String]) -> Result<()> {
    output.push_str("      ");
    output.push_str(name);
    output.push_str(" = [");
    for value in values {
        output.push(' ');
        output.push_str(&nix_string(value)?);
    }
    output.push_str(" ];\n");
    Ok(())
}

fn declarations(
    target: DeploymentTarget,
    entries: &[PreparedEntry],
    templates: &[Template],
) -> Result<Vec<u8>> {
    let mut output = String::from("{ config, ... }:\n{\n  ");
    output.push_str(match target {
        DeploymentTarget::Agenix => "age.secrets",
        DeploymentTarget::SopsNix => "sops.secrets",
        DeploymentTarget::Safix => "safix.importedSecrets",
    });
    output.push_str(" = {\n");

    for entry in entries {
        output.push_str("    ");
        output.push_str(&nix_string(&entry.name)?);
        output.push_str(" = {\n      ");
        output.push_str(match target {
            DeploymentTarget::SopsNix | DeploymentTarget::Safix => "sopsFile",
            DeploymentTarget::Agenix => "file",
        });
        output.push_str(" = builtins.path { path = builtins.toPath ");
        output.push_str(&nix_string(path_string(&entry.destination.path)?)?);
        output.push_str("; name = \"safix-ciphertext\"; };\n");

        if !matches!(target, DeploymentTarget::Agenix) {
            string_field(&mut output, "format", entry.destination.format.as_str())?;
            string_field(&mut output, "key", &entry.destination.key)?;
        }

        deployment_fields(&mut output, &entry.deployment, target)?;
        output.push_str("    };\n");
    }

    output.push_str("  };\n");
    if !templates.is_empty() {
        output.push_str(match target {
            DeploymentTarget::Safix => "  safix.templates = {\n",
            _ => "  sops.templates = {\n",
        });
        for template in templates {
            std::fmt::Write::write_fmt(
                &mut output,
                format_args!(
                    "    {} = {{\n      content = {};\n",
                    nix_string(&template.name)?,
                    template_expression(&template.content, target)?
                ),
            )
            .map_err(|_| refused("cannot render template declaration"))?;
            deployment_fields(&mut output, &template.deployment, target)?;
            output.push_str("    };\n");
        }
        output.push_str("  };\n");
    }
    output.push_str("}\n");
    Ok(output.into_bytes())
}

fn deployment_fields(
    output: &mut String,
    deployment: &Deployment,
    target: DeploymentTarget,
) -> Result<()> {
    if let Some(path) = &deployment.path {
        string_field(output, "path", path_string(path)?)?;
    }
    for (name, value) in [
        ("mode", &deployment.mode),
        ("owner", &deployment.owner),
        ("group", &deployment.group),
    ] {
        if let Some(value) = value {
            string_field(output, name, value)?;
        }
    }
    for (number_name, owner_name, value, named) in [
        ("uid", "owner", deployment.uid, &deployment.owner),
        ("gid", "group", deployment.gid, &deployment.group),
    ] {
        if let Some(value) = value {
            if matches!(target, DeploymentTarget::Agenix) {
                if named.is_none() {
                    string_field(output, owner_name, &value.to_string())?;
                }
            } else {
                std::fmt::Write::write_fmt(
                    output,
                    format_args!("      {number_name} = {value};\n"),
                )
                .map_err(|_| refused("cannot render deployment ownership"))?;
            }
        }
    }
    if !matches!(target, DeploymentTarget::Agenix) {
        for (name, values) in [
            ("restartUnits", &deployment.restart_units),
            ("reloadUnits", &deployment.reload_units),
        ] {
            if let Some(values) = values {
                list_field(output, name, values)?;
            }
        }
        if let Some(needed) = deployment.needed_for_users {
            std::fmt::Write::write_fmt(output, format_args!("      neededForUsers = {needed};\n"))
                .map_err(|_| refused("cannot render early-user deployment"))?;
        }
    }
    Ok(())
}

fn template_parts(content: &str) -> Result<Vec<(bool, &str)>> {
    let mut parts = Vec::new();
    let mut remaining = content;
    while let Some((literal, reference)) = remaining.split_once("<safix:") {
        parts.push((false, literal));
        let (name, rest) = reference
            .split_once('>')
            .ok_or_else(|| refused("unterminated template placeholder"))?;
        parts.push((true, name));
        remaining = rest;
    }
    parts.push((false, remaining));
    Ok(parts)
}

fn template_literal(text: &str) -> Result<String> {
    if text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(refused("unsupported control character in public template"));
    }
    serde_json::to_string(text)
        .map(|quoted| quoted.replace("${", "\\${"))
        .map_err(|_| refused("cannot encode public template"))
}

fn template_expression(content: &str, target: DeploymentTarget) -> Result<String> {
    if matches!(target, DeploymentTarget::Safix) {
        return template_literal(content);
    }
    let expressions = template_parts(content)?
        .into_iter()
        .map(|(reference, text)| {
            if reference {
                Ok(format!("config.sops.placeholder.{}", nix_string(text)?))
            } else {
                template_literal(text)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(expressions.join(" + "))
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

impl FileIdentity {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

struct Candidate {
    directory: PathBuf,
    path: PathBuf,
    destination: PathBuf,
    kind: OutputKind,
    parent_identity: FileIdentity,
    verified_identity: Option<FileIdentity>,
}

struct Published {
    path: PathBuf,
    identity: FileIdentity,
}

/// A mode-700 directory nobody else can enter, beside the file it stages for.
fn private_directory(parent: &Path) -> Result<PathBuf> {
    for _ in 0..256 {
        let sequence = NEXT_CANDIDATE.fetch_add(1, Ordering::Relaxed);
        let directory = parent.join(format!("{STAGING_PREFIX}{}-{sequence}", std::process::id()));
        let mut builder = DirBuilder::new();
        builder.mode(0o700);
        match builder.create(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(refused("cannot create private migration staging directory")),
        }
    }
    Err(refused(
        "cannot allocate a unique private migration staging directory",
    ))
}

/// Remove a staging directory this migration created, and only such a
/// directory.
///
/// The name, the type and the mode are all checked, because this is reached
/// from a journal — and a journal is a file on disk, which is to say something
/// an operator or anything else could have edited. A directory that is already
/// gone is not an error: an interrupted run may have removed it before the
/// journal naming it was rewritten.
fn remove_private_directory(directory: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(refused("cannot inspect a migration staging directory")),
    };
    let named_by_us = directory
        .file_name()
        .is_some_and(|name| name.as_bytes().starts_with(STAGING_PREFIX.as_bytes()));
    if !named_by_us
        || !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(refused(format!(
            "{} is not a private staging directory this migration created",
            directory.display()
        )));
    }
    let entries = io_result(
        fs::read_dir(directory),
        "cannot read a migration staging directory",
    )?;
    for entry in entries {
        let entry = io_result(entry, "cannot read a migration staging directory")?;
        let staged = entry.path();
        let metadata = io_result(
            fs::symlink_metadata(&staged),
            "cannot inspect a staged migration file",
        )?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(refused(format!(
                "{} holds something this migration did not stage",
                directory.display()
            )));
        }
        io_result(
            fs::remove_file(&staged),
            "cannot remove a staged migration file",
        )?;
    }
    io_result(
        fs::remove_dir(directory),
        "cannot remove migration staging directory",
    )
}

/// The journal file, and the record it currently states.
///
/// Written with the receipt's own discipline — create-new in a private
/// directory, fsync, rename — so the file at the journal's path is always a
/// complete record of what has been published, never a half-written one.
struct JournalWriter {
    path: PathBuf,
    directory: PathBuf,
    record: Journal,
    /// How much of the record an earlier run wrote. A rollback returns the
    /// journal to exactly that, because those outputs are still on disk.
    resumed_outputs: usize,
    resumed_staging: usize,
}

impl JournalWriter {
    fn write(&self) -> Result<()> {
        let mut bytes = serde_json::to_vec_pretty(&self.record)
            .map_err(|_| refused("cannot serialize the migration journal"))?;
        bytes.push(b'\n');
        let sequence = NEXT_CANDIDATE.fetch_add(1, Ordering::Relaxed);
        let candidate = self.directory.join(format!("journal-{sequence}"));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true).mode(0o600);
        let mut file = io_result(
            options.open(&candidate),
            "cannot create the migration journal candidate",
        )?;
        io_result(
            file.write_all(&bytes),
            "cannot write the migration journal candidate",
        )?;
        io_result(
            file.sync_all(),
            "cannot synchronize the migration journal candidate",
        )?;
        drop(file);
        io_result(
            fs::rename(&candidate, &self.path),
            "cannot publish the migration journal",
        )?;
        sync_directory(
            self.path
                .parent()
                .ok_or_else(|| refused("the migration journal has no parent"))?,
        )
    }

    fn remove(&self) -> Result<()> {
        remove_journal(&self.path)
    }
}

fn remove_journal(path: &Path) -> Result<()> {
    io_result(fs::remove_file(path), "cannot remove the migration journal")?;
    sync_directory(
        path.parent()
            .ok_or_else(|| refused("the migration journal has no parent"))?,
    )
}

struct Transaction {
    candidates: Vec<Candidate>,
    published: Vec<Published>,
    journal: Option<JournalWriter>,
    committed: bool,
}

impl Transaction {
    fn new(capacity: usize) -> Self {
        Self {
            candidates: Vec::with_capacity(capacity),
            published: Vec::with_capacity(capacity),
            journal: None,
            committed: false,
        }
    }

    fn stage(&mut self, destination: &Path, kind: OutputKind) -> Result<usize> {
        let parent = destination
            .parent()
            .ok_or_else(|| refused("migration output has no parent"))?;
        check_parent(parent)?;
        let parent_metadata = io_result(
            fs::metadata(parent),
            "cannot inspect migration staging parent",
        )?;
        let parent_identity = FileIdentity::of(&parent_metadata);
        let directory = private_directory(parent)?;
        let index = self.candidates.len();
        self.candidates.push(Candidate {
            path: directory.join("candidate"),
            directory,
            destination: destination.to_owned(),
            kind,
            parent_identity,
            verified_identity: None,
        });
        Ok(index)
    }

    fn candidate(&self, index: usize) -> Result<&Candidate> {
        self.candidates
            .get(index)
            .ok_or_else(|| refused("missing migration candidate"))
    }

    fn write_public_bytes(&mut self, index: usize, bytes: &[u8]) -> Result<()> {
        let path = &self.candidate(index)?.path;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true).mode(0o600);
        let mut file = io_result(
            options.open(path),
            "cannot create migration artifact candidate",
        )?;
        io_result(
            file.write_all(bytes),
            "cannot write migration artifact candidate",
        )?;
        io_result(
            file.sync_all(),
            "cannot synchronize migration artifact candidate",
        )?;
        self.seal(index)
    }

    fn seal(&mut self, index: usize) -> Result<()> {
        let candidate = self
            .candidates
            .get_mut(index)
            .ok_or_else(|| refused("missing migration candidate"))?;
        let metadata = io_result(
            fs::symlink_metadata(&candidate.path),
            "cannot inspect migration candidate",
        )?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.nlink() != 1 {
            return Err(refused("migration candidate is not a private regular file"));
        }
        io_result(
            fs::set_permissions(&candidate.path, fs::Permissions::from_mode(0o600)),
            "cannot restrict migration candidate permissions",
        )?;
        let file = io_result(
            File::open(&candidate.path),
            "cannot open migration candidate",
        )?;
        let opened = io_result(file.metadata(), "cannot inspect open migration candidate")?;
        if FileIdentity::of(&metadata) != FileIdentity::of(&opened) {
            return Err(refused("migration candidate changed while being prepared"));
        }
        io_result(file.sync_all(), "cannot synchronize migration candidate")?;
        candidate.verified_identity = Some(FileIdentity::of(&opened));
        Ok(())
    }

    /// Publish every candidate, recording each in the journal as it lands.
    ///
    /// `Some(status)` is an interruption asked for between two steps: nothing
    /// further is published and the drop rolls the run back. Removing the
    /// journal is the commit point, which is why the last check sits in front
    /// of it.
    ///
    /// A record is written *before* its hard link rather than after, which is
    /// what makes the kill window empty. A hard link shares its file's device,
    /// inode and bytes, so the record taken from the staged candidate is
    /// already true of the destination the link will create; recorded
    /// afterwards, a process killed between the two calls would leave an output
    /// on disk that the journal does not name, which is exactly the state
    /// neither a rerun nor an abandonment could act on. A record whose path is
    /// absent is the other side of the same window and means the link never
    /// happened: the next run publishes it.
    fn publish_all(&mut self) -> Result<Option<i32>> {
        for candidate in &self.candidates {
            validate_candidate(candidate)?;
            require_absent(&candidate.destination)?;
        }

        if let Some(journal) = &self.journal {
            journal.write()?;
        }

        for index in 0..self.candidates.len() {
            if let Some(status) = scratch::interrupted() {
                return Ok(Some(status));
            }
            let candidate = self.candidate(index)?;
            validate_candidate(candidate)?;
            let identity = candidate
                .verified_identity
                .ok_or_else(|| refused("migration candidate has not been verified"))?;
            let staged = candidate.path.clone();
            let destination = candidate.destination.clone();
            let kind = candidate.kind;
            let parent = destination
                .parent()
                .ok_or_else(|| refused("migration output has no parent"))?
                .to_owned();

            // Re-checked here and not only in the pass above: a record is
            // written before its link, so nothing is recorded that the link
            // would then have to refuse.
            require_absent(&destination)?;
            if let Some(journal) = &mut self.journal {
                journal.record.outputs.push(JournalOutput {
                    sha256: published_digest(&staged)?,
                    path: destination.clone(),
                    kind,
                    device: identity.device,
                    inode: identity.inode,
                });
                journal.write()?;
            }

            io_result(
                fs::hard_link(&staged, &destination),
                "cannot publish migration output without overwriting an existing path",
            )?;
            self.published.push(Published {
                path: destination.clone(),
                identity,
            });

            let metadata = io_result(
                fs::symlink_metadata(&destination),
                "cannot inspect newly published migration output",
            )?;
            if !metadata.is_file() || FileIdentity::of(&metadata) != identity {
                return Err(refused("migration output changed during publication"));
            }
            io_result(
                fs::remove_file(&staged),
                "cannot unlink published migration staging file",
            )?;
            sync_directory(&parent)?;
        }

        if let Some(status) = scratch::interrupted() {
            return Ok(Some(status));
        }
        // Before the journal goes, so a kill here leaves directories the
        // journal still names rather than orphans nothing names.
        for candidate in &self.candidates {
            remove_private_directory(&candidate.directory)?;
        }
        if let Some(journal) = &self.journal {
            for directory in &journal.record.staging {
                remove_private_directory(directory)?;
            }
            journal.remove()?;
        }

        for candidate in &self.candidates {
            sync_directory(
                candidate
                    .destination
                    .parent()
                    .ok_or_else(|| refused("migration output has no parent"))?,
            )?;
        }

        self.committed = true;
        Ok(None)
    }
}

fn check_parent(parent: &Path) -> Result<()> {
    let current = io_result(
        fs::canonicalize(parent),
        "cannot resolve migration output parent during transaction",
    )?;
    if current != parent {
        return Err(refused(
            "migration output parent changed or became a symlink",
        ));
    }
    Ok(())
}

fn validate_candidate(candidate: &Candidate) -> Result<()> {
    let parent = candidate
        .destination
        .parent()
        .ok_or_else(|| refused("migration output has no parent"))?;
    check_parent(parent)?;
    let parent_metadata = io_result(
        fs::metadata(parent),
        "cannot inspect migration output parent during transaction",
    )?;
    if !parent_metadata.is_dir() || FileIdentity::of(&parent_metadata) != candidate.parent_identity
    {
        return Err(refused(
            "migration output parent changed during transaction",
        ));
    }

    let directory_metadata = io_result(
        fs::symlink_metadata(&candidate.directory),
        "cannot inspect private migration staging directory",
    )?;
    if !directory_metadata.is_dir()
        || directory_metadata.file_type().is_symlink()
        || directory_metadata.mode() & 0o777 != 0o700
    {
        return Err(refused("migration staging directory is no longer private"));
    }

    let metadata = io_result(
        fs::symlink_metadata(&candidate.path),
        "cannot inspect verified migration candidate",
    )?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || Some(FileIdentity::of(&metadata)) != candidate.verified_identity
    {
        return Err(refused(
            "verified migration candidate changed before publication",
        ));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    let directory = io_result(File::open(path), "cannot open migration output directory")?;
    io_result(
        directory.sync_all(),
        "cannot synchronize migration output directory",
    )
}

/// The digest of what is at a published path.
///
/// Of the ciphertext, the declarations or the receipt — all three are public
/// documents, and the digest of one carries nothing a reader of the file does
/// not already hold. No plaintext is hashed anywhere in this module.
fn published_digest(path: &Path) -> Result<String> {
    let bytes = io_result(fs::read(path), "cannot read a published migration output")?;
    Ok(sha256_hex(&bytes))
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        for published in self.published.iter().rev() {
            if let Ok(metadata) = fs::symlink_metadata(&published.path)
                && metadata.is_file()
                && FileIdentity::of(&metadata) == published.identity
            {
                let _ = fs::remove_file(&published.path);
                if let Some(parent) = published.path.parent() {
                    let _ = sync_directory(parent);
                }
            }
        }

        if let Some(journal) = &mut self.journal {
            journal.record.outputs.truncate(journal.resumed_outputs);
            journal.record.staging.truncate(journal.resumed_staging);
            if journal.resumed_outputs == 0 {
                // Nothing an earlier run published is left, so neither is the
                // journal: its absence is what tells the next run there is
                // nothing to resume.
                let _ = fs::remove_file(&journal.path);
                if let Some(parent) = journal.path.parent() {
                    let _ = sync_directory(parent);
                }
            } else {
                let _ = journal.write();
            }
            let _ = remove_private_directory(&journal.directory);
        }

        for candidate in self.candidates.iter().rev() {
            let _ = fs::remove_file(&candidate.path);
            let _ = fs::remove_dir(&candidate.directory);
        }
    }
}

struct PreparedPlan {
    source_identities: Identities,
    target_identities: Identities,
    deployment_target: DeploymentTarget,
    entries: Vec<PreparedEntry>,
    templates: Vec<Template>,
    receipt_path: PathBuf,
    deployment_output: PathBuf,
    /// The canonical plan, named in a refusal so an operator can see which two
    /// plans a journal is between.
    plan_path: PathBuf,
    plan_digest: String,
    journal_path: PathBuf,
    /// What an earlier interrupted run of this same plan left, if it left
    /// anything. Its digest has already been held against this plan's.
    journal: Option<Journal>,
}

fn validate_output_paths<'a>(paths: impl Iterator<Item = &'a Path>) -> Result<()> {
    let mut ordered = std::collections::BTreeSet::new();
    for path in paths {
        if !ordered.insert(path) {
            return Err(refused(
                "migration output names or deployment paths must be unique",
            ));
        }
    }
    if ordered
        .iter()
        .zip(ordered.iter().skip(1))
        .any(|(parent, child)| child.starts_with(parent))
    {
        return Err(refused(
            "migration output names or deployment paths must not overlap",
        ));
    }
    Ok(())
}

fn validate_names(plan: &Plan) -> Result<()> {
    let names = plan
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .chain(plan.templates.iter().map(|template| template.name.as_str()));
    for name in names.clone() {
        if name.is_empty()
            || name.chars().any(char::is_control)
            || Path::new(name)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(refused("migration outputs require safe, nonempty names"));
        }
    }
    validate_output_paths(names.map(Path::new))
}

fn prepare_entries(
    requested_entries: Vec<Entry>,
    base: &Path,
    target: DeploymentTarget,
    output_names: &mut OutputNames,
) -> Result<Vec<PreparedEntry>> {
    let mut entries = Vec::with_capacity(requested_entries.len());
    let mut requested_destinations = Vec::with_capacity(requested_entries.len());
    // Protect every source before reserving any destination, including later entries.
    for entry in requested_entries {
        validate_source_shape(&entry.source)?;
        validate_source_shape(&entry.destination)?;
        validate_recipients(&entry.recipients, entry.destination.format)?;
        let original_source = absolute(base, &entry.source.path)?;
        let source_path = existing_file(&original_source, "cannot resolve migration source file")?;
        output_names.protect(&original_source, &source_path)?;
        requested_destinations.push(entry.destination.path);
        entries.push(PreparedEntry {
            name: entry.name,
            source: Source {
                path: source_path,
                format: entry.source.format,
                key: entry.source.key,
            },
            destination: Source {
                path: PathBuf::new(),
                format: entry.destination.format,
                key: entry.destination.key,
            },
            recipients: entry.recipients,
            deployment: entry.deployment,
        });
    }
    for (entry, requested) in entries.iter_mut().zip(requested_destinations) {
        entry.destination.path = output_names.reserve(base, &requested)?;
        validate_deployment(&mut entry.deployment, target, &entry.destination, base)?;
    }
    Ok(entries)
}

fn prepare_templates(
    templates: &mut [Template],
    entries: &[PreparedEntry],
    target: DeploymentTarget,
    base: &Path,
) -> Result<()> {
    if matches!(target, DeploymentTarget::Agenix) && !templates.is_empty() {
        return Err(refused(
            "agenix has no native runtime templates; select safix or sops-nix to preserve them",
        ));
    }
    for template in templates {
        if template.deployment.needed_for_users == Some(true) {
            return Err(refused("templates cannot be early-user outputs"));
        }
        template.deployment.needed_for_users = None;
        let whole = Source {
            path: PathBuf::new(),
            format: Format::Binary,
            key: String::new(),
        };
        validate_deployment(&mut template.deployment, target, &whole, base)?;
        for (reference, name) in template_parts(&template.content)? {
            if reference
                && !entries.iter().any(|entry| {
                    entry.name == name && entry.deployment.needed_for_users != Some(true)
                })
            {
                return Err(refused(
                    "template references an unknown or early-user secret",
                ));
            }
        }
    }
    Ok(())
}

/// The journal's path, derived from the receipt's.
fn journal_beside(receipt: &Path) -> Result<PathBuf> {
    let mut name = OsString::from(
        receipt
            .file_name()
            .ok_or_else(|| refused("the migration receipt must name a file"))?,
    );
    name.push(JOURNAL_SUFFIX);
    Ok(receipt.with_file_name(name))
}

/// The plan's identity: its bytes, and the directory every relative path in it
/// is resolved against. The same JSON in another directory names other files
/// and is therefore another plan.
fn digest_of_plan(bytes: &[u8], directory: &Path) -> String {
    let mut material = Zeroizing::new(Vec::new());
    material.extend_from_slice(directory.as_os_str().as_bytes());
    material.push(0);
    material.extend_from_slice(bytes);
    sha256_hex(&material)
}

/// The journal at a path, when there is one this release can read.
fn read_journal(path: &Path) -> Result<Option<Journal>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(refused(
                "cannot determine whether a migration journal exists",
            ));
        }
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err(refused(format!(
                "{} is not a regular file; a migration journal cannot be read from it",
                path.display()
            )));
        }
        Ok(_) => {}
    }
    let bytes = io_result(fs::read(path), "cannot read the migration journal")?;
    let journal: Journal = serde_json::from_slice(&bytes).map_err(|_| {
        refused(format!(
            "the migration journal at {} is not one this release can read; unknown fields are forbidden",
            path.display()
        ))
    })?;
    if journal.version != JOURNAL_VERSION {
        return Err(refused(format!(
            "the migration journal at {} states version {}; this release writes and reads version {JOURNAL_VERSION}",
            path.display(),
            journal.version
        )));
    }
    Ok(Some(journal))
}

/// What a preparation is for.
///
/// The two differ in one place only: an abandonment with no journal beside the
/// receipt has to be refused for that, and refused before an output reservation
/// can refuse it for something else.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Run,
    Abandon,
}

/// The journal beside a plan's receipt, held against that plan.
///
/// Answered before any output is reserved, because the journal is what decides
/// which outputs may already exist — and because an abandonment with nothing to
/// abandon has to be refused for that rather than for the first output a
/// reservation happens to find.
fn resume_state(
    base: &Path,
    receipt: &Path,
    canonical_plan: &Path,
    plan_digest: &str,
    intent: Intent,
) -> Result<(PathBuf, Option<Journal>)> {
    let (_, receipt_canonical) = canonical_output(base, receipt)?;
    let journal_path = journal_beside(&receipt_canonical)?;
    let journal = read_journal(&journal_path)?;
    if let Some(recorded) = &journal
        && recorded.plan_digest != plan_digest
    {
        return Err(refused(format!(
            "the journal at {} was written for {}, not for {}; resume or abandon that migration before running this one",
            journal_path.display(),
            recorded.plan.display(),
            canonical_plan.display()
        )));
    }
    if intent == Intent::Abandon && journal.is_none() {
        return Err(refused(format!(
            "no migration journal at {}; there is no interrupted run to abandon, and a completed migration's outputs are not this command's to remove",
            journal_path.display()
        )));
    }
    Ok((journal_path, journal))
}

fn prepare_plan(plan_path: &Path, intent: Intent) -> Result<PreparedPlan> {
    let working_directory = io_result(
        std::env::current_dir(),
        "cannot determine the migration working directory",
    )?;
    let requested_plan = absolute(&working_directory, plan_path)?;
    let base = io_result(
        fs::canonicalize(
            requested_plan
                .parent()
                .ok_or_else(|| refused("migration plan has no parent directory"))?,
        ),
        "cannot resolve migration plan directory",
    )?;
    let canonical_plan = existing_file(&requested_plan, "cannot resolve migration plan file")?;
    let plan_bytes = Zeroizing::new(io_result(
        fs::read(&canonical_plan),
        "cannot read migration plan",
    )?);
    let plan: Plan = serde_json::from_slice(&plan_bytes).map_err(|error| {
        refused(format!(
            "invalid version 1 migration plan at line {}, column {}; unknown fields are forbidden",
            error.line(),
            error.column()
        ))
    })?;
    let plan_digest = digest_of_plan(&plan_bytes, &base);
    drop(plan_bytes);
    if plan.version != 1 {
        return Err(refused(
            "unsupported migration plan version; expected version 1",
        ));
    }
    if plan.entries.is_empty() {
        return Err(refused("migration plan must contain at least one entry"));
    }
    validate_names(&plan)?;
    let (journal_path, journal) =
        resume_state(&base, &plan.receipt, &canonical_plan, &plan_digest, intent)?;
    // The journal itself and the outputs it records are the only paths a
    // reservation may find already present, and only while that journal is the
    // one this plan wrote.
    let mut resumed: HashSet<PathBuf> = HashSet::new();
    if let Some(recorded) = &journal {
        resumed.insert(journal_path.clone());
        resumed.extend(recorded.outputs.iter().map(|output| output.path.clone()));
    }
    let mut output_names = OutputNames::new(resumed);
    output_names.protect(&requested_plan, &canonical_plan)?;
    let source_identities = prepare_identities(plan.source_identities, &base, &mut output_names)?;
    let target_identities = prepare_identities(plan.target_identities, &base, &mut output_names)?;
    let entries = prepare_entries(
        plan.entries,
        &base,
        plan.deployment_target,
        &mut output_names,
    )?;
    let receipt_path = output_names.reserve(&base, &plan.receipt)?;
    let deployment_output = output_names.reserve(&base, &plan.deployment_output)?;
    output_names.reserve(&base, &journal_path)?;
    for identities in [&source_identities, &target_identities] {
        if let Some(home) = &identities.gnupg_home
            && output_names
                .outputs
                .iter()
                .any(|path| path.starts_with(home))
        {
            return Err(refused(
                "migration outputs must not be placed inside a GnuPG home",
            ));
        }
    }
    let mut templates = plan.templates;
    prepare_templates(&mut templates, &entries, plan.deployment_target, &base)?;
    validate_output_paths(
        entries
            .iter()
            .filter_map(|entry| entry.deployment.path.as_deref())
            .chain(
                templates
                    .iter()
                    .filter_map(|template| template.deployment.path.as_deref()),
            ),
    )?;
    Ok(PreparedPlan {
        source_identities,
        target_identities,
        entries,
        templates,
        deployment_target: plan.deployment_target,
        receipt_path,
        deployment_output,
        plan_path: canonical_plan,
        plan_digest,
        journal_path,
        journal,
    })
}

fn verify_entry(
    transaction: &mut Transaction,
    entry: &PreparedEntry,
    plan: &PreparedPlan,
) -> Result<()> {
    let index = transaction.stage(&entry.destination.path, OutputKind::Ciphertext)?;
    let candidate = Source {
        path: transaction.candidate(index)?.path.clone(),
        format: entry.destination.format,
        key: entry.destination.key.clone(),
    };
    let source_value = ciphertext::read(&entry.source, &plan.source_identities)
        .map_err(|_| refused("migration source decryption failed; no outputs were published"))?;
    if matches!(plan.deployment_target, DeploymentTarget::SopsNix)
        && !candidate.key.is_empty()
        && (!source_value.is_utf8() || source_value.is_empty())
    {
        return Err(refused(
            "sops-nix cannot decode safix byte envelopes; select a whole binary destination for binary or intentional empty values",
        ));
    }
    ciphertext::write(
        &candidate,
        &source_value,
        &entry.recipients,
        &plan.target_identities,
    )
    .map_err(|_| refused("migration candidate encryption failed; no outputs were published"))?;
    transaction.seal(index)?;
    let target_value = ciphertext::read(&candidate, &plan.target_identities)
        .map_err(|_| refused("independent target decryption failed; no outputs were published"))?;
    if !source_value.equals(&target_value) {
        return Err(refused(
            "migration byte verification failed; sources retained and no outputs published",
        ));
    }
    validate_candidate(transaction.candidate(index)?)
}

/// What a path that no longer answers for its record is refused with.
///
/// The path is named and nothing is touched: a file that does not match the
/// journal is not the tool's to remove, whichever of the two is wrong about it.
fn mismatched(path: &Path) -> Error {
    refused(format!(
        "{} no longer matches the migration journal's record of it; nothing was published or removed",
        path.display()
    ))
}

/// What a recorded output is now.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Recorded {
    /// The file is there and answers for its record.
    Published,
    /// Nothing is at the path. The record was written and the link never
    /// happened, or the file has since been removed; either way there is
    /// nothing to keep and nothing to remove, and the path is free to publish.
    Absent,
}

/// Hold a recorded output to its record: a regular file, that identity, those
/// bytes.
fn inspect_recorded(record: &JournalOutput) -> Result<Recorded> {
    let metadata = match fs::symlink_metadata(&record.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Recorded::Absent);
        }
        Err(_) => return Err(mismatched(&record.path)),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.dev() != record.device
        || metadata.ino() != record.inode
        || published_digest(&record.path)? != record.sha256
    {
        return Err(mismatched(&record.path));
    }
    Ok(Recorded::Published)
}

/// Re-read a kept ciphertext output through the target identities and hold it
/// against its source, exactly as a fresh candidate is held.
fn reverify_published(entry: &PreparedEntry, plan: &PreparedPlan) -> Result<()> {
    let source_value = ciphertext::read(&entry.source, &plan.source_identities)
        .map_err(|_| refused("migration source decryption failed; no outputs were published"))?;
    let target_value =
        ciphertext::read(&entry.destination, &plan.target_identities).map_err(|_| {
            refused(format!(
                "independent target decryption of the kept output {} failed; nothing was published",
                entry.destination.path.display()
            ))
        })?;
    if !source_value.equals(&target_value) {
        return Err(refused(format!(
            "the kept migration output {} does not hold its source's bytes; nothing was published",
            entry.destination.path.display()
        )));
    }
    Ok(())
}

/// A kept public artifact is the one this plan would write, or the run refuses.
fn require_recomputed(record: &JournalOutput, expected: &Path, bytes: &[u8]) -> Result<()> {
    if record.path != expected {
        return Err(refused(format!(
            "the migration journal records {} as an output this plan does not write",
            record.path.display()
        )));
    }
    if sha256_hex(bytes) != record.sha256 {
        return Err(refused(format!(
            "{} differs from what this plan would write; nothing was published",
            record.path.display()
        )));
    }
    Ok(())
}

/// Everything the journal claims, checked before anything is staged.
///
/// A claim is never trusted for being written down: the identity and the bytes
/// are read back, and a kept ciphertext is decrypted through the target
/// identities the way a fresh candidate is. What comes back is the set of
/// outputs the run may keep; a record whose path is absent is not among them
/// and is published afresh.
fn verify_resumed(
    plan: &PreparedPlan,
    module_bytes: &[u8],
    receipt_bytes: &[u8],
) -> Result<HashSet<PathBuf>> {
    let mut kept = HashSet::new();
    let Some(journal) = &plan.journal else {
        return Ok(kept);
    };
    let mut seen: HashSet<&Path> = HashSet::new();
    for record in &journal.outputs {
        if !seen.insert(record.path.as_path()) {
            return Err(refused(format!(
                "the migration journal records {} twice",
                record.path.display()
            )));
        }
        if inspect_recorded(record)? == Recorded::Absent {
            continue;
        }
        match record.kind {
            OutputKind::Ciphertext => {
                let entry = plan
                    .entries
                    .iter()
                    .find(|entry| entry.destination.path == record.path)
                    .ok_or_else(|| {
                        refused(format!(
                            "the migration journal records {}, which this plan does not name as a destination",
                            record.path.display()
                        ))
                    })?;
                reverify_published(entry, plan)?;
            }
            OutputKind::Declarations => {
                require_recomputed(record, &plan.deployment_output, module_bytes)?;
            }
            OutputKind::Receipt => {
                require_recomputed(record, &plan.receipt_path, receipt_bytes)?;
            }
        }
        kept.insert(record.path.clone());
    }
    Ok(kept)
}

/// The journal this run will keep, carrying forward what an interrupted run of
/// the same plan already published and every staging directory either run made.
fn open_journal(
    plan: &PreparedPlan,
    previous: Option<Journal>,
    staged: &[PathBuf],
) -> Result<JournalWriter> {
    let parent = plan
        .journal_path
        .parent()
        .ok_or_else(|| refused("the migration journal has no parent"))?;
    let directory = private_directory(parent)?;
    let (mut staging, outputs) = match previous {
        Some(journal) => (journal.staging, journal.outputs),
        None => (Vec::new(), Vec::new()),
    };
    let resumed_staging = staging.len();
    let resumed_outputs = outputs.len();
    staging.extend(staged.iter().cloned());
    staging.push(directory.clone());
    Ok(JournalWriter {
        path: plan.journal_path.clone(),
        directory,
        record: Journal {
            version: JOURNAL_VERSION,
            plan_digest: plan.plan_digest.clone(),
            plan: plan.plan_path.clone(),
            staging,
            outputs,
        },
        resumed_outputs,
        resumed_staging,
    })
}

/// Execute a versioned migration plan and publish its verified deployment artifacts.
///
/// A rerun of a plan whose journal is still beside its receipt resumes that
/// run: the outputs the journal proves are its own are re-verified and kept,
/// the rest are published, and the journal is removed last. The status is the
/// one the run is to exit with — zero, or the interruption's own.
///
/// # Errors
///
/// Refuses aliases, existing outputs, unsupported deployment semantics, any
/// verification failure, a journal written for another plan and a recorded
/// output that no longer matches its record.
pub fn run(plan_path: &Path, progress: &dyn Progress) -> Result<i32> {
    let mut plan = prepare_plan(plan_path, Intent::Run)?;
    let module_bytes = declarations(plan.deployment_target, &plan.entries, &plan.templates)?;
    let receipt = Receipt {
        version: 1,
        verification: "independent-target-decryption-byte-equal",
        source_retained: true,
        deployment_target: plan.deployment_target,
        deployment_output: &plan.deployment_output,
        source_identities: &plan.source_identities,
        target_identities: &plan.target_identities,
        templates: &plan.templates,
        entries: plan
            .entries
            .iter()
            .map(|entry| ReceiptEntry {
                name: &entry.name,
                source: &entry.source,
                destination: &entry.destination,
                recipients: &entry.recipients,
                deployment: &entry.deployment,
                verification: "independent-target-decryption-byte-equal",
            })
            .collect(),
    };
    let mut receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|_| refused("cannot serialize migration receipt metadata"))?;
    receipt_bytes.push(b'\n');
    let kept = verify_resumed(&plan, &module_bytes, &receipt_bytes)?;
    if plan.journal.is_some() {
        progress.write(&format!(
            "Resuming an interrupted migration: {} output(s) it published were re-verified and kept.\n",
            kept.len()
        ));
    }
    let capacity = plan
        .entries
        .len()
        .checked_add(2)
        .ok_or_else(|| refused("migration plan contains too many entries"))?;
    // Held for the whole publication, so an interruption is acted on between
    // two steps rather than inside one: the handler's own sweep and exit wait
    // on this, and the checks below are where the run stops. Declared before
    // the transaction so it outlives the rollback its drop performs.
    let _quiet = scratch::quiet();
    let mut transaction = Transaction::new(capacity);
    progress.write("Preparing private migration candidates; sources will be retained.\n");
    for entry in &plan.entries {
        if kept.contains(&entry.destination.path) {
            continue;
        }
        if let Some(status) = scratch::interrupted() {
            return Ok(status);
        }
        verify_entry(&mut transaction, entry, &plan)?;
    }
    if let Some(status) = scratch::interrupted() {
        return Ok(status);
    }
    if !kept.contains(&plan.deployment_output) {
        let module_index = transaction.stage(&plan.deployment_output, OutputKind::Declarations)?;
        transaction.write_public_bytes(module_index, &module_bytes)?;
    }
    if !kept.contains(&plan.receipt_path) {
        let receipt_index = transaction.stage(&plan.receipt_path, OutputKind::Receipt)?;
        transaction.write_public_bytes(receipt_index, &receipt_bytes)?;
    }
    if let Some(status) = scratch::interrupted() {
        return Ok(status);
    }
    let staged: Vec<PathBuf> = transaction
        .candidates
        .iter()
        .map(|candidate| candidate.directory.clone())
        .collect();
    let previous = plan.journal.take().map(|mut journal| {
        // A record whose output is absent described a link that never
        // happened; this run is publishing that output afresh and will record
        // it again.
        journal.outputs.retain(|output| kept.contains(&output.path));
        journal
    });
    transaction.journal = Some(open_journal(&plan, previous, &staged)?);
    progress.write("All candidates verified; publishing ciphertext, declarations, and receipt.\n");
    if let Some(status) = transaction.publish_all()? {
        return Ok(status);
    }
    progress.write("Migration complete; original source files were retained.\n");
    Ok(0)
}

/// Discard an interrupted migration: remove what its journal records, and
/// nothing else.
///
/// # Errors
///
/// Refuses when no journal exists beside the plan's receipt, when the journal
/// was written for another plan, and when any recorded output no longer matches
/// its record. Sources are never touched on any path.
pub fn abandon(plan_path: &Path, progress: &dyn Progress) -> Result<i32> {
    let plan = prepare_plan(plan_path, Intent::Abandon)?;
    let Some(journal) = &plan.journal else {
        return Err(refused(
            "the preparation admitted an abandonment with no journal",
        ));
    };
    for record in &journal.outputs {
        inspect_recorded(record)?;
    }
    if let Some(status) = scratch::interrupted() {
        return Ok(status);
    }
    // An abandonment that stopped half way would leave a journal claiming files
    // it had already removed, which neither a rerun nor a second abandonment
    // could then act on. Held, the signal is acted on when this returns.
    let _quiet = scratch::quiet();
    let mut removed = 0_usize;
    for record in &journal.outputs {
        // Re-checked immediately before the unlink rather than trusting the
        // pass above: what is removed is what still answers for its record.
        if inspect_recorded(record)? == Recorded::Absent {
            continue;
        }
        io_result(
            fs::remove_file(&record.path),
            "cannot remove an abandoned migration output",
        )?;
        sync_directory(
            record
                .path
                .parent()
                .ok_or_else(|| refused("an abandoned migration output has no parent"))?,
        )?;
        removed = removed.saturating_add(1);
    }
    for directory in &journal.staging {
        remove_private_directory(directory)?;
    }
    remove_journal(&plan.journal_path)?;
    progress.write(&format!(
        "Abandoned the interrupted migration: {removed} output(s) removed, every source retained.\n"
    ));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::{
        Journal, JournalOutput, JournalWriter, OutputKind, Transaction, private_directory,
    };
    use std::os::unix::fs::MetadataExt as _;
    use std::path::PathBuf;

    /// A directory of this test's own, removed by the test that made it.
    fn scratch(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "safix-migrate-journal-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        directory
    }

    fn record() -> Journal {
        Journal {
            version: 1,
            plan_digest: "0".repeat(64),
            plan: PathBuf::from("/plans/move.json"),
            staging: vec![PathBuf::from("/out/.safix-migrate-7-0")],
            outputs: vec![JournalOutput {
                path: PathBuf::from("/out/api-token.age"),
                kind: OutputKind::Ciphertext,
                device: 64_513,
                inode: 12_345,
                sha256: "a".repeat(64),
            }],
        }
    }

    /// The journal is the record a later run reads, so its JSON has to survive
    /// the round trip and refuse anything it does not know.
    #[test]
    fn the_journal_round_trips_and_refuses_unknown_fields() {
        let written = serde_json::to_string(&record()).expect("the journal serializes");
        let read: Journal = serde_json::from_str(&written).expect("the journal parses");
        assert_eq!(read, record());
        assert!(
            written.contains("\"planDigest\""),
            "the journal's fields are camelCase: {written}"
        );
        assert!(
            written.contains("\"kind\":\"ciphertext\""),
            "an output names its kind: {written}"
        );

        let widened = written.replace("{\"version\":1", "{\"version\":1,\"notes\":\"extra\"");
        assert!(
            serde_json::from_str::<Journal>(&widened).is_err(),
            "a journal carrying an unknown field was accepted"
        );
        let unknown_output = written.replace("\"sha256\"", "\"plaintextSha256\"");
        assert!(
            serde_json::from_str::<Journal>(&unknown_output).is_err(),
            "an output record carrying an unknown field was accepted"
        );
    }

    /// What the journal says while a publication is under way, and what is
    /// left once the transaction it belongs to rolls back.
    ///
    /// The second candidate is aimed at the first's destination, which no plan
    /// can produce and a hand-built transaction can: the first link lands, the
    /// second cannot, and the journal is read in between — which is the only
    /// moment its intermediate state is observable without a hook in the
    /// runtime.
    #[test]
    fn the_journal_records_each_output_as_it_lands_and_goes_with_a_rollback() {
        let directory = scratch("mid-run");
        let destination = directory.join("api-token.age");
        let journal_path = directory.join("receipt.json.journal");

        let mut transaction = Transaction::new(2);
        for _ in 0..2 {
            let index = transaction
                .stage(&destination, OutputKind::Ciphertext)
                .expect("a staged candidate");
            transaction
                .write_public_bytes(index, b"abc")
                .expect("a written candidate");
        }
        let journal_directory = private_directory(&directory).expect("a journal directory");
        let staging: Vec<PathBuf> = transaction
            .candidates
            .iter()
            .map(|candidate| candidate.directory.clone())
            .collect();
        transaction.journal = Some(JournalWriter {
            path: journal_path.clone(),
            directory: journal_directory.clone(),
            record: Journal {
                version: 1,
                plan_digest: "0".repeat(64),
                plan: directory.join("plan.json"),
                staging,
                outputs: Vec::new(),
            },
            resumed_outputs: 0,
            resumed_staging: 0,
        });

        assert!(
            transaction.publish_all().is_err(),
            "two candidates cannot both take one destination"
        );

        let mid_run: Journal =
            serde_json::from_slice(&std::fs::read(&journal_path).expect("a journal mid-run"))
                .expect("the journal parses");
        assert_eq!(mid_run.outputs.len(), 1, "the journal recorded {mid_run:?}");
        let recorded = mid_run.outputs.first().expect("one recorded output");
        assert_eq!(recorded.path, destination);
        // FIPS 180-4's own digest of "abc", not one this module computed.
        assert_eq!(
            recorded.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let published = std::fs::metadata(&destination).expect("the first output landed");
        assert_eq!(recorded.device, published.dev());
        assert_eq!(recorded.inode, published.ino());

        drop(transaction);
        assert!(
            !destination.exists(),
            "the rollback left the output it published"
        );
        assert!(!journal_path.exists(), "the rollback left its journal");
        assert!(
            !journal_directory.exists(),
            "the rollback left the journal's staging directory"
        );
        assert_eq!(
            std::fs::read_dir(&directory)
                .expect("the scratch directory")
                .count(),
            0,
            "the rollback left something in {}",
            directory.display()
        );
        std::fs::remove_dir_all(&directory).expect("the scratch directory is removable");
    }
}
