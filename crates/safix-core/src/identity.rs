//! Protected local identities and independently verified encrypted recovery.

use std::{
    collections::BTreeSet,
    env,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsFd,
        unix::{
            ffi::OsStrExt,
            fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::{Error, Progress, Result, Secret, ciphertext};

const MAX_PACKAGE: u64 = 64 * 1024 * 1024;

fn refused(reason: &str) -> Error {
    Error::KeyManagement {
        reason: reason.to_owned(),
    }
}

fn io_error(_: std::io::Error) -> Error {
    refused("Private identity operation failed")
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(refused("Identity paths must be absolute and normalized"));
    }

    if path.starts_with("/nix/store")
        || path
            .ancestors()
            .any(|parent| parent.join(".git").exists() || parent.join(".jj").exists())
    {
        return Err(refused(
            "Private identity paths cannot be inside the working tree or Nix store",
        ));
    }

    let mut prefix = PathBuf::new();
    for component in path.components() {
        prefix.push(component.as_os_str());
        match fs::symlink_metadata(&prefix) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(refused("Symlinks are not permitted in identity paths"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(e)),
        }
    }
    Ok(path.to_owned())
}

fn private_directory(path: &Path, create: bool) -> Result<PathBuf> {
    let path = absolute(path)?;
    if create && !path.try_exists().map_err(io_error)? {
        let parent = path
            .parent()
            .ok_or_else(|| refused("Missing private directory parent"))?;
        if !parent.try_exists().map_err(io_error)? {
            private_directory(parent, true)?;
        }
        match DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_error(e)),
        }
    }

    let meta = fs::symlink_metadata(&path).map_err(io_error)?;
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o7777 != 0o700
    {
        return Err(refused(
            "Private directory must be owned by the effective user with mode 0700",
        ));
    }
    Ok(path)
}

fn config_path(variable: &str, base: &str, fallback: &str, suffix: &str) -> Result<PathBuf> {
    if let Some(value) = env::var_os(variable) {
        return absolute(Path::new(&value));
    }
    let root = if let Some(value) = env::var_os(base) {
        PathBuf::from(value)
    } else {
        PathBuf::from(env::var_os("HOME").ok_or_else(|| refused("HOME is unavailable"))?)
            .join(fallback)
    };
    absolute(&root.join(suffix))
}

fn age_identity_path() -> Result<PathBuf> {
    config_path(
        "SAFIX_AGE_KEY_FILE",
        "XDG_CONFIG_HOME",
        ".config",
        "sops/age/keys.txt",
    )
}

/// Resolve and protect safix's `GnuPG` home.
///
/// # Errors
///
/// Refuses repository/store paths, links and insecure directory ownership or modes.
pub fn managed_gnupg_home() -> Result<PathBuf> {
    private_directory(
        &config_path(
            "SAFIX_GNUPGHOME",
            "XDG_DATA_HOME",
            ".local/share",
            "safix/gnupg",
        )?,
        true,
    )
}

fn private_file(path: &Path) -> Result<File> {
    absolute(path)?;
    private_directory(
        path.parent()
            .ok_or_else(|| refused("Missing identity parent"))?,
        false,
    )?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(i32::from_ne_bytes(
            rustix::fs::OFlags::NOFOLLOW.bits().to_ne_bytes(),
        ))
        .open(path)
        .map_err(io_error)?;
    validate_private_file(&file)?;
    Ok(file)
}

fn validate_private_file(file: &File) -> Result<()> {
    let meta = file.metadata().map_err(io_error)?;
    if !meta.is_file()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o7777 != 0o600
        || meta.nlink() != 1
        || meta.len() > MAX_PACKAGE
    {
        return Err(refused(
            "Identity file must be a private, singly linked, user-owned mode 0600 file",
        ));
    }
    Ok(())
}

fn read_private(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
    let value = Secret::read_from(&mut private_file(path)?.take(MAX_PACKAGE.saturating_add(1)))?;
    if value.len() as u64 > MAX_PACKAGE {
        return Err(refused("Identity exceeds the supported size"));
    }
    secret_bytes(&value)
}

fn custody_lock(directory: &Path) -> Result<File> {
    private_directory(directory, false)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(i32::from_ne_bytes(
            rustix::fs::OFlags::NOFOLLOW.bits().to_ne_bytes(),
        ))
        .open(directory.join(".safix-custody.lock"))
        .map_err(io_error)?;
    validate_private_file(&file)?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
        .map_err(|_| refused("Could not lock private identity custody"))?;
    Ok(file)
}

pub(crate) fn append_age_identity(path: &Path, addition: &[u8]) -> Result<()> {
    validate_age_location(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| refused("Missing age identity parent"))?;
    let _lock = custody_lock(parent)?;
    let previous = match fs::symlink_metadata(path) {
        Ok(_) => Some(read_private(path)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(io_error(error)),
    };
    let old_length = previous.as_ref().map_or(0, |bytes| bytes.len());
    if old_length
        .checked_add(addition.len())
        .and_then(|size| size.checked_add(1))
        .is_none_or(|size| size as u64 > MAX_PACKAGE)
    {
        return Err(refused("Identity file exceeds the supported size"));
    }
    let scratch = Scratch::new(parent)?;
    let candidate = scratch.0.join("identity");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&candidate)
        .map_err(io_error)?;
    if let Some(previous) = &previous {
        file.write_all(previous).map_err(io_error)?;
        if !previous.is_empty() {
            file.write_all(b"\n").map_err(io_error)?;
        }
    }
    file.write_all(addition)
        .and_then(|()| file.sync_all())
        .map_err(io_error)?;
    if previous.is_some() {
        fs::rename(&candidate, path).map_err(io_error)?;
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(io_error)
    } else {
        publish(&candidate, path)
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(parent: &Path) -> Result<Self> {
        private_directory(parent, false)?;
        Self::create(parent)
    }

    // A staging directory this code makes is also a GnuPG home, and GnuPG binds
    // its agent sockets inside a home whenever there is no per-user runtime
    // directory to redirect them into — which is every system without
    // `/run/user/<uid>`, the build sandbox among them. `sun_path` is 108 bytes,
    // so a home deeper than 83 bytes has no agent and every private-key
    // operation in it fails; the name is therefore as short as a collision-free
    // name can be rather than as long as it reads well. Eight random bytes name
    // it: the parent is owner-only and verified so, so the name is a collision
    // guard rather than a secret, and a taken name is retried.
    fn create(parent: &Path) -> Result<Self> {
        for _ in 0..32 {
            let mut random = [0_u8; 8];
            File::open("/dev/urandom")
                .and_then(|mut f| f.read_exact(&mut random))
                .map_err(io_error)?;
            let mut name = String::with_capacity(16);
            for byte in random {
                std::fmt::Write::write_fmt(&mut name, format_args!("{byte:02x}"))
                    .map_err(|_| refused("Cannot encode a private staging name"))?;
            }
            let path = parent.join(format!(".safix-{name}"));
            match DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(io_error(e)),
            }
        }
        Err(refused("Could not allocate private staging directory"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.join("private-keys-v1.d").is_dir() {
            let configured = env::var_os("SAFIX_GPGCONF").map(PathBuf::from);
            let sibling = env::var_os("SAFIX_GPG")
                .map(PathBuf::from)
                .filter(|path| path.file_name().is_some_and(|name| name == "gpg"))
                .and_then(|path| path.parent().map(|parent| parent.join("gpgconf")));
            let program = configured
                .or(sibling)
                .unwrap_or_else(|| PathBuf::from("gpgconf"));
            let _ = Command::new(program)
                .arg("--homedir")
                .arg(&self.0)
                .args(["--kill", "gpg-agent"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn validate_age_location(path: &Path) -> Result<()> {
    absolute(path)?;
    private_directory(
        path.parent()
            .ok_or_else(|| refused("Missing age identity directory"))?,
        true,
    )?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            private_file(path)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error(error)),
    }
    Ok(())
}

/// Generate a native age identity into the protected local identity file.
///
/// Existing identities are retained. Only the public recipient is returned.
///
/// # Errors
///
/// Refuses unsafe custody paths and unsuccessful upstream key generation.
pub fn generate_age() -> Result<String> {
    let path = age_identity_path()?;
    validate_age_location(&path)?;
    let command = Command::new(crate::keygen::keygen_binary());
    let private = execute(command, None)?;
    let recipients = age_recipients(&private)?;
    let [recipient]: [String; 1] = recipients
        .try_into()
        .map_err(|_| refused("Age key generation returned no unique recipient"))?;
    append_age_identity(&path, &private)?;
    Ok(recipient)
}

/// Preserve the caller's terminal while cryptographic children consume piped input.
pub(crate) fn configure_pinentry(command: &mut Command) {
    if env::var_os("GPG_TTY").is_none_or(|value| value.is_empty()) {
        let stdin = std::io::stdin();
        let stderr = std::io::stderr();
        let stdout = std::io::stdout();
        for descriptor in [stdin.as_fd(), stderr.as_fd(), stdout.as_fd()] {
            if let Ok(terminal) = rustix::termios::ttyname(descriptor, Vec::new()) {
                command.env("GPG_TTY", std::ffi::OsStr::from_bytes(terminal.as_bytes()));
                break;
            }
        }
    }
}

fn gpg(home: &Path) -> Command {
    let mut command = Command::new(env::var_os("SAFIX_GPG").unwrap_or_else(|| "gpg".into()));
    configure_pinentry(&mut command);
    command
        .arg("--no-options")
        .arg("--homedir")
        .arg(home)
        .arg("--no-tty");
    command
}

fn execute(mut command: Command, input: Option<&[u8]>) -> Result<Zeroizing<Vec<u8>>> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|_| refused("Identity cryptographic tool could not start"))?;
    let (value, written) = std::thread::scope(|scope| {
        let writer = input.map(|bytes| {
            let stdin = child.stdin.take();
            scope.spawn(move || match stdin {
                Some(mut stdin) => stdin.write_all(bytes),
                None => Err(std::io::Error::other("Missing tool input")),
            })
        });
        let value = match child.stdout.take() {
            Some(stdout) => Secret::read_from(&mut stdout.take(MAX_PACKAGE.saturating_add(1)))
                .and_then(|value| {
                    if value.len() as u64 > MAX_PACKAGE {
                        Err(refused("Cryptographic output exceeds the supported size"))
                    } else {
                        Ok(value)
                    }
                }),
            None => Err(refused("Missing cryptographic tool output")),
        };
        if value.is_err() {
            let _ = child.kill();
        }
        (value, writer.map(std::thread::ScopedJoinHandle::join))
    });
    let status = child
        .wait()
        .map_err(|_| refused("Identity cryptographic tool failed"))?;
    if !status.success() || matches!(written, Some(Err(_) | Ok(Err(_)))) {
        return Err(refused(
            "Identity cryptographic tool rejected the operation",
        ));
    }
    secret_bytes(&value?)
}

fn fingerprint(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

#[derive(Default)]
struct Profile {
    primary: String,
    algorithm: String,
    capabilities: String,
    local: bool,
    subkeys: Vec<Subkey>,
    grips: Vec<String>,
}

struct Subkey {
    fingerprint: String,
    algorithm: String,
    capabilities: String,
    local: bool,
}

fn profiles(home: &Path) -> Result<Vec<Profile>> {
    private_directory(home, false)?;
    let mut command = gpg(home);
    command.args([
        "--batch",
        "--with-colons",
        "--with-secret",
        "--with-keygrip",
        "--fixed-list-mode",
        "--fingerprint",
        "--fingerprint",
        "--list-secret-keys",
    ]);
    let output = execute(command, None)?;
    let text = std::str::from_utf8(&output)
        .map_err(|_| refused("GnuPG returned invalid public metadata"))?;
    parse_profiles(text)
}

fn parse_profiles(text: &str) -> Result<Vec<Profile>> {
    let mut result = Vec::<Profile>::new();
    let mut need_primary = false;
    for line in text.lines() {
        let Some((kind, rest)) = line.split_once(':') else {
            continue;
        };
        let mut fields = rest.split(':');
        match kind {
            "sec" | "ssb" => {
                let algorithm = fields
                    .nth(2)
                    .ok_or_else(|| refused("Invalid GnuPG key listing"))?;
                let capabilities = fields
                    .nth(7)
                    .ok_or_else(|| refused("Invalid GnuPG key listing"))?;
                let local = fields.nth(2) == Some("+");
                if kind == "sec" {
                    result.push(Profile {
                        algorithm: algorithm.into(),
                        capabilities: capabilities.into(),
                        local,
                        ..Profile::default()
                    });
                    need_primary = true;
                } else {
                    result
                        .last_mut()
                        .ok_or_else(|| refused("Invalid GnuPG subkey listing"))?
                        .subkeys
                        .push(Subkey {
                            fingerprint: String::new(),
                            algorithm: algorithm.into(),
                            capabilities: capabilities.into(),
                            local,
                        });
                    need_primary = false;
                }
            }
            "fpr" => {
                let fingerprint_text = fields
                    .nth(8)
                    .filter(|value| fingerprint(value))
                    .ok_or_else(|| refused("GnuPG returned an invalid full fingerprint"))?;
                let profile = result
                    .last_mut()
                    .ok_or_else(|| refused("Invalid GnuPG fingerprint listing"))?;
                let target = if need_primary {
                    &mut profile.primary
                } else {
                    &mut profile
                        .subkeys
                        .last_mut()
                        .ok_or_else(|| refused("Invalid GnuPG subkey fingerprint"))?
                        .fingerprint
                };
                if !target.is_empty() {
                    return Err(refused("GnuPG returned a duplicate fingerprint record"));
                }
                target.push_str(fingerprint_text);
                need_primary = false;
            }
            "grp" => {
                let grip = fields
                    .nth(8)
                    .filter(|value| value.len() == 40 && fingerprint(value))
                    .ok_or_else(|| refused("GnuPG returned an invalid keygrip"))?;
                result
                    .last_mut()
                    .ok_or_else(|| refused("Invalid GnuPG keygrip listing"))?
                    .grips
                    .push(grip.into());
            }
            _ => {}
        }
    }
    if result.iter().any(|profile| {
        profile.primary.is_empty()
            || profile.subkeys.len().checked_add(1) != Some(profile.grips.len())
            || profile
                .subkeys
                .iter()
                .any(|subkey| subkey.fingerprint.is_empty())
    }) {
        return Err(refused("GnuPG omitted a key fingerprint or keygrip"));
    }
    Ok(result)
}

fn available_fingerprints(profiles: &[Profile]) -> BTreeSet<&str> {
    profiles
        .iter()
        .flat_map(|profile| {
            std::iter::once((profile.primary.as_str(), profile.local)).chain(
                profile
                    .subkeys
                    .iter()
                    .map(|key| (key.fingerprint.as_str(), key.local)),
            )
        })
        .filter_map(|(fingerprint, local)| local.then_some(fingerprint))
        .collect()
}

fn complete_fingerprints(profiles: &[Profile]) -> Result<BTreeSet<&str>> {
    if profiles.is_empty()
        || profiles
            .iter()
            .any(|profile| !profile.local || profile.subkeys.iter().any(|key| !key.local))
    {
        return Err(refused(
            "Self-contained recovery requires every private key locally available; card and unavailable-key stubs cannot be backed up",
        ));
    }
    Ok(available_fingerprints(profiles))
}

fn private_keygrips(home: &Path) -> Result<BTreeSet<String>> {
    let directory = home.join("private-keys-v1.d");
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(io_error(error)),
        Ok(_) => {}
    }
    private_directory(&directory, false)?;
    let mut grips = BTreeSet::new();
    for entry in fs::read_dir(&directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry.file_name();
        let grip = name
            .to_str()
            .and_then(|name| name.strip_suffix(".key"))
            .filter(|grip| grip.len() == 40 && fingerprint(grip))
            .ok_or_else(|| refused("Unrecognized file in the GnuPG private-key directory"))?;
        drop(private_file(&entry.path())?);
        grips.insert(grip.to_owned());
    }
    Ok(grips)
}

fn export(home: &Path, profiles: &[Profile]) -> Result<Zeroizing<Vec<u8>>> {
    let expected = complete_fingerprints(profiles)?;
    let expected_grips = profiles
        .iter()
        .flat_map(|profile| profile.grips.iter().cloned())
        .collect();
    if private_keygrips(home)? != expected_grips {
        return Err(refused(
            "GnuPG backup cannot account for every private component; restore its public certificate before backing up",
        ));
    }
    let mut command = gpg(home);
    command.args(["--export-secret-keys", "--"]);
    for profile in profiles {
        command.arg(&profile.primary);
    }
    let bytes = execute(command, None)?;
    if bytes.is_empty() {
        return Err(refused("GnuPG did not export private identities"));
    }
    let verification = Scratch::new(home)?;
    import(&verification.0, &bytes)?;
    let recovered = self::profiles(&verification.0)?;
    if complete_fingerprints(&recovered)? != expected {
        return Err(refused(
            "Exported OpenPGP material does not recover every primary and subkey",
        ));
    }
    Ok(bytes)
}

fn import(home: &Path, bytes: &[u8]) -> Result<()> {
    let mut command = gpg(home);
    command.args(["--batch", "--import"]);
    execute(command, Some(bytes))?;
    Ok(())
}

fn import_new(staged: &Path, managed: &Path, bytes: &[u8]) -> Result<()> {
    let _lock = custody_lock(managed)?;
    let incoming = profiles(staged)?;
    let expected = complete_fingerprints(&incoming)?;
    let existing = profiles(managed)?;
    let existing_primaries: BTreeSet<_> = existing.iter().map(|profile| &profile.primary).collect();
    let existing_grips = private_keygrips(managed)?;
    if incoming.is_empty()
        || incoming.iter().any(|new| {
            existing_primaries.contains(&new.primary)
                || new.grips.iter().any(|grip| existing_grips.contains(grip))
        })
    {
        return Err(refused(
            "Import refused because an incoming private identity already exists",
        ));
    }
    import(managed, bytes)?;
    let after = profiles(managed)?;
    if !expected.is_subset(&available_fingerprints(&after)) {
        return Err(refused(
            "GnuPG import did not retain every private primary and subkey",
        ));
    }
    Ok(())
}

/// Generate an expiring Ed25519 certification key and cv25519 encryption subkey.
///
/// # Errors
///
/// Refuses invalid identity labels, unsupported expiration and incomplete key generation.
pub fn generate_pgp(uid: &str, expires: &str, progress: &dyn Progress) -> Result<()> {
    let valid_expiry = expires
        .strip_suffix(['d', 'w', 'm', 'y'])
        .filter(|number| number.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|number| number.parse::<u32>().ok())
        .is_some_and(|number| number > 0);
    if !valid_expiry {
        return Err(refused(
            "Expiration must be a positive number followed by d, w, m, or y",
        ));
    }
    if uid.trim().is_empty()
        || uid.len() > 1024
        || uid.starts_with('-')
        || uid.chars().any(char::is_control)
    {
        return Err(refused("Invalid OpenPGP user identity"));
    }

    let managed = managed_gnupg_home()?;
    let scratch = Scratch::new(&managed)?;
    let mut command = gpg(&scratch.0);
    command.args(["--quick-generate-key", uid, "ed25519", "cert", expires]);
    execute(command, None)?;

    let generated = profiles(&scratch.0)?;
    let [generated] = generated.as_slice() else {
        return Err(refused(
            "Generation did not produce exactly one primary key",
        ));
    };
    let primary = &generated.primary;
    let mut command = gpg(&scratch.0);
    command.args(["--quick-add-key", primary, "cv25519", "encr", expires]);
    execute(command, None)?;

    let complete = profiles(&scratch.0)?;
    let [profile] = complete.as_slice() else {
        return Err(refused("Generated OpenPGP profile is incomplete"));
    };
    let [encryption] = profile.subkeys.as_slice() else {
        return Err(refused("Generated OpenPGP encryption subkey is missing"));
    };
    if profile.primary != *primary
        || profile.algorithm != "22"
        || !profile.capabilities.contains('c')
        || profile.capabilities.contains('s')
        || encryption.algorithm != "18"
        || !encryption.capabilities.contains('e')
    {
        return Err(refused("Generated OpenPGP profile is incomplete"));
    }

    let private = export(&scratch.0, &complete)?;
    import_new(&scratch.0, &managed, &private)?;
    progress.write(&format!("Public recipient: pgp:{}\nManaged GnuPG home: {}\nDeclare this public recipient in flake.safix.users.<name>.recipient; back up to an independent recovery identity.\n", profile.primary, managed.display()));
    Ok(())
}

fn age_recipients(bytes: &[u8]) -> Result<Vec<String>> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| refused("Native age identity file is not UTF-8"))?;
    let mut recipients = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with("AGE-SECRET-KEY-1") {
            return Err(refused("Only native age private identities are supported"));
        }
        let mut command = Command::new(crate::keygen::keygen_binary());
        command.arg("-y");
        let mut input = Zeroizing::new(line.as_bytes().to_vec());
        input.push(b'\n');
        let output = execute(command, Some(&input))?;
        let recipient = std::str::from_utf8(&output)
            .map_err(|_| refused("age-keygen returned invalid public metadata"))?
            .trim();
        if !recipient.starts_with("age1") || recipient.chars().any(char::is_whitespace) {
            return Err(refused("age-keygen returned an invalid recipient"));
        }
        recipients.insert(recipient.to_owned());
    }
    if recipients.is_empty() {
        return Err(refused("No native age identities were found"));
    }
    Ok(recipients.into_iter().collect())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Package {
    version: u32,
    kind: String,
    private: Vec<u8>,
    public_recipients: Vec<String>,
}

impl Drop for Package {
    fn drop(&mut self) {
        self.private.zeroize();
    }
}

fn secret_bytes(secret: &Secret) -> Result<Zeroizing<Vec<u8>>> {
    let mut bytes = Zeroizing::new(Vec::new());
    secret.write_to(&mut *bytes).map_err(io_error)?;
    Ok(bytes)
}

fn binary(path: &Path) -> ciphertext::Source {
    ciphertext::Source {
        path: path.to_owned(),
        format: ciphertext::Format::Binary,
        key: String::new(),
    }
}

fn normalize_recipient(value: &str) -> Result<String> {
    if let Some(value) = value.strip_prefix("pgp:") {
        let value = value.to_ascii_uppercase();
        if fingerprint(&value) {
            return Ok(format!("pgp:{value}"));
        }
    } else if value.starts_with("age1") && !value.chars().any(char::is_whitespace) {
        return Ok(value.to_owned());
    }
    Err(refused(
        "Recovery requires native age recipients or full pgp fingerprints",
    ))
}

fn verify_separation(
    identities: &ciphertext::Identities,
    own: &BTreeSet<String>,
    own_grips: &BTreeSet<String>,
) -> Result<()> {
    if identities.inherit_environment || !identities.age_ssh_key_paths.is_empty() {
        return Err(refused(
            "Recovery verification requires explicit native identities without ambient keys",
        ));
    }
    let mut available = BTreeSet::new();
    if let Some(path) = &identities.age_key_file {
        available.extend(age_recipients(&read_private(path)?)?);
    }
    if let Some(home) = &identities.gnupg_home {
        private_directory(home, false)?;
        if !private_keygrips(home)?.is_disjoint(own_grips) {
            return Err(refused(
                "Recovery identities share a private component with the identities being backed up",
            ));
        }
        available.extend(
            profiles(home)?
                .into_iter()
                .map(|profile| format!("pgp:{}", profile.primary)),
        );
    }
    if available.is_empty() || !available.is_disjoint(own) {
        return Err(refused(
            "Verification identities must be explicit and independent of the backed-up identities",
        ));
    }
    Ok(())
}

fn unused_target(path: &Path) -> Result<PathBuf> {
    let path = absolute(path)?;
    private_directory(
        path.parent()
            .ok_or_else(|| refused("Missing destination parent"))?,
        false,
    )?;
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Ok(_) => Err(refused("Destination already exists")),
        Err(e) => Err(io_error(e)),
    }
}

fn encrypted_target(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path.starts_with("/nix/store")
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(refused(
            "Encrypted backup paths must be absolute, normalized and outside the Nix store",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| refused("Missing backup directory"))?;
    for directory in parent.ancestors() {
        if fs::symlink_metadata(directory)
            .map_err(io_error)?
            .file_type()
            .is_symlink()
        {
            return Err(refused(
                "Encrypted backup directories must not use symlinks",
            ));
        }
    }
    let metadata = fs::metadata(parent).map_err(io_error)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o022 != 0
    {
        return Err(refused(
            "Encrypted backup directory must be user-owned and not writable by others",
        ));
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_owned()),
        Ok(_) => Err(refused("Destination already exists")),
        Err(error) => Err(io_error(error)),
    }
}

fn publish(candidate: &Path, destination: &Path) -> Result<()> {
    OpenOptions::new()
        .read(true)
        .custom_flags(i32::from_ne_bytes(
            rustix::fs::OFlags::NOFOLLOW.bits().to_ne_bytes(),
        ))
        .open(candidate)
        .and_then(|file| file.sync_all())
        .map_err(io_error)?;
    fs::hard_link(candidate, destination)
        .map_err(|_| refused("No-clobber identity publication failed"))?;
    File::open(
        destination
            .parent()
            .ok_or_else(|| refused("Missing publication parent"))?,
    )
    .and_then(|f| f.sync_all())
    .map_err(io_error)?;
    Ok(())
}

/// Export identities into an encrypted backup verified by an independent identity.
///
/// # Errors
///
/// Refuses self-only recovery, existing destinations and failed independent decryption.
pub fn backup(
    kind: &str,
    destination: &Path,
    recovery_recipients: &[String],
    verification: &ciphertext::Identities,
    progress: &dyn Progress,
) -> Result<()> {
    let destination = encrypted_target(destination)?;
    let (mut private, public_recipients, own_grips) = match kind {
        "age" => {
            let bytes = read_private(&age_identity_path()?)?;
            let recipients = age_recipients(&bytes)?;
            (bytes, recipients, BTreeSet::new())
        }
        "pgp" => {
            let home = managed_gnupg_home()?;
            let listed = profiles(&home)?;
            let bytes = export(&home, &listed)?;
            let recipients = listed
                .iter()
                .map(|profile| format!("pgp:{}", profile.primary))
                .collect();
            let grips = listed
                .iter()
                .flat_map(|profile| profile.grips.iter().cloned())
                .collect();
            (bytes, recipients, grips)
        }
        _ => return Err(refused("Identity kind must be age or pgp")),
    };
    let own: BTreeSet<_> = public_recipients.iter().cloned().collect();
    let recipients = recovery_recipients
        .iter()
        .map(|s| normalize_recipient(s))
        .collect::<Result<Vec<_>>>()?;
    if !recipients.iter().any(|r| !own.contains(r)) {
        return Err(refused(
            "At least one independent recovery recipient is required",
        ));
    }
    verify_separation(verification, &own, &own_grips)?;

    let package = Package {
        version: 1,
        kind: kind.to_owned(),
        private: std::mem::take(&mut *private),
        public_recipients,
    };
    let encoded = ciphertext::encode_private_json(&package, MAX_PACKAGE)?;
    let scratch = Scratch::create(
        destination
            .parent()
            .ok_or_else(|| refused("Missing backup directory"))?,
    )?;
    let candidate = scratch.0.join("backup");
    ciphertext::write(&binary(&candidate), &encoded, &recipients, verification)
        .map_err(|_| refused("Backup encryption failed"))?;

    let decrypted = ciphertext::read_bounded(&binary(&candidate), verification, MAX_PACKAGE)
        .map_err(|_| refused("Independent backup verification failed"))?;
    if !decrypted.equals(&encoded) {
        return Err(refused("Independent backup verification did not match"));
    }

    // The staging directory is private; tighten a crypto tool's output mode
    // before requiring the publication invariant.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(i32::from_ne_bytes(
            rustix::fs::OFlags::NOFOLLOW.bits().to_ne_bytes(),
        ))
        .open(&candidate)
        .map_err(io_error)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(io_error)?;
    publish(&candidate, &destination)?;
    progress.write(&format!(
        "Verified encrypted backup: {}\nOriginal identities retained.\n",
        destination.display()
    ));
    Ok(())
}

/// Restore a verified backup without replacing an existing private identity.
///
/// # Errors
///
/// Refuses invalid packages, unsafe custody locations and identity collisions.
pub fn restore(
    source: &Path,
    identities: &ciphertext::Identities,
    progress: &dyn Progress,
) -> Result<()> {
    // Ciphertext may be public. Cap both its container and the decrypted package.
    if fs::metadata(source).map_err(io_error)?.len() > MAX_PACKAGE.saturating_mul(2) {
        return Err(refused("Encrypted backup exceeds the supported size"));
    }
    let decrypted = ciphertext::read_bounded(&binary(source), identities, MAX_PACKAGE)
        .map_err(|_| refused("Backup decryption failed or exceeded its size limit"))?;
    let encoded = secret_bytes(&decrypted)?;

    // Parsing to Value first permits wiping the parsed secret JSON on both
    // schema success and schema failure.
    let mut value: serde_json::Value = serde_json::from_slice(&encoded)
        .map_err(|_| refused("Backup package is not valid JSON"))?;
    let parsed = Package::deserialize(&value);
    ciphertext::wipe_json(&mut value);
    let package = parsed.map_err(|_| refused("Backup package schema is invalid"))?;
    if package.version != 1 || package.private.is_empty() {
        return Err(refused("Unsupported or empty backup package"));
    }

    let declared = package
        .public_recipients
        .iter()
        .map(|r| normalize_recipient(r))
        .collect::<Result<BTreeSet<_>>>()?;
    if declared.is_empty() || declared.len() != package.public_recipients.len() {
        return Err(refused("Backup recipient roster is invalid"));
    }

    match package.kind.as_str() {
        "age" => {
            let actual: BTreeSet<_> = age_recipients(&package.private)?.into_iter().collect();
            if actual != declared {
                return Err(refused("Backup age recipient roster does not match"));
            }
            let target = age_identity_path()?;
            private_directory(
                target
                    .parent()
                    .ok_or_else(|| refused("Missing age identity parent"))?,
                true,
            )?;
            let target = unused_target(&target)?;
            let scratch = Scratch::new(
                target
                    .parent()
                    .ok_or_else(|| refused("Missing age identity parent"))?,
            )?;
            let candidate = scratch.0.join("identity");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(i32::from_ne_bytes(
                    rustix::fs::OFlags::NOFOLLOW.bits().to_ne_bytes(),
                ))
                .open(&candidate)
                .map_err(io_error)?;
            file.write_all(&package.private).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
            publish(&candidate, &target)
        }
        "pgp" => {
            let managed = managed_gnupg_home()?;
            let scratch = Scratch::new(&managed)?;
            import(&scratch.0, &package.private)?;
            let listed = profiles(&scratch.0)?;
            if listed.is_empty() {
                return Err(refused("Backup contains no OpenPGP private identities"));
            }
            let actual: BTreeSet<_> = listed
                .iter()
                .map(|profile| format!("pgp:{}", profile.primary))
                .collect();
            if actual != declared {
                return Err(refused("Backup OpenPGP recipient roster does not match"));
            }
            // Re-export only enumerated secret profiles, rather than importing
            // an untrusted backup packet stream into the managed keyring.
            let private = export(&scratch.0, &listed)?;
            import_new(&scratch.0, &managed, &private)
        }
        _ => Err(refused("Unsupported backup identity kind")),
    }?;
    progress.write("Private identity recovery verified; existing identities were not replaced.\n");
    Ok(())
}
