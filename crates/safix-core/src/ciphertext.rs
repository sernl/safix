//! Explicit ciphertext formats, recipients and isolated identity selection.

use crate::Secret;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::{Zeroize, Zeroizing};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// A supported ciphertext container, independently of its recipient type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    /// SOPS YAML.
    Yaml,
    /// SOPS JSON.
    Json,
    /// SOPS dotenv.
    Dotenv,
    /// SOPS INI.
    Ini,
    /// SOPS binary document.
    Binary,
    /// Native age file.
    Age,
}

impl Format {
    /// Infer a declared storage format from a recognized extension.
    ///
    /// # Errors
    ///
    /// Refuses unknown extensions rather than guessing a text format.
    pub fn from_path(path: &Path) -> crate::Result<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("yaml" | "yml") => Ok(Self::Yaml),
            Some("json") => Ok(Self::Json),
            Some("env" | "dotenv") => Ok(Self::Dotenv),
            Some("ini") => Ok(Self::Ini),
            Some("bin" | "binary") => Ok(Self::Binary),
            Some("age") => Ok(Self::Age),
            _ if path.file_name().is_some_and(|name| name == ".env") => Ok(Self::Dotenv),
            _ => Err(document_error(
                "detect format",
                path,
                "unrecognized ciphertext extension; select a format explicitly",
            )),
        }
    }

    /// The format spelling accepted by declarations and SOPS.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Dotenv => "dotenv",
            Self::Ini => "ini",
            Self::Binary => "binary",
            Self::Age => "age",
        }
    }
}

/// Identity locations used by a single cryptographic operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Identities {
    /// Native age identities in a protected file.
    pub age_key_file: Option<PathBuf>,
    /// SSH private keys accepted directly by native age.
    pub age_ssh_key_paths: Vec<PathBuf>,
    /// An explicitly selected `GnuPG` keyring.
    pub gnupg_home: Option<PathBuf>,
    /// Only process-level CLI discovery may inherit ambient identity sources.
    #[serde(skip)]
    pub inherit_environment: bool,
}

impl Identities {
    /// Discover the current CLI's configured identities without reading their contents.
    #[must_use]
    pub fn from_environment() -> Self {
        let age_key_file = nonempty_environment("SOPS_AGE_KEY_FILE")
            .or_else(|| nonempty_environment("SAFIX_AGE_KEY_FILE"))
            .map(PathBuf::from)
            .or_else(|| {
                let root = nonempty_environment("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        nonempty_environment("HOME").map(|home| PathBuf::from(home).join(".config"))
                    })?;
                let key = root.join("sops/age/keys.txt");
                key.is_file().then_some(key)
            });

        let age_ssh_key_paths = nonempty_environment("SAFIX_AGE_SSH_KEY_PATHS")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();

        let gnupg_home = nonempty_environment("GNUPGHOME")
            .or_else(|| nonempty_environment("SAFIX_GNUPGHOME"))
            .map(PathBuf::from)
            .or_else(|| {
                let data_home = nonempty_environment("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        nonempty_environment("HOME")
                            .map(|home| PathBuf::from(home).join(".local/share"))
                    })?;
                let home = data_home.join("safix/gnupg");
                fs::symlink_metadata(&home)
                    .ok()
                    .filter(|metadata| metadata.file_type().is_dir())
                    .map(|_| home)
            });

        Self {
            age_key_file,
            age_ssh_key_paths,
            gnupg_home,
            inherit_environment: true,
        }
    }
}

/// One whole encrypted document or a key selected from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Source {
    /// The ciphertext file.
    pub path: PathBuf,
    /// Explicit container format; defaults to YAML.
    #[serde(default)]
    pub format: Format,
    /// Slash-separated selection, or empty for the entire document.
    #[serde(default)]
    pub key: String,
}

fn nonempty_environment(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

fn document_error(operation: &'static str, path: &Path, cause: &'static str) -> crate::Error {
    crate::Error::DocumentOperation {
        operation,
        path: path.to_string_lossy().into_owned(),
        cause: cause.to_owned(),
    }
}

/// A writer whose entire initialized allocation is wiped on drop.
#[derive(Default)]
struct PrivateBytes {
    bytes: Zeroizing<Vec<u8>>,
    limit: Option<u64>,
}

impl PrivateBytes {
    fn as_slice(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl Write for PrivateBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Reserve into a fresh allocation so realloc never frees unwiped plaintext.
        let required =
            self.bytes.len().checked_add(bytes.len()).ok_or_else(|| {
                io::Error::new(io::ErrorKind::OutOfMemory, "document is too large")
            })?;
        if self
            .limit
            .is_some_and(|limit| !u64::try_from(required).is_ok_and(|size| size <= limit))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "private output exceeds its size limit",
            ));
        }
        if required > self.bytes.capacity() {
            let capacity = required
                .max(self.bytes.capacity().saturating_mul(2))
                .max(4096);
            let mut replacement = Zeroizing::new(Vec::new());
            replacement.try_reserve_exact(capacity).map_err(|_| {
                io::Error::new(io::ErrorKind::OutOfMemory, "document allocation failed")
            })?;
            replacement.extend_from_slice(&self.bytes);
            self.bytes = replacement;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn wipe_json(value: &mut Value) {
    match value {
        Value::String(string) => string.zeroize(),
        Value::Array(values) => {
            for value in values {
                wipe_json(value);
            }
        }
        Value::Object(values) => {
            for (mut key, mut value) in std::mem::take(values) {
                key.zeroize();
                wipe_json(&mut value);
            }
        }
        _ => {}
    }
    *value = Value::Null;
}

struct PrivateJson(Value);

impl Drop for PrivateJson {
    fn drop(&mut self) {
        wipe_json(&mut self.0);
    }
}

fn executable(variable: &str, fallback: &str) -> OsString {
    nonempty_environment(variable).unwrap_or_else(|| fallback.into())
}

fn sops_command(identities: &Identities) -> Command {
    let mut command = Command::new(executable("SAFIX_SOPS", "sops"));
    crate::identity::configure_pinentry(&mut command);
    if !identities.inherit_environment {
        for variable in [
            "SOPS_AGE_KEY",
            "SOPS_AGE_KEY_CMD",
            "SOPS_AGE_KEY_FILE",
            "SOPS_AGE_SSH_PRIVATE_KEY_FILE",
            "SSH_AUTH_SOCK",
            "GNUPGHOME",
            "GPG_AGENT_INFO",
        ] {
            command.env_remove(variable);
        }
        command
            .env("HOME", "/var/empty")
            .env("XDG_CONFIG_HOME", "/var/empty")
            .env("SOPS_AGE_KEY_FILE", "/dev/null")
            .env("GNUPGHOME", "/var/empty/.gnupg");
    }
    if let Some(home) = &identities.gnupg_home {
        command.env("GNUPGHOME", home);
    }
    if let Some(path) = &identities.age_key_file {
        command.env("SOPS_AGE_KEY_FILE", path);
    }
    if nonempty_environment("SOPS_GPG_EXEC").is_none()
        && let Some(program) = nonempty_environment("SAFIX_GPG")
    {
        command.env("SOPS_GPG_EXEC", program);
    }
    command
}

fn age_command(identities: &Identities, decrypt: bool) -> Command {
    let mut command = Command::new(executable("SAFIX_AGE", "age"));
    if decrypt {
        command.arg("--decrypt");
        if let Some(path) = &identities.age_key_file {
            command.arg("-i").arg(path);
        }
        for path in &identities.age_ssh_key_paths {
            command.arg("-i").arg(path);
        }
    }
    command
}

/// Run a filter without retaining diagnostics that could contain decrypted data.
/// The feeder and reader run concurrently to avoid pipe-capacity deadlocks.
fn filter(
    command: &mut Command,
    input: Option<&[u8]>,
    source: &Source,
    operation: &'static str,
    limit: Option<u64>,
) -> crate::Result<PrivateBytes> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = command.spawn().map_err(|_| {
        document_error(
            operation,
            &source.path,
            "could not start cryptographic subprocess",
        )
    })?;

    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(document_error(
            operation,
            &source.path,
            "cryptographic subprocess output is unavailable",
        ));
    };

    let mut output = PrivateBytes {
        limit,
        ..PrivateBytes::default()
    };
    let (writer_ok, reader_ok) = std::thread::scope(|scope| {
        let feeder = match input {
            Some(bytes) => match child.stdin.take() {
                Some(mut stdin) => Some(scope.spawn(move || stdin.write_all(bytes).is_ok())),
                None => return (false, false),
            },
            None => None,
        };

        let mut scratch = Zeroizing::new([0_u8; 16 * 1024]);
        let reader_ok = loop {
            match stdout.read(&mut *scratch) {
                Ok(0) => break true,
                Ok(count) => {
                    let Some(chunk) = scratch.get(..count) else {
                        break false;
                    };
                    if output.write_all(chunk).is_err() {
                        break false;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break false,
            }
        };
        drop(stdout);
        if !reader_ok {
            let _ = child.kill();
        }
        let writer_ok = feeder.is_none_or(|thread| thread.join().unwrap_or(false));
        (writer_ok, reader_ok)
    });

    if !writer_ok || !reader_ok {
        let _ = child.kill();
    }
    let status = child.wait();
    if !writer_ok || !reader_ok || !status.is_ok_and(|status| status.success()) {
        return Err(document_error(
            operation,
            &source.path,
            "cryptographic subprocess failed",
        ));
    }
    Ok(output)
}

fn check_key(source: &Source, operation: &'static str) -> crate::Result<()> {
    if source.key.is_empty() {
        return Ok(());
    }
    if matches!(source.format, Format::Binary | Format::Age) {
        return Err(document_error(
            operation,
            &source.path,
            "this format does not support keyed documents",
        ));
    }
    if source.key.split('/').any(str::is_empty) {
        return Err(document_error(
            operation,
            &source.path,
            "document key contains an empty path component",
        ));
    }
    Ok(())
}

fn decrypt_command(
    source: &Source,
    identities: &Identities,
    json: bool,
    ssh: Option<&Path>,
) -> Command {
    if source.format == Format::Age {
        let mut command = age_command(identities, true);
        command.arg("--").arg(&source.path);
        return command;
    }
    let mut command = sops_command(identities);
    if let Some(path) = ssh {
        command.env("SOPS_AGE_SSH_PRIVATE_KEY_FILE", path);
    }
    command
        .arg("decrypt")
        .arg("--input-type")
        .arg(source.format.as_str())
        .arg("--output-type")
        .arg(if json { "json" } else { source.format.as_str() })
        .arg("--")
        .arg(&source.path);
    command
}

fn require_local_recipients(source: &Source) -> crate::Result<()> {
    if source.format == Format::Age {
        return Ok(());
    }
    let text = fs::read_to_string(&source.path).map_err(|_| {
        document_error(
            "decrypt",
            &source.path,
            "could not inspect recipient metadata",
        )
    })?;
    let recipients = crate::sops::document::recipients_of(&text)?;
    if recipients.iter().any(|recipient| {
        !recipient.starts_with("age1")
            && !recipient.starts_with("pgp:")
            && !is_ssh_recipient(recipient)
            && recipient != crate::sops::document::THRESHOLD_GROUPS
    }) {
        return Err(document_error(
            "decrypt",
            &source.path,
            "explicit identities cannot authorize unsupported or missing recipient metadata",
        ));
    }
    Ok(())
}

pub(crate) fn prove_with_age_identity(source: &Source, identity_file: &Path) -> crate::Result<i32> {
    require_local_recipients(source)?;
    let identities = Identities {
        age_key_file: Some(identity_file.into()),
        ..Identities::default()
    };
    decrypt_command(source, &identities, false, None)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map(|status| status.code().unwrap_or(1))
        .map_err(|_| {
            document_error(
                "prove identity",
                &source.path,
                "could not run cryptographic tool",
            )
        })
}

fn decrypt(
    source: &Source,
    identities: &Identities,
    json: bool,
    limit: Option<u64>,
) -> crate::Result<PrivateBytes> {
    if !identities.inherit_environment {
        require_local_recipients(source)?;
    }
    if source.format == Format::Age {
        let mut command = decrypt_command(source, identities, json, None);
        return filter(&mut command, None, source, "decrypt", limit);
    }
    let attempts = identities
        .age_ssh_key_paths
        .iter()
        .map(Some)
        .chain(std::iter::once(None).filter(|_| identities.age_ssh_key_paths.is_empty()));
    let mut refusal = None;
    for ssh in attempts {
        let mut command = decrypt_command(source, identities, json, ssh.map(PathBuf::as_path));
        match filter(&mut command, None, source, "decrypt", limit) {
            Ok(value) => return Ok(value),
            Err(error) => refusal = Some(error),
        }
    }
    Err(refusal.unwrap_or_else(|| {
        document_error("decrypt", &source.path, "no identity source configured")
    }))
}

fn parse_document(bytes: &[u8], source: &Source) -> crate::Result<PrivateJson> {
    serde_json::from_slice(bytes)
        .map(PrivateJson)
        .map_err(|_| document_error("decode", &source.path, "invalid decrypted JSON document"))
}

/// Decrypt a whole document or extract one byte-preserving stored value.
///
/// # Errors
///
/// Refuses unavailable identities, corrupt ciphertext and unsupported selections.
pub fn read(source: &Source, identities: &Identities) -> crate::Result<Secret> {
    check_key(source, "read")?;
    let plaintext = decrypt(source, identities, !source.key.is_empty(), None)?;
    if source.key.is_empty() {
        return Secret::read_from(&mut plaintext.as_slice()).map_err(|_| {
            document_error("read", &source.path, "could not load decrypted document")
        });
    }

    extract_json(plaintext.as_slice(), source)
}

pub(crate) fn read_bounded(
    source: &Source,
    identities: &Identities,
    limit: u64,
) -> crate::Result<Secret> {
    if !source.key.is_empty() {
        return Err(document_error(
            "read",
            &source.path,
            "bounded recovery reads require a whole document",
        ));
    }
    let plaintext = decrypt(source, identities, false, Some(limit))?;
    Secret::read_from(&mut plaintext.as_slice())
}

pub(crate) fn encode_private_json(value: &impl Serialize, limit: u64) -> crate::Result<Secret> {
    let mut bytes = PrivateBytes {
        limit: Some(limit),
        ..PrivateBytes::default()
    };
    serde_json::to_writer(&mut bytes, value).map_err(|_| {
        document_error(
            "encode",
            Path::new("<memory>"),
            "private JSON exceeds its limit or cannot be encoded",
        )
    })?;
    Secret::read_from(&mut bytes.as_slice())
}

pub(crate) fn extract_json(bytes: &[u8], source: &Source) -> crate::Result<Secret> {
    let document = parse_document(bytes, source)?;
    let mut selected = &document.0;
    for component in source.key.split('/') {
        selected = match selected {
            Value::Object(values) => values.get(component),
            Value::Array(values) => component
                .parse::<usize>()
                .ok()
                .and_then(|index| values.get(index)),
            _ => None,
        }
        .ok_or_else(|| document_error("read", &source.path, "document key does not exist"))?;
    }
    Secret::from_storage_json(selected).map_err(|_| {
        document_error(
            "read",
            &source.path,
            "document value is not a supported secret",
        )
    })
}

/// Decrypt one structured document to JSON for a cached installation pass.
///
/// # Errors
///
/// Refuses whole-file formats and failed cryptographic operations.
pub fn read_document_json(source: &Source, identities: &Identities) -> crate::Result<Secret> {
    if matches!(source.format, Format::Binary | Format::Age) {
        return Err(document_error(
            "decode",
            &source.path,
            "format has no structured document",
        ));
    }
    let plaintext = decrypt(source, identities, true, None)?;
    Secret::read_from(&mut plaintext.as_slice())
}

fn insert_value(
    document: &mut Value,
    components: &[&str],
    replacement: &mut Value,
) -> Result<(), ()> {
    let Some((component, remaining)) = components.split_first() else {
        wipe_json(document);
        *document = std::mem::take(replacement);
        return Ok(());
    };

    match document {
        Value::Object(values) => {
            if remaining.is_empty() {
                if let Some(old) = values.get_mut(*component) {
                    wipe_json(old);
                    *old = std::mem::take(replacement);
                } else {
                    values.insert((*component).to_owned(), std::mem::take(replacement));
                }
                Ok(())
            } else {
                let next = values
                    .entry((*component).to_owned())
                    .or_insert_with(|| Value::Object(Map::new()));
                insert_value(next, remaining, replacement)
            }
        }
        Value::Array(values) => {
            let index = component.parse::<usize>().map_err(|_| ())?;
            let next = values.get_mut(index).ok_or(())?;
            insert_value(next, remaining, replacement)
        }
        _ => Err(()),
    }
}

pub(crate) fn is_ssh_recipient(recipient: &str) -> bool {
    let mut fields = recipient.split_ascii_whitespace();
    let kind = fields.next().unwrap_or("");
    let key = fields.next().unwrap_or("");
    matches!(kind, "ssh-ed25519" | "ssh-rsa")
        && !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+/=".contains(&byte))
        && !recipient.chars().any(char::is_control)
}

fn recipient_sets<'a>(
    destination: &Source,
    recipients: &'a [String],
) -> crate::Result<(Vec<&'a str>, Vec<&'a str>)> {
    let mut age = Vec::new();
    let mut pgp = Vec::new();

    for recipient in recipients {
        if let Some(fingerprint) = recipient.strip_prefix("pgp:") {
            if destination.format == Format::Age {
                return Err(document_error(
                    "encrypt",
                    &destination.path,
                    "native age does not support PGP recipients",
                ));
            }
            if !matches!(fingerprint.len(), 40 | 64)
                || !fingerprint
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            {
                return Err(document_error(
                    "encrypt",
                    &destination.path,
                    "invalid PGP recipient fingerprint",
                ));
            }
            pgp.push(fingerprint);
        } else {
            let native = recipient.starts_with("age1")
                && recipient.len() > 4
                && recipient
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
            let ssh = is_ssh_recipient(recipient)
                && (destination.format == Format::Age || !recipient.contains(','));
            if !native && !ssh {
                return Err(document_error(
                    "encrypt",
                    &destination.path,
                    "invalid age recipient",
                ));
            }
            age.push(recipient.as_str());
        }
    }
    if age.is_empty() && pgp.is_empty() {
        return Err(document_error(
            "encrypt",
            &destination.path,
            "at least one explicit recipient is required",
        ));
    }
    Ok((age, pgp))
}

fn destination_exists(destination: &Source) -> crate::Result<bool> {
    match fs::symlink_metadata(&destination.path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(document_error(
            "write",
            &destination.path,
            "destination is not a regular file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(document_error(
            "write",
            &destination.path,
            "could not inspect destination",
        )),
    }
}

fn encode_value(destination: &Source, value: &Secret) -> crate::Result<PrivateBytes> {
    let mut encoded = PrivateBytes::default();
    if matches!(destination.format, Format::Ini | Format::Dotenv) {
        value.write_json_to(&mut encoded)
    } else {
        value.write_storage_json_to(&mut encoded)
    }
    .map_err(|_| {
        document_error(
            "encode",
            &destination.path,
            "secret is not representable in the selected format",
        )
    })?;
    Ok(encoded)
}

fn prepare_plaintext(
    destination: &Source,
    value: &Secret,
    identities: &Identities,
    exists: bool,
) -> crate::Result<PrivateBytes> {
    let mut encoded = PrivateBytes::default();
    if destination.key.is_empty() {
        value.write_to(&mut encoded).map_err(|_| {
            document_error("encode", &destination.path, "could not encode plaintext")
        })?;
        if !matches!(destination.format, Format::Binary | Format::Age)
            && std::str::from_utf8(encoded.as_slice()).is_err()
        {
            return Err(document_error(
                "encode",
                &destination.path,
                "whole-document text must be valid UTF-8",
            ));
        }
        return Ok(encoded);
    }

    encoded = encode_value(destination, value)?;
    let mut replacement = parse_document(encoded.as_slice(), destination)?;
    let mut document = if exists {
        let plaintext = decrypt(destination, identities, true, None)?;
        parse_document(plaintext.as_slice(), destination)?
    } else {
        PrivateJson(Value::Object(Map::new()))
    };
    let components: Vec<_> = destination.key.split('/').collect();
    insert_value(&mut document.0, &components, &mut replacement.0).map_err(|()| {
        document_error(
            "encode",
            &destination.path,
            "document key crosses an incompatible value",
        )
    })?;

    // SOPS' JSON-to-dotenv/INI conversion is authoritative. Reject shapes whose
    // conversion could otherwise silently stringify values or discard nesting.
    if matches!(destination.format, Format::Dotenv | Format::Ini) {
        check_text_document(&document.0, destination)?;
    }

    let mut output = PrivateBytes::default();
    serde_json::to_writer(&mut output, &document.0)
        .map_err(|_| document_error("encode", &destination.path, "could not encode document"))?;
    Ok(output)
}

fn check_text_document(value: &Value, destination: &Source) -> crate::Result<()> {
    fn line_value(value: &Value) -> bool {
        value
            .as_str()
            .is_some_and(|text| !text.contains(['\0', '\r', '\n']) && text.trim() == text)
    }

    fn key(name: &str) -> bool {
        !name.is_empty()
            && name.trim() == name
            && !name.contains(['\0', '\r', '\n', '=', '[', ']', '#', ';'])
    }

    let valid = match value {
        Value::Object(values) if destination.format == Format::Dotenv => {
            values.iter().all(|(name, value)| {
                let mut bytes = name.bytes();
                bytes
                    .next()
                    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                    && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    && line_value(value)
            })
        }
        Value::Object(sections) => sections.iter().all(|(section, values)| {
            key(section)
                && values.as_object().is_some_and(|values| {
                    values
                        .iter()
                        .all(|(name, value)| key(name) && line_value(value))
                })
        }),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(document_error(
            "encode",
            &destination.path,
            "document is not losslessly representable in the selected text format",
        ))
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TemporaryFile {
    path: PathBuf,
    file: File,
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn atomic_replace(destination: &Source, ciphertext: &[u8]) -> crate::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = ciphertext;
        return Err(document_error(
            "write",
            &destination.path,
            "private atomic ciphertext writes require Unix",
        ));
    }

    #[cfg(unix)]
    {
        let parent = destination
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let mut temporary = None;
        for _ in 0..128 {
            let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".safix-ciphertext-{}-{serial}.tmp",
                std::process::id()
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => {
                    temporary = Some(TemporaryFile { path, file });
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(_) => {
                    return Err(document_error(
                        "write",
                        &destination.path,
                        "could not create private ciphertext staging file",
                    ));
                }
            }
        }
        let mut temporary = temporary.ok_or_else(|| {
            document_error(
                "write",
                &destination.path,
                "could not allocate ciphertext staging file",
            )
        })?;
        temporary
            .file
            .write_all(ciphertext)
            .and_then(|()| temporary.file.sync_all())
            .map_err(|_| {
                document_error("write", &destination.path, "could not persist ciphertext")
            })?;
        destination_exists(destination)?;
        fs::rename(&temporary.path, &destination.path).map_err(|_| {
            document_error("write", &destination.path, "could not replace ciphertext")
        })?;
        Ok(())
    }
}

/// Update a keyed candidate without changing its recipients or unrelated
/// ciphertext. A false result asks the caller to encrypt the document afresh.
pub(crate) fn set_value(
    destination: &Source,
    value: &Secret,
    identities: &Identities,
) -> crate::Result<bool> {
    check_key(destination, "update")?;
    let public = Zeroizing::new(std::fs::read_to_string(&destination.path).map_err(|_| {
        document_error(
            "update",
            &destination.path,
            "cannot read existing ciphertext",
        )
    })?);
    let Some(index) = crate::sops::document::encrypted_update_index(&public, &destination.key)?
    else {
        return Ok(false);
    };
    let encoded = encode_value(destination, value)?;
    if matches!(destination.format, Format::Ini | Format::Dotenv) {
        let mut replacement = parse_document(encoded.as_slice(), destination)?;
        let mut shape = PrivateJson(Value::Object(Map::new()));
        let components: Vec<_> = destination.key.split('/').collect();
        insert_value(&mut shape.0, &components, &mut replacement.0).map_err(|()| {
            document_error("update", &destination.path, "invalid text document key")
        })?;
        check_text_document(&shape.0, destination)?;
    }
    if !identities.inherit_environment {
        require_local_recipients(destination)?;
    }
    let attempts = identities
        .age_ssh_key_paths
        .iter()
        .map(Some)
        .chain(std::iter::once(None).filter(|_| identities.age_ssh_key_paths.is_empty()));
    let mut refusal = None;
    for ssh in attempts {
        let mut command = sops_command(identities);
        if let Some(path) = ssh {
            command.env("SOPS_AGE_SSH_PRIVATE_KEY_FILE", path);
        }
        command
            .args(["set", "--value-stdin", "--idempotent", "--input-type"])
            .arg(destination.format.as_str())
            .arg("--output-type")
            .arg(destination.format.as_str())
            .arg(&destination.path)
            .arg(&index);
        match filter(
            &mut command,
            Some(encoded.as_slice()),
            destination,
            "update",
            None,
        ) {
            Ok(_) => return Ok(true),
            Err(error) => refusal = Some(error),
        }
    }
    Err(refusal.unwrap_or_else(|| {
        document_error("update", &destination.path, "no identity source configured")
    }))
}

/// Encrypt a candidate to the supplied recipients. The caller must verify its
/// plaintext before publishing it; structured formats may canonicalize input.
///
/// # Errors
///
/// Refuses unrepresentable values, invalid recipients and failed cryptographic or filesystem operations.
pub(crate) fn write(
    destination: &Source,
    value: &Secret,
    recipients: &[String],
    identities: &Identities,
) -> crate::Result<()> {
    check_key(destination, "write")?;
    let (age, pgp) = recipient_sets(destination, recipients)?;
    let exists = destination_exists(destination)?;
    let plaintext = prepare_plaintext(destination, value, identities, exists)?;

    let mut command = if destination.format == Format::Age {
        let mut command = age_command(identities, false);
        command.arg("--encrypt");
        for recipient in &age {
            command.arg("--recipient").arg(recipient);
        }
        command.arg("-");
        command
    } else {
        let mut command = sops_command(identities);
        // Explicit recipient arguments and an empty configuration isolate writes
        // from repository creation rules and ambient recipient environment.
        for name in [
            "SOPS_AGE_RECIPIENTS",
            "SOPS_PGP_FP",
            "SOPS_KMS_ARN",
            "SOPS_KMS_CONTEXT",
            "SOPS_GCP_KMS_IDS",
            "SOPS_AZURE_KEYVAULT_URLS",
            "SOPS_HC_VAULT_TRANSIT_URI",
        ] {
            command.env_remove(name);
        }
        command
            .arg("--encrypt")
            .arg("--config")
            .arg("/dev/null")
            .arg("--input-type")
            .arg(if destination.key.is_empty() {
                destination.format.as_str()
            } else {
                "json"
            })
            .arg("--output-type")
            .arg(destination.format.as_str());
        if !age.is_empty() {
            command.arg("--age").arg(age.join(","));
        }
        if !pgp.is_empty() {
            command.arg("--pgp").arg(pgp.join(","));
        }
        // Encrypt every leaf, including storage envelopes and keys whose names
        // would otherwise match SOPS' default unencrypted suffix.
        command.arg("--encrypted-regex").arg(".*");
        command.arg("/dev/stdin");
        command
    };

    let ciphertext = filter(
        &mut command,
        Some(plaintext.as_slice()),
        destination,
        "encrypt",
        None,
    )?;
    atomic_replace(destination, ciphertext.as_slice())
}
