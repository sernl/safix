//! The envelope a generator fragment runs inside.
//!
//! This is clan's envelope, adopted rather than invented, read off
//! `pkgs/clan-cli/clan_lib/sandbox_exec/__init__.py` at the revision this fleet
//! pins. Interop is the reason it is adopted: a fragment written against the
//! shared executor interface — see [`crate::inputs`] — meets the same
//! confinement under either system's default executor, so one that runs here
//! runs there and one that is refused here is refused there.
//!
//! Inside the envelope the staging root is the only writable path, the nix store
//! is readable, and there is no network. What that buys is the gap
//! [`crate::staging`] states it cannot close on its own: a fragment holding
//! plaintext can no longer copy it somewhere safix does not look and cannot
//! shred, because there is nowhere else to write.
//!
//! # The two backends
//!
//! On linux, bubblewrap: a tmpfs root, the store bound read-only, `/dev` and
//! `/proc` provided, a tmpfs `/tmp`, the staging root bound read-write, and
//! every namespace unshared. On darwin, the system `sandbox-exec` running
//! clan's deny-by-default profile, which is itself derived from nix's own
//! `sandbox-defaults.sb`. Every other platform has no envelope and is refused;
//! see [`Envelope::probe`].
//!
//! # Where the deviations are
//!
//! Three, all deliberate, all recorded in
//! `openspec/changes/adopt-generator-sandbox/design.md`:
//!
//! - clan passes `--uid 1000 --gid 1000`; this omits the pair and keeps the
//!   caller's uid mapped, because the staging root is created mode 0700 and
//!   owned by the caller before the fragment starts, so a synthetic uid inside
//!   the namespace could not write `$out` without loosening that root.
//! - clan chdirs to `/` and lets the fragment address its directories by
//!   absolute path; here the working directory is the staging root, because
//!   `secret-generators` requires it and a fragment written against it would
//!   otherwise find itself somewhere else.
//! - the darwin profile is handed to `sandbox-exec` with `-p` rather than
//!   written to a file and passed with `-f`. The profile is a function of the
//!   staging root and the store, so a file would be one more thing to create,
//!   register for the sweep and remove on every exit path, for no property the
//!   argument does not already have.

use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;

use crate::error::{Error, Result};
use crate::nix::Nix;

/// The store directory a fragment reads its declared tools out of.
pub const STORE: &str = "/nix/store";

/// The darwin backend, at the path the system puts it.
pub const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// The bubblewrap program, as the argument vector names it.
const BWRAP: &str = "bwrap";

/// The two platforms with an envelope, as the compiler names them.
const LINUX: &str = "linux";
const DARWIN: &str = "macos";

/// The mount point a fragment's own temporary files go to, as a tmpfs inside the
/// envelope rather than the caller's.
const TMP: &str = "/tmp";

/// The shell that runs a fragment, and the one the probe runs nothing in.
///
/// Resolved from nixpkgs beside the backend rather than taken off the caller's
/// `PATH`, because the caller's `bash` is reachable through a path — `/bin/bash`,
/// a profile's symlink tree — that a tmpfs root removes. clan resolves it the
/// same way and for the same reason.
pub const SHELL: &str = "bash";

/// Which confinement this platform supplies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// bubblewrap, resolved from nixpkgs the way a generator's own tools are.
    Bubblewrap,
    /// The system `sandbox-exec`, which nix itself and clan both run on darwin.
    SandboxExec,
}

impl Backend {
    /// The program a refusal names when this backend is the one that was looked
    /// for.
    #[must_use]
    pub const fn program(self) -> &'static str {
        match self {
            Self::Bubblewrap => BWRAP,
            Self::SandboxExec => SANDBOX_EXEC,
        }
    }

    /// The nixpkgs attributes this backend's words are resolved from.
    ///
    /// `bash` is in both, because the fragment's shell has to come out of the
    /// store for the same reason the backend does. `bubblewrap` joins it on
    /// linux; darwin's backend is the system's and is acquired from nowhere.
    #[must_use]
    pub const fn tools(self) -> &'static [&'static str] {
        match self {
            Self::Bubblewrap => &[SHELL, "bubblewrap"],
            Self::SandboxExec => &[SHELL],
        }
    }
}

/// What one fragment's spawn needs to be inside the envelope.
#[derive(Debug)]
pub struct Confinement {
    /// The words the fragment's own command is appended to.
    pub words: Vec<OsString>,
    /// The nixpkgs attributes those words are resolved from.
    pub tools: &'static [&'static str],
}

/// The envelope one generation run puts every fragment inside.
///
/// Held for the length of a run rather than constructed per fragment, because
/// what it carries is the answer to a question asked once — see
/// [`Envelope::probe`].
#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    backend: Backend,
}

impl Envelope {
    /// Which backend this platform supplies, asked once before the first
    /// fragment runs.
    ///
    /// On linux the question is whether bubblewrap runs at all, and it is
    /// answered by running it over the same envelope a fragment gets, so a
    /// kernel that refuses the namespaces is a refusal here rather than a
    /// failure inside generator three. On darwin the question is whether the
    /// system's `sandbox-exec` is where the system puts it. Every other platform
    /// has no envelope, which is a refusal and not a fallback.
    ///
    /// Asked once because availability does not change mid-run, and a refusal
    /// after a generator has committed is worse than the same refusal before any
    /// generator started.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxUnavailable`] when the platform's backend is there to be
    /// looked for and does not run, and [`Error::SandboxUnsupported`] on a
    /// platform that has no backend to look for.
    pub fn probe(nix: &Nix, root: &Path) -> Result<Self> {
        let platform = std::env::consts::OS;
        let answers = match platform {
            LINUX => bubblewrap_runs(nix, root),
            DARWIN => Path::new(SANDBOX_EXEC).exists(),
            // A platform with no backend is not asked anything: there is nothing
            // to ask, which is what [`backend_of`] turns into its own refusal.
            _ => false,
        };
        backend_of(platform, answers).map(|backend| Self { backend })
    }

    /// The envelope a caller has already established, for the two callers that
    /// know the backend without probing: this module's own tests, and a check
    /// driving one platform's construction from another.
    #[must_use]
    pub const fn of(backend: Backend) -> Self {
        Self { backend }
    }

    /// Which backend this envelope is.
    #[must_use]
    pub const fn backend(self) -> Backend {
        self.backend
    }

    /// The words that put one fragment inside the envelope.
    ///
    /// `staging` is the run's staging root, which becomes the only writable path
    /// and the working directory. It is absent for a validation fragment: by the
    /// time a candidate is judged the staging root has already been shredded, so
    /// that fragment gets the same envelope with nothing on the host writable at
    /// all, which is the envelope's own tmpfs and nothing else.
    ///
    /// `network` is the generator's own grant, and it re-shares the network and
    /// nothing else: the filesystem confinement is the same either way.
    #[must_use]
    pub fn confine(self, staging: Option<&Path>, network: bool) -> Confinement {
        let stores = [Path::new(STORE)];
        let words = match self.backend {
            Backend::Bubblewrap => {
                let mut words = vec![OsString::from(BWRAP)];
                words.extend(bubblewrap_arguments(staging, &stores, network));
                words
            }
            Backend::SandboxExec => vec![
                OsString::from(SANDBOX_EXEC),
                OsString::from("-p"),
                OsString::from(sandbox_exec_profile(staging, &stores, network)),
            ],
        };
        Confinement {
            words,
            tools: self.backend.tools(),
        }
    }
}

/// Which backend a platform supplies, given whether the one it should have
/// answers when it is asked.
///
/// Split out of [`Envelope::probe`] so that all three answers are reachable from
/// every platform. What a machine can do is a fact about that machine and is
/// asked there; what each platform's answer *means* is a claim this repository
/// makes, and a claim only one platform can read is one the other two platforms'
/// checks cannot hold to anything.
///
/// # Errors
///
/// [`Error::SandboxUnavailable`] for a platform whose backend did not answer, and
/// [`Error::SandboxUnsupported`] for a platform that has none.
pub fn backend_of(platform: &'static str, answers: bool) -> Result<Backend> {
    let expected = match platform {
        LINUX => Backend::Bubblewrap,
        DARWIN => Backend::SandboxExec,
        other => return Err(Error::SandboxUnsupported { platform: other }),
    };
    if answers {
        Ok(expected)
    } else {
        Err(Error::SandboxUnavailable {
            backend: expected.program(),
        })
    }
}

/// The bubblewrap arguments that hold a fragment to its staging root, up to and
/// including the `--` the fragment's own command follows.
///
/// clan's argument vector at the pinned revision, with the deviations this
/// module's header records. The order is clan's too, because a mount sequence is
/// order-dependent and reordering it would be a second design rather than an
/// adoption.
///
/// No `staging` means nothing on the host is writable and the fragment starts at
/// the envelope's own tmpfs root. That is what a validation fragment gets, and
/// what the availability probe asks its question inside.
#[must_use]
pub fn bubblewrap_arguments(
    staging: Option<&Path>,
    stores: &[&Path],
    network: bool,
) -> Vec<OsString> {
    let mut arguments = shared_arguments(stores, network);
    match staging {
        // The bind before the chdir, so the directory the fragment starts in is
        // one the envelope has already established.
        Some(staging) => {
            push(&mut arguments, ["--bind"], [staging, staging]);
            push(&mut arguments, ["--chdir"], [staging]);
        }
        None => push(&mut arguments, ["--chdir"], [Path::new("/")]),
    }
    arguments.push(OsString::from("--"));
    arguments
}

/// The `sandbox-exec` profile that holds a fragment to its staging root.
///
/// clan's profile, which is itself derived from nix's `sandbox-defaults.sb`: it
/// denies by default, reads the store, grants the staging root, and allows
/// localhost networking alone. The store is named from `stores` rather than
/// written in, so the profile and the bubblewrap argument vector read the store
/// out of one place. No `staging` grants no writable path at all, which is what
/// a validation fragment gets — see [`Envelope::confine`].
///
/// The localhost allowance is clan's and is kept. It is the one place the two
/// platforms differ in what "no network" means, and adopting the profile means
/// adopting that: a darwin fragment can reach a listener on the machine running
/// it, where a linux fragment gets a network namespace holding nothing but its
/// own loopback.
#[must_use]
pub fn sandbox_exec_profile(staging: Option<&Path>, stores: &[&Path], network: bool) -> String {
    let mut profile = String::from(
        "(version 1)\n\
         \n\
         (deny default)\n\
         \n\
         ; Disallow creating setuid/setgid binaries, since that\n\
         ; would allow breaking build user isolation.\n\
         (deny file-write-setugid)\n\
         \n\
         ; Allow forking.\n\
         (allow process-fork)\n\
         \n\
         ; Allow reading system information like #CPUs, etc.\n\
         (allow sysctl-read)\n\
         \n\
         ; Allow POSIX semaphores and shared memory.\n\
         (allow ipc-posix*)\n\
         \n\
         ; Allow SYSV semaphores and shared memory.\n\
         (allow ipc-sysv*)\n\
         \n\
         ; Allow socket creation.\n\
         (allow system-socket)\n\
         \n\
         ; Allow sending signals within the sandbox.\n\
         (allow signal (target same-sandbox))\n\
         \n\
         ; Allow getpwuid.\n\
         (allow mach-lookup (global-name \"com.apple.system.opendirectoryd.libinfo\"))\n\
         \n\
         ; Allow read access to the nix store\n",
    );
    for store in stores {
        profile.push_str(&subpath("allow file-read*", store));
    }
    profile.push_str(
        "\n\
         ; Allow reading directory structure for getcwd\n\
         (allow file-read-metadata (subpath \"/\"))\n\
         (allow file-read-metadata (subpath \"/private\"))\n\
         \n\
         ; Some packages like to read the system version.\n\
         (allow file-read*\n\
         \x20(literal \"/System/Library/CoreServices/SystemVersion.plist\")\n\
         \x20(literal \"/System/Library/CoreServices/SystemVersionCompat.plist\"))\n\
         \n\
         ; Without this line clang cannot write to /dev/null, breaking some configure tests.\n\
         (allow file-read-metadata (literal \"/dev\"))\n\
         \n\
         ; Allow read and write access to /dev/null\n\
         (allow file-read* file-write* (literal \"/dev/null\"))\n\
         (allow file-read* (literal \"/dev/random\"))\n\
         (allow file-read* (literal \"/dev/urandom\"))\n\
         \n\
         ; Allow local networking (localhost only)\n\
         (allow network* (remote ip \"localhost:*\"))\n\
         (allow network-inbound (local ip \"*:*\"))\n\
         \n\
         ; Allow access to /etc/resolv.conf for DNS resolution\n\
         (allow file-read* (literal \"/etc/resolv.conf\"))\n\
         (allow file-read* (literal \"/private/etc/resolv.conf\"))\n\
         \n\
         ; Allow reading from common system paths that scripts might need\n\
         (allow file-read* (literal \"/\"))\n\
         (allow file-read* (literal \"/usr\"))\n\
         (allow file-read* (literal \"/bin\"))\n\
         (allow file-read* (literal \"/sbin\"))\n\
         \n\
         ; Allow execution of binaries from Nix store and system paths\n",
    );
    for store in stores {
        profile.push_str(&subpath("allow process-exec", store));
    }
    profile.push_str(
        "(allow process-exec (literal \"/bin/bash\"))\n\
         (allow process-exec (literal \"/bin/sh\"))\n\
         (allow process-exec (literal \"/usr/bin/env\"))\n",
    );
    if let Some(staging) = staging {
        profile.push_str("\n; The staging root, which is the only path a fragment may write\n");
        profile.push_str(&subpath(
            "allow file* process-exec network-outbound network-inbound",
            staging,
        ));
    }
    if network {
        profile.push_str(
            "\n\
             ; The generator's own network grant\n\
             (allow network-outbound)\n\
             (allow network-bind)\n",
        );
    }
    profile
}

/// One profile rule over a subpath.
fn subpath(rule: &str, path: &Path) -> String {
    format!("({rule} (subpath \"{}\"))\n", path.display())
}

/// The arguments both the fragment's envelope and the probe's are made of.
///
/// Everything but where the fragment starts and what it may write, which is the
/// whole of the difference between confining a fragment and asking whether the
/// backend runs at all.
fn shared_arguments(stores: &[&Path], network: bool) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("--unshare-all")];
    // Beside `--unshare-all` rather than instead of it: bubblewrap's own
    // spelling for re-sharing one namespace out of the set, which is what makes
    // the grant the network and nothing else.
    if network {
        arguments.push(OsString::from("--share-net"));
    }
    push(&mut arguments, ["--tmpfs"], [Path::new("/")]);
    for store in stores {
        push(&mut arguments, ["--ro-bind"], [*store, *store]);
    }
    // `-try` because a host without `/bin/sh` is a host where binding it is not
    // a failure: a fragment's shell comes from the store either way.
    let shell = Path::new("/bin/sh");
    push(&mut arguments, ["--ro-bind-try"], [shell, shell]);
    push(&mut arguments, ["--dev"], [Path::new("/dev")]);
    let proc = Path::new("/proc");
    push(&mut arguments, ["--bind"], [proc, proc]);
    push(&mut arguments, ["--tmpfs"], [Path::new(TMP)]);
    arguments
}

/// One bubblewrap operation: its flag, then its paths.
fn push<const FLAGS: usize, const PATHS: usize>(
    arguments: &mut Vec<OsString>,
    flags: [&str; FLAGS],
    paths: [&Path; PATHS],
) {
    arguments.extend(flags.into_iter().map(OsString::from));
    arguments.extend(
        paths
            .into_iter()
            .map(|path| path.as_os_str().to_os_string()),
    );
}

/// Whether bubblewrap runs on this machine, asked by running it.
///
/// The probe's own envelope is the fragment's minus the staging root, and the
/// command inside it does nothing: what is being asked is whether the kernel
/// grants the namespaces the envelope is made of, and a machine that answers no
/// answers it here rather than at the first generator. This is `sandbox_works()`
/// in clan, which asks the same question the same way.
///
/// Both streams are discarded. bubblewrap's own complaint about a kernel that
/// refuses is not the refusal an operator needs; [`Error::SandboxUnavailable`]
/// is, and it says what to do.
fn bubblewrap_runs(nix: &Nix, root: &Path) -> bool {
    let mut command = nix.shell(root, Backend::Bubblewrap.tools());
    command.arg(BWRAP);
    // No staging root and no grant: what is being asked is what the machine will
    // do, and a generator's own declaration is not part of that question.
    for argument in bubblewrap_arguments(None, &[Path::new(STORE)], false) {
        command.arg(argument);
    }
    command
        .arg(SHELL)
        .arg("-c")
        .arg(":")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    /// The arguments as text, which is how a reader of a failure sees them.
    fn text(arguments: &[OsString]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_default_envelope_binds_the_staging_root_and_shares_no_namespace() {
        let arguments = text(&bubblewrap_arguments(
            Some(Path::new("/dev/shm/safix-stage-1")),
            &[Path::new(STORE)],
            false,
        ));

        assert_eq!(
            arguments,
            [
                "--unshare-all",
                "--tmpfs",
                "/",
                "--ro-bind",
                "/nix/store",
                "/nix/store",
                "--ro-bind-try",
                "/bin/sh",
                "/bin/sh",
                "--dev",
                "/dev",
                "--bind",
                "/proc",
                "/proc",
                "--tmpfs",
                "/tmp",
                "--bind",
                "/dev/shm/safix-stage-1",
                "/dev/shm/safix-stage-1",
                "--chdir",
                "/dev/shm/safix-stage-1",
                "--",
            ],
            "the envelope is no longer clan's argument vector minus the uid pair"
        );
    }

    #[test]
    fn the_network_variant_adds_the_one_flag_and_moves_nothing_else() {
        let staging = Path::new("/dev/shm/safix-stage-2");
        let store = [Path::new(STORE)];
        let default = text(&bubblewrap_arguments(Some(staging), &store, false));
        let granted = text(&bubblewrap_arguments(Some(staging), &store, true));

        assert_eq!(
            granted.iter().filter(|word| *word == "--share-net").count(),
            1,
            "the grant did not re-share the network"
        );
        let without: Vec<&String> = granted
            .iter()
            .filter(|word| *word != "--share-net")
            .collect();
        assert_eq!(
            without,
            default.iter().collect::<Vec<&String>>(),
            "the grant changed something besides the network"
        );
        assert!(
            granted.contains(&"--bind".to_owned())
                && granted.windows(3).any(|window| window
                    == [
                        "--bind".to_owned(),
                        staging.display().to_string(),
                        staging.display().to_string()
                    ]),
            "the grant loosened the filesystem confinement"
        );
    }

    #[test]
    fn every_store_in_the_set_is_bound_read_only() {
        let arguments = text(&bubblewrap_arguments(
            Some(Path::new("/dev/shm/safix-stage-3")),
            &[Path::new(STORE), Path::new("/build/store/nix/store")],
            false,
        ));

        assert_eq!(
            arguments.iter().filter(|word| *word == "--ro-bind").count(),
            2,
            "a store in the set was not bound"
        );
        assert!(arguments.contains(&"/build/store/nix/store".to_owned()));
    }

    #[test]
    fn without_a_staging_root_nothing_on_the_host_is_writable() {
        let arguments = text(&bubblewrap_arguments(None, &[Path::new(STORE)], false));

        assert_eq!(
            arguments.last().map(String::as_str),
            Some("--"),
            "the arguments do not end where the fragment's own command begins"
        );
        // `/proc` is the one writable bind the envelope keeps, and it is clan's.
        // Anything else bound here would be a path on the host a fragment with no
        // staging root could still write into.
        let bound: Vec<&String> = arguments
            .windows(2)
            .filter(|window| window.first().is_some_and(|flag| flag == "--bind"))
            .filter_map(|window| window.get(1))
            .collect();
        assert_eq!(
            bound,
            ["/proc"],
            "something other than /proc was bound writable"
        );
        assert_eq!(
            arguments
                .windows(2)
                .filter(|window| *window == ["--chdir".to_owned(), "/".to_owned()])
                .count(),
            1,
            "the fragment starts somewhere other than the envelope's own root"
        );
    }

    #[test]
    fn the_darwin_profile_denies_by_default_and_grants_the_staging_root_alone() {
        let profile = sandbox_exec_profile(
            Some(Path::new("/private/var/folders/safix-stage-4")),
            &[Path::new(STORE)],
            false,
        );

        assert!(profile.starts_with("(version 1)\n"));
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(allow file-read* (subpath \"/nix/store\"))"));
        assert!(profile.contains(
            "(allow file* process-exec network-outbound network-inbound \
             (subpath \"/private/var/folders/safix-stage-4\"))"
        ));
        assert!(
            !profile.contains("(allow network-outbound)"),
            "the default profile allows outbound traffic"
        );
    }

    #[test]
    fn the_darwin_network_variant_adds_the_outbound_allowance_and_nothing_else() {
        let staging = Some(Path::new("/private/var/folders/safix-stage-5"));
        let store = [Path::new(STORE)];
        let default = sandbox_exec_profile(staging, &store, false);
        let granted = sandbox_exec_profile(staging, &store, true);

        assert!(
            granted.starts_with(&default),
            "the grant rewrote the profile"
        );
        assert!(granted.contains("(allow network-outbound)"));
        assert!(
            granted.contains("(deny default)"),
            "the grant stopped the profile denying by default"
        );
    }

    #[test]
    fn each_platform_gets_its_own_backend_or_its_own_refusal() {
        assert_eq!(backend_of(LINUX, true).ok(), Some(Backend::Bubblewrap));
        assert_eq!(backend_of(DARWIN, true).ok(), Some(Backend::SandboxExec));

        let Err(Error::SandboxUnavailable { backend }) = backend_of(LINUX, false) else {
            unreachable!("a linux without bubblewrap is refused as having no backend running");
        };
        assert_eq!(backend, BWRAP);

        let Err(Error::SandboxUnavailable { backend }) = backend_of(DARWIN, false) else {
            unreachable!("a darwin without sandbox-exec is refused as having no backend running");
        };
        assert_eq!(backend, SANDBOX_EXEC);

        let Err(Error::SandboxUnsupported { platform }) = backend_of("freebsd", true) else {
            unreachable!("a platform with no backend is refused as having no envelope");
        };
        assert_eq!(
            platform, "freebsd",
            "the refusal does not name the platform that has no envelope"
        );
    }

    #[test]
    fn the_backend_names_what_supplies_it() {
        assert_eq!(Backend::Bubblewrap.program(), "bwrap");
        assert_eq!(Backend::SandboxExec.program(), "/usr/bin/sandbox-exec");
        assert!(Backend::Bubblewrap.tools().contains(&"bubblewrap"));
        assert!(Backend::Bubblewrap.tools().contains(&SHELL));
        assert_eq!(
            Backend::SandboxExec.tools(),
            [SHELL],
            "darwin acquires a backend it is handed by the system"
        );
    }

    #[test]
    fn each_backend_confines_a_fragment_with_its_own_words() {
        let staging = Some(Path::new("/dev/shm/safix-stage-6"));

        let linux = Envelope::of(Backend::Bubblewrap).confine(staging, false);
        assert_eq!(
            linux.words.first().map(OsString::as_os_str),
            Some(OsStr::new(BWRAP))
        );
        assert_eq!(
            linux.words.last().map(OsString::as_os_str),
            Some(OsStr::new("--"))
        );
        assert_eq!(linux.tools, Backend::Bubblewrap.tools());

        let darwin = Envelope::of(Backend::SandboxExec).confine(staging, true);
        let words = text(&darwin.words);
        assert_eq!(
            words.first().map(String::as_str),
            Some(SANDBOX_EXEC),
            "the darwin envelope does not run the system's backend"
        );
        assert_eq!(words.get(1).map(String::as_str), Some("-p"));
        assert!(
            words
                .get(2)
                .is_some_and(|profile| profile.contains("(deny default)")
                    && profile.contains("(allow network-outbound)")),
            "the profile does not travel as the argument it is passed as"
        );
    }
}
