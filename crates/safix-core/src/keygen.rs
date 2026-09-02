//! An age identity for a person who has none, written where sops looks.
//!
//! Custody is the whole of what makes this delicate. The identity minted here is
//! the private half of what will become that person's `recipient`, and
//! everything they own is encrypted to it. It is therefore meant to run on their
//! own machine, under their own account, and what leaves this module is the
//! public half alone.
//!
//! # Why it appends
//!
//! `age-keygen -o <file>` refuses an existing file outright, which is the right
//! refusal for the wrong shape here: sops reads every identity in `keys.txt` and
//! tries each, so a second identity beside a first is a working state, and
//! truncating the file is how someone loses the key to everything they hold.
//! Appending never rewrites a line that is already there.
//!
//! # What is never printed
//!
//! The private half. `age-keygen` writes the identity to standard output and the
//! public key to standard error; this connects the first to the identity file and
//! reads only the second, so the private half is never a string this process
//! formats.

use std::fs::OpenOptions;
use std::io::Read as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::progress::{Progress, log, note};
use crate::workspace::{Workspace, login_name};

/// The environment variable naming the identity file, overriding the default.
pub const KEY_FILE_VARIABLE: &str = "SAFIX_AGE_KEY_FILE";

/// The line `age-keygen` writes its public half on.
const PUBLIC_KEY_PREFIX: &str = "Public key: ";

/// The prefix `age-keygen` writes on the comment line preceding each identity
/// it appends to the file, distinct from [`PUBLIC_KEY_PREFIX`], which is what
/// it writes to standard error instead.
const PUBLIC_KEY_COMMENT_PREFIX: &str = "# public key: ";

/// Mint an identity for this user and append it to their identity file.
///
/// # Errors
///
/// [`Error::UnknownUser`] for a user no declaration names,
/// [`Error::KeygenForSomeoneElse`] when the user is not the one running this and
/// that was not said out loud, [`Error::FileUnwritable`] when the identity file
/// cannot be opened, [`Error::KeygenFailed`] when `age-keygen` cannot be run or
/// refuses, and [`Error::KeygenNoPublicKey`] when it runs and names none.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    user: &str,
    for_someone_else: bool,
) -> Result<()> {
    workspace.require_user(user)?;

    if user != login_name() && !for_someone_else {
        return Err(Error::KeygenForSomeoneElse {
            user: user.to_owned(),
        });
    }
    if for_someone_else {
        log(
            progress,
            &format!(
                "safix: minting an identity for '{user}' in YOUR identity file. \
                You will hold their private key and be able to read everything they own."
            ),
        );
    }

    let keyfile = identity_file();
    prepare_identity_file(&keyfile, progress)?;

    let public = append_identity(&keyfile)?;
    let _ = std::fs::set_permissions(&keyfile, std::fs::Permissions::from_mode(0o600));

    let public = public.ok_or_else(|| Error::KeygenNoPublicKey {
        file: keyfile.display().to_string(),
    })?;

    progress.write(&epilogue(user, &keyfile.display().to_string(), &public));
    Ok(())
}

/// Print the public recipient already minted on this machine, and mint
/// nothing.
///
/// Reads the last identity's comment line out of the identity file directly,
/// rather than re-deriving it: `age-keygen` writes that comment ahead of every
/// identity it appends, and the most recently appended one is the one this
/// machine's own `recipient` is minted from.
///
/// # Errors
///
/// [`Error::KeygenNoIdentityYet`] when no identity has been minted on this
/// machine yet, naming plain `keygen` as the remedy.
pub fn show(progress: &dyn Progress) -> Result<()> {
    let keyfile = identity_file();
    let no_identity_yet = || Error::KeygenNoIdentityYet {
        file: keyfile.display().to_string(),
    };
    let text = std::fs::read_to_string(&keyfile).map_err(|_| no_identity_yet())?;
    let public = text
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(PUBLIC_KEY_COMMENT_PREFIX))
        .ok_or_else(no_identity_yet)?;

    let mut printed = public.as_bytes().to_vec();
    printed.push(b'\n');
    progress.write_output(&printed);
    Ok(())
}

/// Where sops looks for identities, or what the environment names instead.
///
/// Public because it is not this subcommand's answer but the package's: an
/// identity minted here and a card's stub appended by
/// [`enroll`](crate::enroll) are peers in one file, and two callers computing
/// that path separately is how they stop being.
#[must_use]
pub fn identity_file() -> PathBuf {
    if let Some(named) = std::env::var_os(KEY_FILE_VARIABLE)
        && !named.is_empty()
    {
        return PathBuf::from(named);
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"),
            PathBuf::from,
        );
    config.join("sops").join("age").join("keys.txt")
}

/// Make the identity file's directory, at mode `0700`, and say whether the file
/// is already there.
///
/// Shared with [`enroll`](crate::enroll) rather than duplicated, for the reason
/// [`identity_file`] is public: the directory's mode and the note about
/// appending are one behaviour, and two copies of it are two behaviours waiting
/// to differ.
///
/// # Errors
///
/// [`Error::FileUnwritable`] when the directory cannot be made.
pub fn prepare_identity_file(keyfile: &std::path::Path, progress: &dyn Progress) -> Result<()> {
    if let Some(directory) = keyfile.parent() {
        std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
            path: directory.display().to_string(),
            cause,
        })?;
        let _ = std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700));
    }
    if keyfile.exists() {
        note(
            progress,
            &format!(
                "{} already holds an identity; appending. Nothing already in it is rewritten, \
                and sops tries every identity in the file.",
                keyfile.display()
            ),
        );
    }
    Ok(())
}

/// Append text to the identity file, creating it at mode `0600`.
///
/// The whole of the file discipline this package has: opened for appending and
/// never for writing, so nothing already in it is rewritten, and created with
/// the mode rather than chmodded into it afterwards. `age-keygen -o <file>`
/// refuses an existing file outright, which is the right refusal for the wrong
/// shape here — sops tries every identity in the file, so a second identity
/// beside a first is a working state and truncating is how someone loses the key
/// to everything they hold.
///
/// # Errors
///
/// [`Error::FileUnwritable`] when the file cannot be opened or written.
pub fn append_to_identity_file(keyfile: &std::path::Path, text: &str) -> Result<()> {
    use std::io::Write as _;

    let mut sink = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(keyfile)
        .map_err(|cause| Error::FileUnwritable {
            path: keyfile.display().to_string(),
            cause,
        })?;
    sink.write_all(text.as_bytes())
        .and_then(|()| sink.flush())
        .map_err(|cause| Error::FileUnwritable {
            path: keyfile.display().to_string(),
            cause,
        })?;
    let _ = std::fs::set_permissions(keyfile, std::fs::Permissions::from_mode(0o600));
    Ok(())
}

/// Run `age-keygen`, appending the identity and returning the public half.
///
/// Standard output is the identity and goes straight into the file; the public
/// half is the only thing `age-keygen` puts on standard error, and the only thing
/// this reads.
fn append_identity(keyfile: &std::path::Path) -> Result<Option<String>> {
    let sink = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(keyfile)
        .map_err(|cause| Error::FileUnwritable {
            path: keyfile.display().to_string(),
            cause,
        })?;

    let mut child = Command::new("age-keygen")
        .stdin(Stdio::null())
        .stdout(Stdio::from(sink))
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| Error::KeygenFailed)?;

    let mut announced = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut announced);
    }
    if !child.wait().map_err(|_| Error::KeygenFailed)?.success() {
        return Err(Error::KeygenFailed);
    }

    Ok(announced
        .lines()
        .find_map(|line| line.strip_prefix(PUBLIC_KEY_PREFIX))
        .map(str::to_owned))
}

/// What to do with the half that just left this process.
fn epilogue(user: &str, keyfile: &str, public: &str) -> String {
    format!(
        "\nsafix: appended an identity for {user} to {keyfile}\n\
        \n\
        The private half stays in that file and is not printed. Hand over the\n\
        public half, which is public data:\n\
        \n\
        \x20   {public}\n\
        \n\
        It becomes their recipient:\n\
        \n\
        \x20   flake.safix.users.{user}.recipient = \"{public}\";\n\
        \n\
        Then re-wrap the files their audience now names, and review the diff:\n\
        \n\
        \x20   safix fix\n\
        \x20   git diff\n\
        \n\
        An existing ssh key can be a recipient instead of a fresh identity:\n\
        ssh-to-age reads an ed25519 public key and prints the age recipient for\n\
        it, and sops.age.sshKeyPaths names the private half.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epilogue_names_the_public_half_and_never_a_file_of_private_ones() {
        let rendered = epilogue(
            "alice",
            "/home/alice/.config/sops/age/keys.txt",
            "age1fixture",
        );
        assert!(rendered.contains("flake.safix.users.alice.recipient = \"age1fixture\";"));
        assert!(rendered.contains("The private half stays in that file and is not printed."));
        assert!(rendered.ends_with("names the private half.\n"));
    }
}
