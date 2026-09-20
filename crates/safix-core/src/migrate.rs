//! Source-preserving migration with independent decryption before publication.

use std::{
    collections::HashSet,
    fs::{self, DirBuilder, File, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    Error, Progress, Result,
    ciphertext::{self, Format, Identities, Source},
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

struct OutputNames {
    protected: HashSet<PathBuf>,
    outputs: HashSet<PathBuf>,
}

impl OutputNames {
    fn new() -> Self {
        Self {
            protected: HashSet::new(),
            outputs: HashSet::new(),
        }
    }

    fn protect(&mut self, original: &Path, canonical: &Path) -> Result<()> {
        self.protected.insert(lexical(original)?);
        self.protected.insert(canonical.to_owned());
        Ok(())
    }

    fn reserve(&mut self, base: &Path, requested: &Path) -> Result<PathBuf> {
        let absolute = absolute(base, requested)?;
        let lexical = lexical(&absolute)?;
        let file_name = absolute
            .file_name()
            .ok_or_else(|| refused("migration output must name a file"))?;
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
        let canonical = parent.join(file_name);

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

        require_absent(&absolute)?;
        require_absent(&canonical)?;
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
    parent_identity: FileIdentity,
    verified_identity: Option<FileIdentity>,
}

struct Published {
    path: PathBuf,
    identity: FileIdentity,
}

struct Transaction {
    candidates: Vec<Candidate>,
    published: Vec<Published>,
    committed: bool,
}

impl Transaction {
    fn new(capacity: usize) -> Self {
        Self {
            candidates: Vec::with_capacity(capacity),
            published: Vec::with_capacity(capacity),
            committed: false,
        }
    }

    fn stage(&mut self, destination: &Path) -> Result<usize> {
        let parent = destination
            .parent()
            .ok_or_else(|| refused("migration output has no parent"))?;
        check_parent(parent)?;
        let parent_metadata = io_result(
            fs::metadata(parent),
            "cannot inspect migration staging parent",
        )?;
        let parent_identity = FileIdentity::of(&parent_metadata);

        for _ in 0..256 {
            let sequence = NEXT_CANDIDATE.fetch_add(1, Ordering::Relaxed);
            let directory =
                parent.join(format!(".safix-migrate-{}-{sequence}", std::process::id()));
            let mut builder = DirBuilder::new();
            builder.mode(0o700);
            match builder.create(&directory) {
                Ok(()) => {
                    let index = self.candidates.len();
                    self.candidates.push(Candidate {
                        path: directory.join("candidate"),
                        directory,
                        destination: destination.to_owned(),
                        parent_identity,
                        verified_identity: None,
                    });
                    return Ok(index);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(refused("cannot create private migration staging directory")),
            }
        }
        Err(refused(
            "cannot allocate a unique private migration staging directory",
        ))
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

    fn publish_all(&mut self) -> Result<()> {
        for candidate in &self.candidates {
            validate_candidate(candidate)?;
            require_absent(&candidate.destination)?;
        }

        for candidate in &self.candidates {
            validate_candidate(candidate)?;
            let identity = candidate
                .verified_identity
                .ok_or_else(|| refused("migration candidate has not been verified"))?;

            io_result(
                fs::hard_link(&candidate.path, &candidate.destination),
                "cannot publish migration output without overwriting an existing path",
            )?;
            self.published.push(Published {
                path: candidate.destination.clone(),
                identity,
            });

            let metadata = io_result(
                fs::symlink_metadata(&candidate.destination),
                "cannot inspect newly published migration output",
            )?;
            if !metadata.is_file() || FileIdentity::of(&metadata) != identity {
                return Err(refused("migration output changed during publication"));
            }
            io_result(
                fs::remove_file(&candidate.path),
                "cannot unlink published migration staging file",
            )?;
            sync_directory(
                candidate
                    .destination
                    .parent()
                    .ok_or_else(|| refused("migration output has no parent"))?,
            )?;
        }

        for candidate in &self.candidates {
            io_result(
                fs::remove_dir(&candidate.directory),
                "cannot remove migration staging directory",
            )?;
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
        Ok(())
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

fn prepare_plan(plan_path: &Path) -> Result<PreparedPlan> {
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
    let mut output_names = OutputNames::new();
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
    })
}

fn verify_entry(
    transaction: &mut Transaction,
    entry: &PreparedEntry,
    plan: &PreparedPlan,
) -> Result<()> {
    let index = transaction.stage(&entry.destination.path)?;
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

/// Execute a versioned migration plan and publish its verified deployment artifacts.
///
/// # Errors
///
/// Refuses aliases, existing outputs, unsupported deployment semantics and any verification failure.
pub fn run(plan_path: &Path, progress: &dyn Progress) -> Result<()> {
    let plan = prepare_plan(plan_path)?;
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
    let capacity = plan
        .entries
        .len()
        .checked_add(2)
        .ok_or_else(|| refused("migration plan contains too many entries"))?;
    let mut transaction = Transaction::new(capacity);
    progress.write("Preparing private migration candidates; sources will be retained.\n");
    for entry in &plan.entries {
        verify_entry(&mut transaction, entry, &plan)?;
    }
    let module_index = transaction.stage(&plan.deployment_output)?;
    transaction.write_public_bytes(module_index, &module_bytes)?;
    let receipt_index = transaction.stage(&plan.receipt_path)?;
    transaction.write_public_bytes(receipt_index, &receipt_bytes)?;
    progress.write("All candidates verified; publishing ciphertext, declarations, and receipt.\n");
    transaction.publish_all()?;
    progress.write("Migration complete; original source files were retained.\n");
    Ok(())
}
