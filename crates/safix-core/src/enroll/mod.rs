//! Enrolling one hardware key, from a blank card to a proven recovery identity.
//!
//! The manual ceremony this replaces was seven steps and ended with nothing
//! asserted: provision access, generate the identity, append the block, edit the
//! recovery list, regenerate the policy, re-wrap, register with clan. Every one
//! of those is automated below, in that order, and one step is added at the end
//! that the ceremony never had — the card opening a file it is now a recipient
//! of. An enrollment without that proof is an enrollment whose only evidence is
//! that a public string was copied correctly.
//!
//! # The one act that stays the operator's
//!
//! The touch. It is not automatable and must not be: a card enrolled with
//! `touch-policy never` so that something could run unattended is a smartcard
//! emulating a file, and the touch is the property the card was bought for. So
//! the run refuses without a terminal to instruct on, and [`identity`] refuses
//! `touch-policy never` outright.
//!
//! # What is additive, and what that costs
//!
//! Everything. A recipient is appended, an identity block is appended, a name is
//! declared; nothing is removed and nothing is replaced, in any file, on any
//! path. A second card is a second run of the same verb and neither run knows
//! about the other. The cost is that enrollment cannot revoke — but re-wrapping
//! never could, and [`fix`] says so at length for the same reason.
//!
//! # The applet that is never touched
//!
//! OTP. The card's other applet holds the challenge-response secret a password
//! database is opened by, and writing that slot ends the database permanently.
//! [`card::every_argument_vector`] is how "no code path issues an OTP command" is
//! a test, and [`Error::OtpRefused`] is what asking for one gets.

pub mod card;
pub mod custody;
pub mod declaration;
pub mod identity;
pub mod proof;
pub mod pty;

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::{Error, Result};
use crate::progress::{Progress, log, note};
use crate::secret::Secret;
use crate::workspace::Workspace;
use crate::{fix, git, keygen, scratch, set, sops, staging};

/// What an invocation asked for beyond the person.
#[derive(Debug, Clone)]
pub struct Options {
    /// The card, when more than one is connected or a particular one is meant.
    pub serial: Option<String>,
    /// The retired slot, when the first empty one is not what is wanted.
    pub slot: Option<String>,
    /// The PIN policy the identity is generated with.
    pub pin_policy: String,
    /// The touch policy the identity is generated with.
    pub touch_policy: String,
    /// Whether the generated credentials become a safix secret. On by default.
    pub store_pin: bool,
    /// Whether and how the credentials are mirrored to the password store.
    pub mirror: custody::Wish,
    /// Whether a disk-backed filesystem is acceptable for the proof's identity
    /// source.
    pub allow_disk_staging: bool,
    /// How long the generator may say nothing before the run gives up.
    pub idle_limit: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            serial: None,
            slot: None,
            pin_policy: identity::DEFAULT_PIN_POLICY.to_owned(),
            touch_policy: identity::DEFAULT_TOUCH_POLICY.to_owned(),
            store_pin: true,
            mirror: custody::Wish::default(),
            allow_disk_staging: false,
            idle_limit: pty::DEFAULT_IDLE_LIMIT,
        }
    }
}

/// What the operator answers, for the two questions a run can have.
pub trait Operator: custody::DatabasePassword {
    /// The PIN of a card safix did not provision.
    ///
    /// Asked once, and only for a card whose access is already set: safix
    /// generated nothing for it and holds nothing for it, so the alternative is
    /// refusing to enroll a second identity onto a card that already works.
    ///
    /// # Errors
    ///
    /// Whatever reading it failed with.
    fn card_pin(&mut self, serial: &str) -> Result<Secret>;
}

/// What one run did.
#[derive(Debug, Clone)]
pub struct Run {
    /// Whether the card was shown to open a governed file.
    pub proven: bool,
}

/// Whether the operator has a terminal to be instructed on.
///
/// Standard error first, because that is where the instruction goes, then
/// `/dev/tty`, because a run whose standard error is redirected to a file still
/// has an operator when a controlling terminal is there. The second is probed by
/// opening it for writing, which is the command's own prompt probe.
#[must_use]
pub fn terminal_present() -> bool {
    rustix::termios::isatty(std::io::stderr())
        || std::fs::File::options()
            .write(true)
            .open("/dev/tty")
            .is_ok()
}

/// Enroll one card for one person, and prove it.
///
/// # Errors
///
/// [`Error::NoTerminal`] before the card is touched, then every refusal on the
/// path: [`Error::UnknownUser`], [`Error::NoCardConnected`],
/// [`Error::CardsAmbiguous`], [`Error::PcscdUnavailable`],
/// [`Error::CardCommandFailed`], [`Error::TouchPolicyNever`],
/// [`Error::CardPinRejected`], [`Error::PluginFailed`],
/// [`Error::NoDeclarationFile`], [`Error::RecipientsLost`] and
/// [`Error::EnrollHookFailed`] among them. A proof that does not pass is not one
/// of them: it comes back in [`Run::proven`].
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    operator: &mut dyn Operator,
    user: &str,
    options: &Options,
) -> Result<Run> {
    if !terminal_present() {
        return Err(Error::NoTerminal);
    }

    scratch::set_floor(workspace.root());
    let _guard = scratch::Guard;

    let ykman = card::Ykman::from_environment();
    let mut ceremony = Ceremony {
        workspace: reload(workspace),
        progress,
        user,
        serial: ykman.select(options.serial.as_deref())?,
        options,
    };
    ceremony.workspace.require_user(user)?;

    let access = ceremony.provision(&ykman, operator)?;
    let captured = ceremony.generate(&access.pin)?;
    ceremony.append_identity(&captured)?;
    let stored = ceremony.wire(&captured)?;

    // Every read past the edit goes through a workspace of its own. The one built
    // above cached the placements, audiences and governed set from before the
    // declaration changed, which is exactly the state the edit invalidated: the
    // name just declared does not resolve against the old cache, and the audience
    // the card just joined is not in it either.
    ceremony.workspace = reload(&ceremony.workspace);

    ceremony.register_with_clan(&captured.recipient)?;
    ceremony.run_hook(&captured.recipient)?;

    if let (Some(name), Some(credentials)) = (stored, access.credentials.as_ref()) {
        ceremony.keep(operator, &name, credentials)?;
    }

    let outcome = ceremony.prove(&captured)?;
    progress.write(&epilogue(
        user,
        &ceremony.serial,
        &captured.recipient,
        &outcome,
    ));
    Ok(Run {
        proven: outcome.proven(),
    })
}

/// A second view of the same repository, with nothing evaluated yet.
fn reload(workspace: &Workspace) -> Workspace {
    Workspace::at(
        workspace.root().to_path_buf(),
        workspace.git().clone(),
        workspace.nix().clone(),
        workspace.sops().clone(),
    )
}

/// The PIN, and the credentials behind it when safix is what generated them.
struct Access {
    pin: Secret,
    credentials: Option<card::Credentials>,
}

/// One run's shared context: the repository, the person, the card, the options.
///
/// A structure rather than five arguments repeated through seven functions, and
/// the workspace is owned because it is replaced partway: the declarations change
/// under the run, and the cached evaluation from before them is not a view of
/// what follows.
struct Ceremony<'run> {
    workspace: Workspace,
    progress: &'run dyn Progress,
    user: &'run str,
    serial: String,
    options: &'run Options,
}

impl Ceremony<'_> {
    /// Set the card's access, or take the PIN of a card that already has it.
    fn provision(&self, ykman: &card::Ykman, operator: &mut dyn Operator) -> Result<Access> {
        let serial = &self.serial;
        match ykman.state(serial)? {
            card::State::Provisioned => {
                note(
                    self.progress,
                    &format!(
                        "{serial} is already provisioned, so nothing about its access is \
                        changed. Its PIN is needed to generate an identity."
                    ),
                );
                Ok(Access {
                    pin: operator.card_pin(serial)?,
                    credentials: None,
                })
            }
            card::State::FactoryFresh => {
                log(
                    self.progress,
                    &format!(
                        "safix: {serial} is factory-fresh. Generating a PIN and a distinct \
                        PUK, and putting a random management key on the card under the PIN."
                    ),
                );
                let credentials = card::Credentials::generate()?;
                ykman.provision(serial, &credentials, self.progress, self.options.idle_limit)?;
                note(
                    self.progress,
                    "the management key is on the card and nowhere else: PIN possession is \
                    management possession, so a stored copy would be a credential with no \
                    reader.",
                );
                Ok(Access {
                    pin: credentials.pin_secret()?,
                    credentials: Some(credentials),
                })
            }
        }
    }

    /// The identity, generated under a pseudo-terminal that supplies the PIN.
    fn generate(&self, pin: &Secret) -> Result<identity::Captured> {
        let mut request = identity::Request::new(self.user, &self.serial);
        request.slot.clone_from(&self.options.slot);
        request.pin_policy.clone_from(&self.options.pin_policy);
        request.touch_policy.clone_from(&self.options.touch_policy);

        log(
            self.progress,
            &format!(
                "safix: generating an age identity on {} for {}. You will be asked to touch \
                the card.",
                self.serial, self.user
            ),
        );
        identity::generate(&request, pin, self.progress, self.options.idle_limit)
    }

    /// The block onto the end of the identity file sops already reads.
    fn append_identity(&self, captured: &identity::Captured) -> Result<()> {
        let keyfile = keygen::identity_file();
        keygen::prepare_identity_file(&keyfile, self.progress)?;
        keygen::append_to_identity_file(&keyfile, &captured.block)?;
        log(
            self.progress,
            &format!(
                "safix: appended the card's identity to {}. It holds no private key: the key \
                is on the card and cannot leave it.",
                keyfile.display()
            ),
        );
        Ok(())
    }

    /// Edit the declaration, regenerate the policy, re-wrap, commit the lot.
    ///
    /// Returns the name the credentials will be set under, when one was declared.
    fn wire(&mut self, captured: &identity::Captured) -> Result<Option<String>> {
        let relative = crate::adduser::scaffold_path(self.user);
        let absolute = self.workspace.absolute(&relative);
        let original =
            self.workspace
                .read_relative(&relative)?
                .ok_or_else(|| Error::NoDeclarationFile {
                    user: self.user.to_owned(),
                    file: relative.clone(),
                })?;

        let mut edited = original.clone();
        edited = self.add_recovery(&relative, edited, &captured.recipient)?;
        let stored = if self.options.store_pin {
            let name = custody::secret_name(&self.serial);
            edited = self.add_private(&relative, edited, &name)?;
            Some(name)
        } else {
            None
        };

        // Recipients present before the re-wrap, per governed file, so the claim
        // that nothing lost the ability to open what it could open is made against
        // the state before rather than against the declarations — which is the
        // direction that catches a re-wrap dropping a stanza.
        let before = recipients_before(&self.workspace)?;

        if edited != original {
            std::fs::write(&absolute, &edited).map_err(|cause| Error::FileUnwritable {
                path: absolute.display().to_string(),
                cause,
            })?;
            if !self.workspace.nix().parses(&absolute) {
                // Put back rather than left broken: the file was a valid record a
                // moment ago and the edit is this module's, so an edit that does
                // not parse is this module's to undo.
                let _ = std::fs::write(&absolute, &original);
                return Err(Error::Unparsable {
                    path: absolute.display().to_string(),
                });
            }
            // Staged before the policy is regenerated, for the reason `adduser`
            // states at length: an evaluation reads the files git tracks, so
            // regenerating first writes the policy of the declarations as they
            // stood without this recipient.
            self.workspace
                .git()
                .stage(self.workspace.root(), std::slice::from_ref(&relative))?;
        }

        self.workspace = reload(&self.workspace);
        let status = fix::run(&self.workspace, self.progress, true)?;
        if status != 0 {
            return Err(Error::RewrapUnschedulable {
                cause: format!("a governed file's re-wrap exited {status}; nothing was committed"),
            });
        }
        refuse_lost_recipients(&self.workspace, &before)?;

        // The governed files that exist, because a governed file is a path a
        // declaration implies rather than a file anybody has written yet, and
        // staging one that is not there refuses the whole commit.
        let mut written = vec![relative, String::from(".sops.yaml")];
        written.extend(
            self.workspace
                .governed_files()?
                .managed
                .iter()
                .filter(|governed| self.workspace.absolute(governed).exists())
                .cloned(),
        );
        git::commit_written_files(
            self.workspace.git(),
            self.workspace.root(),
            self.progress,
            &format!(
                "feat(safix): enroll {} as a recovery recipient for {}",
                self.serial, self.user
            ),
            &written,
        )?;

        Ok(stored)
    }

    /// The card onto `recoveryRecipients`, or a note that it was already there.
    fn add_recovery(&self, relative: &str, text: String, recipient: &str) -> Result<String> {
        match declaration::add_recovery_recipient(&text, recipient) {
            declaration::Edit::Inserted(edited) => {
                log(
                    self.progress,
                    &format!(
                        "safix: adding the card to {}'s recoveryRecipients in {relative}",
                        self.user
                    ),
                );
                Ok(edited)
            }
            declaration::Edit::AlreadyPresent => {
                note(
                    self.progress,
                    "the card's recipient is already in that list; nothing was added to it.",
                );
                Ok(text)
            }
            declaration::Edit::NoAnchor => Err(Error::NoDeclarationFile {
                user: self.user.to_owned(),
                file: relative.to_owned(),
            }),
        }
    }

    /// The custody name onto `private`, so the write path can resolve it.
    fn add_private(&self, relative: &str, text: String, name: &str) -> Result<String> {
        match declaration::add_private_entry(&text, name) {
            declaration::Edit::Inserted(edited) => {
                log(
                    self.progress,
                    &format!(
                        "safix: declaring {name} under {}'s private entries",
                        self.user
                    ),
                );
                Ok(edited)
            }
            declaration::Edit::AlreadyPresent => Ok(text),
            declaration::Edit::NoAnchor => Err(Error::NoDeclarationFile {
                user: self.user.to_owned(),
                file: relative.to_owned(),
            }),
        }
    }

    /// Hand the recipient to clan's own command, when a clan is declared.
    fn register_with_clan(&self, recipient: &str) -> Result<()> {
        let Some(flake) = self.workspace.bridge()?.clan_flake.clone() else {
            note(
                self.progress,
                "no clan is declared, so nothing was registered with one.",
            );
            return Ok(());
        };
        let clan = crate::clan::Clan::new(flake);
        clan.probe()?;
        log(
            self.progress,
            &format!(
                "safix: registering the card with clan as {}'s key",
                self.user
            ),
        );
        clan.register_user(self.user, recipient)
    }

    /// The consumer's own wiring, after safix's commit has landed.
    fn run_hook(&self, recipient: &str) -> Result<()> {
        let hook = self.workspace.enroll_hook()?;
        if hook.is_empty() {
            note(
                self.progress,
                "flake.safix.enrollHook is unset, so nothing further ran.",
            );
            return Ok(());
        }

        log(self.progress, "safix: running flake.safix.enrollHook");
        let status = Command::new("bash")
            .arg("-euo")
            .arg("pipefail")
            .arg("-c")
            .arg(hook)
            .arg("safix-enroll-hook")
            .arg(self.user)
            .arg(&self.serial)
            .arg(recipient)
            .current_dir(self.workspace.root())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|_| Error::EnrollHookFailed { status: 127 })?
            .code()
            .unwrap_or(1);

        if status != 0 {
            return Err(Error::EnrollHookFailed { status });
        }
        note(
            self.progress,
            "the hook ran; anything it wrote is uncommitted and yours to review.",
        );
        Ok(())
    }

    /// Put the generated credentials where they can be found again.
    fn keep(
        &self,
        operator: &mut dyn Operator,
        name: &str,
        credentials: &card::Credentials,
    ) -> Result<()> {
        log(
            self.progress,
            &format!(
                "safix: storing the card's PIN and PUK as {}'s {name}",
                self.user
            ),
        );
        note(
            self.progress,
            "a PIN readable by the software identity adds protection only once that identity \
            is retired or absent. --no-store-pin turns this off.",
        );

        let mut source = Held {
            record: Some(credentials.record()?),
        };
        let status = set::run_committing(
            &self.workspace,
            self.progress,
            &mut source,
            self.user,
            name,
            &format!(
                "chore(safix): store the PIV access for {} in {}'s custody",
                self.serial, self.user
            ),
        )?;
        if status != 0 {
            return Err(Error::StoreMirrorFailed {
                transport: "the person's own safix custody",
                status,
                output: String::from("sops refused the write; its own message is above"),
            });
        }

        let transport = custody::choose(&self.options.mirror, custody::service_reachable());
        if let custody::Transport::Skipped { reason } = &transport {
            note(
                self.progress,
                &format!("no password-store mirror: {reason}."),
            );
            return Ok(());
        }
        custody::write(
            &transport,
            self.user,
            &self.serial,
            &credentials.record()?,
            operator,
        )?;
        note(
            self.progress,
            "the credentials also reached the password store.",
        );
        Ok(())
    }

    /// The card alone, against a file the person's audience governs.
    fn prove(&self, captured: &identity::Captured) -> Result<proof::Outcome> {
        let stub = proof::stub_of(&captured.block).ok_or(Error::PluginNoIdentity)?;
        let relative = proof::file_to_prove_with(&self.workspace, self.user)?;

        let staging = staging::Staging::establish(self.options.allow_disk_staging)?;
        let directory = staging.directory(Path::new("proof"))?;
        let source = proof::write_isolated_source(&directory, &stub)?;

        log(
            self.progress,
            &format!(
                "safix: proving the card opens {relative} with no software identity \
                reachable. Touch the card when asked."
            ),
        );
        proof::decrypt_with(&self.workspace, &source, &relative)
    }
}

/// Every governed file's recipients as they stand now.
fn recipients_before(workspace: &Workspace) -> Result<Vec<(String, Vec<String>)>> {
    let mut found = Vec::new();
    for relative in &workspace.governed_files()?.managed {
        let path = workspace.absolute(relative);
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|cause| Error::FileUnreadable {
            path: path.display().to_string(),
            cause,
        })?;
        if let Ok(recipients) = sops::document::recipients_of(&text) {
            found.push((relative.clone(), recipients));
        }
    }
    Ok(found)
}

/// Refuse a re-wrap that dropped a recipient a file had before it.
///
/// The other half of "additive always", and the half a re-wrap could break
/// without anything else noticing: a file whose stanza for somebody was removed
/// is a file they can no longer open, and the policy's own drift report cannot
/// distinguish that from a narrowing the declarations asked for.
fn refuse_lost_recipients(workspace: &Workspace, before: &[(String, Vec<String>)]) -> Result<()> {
    for (relative, had) in before {
        let path = workspace.absolute(relative);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(now) = sops::document::recipients_of(&text) else {
            continue;
        };
        let lost: Vec<String> = had
            .iter()
            .filter(|recipient| !now.contains(recipient))
            .cloned()
            .collect();
        if !lost.is_empty() {
            return Err(Error::RecipientsLost {
                file: relative.clone(),
                lost,
            });
        }
    }
    Ok(())
}

/// A value already in hand, for the one write path that takes a source.
struct Held {
    record: Option<Secret>,
}

impl set::ValueSource for Held {
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        self.record.take().ok_or(Error::NoValueRead)
    }
}

/// What was done, what it proves, and what is still outstanding.
fn epilogue(user: &str, serial: &str, recipient: &str, outcome: &proof::Outcome) -> String {
    let mut text = format!(
        "\nsafix: {serial} is enrolled for {user}.\n\
        \n\
        What was done:\n\
        \x20 - the card's age identity, in its first empty retired slot\n\
        \x20 - its stub appended to your identity file, beside whatever was there\n\
        \x20 - its recipient added to {user}'s recoveryRecipients\n\
        \x20 - .sops.yaml regenerated and every governed file re-wrapped to it\n\
        \x20 - all of it committed together\n\
        \n\
        \x20   {recipient}\n\
        \n"
    );
    let tail = match outcome {
        proof::Outcome::Proven { file } => format!(
            "The proof passed: {file} opened with the card's stub as the only\n\
            identity reachable, which exercised the PIN and the touch. That is the\n\
            claim the hand ceremony never made.\n"
        ),
        proof::Outcome::Refused { file, status } => format!(
            "The enrollment is INCOMPLETE: the proof did not pass. sops exited {status}\n\
            trying to open {file} with the card alone, and its own message is above.\n\
            \n\
            Nothing has been undone, because nothing is wrong with what was written:\n\
            the identity, the recipient and the re-wrap are additive and correct on\n\
            their own. Re-run this verb once the card answers.\n"
        ),
    };
    text.push_str(&tail);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_fleets_measured_policies_and_custody_is_on() {
        let options = Options::default();
        assert_eq!(options.pin_policy, "once");
        assert_eq!(options.touch_policy, "cached");
        assert!(options.store_pin, "the safix-side copy is the default");
        assert!(!options.mirror.mirror, "the store mirror is the opt-in");
        assert_eq!(options.serial, None);
        assert_eq!(options.slot, None);
    }

    #[test]
    fn an_incomplete_run_says_so_and_says_that_nothing_was_undone() {
        let rendered = epilogue(
            "ana",
            "12345678",
            "age1yubikey1qfixture",
            &proof::Outcome::Refused {
                file: String::from("secrets/safix/users/ana/secrets.yaml"),
                status: 1,
            },
        );
        assert!(rendered.contains("INCOMPLETE"));
        assert!(rendered.contains("Nothing has been undone"));
        assert!(rendered.contains("age1yubikey1qfixture"));
    }

    #[test]
    fn a_proven_run_names_the_file_it_opened() {
        let rendered = epilogue(
            "ana",
            "12345678",
            "age1yubikey1qfixture",
            &proof::Outcome::Proven {
                file: String::from("secrets/safix/users/ana/secrets.yaml"),
            },
        );
        assert!(rendered.contains("The proof passed"));
        assert!(rendered.contains("secrets/safix/users/ana/secrets.yaml"));
        assert!(!rendered.contains("INCOMPLETE"));
    }

    #[test]
    fn the_hook_is_read_from_the_attribute_the_nix_half_publishes() {
        assert_eq!(
            crate::nix::Attribute::EnrollHook.as_str(),
            "safix.enrollHook"
        );
        assert_eq!(
            crate::nix::Attribute::EnrollHook.declared_as(),
            "flake.safix.enrollHook"
        );
    }
}
