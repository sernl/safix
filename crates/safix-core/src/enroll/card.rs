//! The card's PIV access: enumerated, probed, and provisioned.
//!
//! This module is a set of argument vectors and a driver that runs them. The
//! vectors are built by functions that take no card and touch no reader, which is
//! what lets every construction below be asserted without one — the card is
//! present in exactly one place, [`Ykman`], and every claim about *what is said to
//! it* is a claim about a `Vec<String>`.
//!
//! # No generated credential reaches an argument vector
//!
//! `ykman piv access` takes each credential as an option and prompts for it when
//! the option is omitted, so the options are omitted and the prompts are answered
//! on a pseudo-terminal — see [`pty`]. An argument vector is readable
//! by every process on the machine, which for a PIN is the difference between a
//! credential and a published one, and there is no version of "the flag is
//! convenient" that survives that.
//!
//! Two strings do travel as options, and neither is a credential. The serial
//! selects a card and is public. And the *current* PIN and PUK of a factory-fresh
//! card are the values Yubico documents and every card ships with, identical
//! everywhere, granting nothing to whoever reads them — supplying those as options
//! is what leaves each remaining prompt asking for one value, which is what makes
//! the pseudo-terminal drive sound rather than a guess at prompt boundaries.
//! Provisioning only ever runs against a card the state probe found factory-fresh,
//! so those are the only values they can be.
//!
//! # What is never issued
//!
//! Any OTP command. The two applets are disjoint and safix drives one of them;
//! the other holds the challenge-response secret a password database is opened
//! by, and programming that slot ends the database. [`every_argument_vector`]
//! exists so that "no code path issues an OTP command" is a test rather than a
//! promise, and so is "no construction carries anything but public words".
//!
//! # Why [`Credentials`] is not a [`Secret`]
//!
//! Because the custody record is a function of both halves, and [`Secret`] has no
//! accessor to compute one from — that absence is the whole of its discipline.
//! So this type holds the two values and hands out [`Secret`]s: one per half for
//! the prompts, and one for the record. It has no accessor of its own, and the
//! traits that would amount to one are absent under the same compiled assertions
//! [`Secret`] carries.
//!
//! # Two couplings to ykman's own wording
//!
//! The state probe and the reader refusal read sentences `ykman` writes, as
//! substrings, for the reason [`clan`](crate::clan)'s outcome lines are matched
//! the same way: the question has no exit status of its own, and the alternative
//! is treating a distinguishable outcome as a generic failure. Both are named
//! constants so the coupling is one line rather than a scattered idiom.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use zeroize::Zeroizing;

use crate::error::{Error, Result};
use crate::probe::{DebugFallback as _, DisplayFallback as _, Implements, SerializeFallback as _};
use crate::progress::Progress;
use crate::secret::Secret;

use super::pty;

/// The environment variable that replaces the program, for checks.
///
/// Mirrors `SAFIX_SOPS` and `SAFIX_CLAN`. Every check that drives enrollment
/// drives a stub through this, because the alternative is a check that talks to
/// whatever card is plugged into the machine running it.
pub const PROGRAM_OVERRIDE: &str = "SAFIX_YKMAN";

/// The PIN every `YubiKey` leaves the factory with.
pub const FACTORY_PIN: &str = "123456";

/// The PUK every `YubiKey` leaves the factory with.
pub const FACTORY_PUK: &str = "12345678";

/// The sentence `ykman piv info` writes about a management key held on the card
/// under the PIN.
///
/// The provisioned/factory question, and the whole of how it is answered. A card
/// safix provisioned has a PIN-protected management key and a factory-fresh one
/// has the standard key, so this sentence's presence is the state — and asking
/// costs no PIN retry, where verifying a candidate PIN would cost one on every
/// card that is not factory-fresh.
const MANAGEMENT_KEY_PROTECTED: &str = "protected by PIN";

/// What `ykman` says when the smartcard service is not there to talk to.
///
/// Matched case-insensitively as a substring. The refusal it produces names
/// `services.pcscd.enable`, which is the remedy `ykman`'s own message does not
/// know about.
const NO_SMARTCARD_SERVICE: &str = "pcsc";

/// The alphabet a generated PIN and PUK are drawn from.
///
/// Digits alone. A PIV PIN is entered on whatever keypad the card is presented
/// to, and the readers this fleet has are numeric.
const DIGITS: &[u8] = b"0123456789";

/// How long a generated PIN and PUK are.
///
/// Eight, which is the maximum PIV allows and the maximum the generator's own
/// prompt accepts (`age-plugin-yubikey` requires six to eight characters). The
/// maximum rather than the minimum because nothing here has to be typed by hand.
const CREDENTIAL_LENGTH: usize = 8;

/// The first byte value that would bias the digit it maps to.
///
/// Two hundred and fifty is twenty-five whole runs of ten, so every byte below
/// it maps to a digit with equal probability and every byte at or above it is
/// drawn again.
const UNBIASED_CEILING: u8 = 250;

/// Where entropy comes from.
///
/// The kernel, read as a stream, rather than a generator seeded in this process:
/// a PIN is a credential and the fleet's own rule for those is that they come
/// from the kernel's pool.
const ENTROPY_SOURCE: &str = "/dev/urandom";

/// The PIN and, when safix generated it, the PUK for one card.
///
/// Not a [`Secret`], deliberately, and the module documentation says why: the
/// custody record is a function of both halves and [`Secret`] has no accessor to
/// compute one from. Everything this hands out is a [`Secret`], and every one of
/// them leaves down a pipe or a pseudo-terminal.
///
/// The absent traits below are the same ones [`Secret`] asserts the absence of,
/// and they are asserted the same way: a `Debug` derived here would print a PIN
/// through a field.
pub struct Credentials {
    pin: Zeroizing<String>,
    puk: Option<Zeroizing<String>>,
}

const _: () = assert!(
    !Implements::<Credentials>::DEBUG,
    "Credentials must not implement Debug"
);
const _: () = assert!(
    !Implements::<Credentials>::DISPLAY,
    "Credentials must not implement Display"
);
const _: () = assert!(
    !Implements::<Credentials>::SERIALIZE,
    "Credentials must not implement serde::Serialize"
);

impl Credentials {
    /// A fresh PIN and a distinct fresh PUK, both from the kernel's pool.
    ///
    /// Distinct because the generator's own factory-default flow collapses them
    /// into one, which is the flow safix pre-empts: a PUK equal to the PIN means
    /// the PIN's own unblock path is the PIN.
    ///
    /// # Errors
    ///
    /// [`Error::EntropyUnreadable`] when the kernel's pool cannot be read.
    pub fn generate() -> Result<Self> {
        let pin = digits(CREDENTIAL_LENGTH)?;
        let mut puk = digits(CREDENTIAL_LENGTH)?;
        while *puk == *pin {
            puk = digits(CREDENTIAL_LENGTH)?;
        }
        Ok(Self {
            pin,
            puk: Some(puk),
        })
    }

    /// A PIN safix did not generate, for a card that is already provisioned.
    ///
    /// The PUK is absent rather than guessed: safix did not set it, so it is not
    /// safix's to record, and a custody entry naming a PUK nobody here knows
    /// would be a credential with no value in it.
    #[must_use]
    pub fn existing(pin: Zeroizing<String>) -> Self {
        Self { pin, puk: None }
    }

    /// Whether safix generated these, and so whether they are safix's to store.
    #[must_use]
    pub fn generated(&self) -> bool {
        self.puk.is_some()
    }

    /// The PIN as a value that can only leave down a pipe.
    ///
    /// What `ykman`'s prompt and the generator's prompt are answered with, and
    /// what the password store receives.
    ///
    /// # Errors
    ///
    /// [`Error::SecretRead`] when the in-memory copy cannot be read, which a
    /// slice cannot fail at.
    pub fn pin_secret(&self) -> Result<Secret> {
        Secret::read_from(&mut self.pin.as_bytes())
    }

    /// The PUK as a value that can only leave down a pipe, when there is one.
    ///
    /// # Errors
    ///
    /// [`Error::SecretRead`] when the in-memory copy cannot be read.
    pub fn puk_secret(&self) -> Result<Option<Secret>> {
        match &self.puk {
            Some(puk) => Ok(Some(Secret::read_from(&mut puk.as_bytes())?)),
            None => Ok(None),
        }
    }

    /// Both halves as one value, in the shape a custody entry holds them.
    ///
    /// Two labelled lines rather than a structure, because every reader of this is
    /// a person looking at one entry of a password store or one key of a safix
    /// secret. No trailing newline, which is the convention every value this
    /// package stores follows: a value is stored exactly as it was given.
    ///
    /// # Errors
    ///
    /// [`Error::SecretRead`] when the in-memory copy cannot be read.
    pub fn record(&self) -> Result<Secret> {
        let mut text = Zeroizing::new(format!("PIN={}", self.pin.as_str()));
        if let Some(puk) = &self.puk {
            text.push_str("\nPUK=");
            text.push_str(puk);
        }
        Secret::read_from(&mut text.as_bytes())
    }
}

/// One string of decimal digits, drawn from the kernel's pool without bias.
fn digits(length: usize) -> Result<Zeroizing<String>> {
    let mut source =
        std::fs::File::open(ENTROPY_SOURCE).map_err(|cause| Error::EntropyUnreadable {
            source: ENTROPY_SOURCE,
            cause,
        })?;

    let mut drawn = Zeroizing::new(String::with_capacity(length));
    let mut byte = Zeroizing::new([0_u8; 1]);
    while drawn.len() < length {
        source
            .read_exact(byte.as_mut_slice())
            .map_err(|cause| Error::EntropyUnreadable {
                source: ENTROPY_SOURCE,
                cause,
            })?;
        let Some(&value) = byte.first() else {
            continue;
        };
        if value >= UNBIASED_CEILING {
            continue;
        }
        let index = usize::from(value).checked_rem(DIGITS.len());
        if let Some(&digit) = index.and_then(|index| DIGITS.get(index)) {
            drawn.push(char::from(digit));
        }
    }
    Ok(drawn)
}

/// What one card's PIV access looks like before enrollment touches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Nobody has provisioned it: the factory PIN and PUK are in force and the
    /// management key is the standard one.
    FactoryFresh,
    /// Its management key is held on the card under its PIN, which is what
    /// provisioning leaves behind and what a factory card does not have.
    Provisioned,
}

/// The `ykman` command, and how it is reached.
#[derive(Debug, Clone)]
pub struct Ykman {
    program: PathBuf,
}

impl Default for Ykman {
    fn default() -> Self {
        Self::from_environment()
    }
}

/// `ykman list --serials`, which prints one serial per line.
#[must_use]
pub fn list_arguments() -> Vec<String> {
    vec!["list".to_owned(), "--serials".to_owned()]
}

/// `ykman --device <serial> piv info`, the state probe.
#[must_use]
pub fn info_arguments(serial: &str) -> Vec<String> {
    device(serial, &["piv", "info"])
}

/// `ykman --device <serial> piv access change-pin -P <factory>`.
///
/// The new PIN is not here. `-n` is omitted so that `ykman` prompts for it, and
/// the prompt is answered on a pseudo-terminal; the module documentation says why
/// an argument vector is not an acceptable channel for it. `-P` carries the
/// factory default, which is a published constant rather than a credential, and
/// carrying it is what leaves the remaining prompts asking for one value.
#[must_use]
pub fn change_pin_arguments(serial: &str) -> Vec<String> {
    device(serial, &["piv", "access", "change-pin", "-P", FACTORY_PIN])
}

/// `ykman --device <serial> piv access change-puk -p <factory>`.
///
/// The new PUK is prompted for, exactly as the new PIN is, and for the same
/// reason.
#[must_use]
pub fn change_puk_arguments(serial: &str) -> Vec<String> {
    device(serial, &["piv", "access", "change-puk", "-p", FACTORY_PUK])
}

/// `ykman --device <serial> piv access change-management-key --protect
/// --generate -f`.
///
/// `--protect` is what puts the key on the card under the PIN and `--generate` is
/// what makes it random. Together they mean the management key is never a string
/// this process holds, which is why nothing stores it: PIN possession is
/// management possession, and a stored copy would be a credential with no reader.
///
/// `--pin` is omitted, so the PIN is prompted for. This is the one drive whose
/// prompted value is submitted to the card and judged by a retry counter, which
/// is why it is answered at most once.
#[must_use]
pub fn protect_management_key_arguments(serial: &str) -> Vec<String> {
    device(
        serial,
        &[
            "piv",
            "access",
            "change-management-key",
            "--protect",
            "--generate",
            "-f",
        ],
    )
}

/// The serials in what `ykman list --serials` printed.
///
/// A pure reading of the output, so the enumeration and the refusals over it are
/// asserted without a reader answering.
#[must_use]
pub fn serials_in(printed: &str) -> Vec<String> {
    printed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The access state in what `ykman piv info` printed.
///
/// A pure reading, for the reason [`serials_in`] is one: the question the runtime
/// asks a card is a question about a string, and the coupling to `ykman`'s own
/// sentence is one line rather than a subprocess a test would have to arrange.
#[must_use]
pub fn state_in(reported: &str) -> State {
    if reported.contains(MANAGEMENT_KEY_PROTECTED) {
        return State::Provisioned;
    }
    State::FactoryFresh
}

/// The serial every construction in [`every_argument_vector`] is built over.
const FIXTURE_SERIAL: &str = "00000000";

/// Every word any construction in this module may contain.
///
/// Read by the module's own test rather than by the runtime, which is why it is
/// marked as such: the claim it carries is a claim about this module's source, and
/// a runtime that consulted it would be checking its own arithmetic.
///
/// The other half of what [`every_argument_vector`] is for: a construction that
/// carried a generated credential would carry a word that is not on this list, and
/// the module's own test is what says so. Extending the list is therefore a
/// deliberate act with a reviewer attached, which is the property wanted — the two
/// credential-shaped entries here are the published factory defaults and are named
/// so that adding a *third* cannot pass unnoticed.
#[cfg(test)]
const PUBLIC_WORDS: [&str; 17] = [
    "list",
    "--serials",
    "--device",
    FIXTURE_SERIAL,
    "piv",
    "info",
    "access",
    "change-pin",
    "change-puk",
    "change-management-key",
    "--protect",
    "--generate",
    "-f",
    // The two options that carry a value, and the two values they carry. Both
    // values are the constants every card ships with, documented by Yubico and
    // identical everywhere, so reading one off a process listing grants nothing.
    // Nothing safix generated is here, and nothing safix generated can be added
    // without this list growing under review.
    "-P",
    FACTORY_PIN,
    "-p",
    FACTORY_PUK,
];

/// Every argument vector this module can construct, over one fixture input.
///
/// The instrument behind two claims. "No code path issues an OTP command" is
/// otherwise something about code nobody read; here it is a statement about a list
/// the module's own tests hold complete. So is "no construction carries a
/// credential", which is the claim the change to prompt-driven provisioning
/// exists to make true.
#[must_use]
pub fn every_argument_vector() -> Vec<Vec<String>> {
    vec![
        list_arguments(),
        info_arguments(FIXTURE_SERIAL),
        change_pin_arguments(FIXTURE_SERIAL),
        change_puk_arguments(FIXTURE_SERIAL),
        protect_management_key_arguments(FIXTURE_SERIAL),
    ]
}

/// The device selector every per-card invocation carries, then the rest.
fn device(serial: &str, rest: &[&str]) -> Vec<String> {
    let mut arguments = vec!["--device".to_owned(), serial.to_owned()];
    arguments.extend(rest.iter().map(|word| (*word).to_owned()));
    arguments
}

impl Ykman {
    /// The binary [`PROGRAM_OVERRIDE`] names, or `ykman`.
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            program: std::env::var_os(PROGRAM_OVERRIDE)
                .filter(|value| !value.is_empty())
                .map_or_else(|| PathBuf::from("ykman"), PathBuf::from),
        }
    }

    /// The program name, for a refusal that has to name it.
    #[must_use]
    pub fn program(&self) -> String {
        self.program.display().to_string()
    }

    /// Every connected card's serial, in the order `ykman` lists them.
    ///
    /// # Errors
    ///
    /// [`Error::YkmanUnavailable`] when the binary cannot be run and
    /// [`Error::PcscdUnavailable`] when it runs and finds no smartcard service.
    pub fn serials(&self) -> Result<Vec<String>> {
        let finished = self.capture(&list_arguments())?;
        let complaint = String::from_utf8_lossy(&finished.stderr);
        if complaint.to_lowercase().contains(NO_SMARTCARD_SERVICE) {
            return Err(Error::PcscdUnavailable);
        }
        Ok(serials_in(&String::from_utf8_lossy(&finished.stdout)))
    }

    /// The one card this run acts on.
    ///
    /// A named serial is taken as given and is not checked against the readers,
    /// because the tools this hands it to check it themselves and produce their
    /// own account of a card that is not there. What is refused here is the
    /// ambiguity a bare invocation cannot resolve.
    ///
    /// # Errors
    ///
    /// [`Error::NoCardConnected`] when nothing answers, and
    /// [`Error::CardsAmbiguous`] when more than one does and none was named.
    pub fn select(&self, named: Option<&str>) -> Result<String> {
        if let Some(serial) = named {
            return Ok(serial.to_owned());
        }
        let serials = self.serials()?;
        match serials.as_slice() {
            [] => Err(Error::NoCardConnected),
            [only] => Ok(only.clone()),
            _ => Err(Error::CardsAmbiguous { serials }),
        }
    }

    /// Whether this card's PIV access has been provisioned.
    ///
    /// # Errors
    ///
    /// [`Error::YkmanUnavailable`] when the binary cannot be run and
    /// [`Error::CardCommandFailed`] when it refuses.
    pub fn state(&self, serial: &str) -> Result<State> {
        let arguments = info_arguments(serial);
        let finished = self.capture(&arguments)?;
        if !finished.status.success() {
            return Err(Self::refused(&arguments, &finished));
        }
        Ok(state_in(&String::from_utf8_lossy(&finished.stdout)))
    }

    /// Set the PIN, the PUK and a protected random management key, in that
    /// order.
    ///
    /// The order is not interchangeable. The PUK is changed from the factory one
    /// while the factory one is still in force, and the management key is
    /// protected under the PIN that is by then the new one — so a run that
    /// stopped between the two leaves a card whose PIN is known and whose PUK is
    /// the factory's, which is recoverable, rather than one whose management key
    /// is protected under a PIN nobody set.
    ///
    /// Each new credential is written down a pseudo-terminal rather than into an
    /// argument vector, which is what the whole shape of this function is for.
    ///
    /// # Errors
    ///
    /// [`Error::YkmanUnavailable`] when the binary cannot be run,
    /// [`Error::CardPinRejected`] when a drive asks past its bound, and
    /// [`Error::CardCommandFailed`] carrying `ykman`'s own message when it
    /// refuses.
    pub fn provision(
        &self,
        serial: &str,
        credentials: &Credentials,
        progress: &dyn Progress,
        idle_limit: Duration,
    ) -> Result<()> {
        let Some(puk) = credentials.puk_secret()? else {
            return Err(Error::CardCommandFailed {
                arguments: change_puk_arguments(serial).join(" "),
                output: String::from(
                    "provisioning was asked for with a PIN that safix did not generate",
                ),
            });
        };
        let pin = credentials.pin_secret()?;

        // Two answers each for the two credential changes: `ykman` asks for the
        // new value and then for it again as a confirmation, and both are the one
        // value. One answer for the management key, whose prompt is the only one
        // here whose value the card judges against a retry counter.
        self.prompted(
            serial,
            &change_puk_arguments(serial),
            &puk,
            2,
            progress,
            idle_limit,
        )?;
        self.prompted(
            serial,
            &change_pin_arguments(serial),
            &pin,
            2,
            progress,
            idle_limit,
        )?;
        self.prompted(
            serial,
            &protect_management_key_arguments(serial),
            &pin,
            1,
            progress,
            idle_limit,
        )
    }

    /// One invocation whose prompts are answered on a pseudo-terminal.
    ///
    /// Standard error is the terminal, so `ykman`'s own account of a refusal
    /// arrives in the transcript rather than on a captured pipe — which is where
    /// the refusal below takes it from.
    fn prompted(
        &self,
        serial: &str,
        arguments: &[String],
        answer: &Secret,
        limit: usize,
        progress: &dyn Progress,
        idle_limit: Duration,
    ) -> Result<()> {
        let mut command = Command::new(&self.program);
        command.args(arguments);

        let session = pty::answering(&mut command, answer, limit, serial, progress, idle_limit)
            // The wrapper names the program it could not run as the plugin,
            // because the plugin is its other caller; here it is ykman, and a
            // refusal that named the wrong binary would send the operator to the
            // wrong missing tool.
            .map_err(|refusal| match refusal {
                Error::PluginUnavailable { program, cause } => {
                    Error::YkmanUnavailable { program, cause }
                }
                other => other,
            })?;

        if session.status == 0 {
            return Ok(());
        }
        Err(Error::CardCommandFailed {
            arguments: arguments.join(" "),
            output: session.terminal.trim_end_matches('\n').to_owned(),
        })
    }

    /// One invocation, with both streams captured.
    ///
    /// Captured rather than inherited because one line of standard error is an
    /// outcome this runtime distinguishes and the rest is carried into a refusal
    /// verbatim, so `ykman`'s own account of a card reaches the operator.
    fn capture(&self, arguments: &[String]) -> Result<std::process::Output> {
        Command::new(&self.program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|cause| Error::YkmanUnavailable {
                program: self.program(),
                cause,
            })
    }

    /// A refusal carrying what was said and what came back.
    ///
    /// The arguments go in verbatim, which they can because no construction in
    /// this module carries a credential — [`PUBLIC_WORDS`] is what holds that, and
    /// it is what let the redaction this used to need be deleted rather than
    /// maintained.
    fn refused(arguments: &[String], finished: &std::process::Output) -> Error {
        let complaint = String::from_utf8_lossy(&finished.stderr);
        Error::CardCommandFailed {
            arguments: arguments.join(" "),
            output: complaint.trim_end_matches('\n').to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_construction_reaches_the_otp_applet() {
        for arguments in every_argument_vector() {
            assert!(
                !arguments.iter().any(|word| word == "otp"),
                "an argument vector names the OTP applet: {arguments:?}"
            );
        }
    }

    #[test]
    fn every_per_card_construction_selects_the_card_it_is_about() {
        let vectors = every_argument_vector();
        let per_card: Vec<&Vec<String>> = vectors
            .iter()
            .filter(|arguments| arguments.iter().any(|word| word == "piv"))
            .collect();
        assert_eq!(per_card.len(), 4, "the per-card constructions are four");
        for arguments in per_card {
            assert_eq!(
                arguments.first().map(String::as_str),
                Some("--device"),
                "a per-card invocation does not select the card: {arguments:?}"
            );
        }
    }

    #[test]
    fn the_management_key_is_generated_and_protected_and_never_named() {
        let arguments = protect_management_key_arguments("12345678");
        assert!(arguments.iter().any(|word| word == "--protect"));
        assert!(arguments.iter().any(|word| word == "--generate"));
        assert!(
            !arguments.iter().any(|word| word == "--management-key"),
            "a management key was named, so one was chosen off the card"
        );
        assert!(
            !arguments.iter().any(|word| word == "--pin"),
            "the PIN was passed as a flag rather than prompted for"
        );
    }

    /// No construction carries anything but a word from the published list.
    ///
    /// The whole of the argument-vector claim, and it is stated over the words
    /// rather than over a redaction: a generated credential is by construction not
    /// a word anybody wrote into this module, so a vector holding one holds a word
    /// that is not on the list. That is why the redaction this replaced could be
    /// deleted rather than kept in step.
    #[test]
    fn no_construction_carries_anything_but_a_public_word() {
        for arguments in every_argument_vector() {
            for word in &arguments {
                assert!(
                    PUBLIC_WORDS.contains(&word.as_str()),
                    "a construction carries a word that is not public: {word} in {arguments:?}"
                );
            }
        }
    }

    /// The two option-borne values are the published factory defaults and nothing
    /// else, and no option takes a new credential.
    #[test]
    fn the_only_credential_shaped_options_are_the_factory_defaults() {
        let flags: Vec<String> = every_argument_vector().concat();
        for asks_for_a_new_value in ["-n", "--new-pin", "--new-puk", "--pin", "--management-key"] {
            assert!(
                !flags.iter().any(|word| word == asks_for_a_new_value),
                "{asks_for_a_new_value} would carry a generated credential in argv"
            );
        }
        // `-P` and `-p` do appear, and what follows each is the constant every
        // card ships with rather than anything this run minted.
        let follows = |flag: &str| -> Option<String> {
            flags
                .iter()
                .position(|word| word == flag)
                .and_then(|at| flags.get(at.saturating_add(1)))
                .cloned()
        };
        assert_eq!(follows("-P").as_deref(), Some(FACTORY_PIN));
        assert_eq!(follows("-p").as_deref(), Some(FACTORY_PUK));
    }

    #[test]
    fn a_generated_pin_and_puk_are_eight_digits_and_differ() {
        let credentials = Credentials::generate().expect("the kernel's pool is readable");
        let shown = |value: &Secret| {
            let mut written = Vec::new();
            value.write_to(&mut written).expect("a vec can be written");
            String::from_utf8(written).expect("digits are text")
        };
        let pin = shown(&credentials.pin_secret().expect("a slice can be read"));
        let puk = shown(
            &credentials
                .puk_secret()
                .expect("a slice can be read")
                .expect("generate produces both halves"),
        );
        for value in [&pin, &puk] {
            assert_eq!(value.len(), CREDENTIAL_LENGTH);
            assert!(value.bytes().all(|byte| byte.is_ascii_digit()));
        }
        assert_ne!(pin, puk, "the PUK collapsed into the PIN");
        assert_ne!(pin, FACTORY_PIN);
        assert_ne!(puk, FACTORY_PUK);
        assert!(credentials.generated());
    }

    #[test]
    fn a_pin_from_elsewhere_carries_no_puk_and_so_is_not_ours_to_store() {
        let credentials = Credentials::existing(Zeroizing::new(String::from("87654321")));
        assert!(
            credentials
                .puk_secret()
                .expect("a slice can be read")
                .is_none()
        );
        assert!(!credentials.generated());

        let mut written = Vec::new();
        credentials
            .record()
            .expect("a slice can be read")
            .write_to(&mut written)
            .expect("a vec can be written");
        assert_eq!(
            written, b"PIN=87654321",
            "a record named a PUK that safix never set"
        );
    }

    #[test]
    fn the_custody_record_names_both_halves_when_both_were_generated() {
        let credentials = Credentials {
            pin: Zeroizing::new(String::from("11111111")),
            puk: Some(Zeroizing::new(String::from("22222222"))),
        };
        let mut written = Vec::new();
        credentials
            .record()
            .expect("a slice can be read")
            .write_to(&mut written)
            .expect("a vec can be written");
        assert_eq!(written, b"PIN=11111111\nPUK=22222222");
    }

    /// What `ykman piv info` prints for a card nobody has provisioned.
    const FACTORY_INFO: &str = "\
PIV version:              5.4.3
PIN tries remaining:      3/3
PUK tries remaining:      3/3
Management key algorithm: TDES
CHUID:  no data available
";

    #[test]
    fn a_factory_card_and_a_provisioned_one_read_apart() {
        assert_eq!(state_in(FACTORY_INFO), State::FactoryFresh);

        let provisioned = FACTORY_INFO.replace(
            "CHUID:",
            "Management key is stored on the YubiKey, protected by PIN.\nCHUID:",
        );
        assert_eq!(state_in(&provisioned), State::Provisioned);
    }

    #[test]
    fn a_card_that_says_nothing_about_its_management_key_reads_as_factory_fresh() {
        // The safe direction: a factory reading provisions, and provisioning a
        // card that was already provisioned fails on the factory PIN with one
        // retry spent. The other way round would skip provisioning on a blank
        // card and then prompt for a PIN nobody has set.
        assert_eq!(state_in(""), State::FactoryFresh);
    }

    #[test]
    fn the_serials_are_the_non_empty_lines_and_nothing_else() {
        assert_eq!(
            serials_in("12345678\n87654321\n"),
            vec![String::from("12345678"), String::from("87654321")]
        );
        assert_eq!(
            serials_in("  12345678  \n\n"),
            vec![String::from("12345678")]
        );
        assert!(serials_in("").is_empty());
        assert!(serials_in("\n\n").is_empty());
    }

    #[test]
    fn an_absent_command_is_refused_by_name() {
        let ykman = Ykman {
            program: PathBuf::from("safix-no-such-ykman-command"),
        };
        assert!(matches!(
            ykman.serials(),
            Err(Error::YkmanUnavailable { .. })
        ));
    }

    #[test]
    fn a_named_serial_is_taken_without_asking_the_readers() {
        let ykman = Ykman {
            program: PathBuf::from("safix-no-such-ykman-command"),
        };
        assert_eq!(
            ykman.select(Some("12345678")).ok(),
            Some(String::from("12345678")),
            "a named serial reached for the readers anyway"
        );
    }
}
