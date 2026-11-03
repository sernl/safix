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

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::progress::{Progress, log, note};
use crate::workspace::{Workspace, login_name};

/// The environment variable naming the identity file, overriding the default.
pub const KEY_FILE_VARIABLE: &str = "SAFIX_AGE_KEY_FILE";

/// The environment variable naming the `age-keygen` binary, overriding the
/// default.
///
/// The same shape [`crate::sops::Sops::from_environment`] gives `SAFIX_SOPS`,
/// and for the same reason: a hermetic check drives the runtime against a
/// script of its own, and a check that could only run where a real
/// `age-keygen` is on `PATH` would be measuring the host as much as the
/// runtime. It is also what makes the home-scope key-generation switch
/// testable at all, since that switch's whole behaviour is invoking this
/// binary.
pub const KEYGEN_BINARY_VARIABLE: &str = "SAFIX_AGE_KEYGEN";

/// The binary [`KEYGEN_BINARY_VARIABLE`] names, or `age-keygen`.
#[must_use]
pub fn keygen_binary() -> PathBuf {
    std::env::var_os(KEYGEN_BINARY_VARIABLE)
        .map_or_else(|| PathBuf::from("age-keygen"), PathBuf::from)
}

/// The public-key comment `age-keygen` stores alongside each private identity.
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
    let public = crate::identity::generate_age(false)?;

    progress.write(&epilogue(user, &keyfile.display().to_string(), &public));
    if let Some(notice) = standalone_sops_notice(
        cfg!(target_os = "macos"),
        &keyfile,
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
        std::env::var_os("SOPS_AGE_KEY_FILE").as_deref(),
    ) {
        progress.write(&notice);
    }
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
    let text =
        zeroize::Zeroizing::new(std::fs::read_to_string(&keyfile).map_err(|_| no_identity_yet())?);
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
    crate::identity::validate_age_location(keyfile)?;
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

/// Append an identity atomically, retaining existing bytes at mode `0600`.
///
/// # Errors
///
/// Refuses unsafe custody paths, concurrent publication failures and failed writes.
pub fn append_to_identity_file(keyfile: &std::path::Path, text: &str) -> Result<()> {
    crate::identity::append_age_identity(keyfile, text.as_bytes())
}

/// The `export SOPS_AGE_KEY_FILE=` line standalone sops and clan need to read
/// `written`, or nothing where they already would.
///
/// On macOS standalone sops falls back to `$HOME/Library/Application Support`
/// rather than `$HOME/.config` when `XDG_CONFIG_HOME` is unset or empty, while
/// [`identity_file`] keeps `.config` on every platform. `macos` is the
/// platform as an argument, so the macOS answer is testable off macOS.
#[must_use]
pub fn standalone_sops_notice(
    macos: bool,
    written: &Path,
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
    sops_age_key_file: Option<&OsStr>,
) -> Option<String> {
    if !macos {
        return None;
    }
    if sops_age_key_file.is_some_and(|named| !named.is_empty() && Path::new(named) == written) {
        return None;
    }
    let config = match xdg_config_home.filter(|value| !value.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => Path::new(home.unwrap_or_default()).join("Library/Application Support"),
    };
    if config.join("sops/age/keys.txt") == written {
        return None;
    }
    let quoted = written.to_string_lossy().replace('\'', "'\\''");
    Some(format!(
        "\nStandalone sops and clan on macOS read a different default identity file.\n\
        To have them read this one:\n\
        \n\
        \x20   export SOPS_AGE_KEY_FILE='{quoted}'\n"
    ))
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
        it, and safix.identity.sshKeyPaths names the private half.\n"
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::standalone_sops_notice;

    const HOME: &str = "/Users/alice";
    const XDG_FALLBACK: &str = "/Users/alice/.config/sops/age/keys.txt";

    fn notice(
        macos: bool,
        written: &str,
        xdg: Option<&str>,
        sops_age_key_file: Option<&str>,
    ) -> Option<String> {
        standalone_sops_notice(
            macos,
            Path::new(written),
            xdg.map(OsStr::new),
            Some(OsStr::new(HOME)),
            sops_age_key_file.map(OsStr::new),
        )
    }

    #[test]
    fn the_notice_is_silent_off_macos() {
        assert_eq!(notice(false, XDG_FALLBACK, None, None), None);
    }

    #[test]
    fn the_notice_names_a_file_under_the_xdg_fallback() {
        let printed = notice(true, XDG_FALLBACK, None, None).unwrap_or_default();
        assert!(
            printed.contains(&format!("export SOPS_AGE_KEY_FILE='{XDG_FALLBACK}'\n")),
            "{printed}"
        );
        assert!(printed.contains("Standalone sops and clan"), "{printed}");
    }

    #[test]
    fn the_notice_is_silent_where_standalone_sops_looks_under_xdg() {
        assert_eq!(
            notice(true, "/xdg/sops/age/keys.txt", Some("/xdg"), None),
            None
        );
    }

    #[test]
    fn the_notice_is_silent_where_standalone_sops_looks_under_application_support() {
        assert_eq!(
            notice(
                true,
                "/Users/alice/Library/Application Support/sops/age/keys.txt",
                Some(""),
                None
            ),
            None
        );
    }

    #[test]
    fn the_notice_is_silent_when_sops_age_key_file_names_the_file() {
        assert_eq!(notice(true, XDG_FALLBACK, None, Some(XDG_FALLBACK)), None);
    }

    #[test]
    fn the_notice_quotes_a_path_with_a_space() {
        let printed = notice(true, "/Users/alice/my keys/it's.txt", None, None).unwrap_or_default();
        assert!(
            printed.contains("export SOPS_AGE_KEY_FILE='/Users/alice/my keys/it'\\''s.txt'\n"),
            "{printed}"
        );
    }
}
