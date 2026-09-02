//! Seed a machine's own host identity before its first activation.
//!
//! A machine's declared recipient (`flake.safix.machines.<m>.recipient`) is
//! the age form of an ed25519 host key the operator holds, wrapped into every
//! audience that names the machine as soon as `safix fix` runs — independent
//! of whether the machine has ever booted. Nothing safix ships mints that
//! key or gets its private half onto the disk of a machine that has not
//! booted yet, so its first activation can decrypt what was already wrapped
//! to it. This module closes that one gap.
//!
//! # The two write modes, and the one that writes nothing
//!
//! [`Target::Directory`] writes a pre-seed tree straight to an
//! operator-named directory and touches no network — for `nixos-anywhere
//! --extra-files` or for hand-copying onto installer media.
//!
//! [`Target::Remote`] is the only mode with a live target to ask: it probes
//! the ed25519 host key the target currently presents, unauthenticated,
//! before writing anything, and takes exactly one of three actions —
//! [`write_remote`] is where the branch lives. A target that already
//! presents the declared key gets an honest no-op rather than a transfer
//! that would either overwrite a live host's own identity or silently do
//! nothing while claiming success.
//!
//! # What this crate mints, and what it does not
//!
//! Nothing. `--identity PATH` names a private key the operator already
//! generated or harvested; this module reads it, never accepts it on its
//! own argument vector, and refuses before writing anything when its
//! derived age recipient does not equal the machine's declared one.
//!
//! # The transport
//!
//! [`Target::Remote`]'s write mirrors clan's own shape
//! (`clan_lib/ssh/upload.py`): root, tar-over-ssh, files at mode `0400` and
//! directories at `0700`, wiping the destination before extracting into it,
//! at the fixed `/mnt/etc/ssh` a fresh install mounts its target root at.
//! The tarball is built inside [`crate::staging::Staging`], the same
//! memory-backed, per-run root generation and editing already use, and
//! removed the same way — before it is streamed and again on every exit
//! path.
//!
//! # The subprocess surface
//!
//! Every external tool this module may spawn is named in [`Program`] and
//! reached through [`Program::command`], its one construction site: five
//! programs total, `ssh-to-age` on its own and `ssh-keygen`, `ssh-keyscan`,
//! `ssh` and `tar` under what the module's own tests call "the ssh
//! transport itself" — no rebuild, no switch, and no activation invocation
//! anywhere on this path.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::progress::{Progress, log, note};
use crate::secret::Secret;
use crate::staging::Staging;
use crate::workspace::Workspace;

/// `ssh_host_ed25519_key`'s own name, the path a fresh install's
/// `sshd-keygen` writes it at, under `etc/ssh/`.
const KEY_NAME: &str = "ssh_host_ed25519_key";

/// [`KEY_NAME`]'s public half.
const PUB_NAME: &str = "ssh_host_ed25519_key.pub";

/// The tarball's own name inside the staging root.
const TARBALL_NAME: &str = "upload.tar.gz";

/// The fixed destination a pre-seed tree is wiped and extracted into, over
/// ssh.
///
/// A fresh install mounts its target root at `/mnt` — the convention
/// `nixos-anywhere --extra-files` itself assumes — so the pre-seed tree
/// lands at `/mnt/etc/ssh`: three path components deep, clearing
/// [`destination_is_safe`]'s threshold by construction. The check travels
/// anyway, as defense in depth against a destination this module does not
/// currently make configurable.
const REMOTE_DESTINATION: &str = "/mnt/etc/ssh";

/// Which of the two write modes a run takes.
///
/// One of the two, decided by the command before [`run`] is called: this
/// module carries no "neither given" state of its own, because that is a
/// question about how the command was invoked rather than about what to
/// write.
#[derive(Debug, Clone)]
pub enum Target {
    /// `--directory DIR`: write straight to a tree, touching no network.
    Directory(PathBuf),
    /// `--to ADDRESS`: probe the target first, then no-op, write, or refuse.
    Remote(String),
}

/// What the operator supplied alongside the target.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The private key file named by `--identity`, if any.
    pub identity: Option<PathBuf>,
    /// Whether `--force` was given, which overrides only the presented-key
    /// mismatch branch of [`write_remote`].
    pub force: bool,
}

/// Seed `machine`'s host identity, locally or over ssh, per `target`.
///
/// # Errors
///
/// [`Error::UnknownMachine`] and [`Error::MachineHasNoRecipient`] for a
/// target this crate cannot resolve to a recipient; [`Error::UploadNeedsIdentity`]
/// when a write has no `--identity` to write; [`Error::SuppliedIdentityMismatch`]
/// when the supplied key does not derive to the declared recipient;
/// [`Error::PresentedIdentityMismatch`] when a remote target presents a
/// different key and `--force` was not given; and whatever the external
/// tools this module drives return.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    machine: &str,
    target: &Target,
    options: &Options,
) -> Result<()> {
    let declared = resolve_machine(workspace, machine)?;
    match target {
        Target::Directory(directory) => {
            let identity = options.identity.as_deref().ok_or(Error::UploadNeedsIdentity)?;
            write_directory(directory, identity, machine, &declared)?;
            log(
                progress,
                &format!(
                    "safix: {machine}'s host identity was written under {}. \
                    {machine}'s own next rebuild is what activates it.",
                    directory.display()
                ),
            );
            Ok(())
        }
        Target::Remote(address) => write_remote(
            progress,
            address,
            options.identity.as_deref(),
            options.force,
            machine,
            &declared,
        ),
    }
}

/// The declared recipient for a machine name, refusing anything else.
///
/// Looked up in [`crate::model::Subjects::machines`] alone, never in
/// placements or recipients directly: an undeclared name and a declared
/// person's name both fail that lookup, and this verb carries no separate
/// code path to tell the two apart — the refusal is already the right shape
/// for both.
///
/// # Errors
///
/// [`Error::UnknownMachine`] when `machine` names nothing declared as a
/// machine, and [`Error::MachineHasNoRecipient`] when it is declared and
/// names no recipient.
fn resolve_machine(workspace: &Workspace, machine: &str) -> Result<String> {
    let subjects = workspace.subjects()?;
    if !subjects.machines.contains_key(machine) {
        return Err(Error::UnknownMachine {
            machine: machine.to_owned(),
            declared: subjects.machines.keys().cloned().collect(),
        });
    }
    let recipients = workspace.recipients()?;
    match recipients.0.get(machine).and_then(|keys| keys.first()) {
        Some(recipient) => Ok(recipient.clone()),
        None => Err(Error::MachineHasNoRecipient {
            machine: machine.to_owned(),
        }),
    }
}

/// `--directory DIR`: write straight to a tree, touching no network.
///
/// No [`Host`] is constructed on this path — the function takes no address
/// and reaches no network-carrying subprocess — which is what makes task
/// 2.5's claim a property of this function's own signature rather than of a
/// runtime check.
///
/// # Errors
///
/// [`Error::SuppliedIdentityMismatch`], and whatever deriving or writing the
/// two files returns.
fn write_directory(directory: &Path, identity: &Path, machine: &str, declared: &str) -> Result<()> {
    let public = public_half(identity)?;
    let recipient = age_recipient(&public)?;
    if recipient != declared {
        return Err(Error::SuppliedIdentityMismatch {
            machine: machine.to_owned(),
            path: identity.display().to_string(),
            declared: declared.to_owned(),
            supplied: recipient,
        });
    }

    let target_dir = directory.join("etc").join("ssh");
    std::fs::create_dir_all(&target_dir).map_err(|cause| Error::FileUnwritable {
        path: target_dir.display().to_string(),
        cause,
    })?;

    let private = read_identity(identity)?;
    write_secret(&target_dir.join(KEY_NAME), &private, 0o600)?;
    write_plain(&target_dir.join(PUB_NAME), format!("{public}\n").as_bytes(), 0o644)?;
    Ok(())
}

/// `--to ADDRESS`: probe, then no-op, write, or refuse.
///
/// The three-way branch D5 describes: a match reports and writes nothing,
/// regardless of `--force`; an absent key requires `--identity` and
/// proceeds; a mismatched key refuses by default and proceeds only with
/// `--force` and a matching `--identity` together.
///
/// # Errors
///
/// [`Error::PresentedIdentityMismatch`], [`Error::UploadNeedsIdentity`],
/// [`Error::SuppliedIdentityMismatch`], and whatever probing or writing
/// returns.
fn write_remote(
    progress: &dyn Progress,
    address: &str,
    identity: Option<&Path>,
    force: bool,
    machine: &str,
    declared: &str,
) -> Result<()> {
    let host = Host::new(address);
    match host.probe()? {
        Some(presented) if presented == declared => {
            log(
                progress,
                &format!(
                    "safix: {machine} already holds its declared identity ({declared}); \
                    nothing was written."
                ),
            );
            if force {
                note(
                    progress,
                    "--force applies only to a mismatch; a match has nothing for it to override.",
                );
            }
            Ok(())
        }
        Some(presented) => {
            if !force {
                return Err(Error::PresentedIdentityMismatch {
                    machine: machine.to_owned(),
                    declared: declared.to_owned(),
                    presented,
                });
            }
            let identity = identity.ok_or(Error::UploadNeedsIdentity)?;
            write_transport(progress, &host, identity, machine, declared)?;
            note(
                progress,
                "a changed host key was overridden rather than discovered absent.",
            );
            Ok(())
        }
        None => {
            let identity = identity.ok_or(Error::UploadNeedsIdentity)?;
            write_transport(progress, &host, identity, machine, declared)
        }
    }
}

/// Build the tarball inside [`Staging`] and stream it to `host`.
///
/// # Errors
///
/// [`Error::SuppliedIdentityMismatch`], and whatever staging, archiving or
/// transporting returns.
fn write_transport(
    progress: &dyn Progress,
    host: &Host<'_>,
    identity: &Path,
    machine: &str,
    declared: &str,
) -> Result<()> {
    let public = public_half(identity)?;
    let recipient = age_recipient(&public)?;
    if recipient != declared {
        return Err(Error::SuppliedIdentityMismatch {
            machine: machine.to_owned(),
            path: identity.display().to_string(),
            declared: declared.to_owned(),
            supplied: recipient,
        });
    }

    // Not accepting disk-backed staging: this verb carries no acknowledgement
    // flag of its own, so the memory-backed rule holds unconditionally.
    let staging = Staging::establish(false)?;

    let private = read_identity(identity)?;
    let key_path = staging.write(Path::new(KEY_NAME), &private)?;
    // D2's mode, 0400: staging.write leaves 0600, which the local
    // --directory tree keeps; the tarball this builds carries the narrower
    // mode clan's own transport uses.
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o400)).map_err(
        |cause| Error::StagingUnusable {
            path: key_path.display().to_string(),
            cause,
        },
    )?;
    write_plain(
        &staging.root().join(PUB_NAME),
        format!("{public}\n").as_bytes(),
        0o400,
    )?;

    let tarball = staging.root().join(TARBALL_NAME);
    build_tarball(&tarball, staging.root())?;
    host.write(&tarball)?;

    log(
        progress,
        &format!(
            "safix: {machine}'s host identity was written. {machine}'s own next rebuild is \
            what activates it."
        ),
    );
    Ok(())
}

/// The private key at `path`, read as a [`Secret`] rather than a `String` —
/// it never reaches an argument vector or an environment variable, on
/// either write mode.
fn read_identity(path: &Path) -> Result<Secret> {
    let mut file = File::open(path).map_err(|cause| Error::FileUnreadable {
        path: path.display().to_string(),
        cause,
    })?;
    Secret::read_from(&mut file)
}

/// The operator-named remote target, and the two subprocess calls that reach
/// it: the probe (D5) and the wipe-then-extract write (D2).
///
/// Module-private, and constructed only by [`write_remote`] — nothing on
/// [`write_directory`]'s path builds one, which is what makes "no network
/// code path is reachable from `--directory`" a fact about this type's own
/// visibility rather than a runtime assertion.
struct Host<'a> {
    /// The address the operator named after `--to`.
    address: &'a str,
}

impl<'a> Host<'a> {
    /// A target at `address`, probed and written to through [`Program::SshKeyscan`],
    /// [`Program::Ssh`] and [`Program::Tar`] alone.
    const fn new(address: &'a str) -> Self {
        Self { address }
    }

    /// The age form of the ed25519 host key this target currently presents,
    /// or `None` when it presents none.
    ///
    /// # Errors
    ///
    /// [`Error::UploadToolUnavailable`] when `ssh-keyscan` cannot be run.
    fn probe(&self) -> Result<Option<String>> {
        let finished = spawn(Program::SshKeyscan, |command| {
            command
                .arg("-t")
                .arg("ed25519")
                .arg(self.address)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
        })?
        .wait_with_output()
        .map_err(|cause| Error::UploadToolUnavailable {
            program: Program::SshKeyscan.program_name(),
            cause,
        })?;

        let presented = String::from_utf8_lossy(&finished.stdout);
        let Some(public_key) = presented.lines().find_map(offered_key) else {
            return Ok(None);
        };
        age_recipient(&public_key).map(Some)
    }

    /// Stream `tarball` to this target and run the wipe-then-extract
    /// sequence at [`REMOTE_DESTINATION`], as root.
    ///
    /// Refuses `known_hosts` entirely rather than trusting it, because this
    /// run has already made its own trust decision in [`Host::probe`]: a
    /// target either matched, was absent, or was overridden with `--force`,
    /// and ssh's own TOFU prompt would ask the same question a second time
    /// with no operator present to answer it.
    ///
    /// # Errors
    ///
    /// [`Error::UploadDestinationUnsafe`], [`Error::UploadToolUnavailable`]
    /// and [`Error::UploadToolFailed`].
    fn write(&self, tarball: &Path) -> Result<()> {
        if !destination_is_safe(REMOTE_DESTINATION) {
            return Err(Error::UploadDestinationUnsafe {
                destination: REMOTE_DESTINATION.to_owned(),
            });
        }

        let file = File::open(tarball).map_err(|cause| Error::UploadToolUnavailable {
            program: Program::Ssh.program_name(),
            cause,
        })?;
        let script = format!(
            "install -d -m 0700 {REMOTE_DESTINATION} && \
            find {REMOTE_DESTINATION} -mindepth 1 -delete && tar -xzf - -C {REMOTE_DESTINATION}"
        );
        let finished = spawn(Program::Ssh, |command| {
            command
                .arg("-o")
                .arg("BatchMode=yes")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("UserKnownHostsFile=/dev/null")
                .arg(format!("root@{}", self.address))
                .arg(script)
                .stdin(Stdio::from(file))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        })?
        .wait_with_output()
        .map_err(|cause| Error::UploadToolUnavailable {
            program: Program::Ssh.program_name(),
            cause,
        })?;

        if finished.status.success() {
            Ok(())
        } else {
            Err(Error::UploadToolFailed {
                program: Program::Ssh.program_name(),
                output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
            })
        }
    }
}

/// One external tool this module may spawn, and how its program name is
/// overridden for a hermetic check.
///
/// The whole of this module's subprocess surface: five programs, reached
/// through [`Program::command`] alone — see that function's own doc and the
/// unit test that holds this file to spawning through no other site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Program {
    /// Derives an ed25519 public key from its private half, locally.
    SshKeygen,
    /// Converts an ed25519 public key to its age recipient form.
    SshToAge,
    /// Reads the ed25519 host key a target presents, unauthenticated.
    SshKeyscan,
    /// Carries the wipe-then-extract sequence to a target, over ssh.
    Ssh,
    /// Builds the local archive [`Host::write`] streams over [`Program::Ssh`].
    Tar,
}

impl Program {
    /// The binary this program names absent an override.
    const fn default_name(self) -> &'static str {
        match self {
            Self::SshKeygen => "ssh-keygen",
            Self::SshToAge => "ssh-to-age",
            Self::SshKeyscan => "ssh-keyscan",
            Self::Ssh => "ssh",
            Self::Tar => "tar",
        }
    }

    /// The environment variable that replaces this program, for checks.
    ///
    /// Mirrors [`crate::clan::PROGRAM_OVERRIDE`]: a hermetic check drives
    /// this module against a stub without a real network tool on `PATH`
    /// meaning something different from the one the package pins.
    const fn override_variable(self) -> &'static str {
        match self {
            Self::SshKeygen => "SAFIX_SSH_KEYGEN",
            Self::SshToAge => "SAFIX_SSH_TO_AGE",
            Self::SshKeyscan => "SAFIX_SSH_KEYSCAN",
            Self::Ssh => "SAFIX_SSH",
            Self::Tar => "SAFIX_TAR",
        }
    }

    /// The program name as a refusal names it.
    fn program_name(self) -> String {
        std::env::var(self.override_variable()).unwrap_or_else(|_| self.default_name().to_owned())
    }

    /// This module's one `Command::new` call site.
    fn command(self) -> Command {
        let program = std::env::var_os(self.override_variable())
            .unwrap_or_else(|| std::ffi::OsString::from(self.default_name()));
        Command::new(program)
    }
}

/// Build one [`Command`] for `program` and spawn it, translating a spawn
/// failure into [`Error::UploadToolUnavailable`].
fn spawn(program: Program, configure: impl FnOnce(&mut Command)) -> Result<std::process::Child> {
    let mut command = program.command();
    configure(&mut command);
    command.spawn().map_err(|cause| Error::UploadToolUnavailable {
        program: program.program_name(),
        cause,
    })
}

/// Run `program` to completion, capturing both streams.
///
/// # Errors
///
/// [`Error::UploadToolUnavailable`] when it cannot be run at all.
fn output(program: Program, configure: impl FnOnce(&mut Command)) -> Result<std::process::Output> {
    let mut command = program.command();
    configure(&mut command);
    command.output().map_err(|cause| Error::UploadToolUnavailable {
        program: program.program_name(),
        cause,
    })
}

/// The public half of the private key at `path`, in OpenSSH wire format.
///
/// # Errors
///
/// [`Error::UploadToolUnavailable`] when `ssh-keygen` cannot be run, and
/// [`Error::UploadToolFailed`] when it refuses — the shape a corrupted or
/// non-ed25519 identity file takes.
fn public_half(path: &Path) -> Result<String> {
    let finished = output(Program::SshKeygen, |command| {
        command
            .arg("-y")
            .arg("-f")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    })?;

    if !finished.status.success() {
        return Err(Error::UploadToolFailed {
            program: Program::SshKeygen.program_name(),
            output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
        });
    }
    Ok(String::from_utf8_lossy(&finished.stdout).trim_end().to_owned())
}

/// The age recipient an OpenSSH public key converts to, over `ssh-to-age`'s
/// own pipe — the conversion `keygen.rs`'s own help text documents for a
/// person's identity, reused here for a machine's.
///
/// # Errors
///
/// [`Error::UploadToolUnavailable`] when `ssh-to-age` cannot be run,
/// [`Error::UploadPipeMissing`] when it was started without the pipe its
/// input travels, and [`Error::UploadToolFailed`] when it refuses a key it
/// cannot parse.
fn age_recipient(public_key: &str) -> Result<String> {
    let mut child = spawn(Program::SshToAge, |command| {
        command
            .arg("-i")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    })?;

    {
        let mut stdin = child.stdin.take().ok_or(Error::UploadPipeMissing)?;
        stdin
            .write_all(public_key.as_bytes())
            .and_then(|()| writeln!(stdin))
            .map_err(|cause| Error::UploadToolUnavailable {
                program: Program::SshToAge.program_name(),
                cause,
            })?;
    }

    let finished = child
        .wait_with_output()
        .map_err(|cause| Error::UploadToolUnavailable {
            program: Program::SshToAge.program_name(),
            cause,
        })?;

    if !finished.status.success() {
        return Err(Error::UploadToolFailed {
            program: Program::SshToAge.program_name(),
            output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
        });
    }
    Ok(String::from_utf8_lossy(&finished.stdout).trim().to_owned())
}

/// One line of `ssh-keyscan`'s output, `<host> <keytype> <base64>...`, as
/// the `<keytype> <base64>` pair [`age_recipient`] converts — the host name
/// dropped, because an OpenSSH public key carries none of its own.
fn offered_key(line: &str) -> Option<String> {
    let mut words = line.split_whitespace();
    let _host = words.next()?;
    let keytype = words.next()?;
    let base64 = words.next()?;
    (keytype == "ssh-ed25519").then(|| format!("{keytype} {base64}"))
}

/// Build a gzip tarball of the two staged files at `destination`, stamping
/// root ownership into the archive's own metadata (D2) — this process need
/// not run as root for that: `--owner`/`--group` override what the archive
/// records, not what created it. Plain `tar` preserves the actual
/// filesystem modes of what it archives, so the `0400`/`0700` split is
/// already true on disk by the time this runs, from the chmod in
/// [`write_transport`] and [`Staging::directory`]'s own default.
///
/// # Errors
///
/// [`Error::UploadToolUnavailable`] when `tar` cannot be run, and
/// [`Error::UploadToolFailed`] when it refuses.
fn build_tarball(destination: &Path, staging_root: &Path) -> Result<()> {
    let finished = output(Program::Tar, |command| {
        command
            .arg("--owner=root")
            .arg("--group=root")
            .arg("--numeric-owner")
            .arg("-czf")
            .arg(destination)
            .arg("-C")
            .arg(staging_root)
            .arg(KEY_NAME)
            .arg(PUB_NAME)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
    })?;

    if finished.status.success() {
        Ok(())
    } else {
        Err(Error::UploadToolFailed {
            program: Program::Tar.program_name(),
            output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
        })
    }
}

/// clan's own path-depth safety (`clan_lib/ssh/upload.py:9-11,34-53`): a
/// destination the wipe clears must be at least three path components deep,
/// or two under `/tmp/`, `/root/` or `/etc/` — because the wipe is a
/// `find -mindepth 1 -delete` under it, and a shallow destination makes
/// that catastrophic.
fn destination_is_safe(destination: &str) -> bool {
    const SHALLOW_ALLOWED: [&str; 3] = ["/tmp/", "/root/", "/etc/"];
    let components = destination
        .split('/')
        .filter(|component| !component.is_empty())
        .count();
    if SHALLOW_ALLOWED
        .iter()
        .any(|prefix| destination.starts_with(prefix))
    {
        components >= 2
    } else {
        components >= 3
    }
}

/// Write a [`Secret`] into a new file at `mode`, refusing to overwrite one
/// already there.
fn write_secret(path: &Path, value: &Secret, mode: u32) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|cause| Error::FileUnwritable {
            path: path.display().to_string(),
            cause,
        })?;
    value.write_to(&mut file).map_err(|cause| Error::FileUnwritable {
        path: path.display().to_string(),
        cause,
    })
}

/// Write plain (non-secret) bytes into a new file at `mode`, refusing to
/// overwrite one already there.
fn write_plain(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|cause| Error::FileUnwritable {
            path: path.display().to_string(),
            cause,
        })?;
    file.write_all(bytes).map_err(|cause| Error::FileUnwritable {
        path: path.display().to_string(),
        cause,
    })
}

/// A subprocess's captured standard error, without its trailing newline.
fn trimmed(complaint: &str) -> String {
    complaint.strip_suffix('\n').unwrap_or(complaint).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Program, destination_is_safe, offered_key};

    /// Task 5.1's positive enumeration: every external tool this module may
    /// spawn, and a live check that [`Program::command`] is the only place
    /// in this file constructing one — so a subprocess call added anywhere
    /// else in `upload.rs` is a spawn site this enumeration stops covering,
    /// and the count below goes red rather than silently passing over it.
    #[test]
    fn every_subprocess_this_module_may_spawn_is_named_here() {
        let names: Vec<&str> = [
            Program::SshKeygen,
            Program::SshToAge,
            Program::SshKeyscan,
            Program::Ssh,
            Program::Tar,
        ]
        .into_iter()
        .map(Program::default_name)
        .collect();
        assert_eq!(names, ["ssh-keygen", "ssh-to-age", "ssh-keyscan", "ssh", "tar"]);

        let source = include_str!("upload.rs");
        let needle = ["Command", "::new("].concat();
        assert_eq!(
            source.matches(needle.as_str()).count(),
            1,
            "upload.rs spawns a subprocess outside Program::command; the positive \
             enumeration above no longer covers every spawn site"
        );
    }

    /// The fixed destination clears the depth safety by construction.
    #[test]
    fn the_fixed_remote_destination_clears_the_depth_safety() {
        assert!(destination_is_safe(super::REMOTE_DESTINATION));
    }

    /// 4.6: the check is live rather than vacuously true — pointing it at a
    /// two-component path turns it red.
    #[test]
    fn a_shallow_destination_fails_the_depth_safety() {
        assert!(!destination_is_safe("/mnt"));
        assert!(!destination_is_safe("/srv/x"));
        assert!(destination_is_safe("/tmp/x"));
        assert!(destination_is_safe("/root/x"));
        assert!(destination_is_safe("/mnt/etc/ssh"));
    }

    #[test]
    fn ssh_keyscan_output_is_parsed_for_its_ed25519_line_and_nothing_else() {
        let text = "example.com ssh-rsa AAAAB3NzaC1yc2E\nexample.com ssh-ed25519 AAAAC3abc\n";
        assert_eq!(
            text.lines().find_map(offered_key).as_deref(),
            Some("ssh-ed25519 AAAAC3abc")
        );
    }

    #[test]
    fn no_ed25519_line_is_read_as_no_key_presented() {
        let text = "example.com ssh-rsa AAAAB3NzaC1yc2E\n";
        assert_eq!(text.lines().find_map(offered_key), None);
    }
}
