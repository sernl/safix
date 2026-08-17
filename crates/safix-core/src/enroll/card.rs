//! The card's PIV access: enumerated, probed, and provisioned by flags alone.
//!
//! Everything `ykman` does to PIV access is non-interactive, so this module is a
//! set of argument vectors and a driver that runs them. The vectors are built by
//! functions that take no card and touch no reader, which is what lets every
//! construction below be asserted without one — the card is present in exactly
//! one place, [`Ykman`], and every claim about *what is said to it* is a claim
//! about a `Vec<String>`.
//!
//! # What is never issued
//!
//! Any OTP command. The two applets are disjoint and safix drives one of them;
//! the other holds the challenge-response secret a password database is opened
//! by, and programming that slot ends the database. [`every_argument_vector`]
//! exists so that "no code path issues an OTP command" is a test rather than a
//! promise.
//!
//! # Where the PIN goes
//!
//! Into `ykman`'s argument vector, because that is the interface `ykman` has:
//! there is no flag that reads a PIN from a pipe, and the alternative is a
//! terminal prompt safix would then be answering on the operator's behalf. That
//! is stated here rather than hidden, and it is why [`Credentials`] is not a
//! [`Secret`](crate::Secret): a type whose only egress is a pipe cannot express
//! a value that has to reach an argument vector, and using one anyway would
//! misdescribe what happens. Every *other* egress — the generator's terminal,
//! the password store, the safix secret — takes [`Credentials::pin_secret`] or
//! [`Credentials::record`] and travels a pipe.
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

use zeroize::Zeroizing;

use crate::error::{Error, Result};
use crate::probe::{DebugFallback as _, DisplayFallback as _, Implements, SerializeFallback as _};
use crate::secret::Secret;

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
/// Not a [`Secret`], deliberately, and the module documentation says why: these
/// reach `ykman`'s argument vector because `ykman` has no other interface for
/// them. What that costs is stated rather than disguised, and everything else
/// this type hands out is a [`Secret`] travelling a pipe.
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

    /// The PIN, on its way into an argument vector.
    ///
    /// The one egress that is not a pipe, named so that every call site reads as
    /// the deliberate act it is. See the module documentation.
    #[must_use]
    pub fn pin_for_flag(&self) -> &str {
        &self.pin
    }

    /// The PUK, on its way into an argument vector, when safix generated one.
    #[must_use]
    pub fn puk_for_flag(&self) -> Option<&str> {
        self.puk.as_ref().map(|puk| puk.as_str())
    }

    /// Whether safix generated these, and so whether they are safix's to store.
    #[must_use]
    pub fn generated(&self) -> bool {
        self.puk.is_some()
    }

    /// The PIN as a value that can only leave down a pipe.
    ///
    /// What the generator's terminal is answered with, and what the password
    /// store receives.
    ///
    /// # Errors
    ///
    /// [`Error::SecretRead`] when the in-memory copy cannot be read, which a
    /// slice cannot fail at.
    pub fn pin_secret(&self) -> Result<Secret> {
        Secret::read_from(&mut self.pin.as_bytes())
    }

    /// Both halves as one value, in the shape a custody entry holds them.
    ///
    /// Two labelled lines rather than a structure, because every reader of this
    /// is a person looking at one entry of a password store or one key of a
    /// safix secret.
    ///
    /// # Errors
    ///
    /// [`Error::SecretRead`] when the in-memory copy cannot be read.
    pub fn record(&self) -> Result<Secret> {
        let mut text = Zeroizing::new(format!("PIN={}\n", self.pin.as_str()));
        if let Some(puk) = &self.puk {
            text.push_str("PUK=");
            text.push_str(puk);
            text.push('\n');
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

/// `ykman --device <serial> piv access change-pin -P <current> -n <new>`.
#[must_use]
pub fn change_pin_arguments(serial: &str, current: &str, new: &str) -> Vec<String> {
    device(
        serial,
        &["piv", "access", "change-pin", "-P", current, "-n", new],
    )
}

/// `ykman --device <serial> piv access change-puk -p <current> -n <new>`.
#[must_use]
pub fn change_puk_arguments(serial: &str, current: &str, new: &str) -> Vec<String> {
    device(
        serial,
        &["piv", "access", "change-puk", "-p", current, "-n", new],
    )
}

/// `ykman --device <serial> piv access change-management-key --protect
/// --generate --pin <pin> -f`.
///
/// `--protect` is what puts the key on the card under the PIN and `--generate`
/// is what makes it random. Together they mean the management key is never a
/// string this process holds, which is why nothing stores it: PIN possession is
/// management possession, and a stored copy would be a credential with no
/// reader.
#[must_use]
pub fn protect_management_key_arguments(serial: &str, pin: &str) -> Vec<String> {
    device(
        serial,
        &[
            "piv",
            "access",
            "change-management-key",
            "--protect",
            "--generate",
            "--pin",
            pin,
            "-f",
        ],
    )
}

/// Every argument vector this module can construct, over one fixture input.
///
/// The instrument behind the OTP refusal. "No code path issues an OTP command"
/// is otherwise a claim about code nobody read; here it is a claim about a list
/// the compiler makes this module keep complete, because a construction added
/// without a line here is a construction no test covers and the module's own
/// test says so.
#[must_use]
pub fn every_argument_vector() -> Vec<Vec<String>> {
    vec![
        list_arguments(),
        info_arguments("00000000"),
        change_pin_arguments("00000000", FACTORY_PIN, "11111111"),
        change_puk_arguments("00000000", FACTORY_PUK, "22222222"),
        protect_management_key_arguments("00000000", "11111111"),
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
        Ok(String::from_utf8_lossy(&finished.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect())
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
        let reported = String::from_utf8_lossy(&finished.stdout);
        if reported.contains(MANAGEMENT_KEY_PROTECTED) {
            return Ok(State::Provisioned);
        }
        Ok(State::FactoryFresh)
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
    /// # Errors
    ///
    /// [`Error::YkmanUnavailable`] when the binary cannot be run and
    /// [`Error::CardCommandFailed`] carrying `ykman`'s own message when it
    /// refuses.
    pub fn provision(&self, serial: &str, credentials: &Credentials) -> Result<()> {
        let Some(puk) = credentials.puk_for_flag() else {
            return Err(Error::CardCommandFailed {
                arguments: change_puk_arguments(serial, FACTORY_PUK, "<generated>").join(" "),
                output: String::from(
                    "provisioning was asked for with a PIN that safix did not generate",
                ),
            });
        };
        let pin = credentials.pin_for_flag();

        self.run(&change_puk_arguments(serial, FACTORY_PUK, puk))?;
        self.run(&change_pin_arguments(serial, FACTORY_PIN, pin))?;
        self.run(&protect_management_key_arguments(serial, pin))
    }

    /// One invocation, refusing on anything but success.
    fn run(&self, arguments: &[String]) -> Result<()> {
        let finished = self.capture(arguments)?;
        if finished.status.success() {
            return Ok(());
        }
        Err(Self::refused(arguments, &finished))
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
    /// The arguments are redacted of anything that is not public: a PIN reaches
    /// `ykman`'s argument vector because it must, and a refusal is a string that
    /// travels further than one process, so it does not reach that.
    fn refused(arguments: &[String], finished: &std::process::Output) -> Error {
        let complaint = String::from_utf8_lossy(&finished.stderr);
        Error::CardCommandFailed {
            arguments: redacted(arguments).join(" "),
            output: complaint.trim_end_matches('\n').to_owned(),
        }
    }
}

/// An argument vector with every credential replaced by a placeholder.
///
/// The flags that carry one are named here rather than inferred, so a flag added
/// to a construction above without a line here shows up as a credential in a
/// message — which the module's own test is what catches.
#[must_use]
pub fn redacted(arguments: &[String]) -> Vec<String> {
    const CARRIES_A_CREDENTIAL: [&str; 4] = ["-P", "-p", "-n", "--pin"];
    let mut out: Vec<String> = Vec::with_capacity(arguments.len());
    let mut hide_next = false;
    for argument in arguments {
        if hide_next {
            out.push(String::from("<redacted>"));
            hide_next = false;
            continue;
        }
        hide_next = CARRIES_A_CREDENTIAL.contains(&argument.as_str());
        out.push(argument.clone());
    }
    out
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
        let arguments = protect_management_key_arguments("12345678", "87654321");
        assert!(arguments.iter().any(|word| word == "--protect"));
        assert!(arguments.iter().any(|word| word == "--generate"));
        assert!(
            !arguments.iter().any(|word| word == "--management-key"),
            "a management key was named, so one was chosen off the card"
        );
    }

    #[test]
    fn a_refusal_names_the_flags_and_never_the_credentials_behind_them() {
        // A serial with no digit run in common with either credential, so
        // "the credential is absent" and "the serial is present" are separable
        // claims: `12345678` would contain the factory PIN as a substring.
        let shown = redacted(&change_pin_arguments("99887766", "123456", "44332211")).join(" ");
        assert!(shown.contains("-P <redacted> -n <redacted>"));
        assert!(!shown.contains("123456"));
        assert!(!shown.contains("44332211"));
        assert!(shown.contains("--device 99887766"), "the serial is public");
    }

    #[test]
    fn every_flag_that_carries_a_credential_hides_it() {
        for arguments in every_argument_vector() {
            let shown = redacted(&arguments).join(" ");
            for (flag, value) in [
                ("-P", FACTORY_PIN),
                ("-p", FACTORY_PUK),
                ("-n", "11111111"),
                ("--pin", "11111111"),
            ] {
                if arguments.iter().any(|word| word == flag) {
                    assert!(!shown.contains(value), "{flag} left its value in {shown}");
                }
            }
        }
    }

    #[test]
    fn a_generated_pin_and_puk_are_eight_digits_and_differ() {
        let credentials = Credentials::generate().expect("the kernel's pool is readable");
        let pin = credentials.pin_for_flag().to_owned();
        let puk = credentials
            .puk_for_flag()
            .expect("generate produces both halves")
            .to_owned();
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
        assert_eq!(credentials.pin_for_flag(), "87654321");
        assert_eq!(credentials.puk_for_flag(), None);
        assert!(!credentials.generated());
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
        assert_eq!(written, b"PIN=11111111\nPUK=22222222\n");
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
