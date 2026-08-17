//! One age identity in one retired slot, and the block that names it.
//!
//! The generator is `age-plugin-yubikey --generate`, driven under
//! [`pty`] because it reads the PIN from a terminal. Everything else
//! it needs is a flag, so the argument vector below is built without a card and
//! is asserted without one.
//!
//! # What comes back, and from where
//!
//! An identity block on standard output. It is a document rather than a message:
//! six comment lines of metadata, one of which names the recipient, and one line
//! holding the stub — which is a pointer to a slot on a card and not a private
//! key, because the private key never leaves the card and cannot.
//!
//! The recipient is read out of that block's own `Recipient:` comment. The
//! plugin also echoes it to standard error when standard output is not a
//! terminal, which under the wrapper it is not, so the same string arrives twice;
//! the block is what is read, because it is the thing being appended and a
//! recipient read from anywhere else could disagree with the block it is filed
//! beside.
//!
//! # Where the block goes
//!
//! Onto the end of the same identity file [`keygen`](crate::keygen) appends to,
//! under [`keygen`](crate::keygen)'s discipline: appended, never truncated, mode
//! `0600`. sops tries every identity in that file, so a card's stub beside the
//! software identities is a working state and the card becomes a peer of them —
//! which is what makes it capable of being the only one left, later, by an act
//! that is not this one's.
//!
//! # The policies, and the one that is refused
//!
//! `pin-policy once` and `touch-policy cached`, which is what the fleet's
//! already-enrolled cards carry. `touch-policy never` is refused rather than
//! passed through: a card that decrypts without a touch is a file with a
//! smartcard's latency, and the touch is the property being bought.

use std::process::Command;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::progress::Progress;
use crate::secret::Secret;

use super::pty;

/// The environment variable that replaces the program, for checks.
pub const PROGRAM_OVERRIDE: &str = "SAFIX_AGE_PLUGIN_YUBIKEY";

/// The program a run reaches for when nothing overrides it.
pub const PROGRAM: &str = "age-plugin-yubikey";

/// The PIN policy the fleet's cards carry.
pub const DEFAULT_PIN_POLICY: &str = "once";

/// The touch policy the fleet's cards carry.
pub const DEFAULT_TOUCH_POLICY: &str = "cached";

/// The touch policy that is refused rather than accepted.
pub const REFUSED_TOUCH_POLICY: &str = "never";

/// The line prefix the identity block names the recipient on.
///
/// Matched with its surrounding whitespace stripped, because it is a comment the
/// plugin indents for readability and the indentation is not part of the datum.
const RECIPIENT_LABEL: &str = "Recipient:";

/// The prefix of the one line of an identity block that is not a comment.
const STUB_PREFIX: &str = "AGE-PLUGIN-YUBIKEY-";

/// The prefix of an age recipient that names a card.
pub const HARDWARE_RECIPIENT_PREFIX: &str = "age1yubikey1";

/// What one generation was asked for.
#[derive(Debug, Clone)]
pub struct Request {
    /// The card, by serial.
    pub serial: String,
    /// The person the identity is named for.
    pub user: String,
    /// The retired slot to use, or the first empty one when absent.
    pub slot: Option<String>,
    /// The PIN policy, defaulting to [`DEFAULT_PIN_POLICY`].
    pub pin_policy: String,
    /// The touch policy, defaulting to [`DEFAULT_TOUCH_POLICY`].
    pub touch_policy: String,
}

impl Request {
    /// A request carrying the fleet's measured policies.
    #[must_use]
    pub fn new(user: &str, serial: &str) -> Self {
        Self {
            serial: serial.to_owned(),
            user: user.to_owned(),
            slot: None,
            pin_policy: DEFAULT_PIN_POLICY.to_owned(),
            touch_policy: DEFAULT_TOUCH_POLICY.to_owned(),
        }
    }

    /// The name the identity carries on the card.
    ///
    /// The person and the serial, because a card holds up to twenty identities
    /// and a slot listing that named only the tool would say nothing about which
    /// person's custody each one is.
    #[must_use]
    pub fn name(&self) -> String {
        format!("safix {} {}", self.user, self.serial)
    }
}

/// The identity block and the recipient inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// Every line of the block, as the plugin printed it, ending in a newline.
    pub block: String,
    /// The recipient the block names.
    pub recipient: String,
}

/// The argument vector `--generate` is driven with.
///
/// # Errors
///
/// [`Error::TouchPolicyNever`] when the touch is asked to be skipped.
pub fn generate_arguments(request: &Request) -> Result<Vec<String>> {
    if request.touch_policy == REFUSED_TOUCH_POLICY {
        return Err(Error::TouchPolicyNever);
    }

    let mut arguments = vec![
        "--generate".to_owned(),
        "--serial".to_owned(),
        request.serial.clone(),
        "--name".to_owned(),
        request.name(),
        "--pin-policy".to_owned(),
        request.pin_policy.clone(),
        "--touch-policy".to_owned(),
        request.touch_policy.clone(),
    ];
    // Absent rather than computed: the plugin picks the first empty retired slot
    // itself, and a slot this side chose would be a second answer to a question
    // only the card can answer.
    if let Some(slot) = &request.slot {
        arguments.push("--slot".to_owned());
        arguments.push(slot.clone());
    }
    Ok(arguments)
}

/// The identity block and its recipient, out of what the plugin printed.
///
/// The block is every line from the first comment through the stub. Anything
/// before or after it — a warning, a blank line, whatever a future version adds
/// around it — is not part of the identity and is not appended to a file sops
/// reads.
#[must_use]
pub fn capture(printed: &str) -> Option<Captured> {
    let stub = printed
        .lines()
        .find(|line| line.trim_start().starts_with(STUB_PREFIX))?
        .trim()
        .to_owned();

    let mut block = String::new();
    for line in printed.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            block.push_str(line);
            block.push('\n');
            continue;
        }
        if trimmed.starts_with(STUB_PREFIX) {
            break;
        }
        // A blank or unrecognised line before the block resets it: the metadata
        // comments are contiguous with the stub they describe.
        block.clear();
    }
    block.push_str(&stub);
    block.push('\n');

    let recipient = printed.lines().find_map(|line| {
        line.trim_start_matches(['#', ' ', '\t'])
            .trim()
            .strip_prefix(RECIPIENT_LABEL)
            .map(|rest| rest.trim().to_owned())
    })?;
    if !recipient.starts_with(HARDWARE_RECIPIENT_PREFIX) {
        return None;
    }

    Some(Captured { block, recipient })
}

/// Generate the identity, answering the PIN prompt once, and capture the block.
///
/// # Errors
///
/// [`Error::TouchPolicyNever`] before anything runs,
/// [`Error::PluginUnavailable`] when the plugin cannot be run,
/// [`Error::CardPinRejected`] when the PIN was refused,
/// [`Error::PluginFailed`] when the plugin ran and refused, and
/// [`Error::PluginNoIdentity`] when it ran, succeeded, and printed no block.
pub fn generate(
    request: &Request,
    pin: &Secret,
    progress: &dyn Progress,
    idle_limit: Duration,
) -> Result<Captured> {
    let arguments = generate_arguments(request)?;
    let mut command = Command::new(program());
    command.args(&arguments);

    // One prompt, so a second one is a rejected PIN: the plugin asks once per
    // generation and asks again only when the card refused what it was given. The
    // bound is what keeps the run from walking a card's three retries to zero.
    let session = pty::answering(&mut command, pin, 1, &request.serial, progress, idle_limit)?;

    if session.status != 0 {
        return Err(Error::PluginFailed {
            status: session.status,
        });
    }
    capture(&String::from_utf8_lossy(&session.stdout)).ok_or(Error::PluginNoIdentity)
}

/// The plugin binary [`PROGRAM_OVERRIDE`] names, or [`PROGRAM`].
#[must_use]
pub fn program() -> std::path::PathBuf {
    std::env::var_os(PROGRAM_OVERRIDE)
        .filter(|value| !value.is_empty())
        .map_or_else(
            || std::path::PathBuf::from(PROGRAM),
            std::path::PathBuf::from,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `age-plugin-yubikey --generate` prints on standard output, in the
    /// shape `util::print_identity` renders it: the metadata comments, the
    /// recipient comment, then the stub. Synthetic throughout — the stub and the
    /// recipient are fixture strings and open nothing.
    const PRINTED: &str = "\
#       Serial: 12345678, Slot: 1
#         Name: safix ana 12345678
#      Created: Mon, 17 Aug 2026 00:00:00 +0000
#   PIN policy: Once   (A PIN is required once per session)
# Touch policy: Cached (A physical touch is required for decryption, and is cached for 15 seconds)
#    Recipient: age1yubikey1qfixture000000000000000000000000000000000000000000000000
AGE-PLUGIN-YUBIKEY-1QFIXTURE000000000000000000
";

    #[test]
    fn the_argument_vector_carries_the_serial_the_name_and_both_policies() {
        let arguments = generate_arguments(&Request::new("ana", "12345678"))
            .expect("the default policies are accepted");
        assert_eq!(
            arguments,
            vec![
                "--generate",
                "--serial",
                "12345678",
                "--name",
                "safix ana 12345678",
                "--pin-policy",
                "once",
                "--touch-policy",
                "cached",
            ]
        );
    }

    #[test]
    fn no_slot_is_named_so_the_plugin_picks_the_first_empty_one() {
        let arguments = generate_arguments(&Request::new("ana", "12345678"))
            .expect("the default policies are accepted");
        assert!(!arguments.iter().any(|word| word == "--slot"));

        let mut named = Request::new("ana", "12345678");
        named.slot = Some(String::from("3"));
        let arguments = generate_arguments(&named).expect("a named slot is accepted");
        assert!(arguments.windows(2).any(|pair| pair == ["--slot", "3"]));
    }

    #[test]
    fn a_touch_policy_of_never_is_refused_before_anything_runs() {
        let mut request = Request::new("ana", "12345678");
        request.touch_policy = String::from("never");
        assert!(matches!(
            generate_arguments(&request),
            Err(Error::TouchPolicyNever)
        ));
    }

    #[test]
    fn the_block_and_the_recipient_come_out_of_what_the_plugin_printed() {
        let captured = capture(PRINTED).expect("the fixture is a whole block");
        assert_eq!(
            captured.recipient,
            "age1yubikey1qfixture000000000000000000000000000000000000000000000000"
        );
        assert_eq!(captured.block, PRINTED);
        assert!(
            captured.block.ends_with('\n'),
            "the block ends in a newline"
        );
    }

    #[test]
    fn the_block_holds_no_private_key_because_there_is_none_to_hold() {
        let captured = capture(PRINTED).expect("the fixture is a whole block");
        assert!(!captured.block.contains("AGE-SECRET-KEY"));
        assert!(captured.block.contains(STUB_PREFIX));
    }

    #[test]
    fn chatter_around_the_block_is_not_part_of_it() {
        let noisy = format!("a warning nobody asked for\n\n{PRINTED}\nand a trailing note\n");
        let captured = capture(&noisy).expect("the block is still in there");
        assert_eq!(captured.block, PRINTED);
    }

    #[test]
    fn output_with_no_stub_captures_nothing() {
        assert_eq!(capture("#    Recipient: age1yubikey1qq\n"), None);
        assert_eq!(capture(""), None);
    }

    #[test]
    fn a_block_whose_recipient_is_not_a_cards_captures_nothing() {
        let software = PRINTED.replace("age1yubikey1q", "age1q");
        assert_eq!(capture(&software), None);
    }

    #[test]
    fn the_identity_is_named_for_the_person_and_the_card() {
        assert_eq!(Request::new("ana", "12345678").name(), "safix ana 12345678");
    }
}
