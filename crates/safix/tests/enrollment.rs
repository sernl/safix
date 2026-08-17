//! Enrolling a hardware key, against a card surface that records what it saw.
//!
//! # What is asserted here, and what cannot be
//!
//! Everything `safix enroll` does: which argument vectors reach `ykman` and that
//! no credential is in any of them or in any environment, on any path; that the
//! generated PUK is distinct from the generated PIN and that both travelled a
//! hidden prompt; that the management key is generated on the card and named
//! nowhere; that each prompted drive is answered a bounded number of times; that
//! the identity block lands where `keygen` appends and the recipient lands in
//! `recoveryRecipients`; that the edit moves the regenerated policy and the commit
//! is exactly the ceremony's own paths; that clan is reached through clan's command
//! and the hook receives three arguments; that the credentials travel standard
//! input; and that the proof's identity source holds one line with no ambient
//! identity reachable from it.
//!
//! What cannot be asserted without a card is the other side of two of those
//! boundaries. `ykman`'s arguments and prompts meaning to `ykman` what safix
//! thinks they mean, and an `age1yubikey1…` recipient being one real sops can wrap
//! a data key to, both need the hardware and the plugin: wrapping to a plugin
//! recipient runs the plugin, and the plugin runs the card. So the fixture's nix
//! half projects a recovery recipient into the policy's anchors, where the claim
//! that the edit moved the policy lives, and not into its creation rules, where
//! the wrap would be attempted — and no file in these tests carries the card's
//! stanza. The proof therefore does not pass here, and that is the observation
//! rather than a gap: a proof that passed with no card and a software identity
//! ambient would mean the isolation had failed, which is exactly what
//! [`the_proof_is_isolated_from_every_ambient_identity`] reads.
//!
//! The proof machinery's passing path is asserted separately and hardware-free,
//! by [`the_proof_opens_a_file_with_the_isolated_source_alone`], which drives it
//! against real sops with a real identity standing where the card's stub goes.
//! Together the two say: the isolation is what decides the outcome.
//!
//! # Why nothing here runs the real tools
//!
//! The two cards this fleet has hold the master identities for everything it
//! owns, and the password database that opens them is guarded by a
//! challenge-response secret on an OTP slot whose loss is total. A suite that
//! drove the real `ykman` would be one argument away from an irreversible loss,
//! and one that drove a real `keepassxc-cli` or the session's secret service
//! would write into the operator's own store. Both are refused by construction:
//! see the head of `tests/support/card-stubs.rs`.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use harness::{ANA_FILE, Fixture};
use safix_core::enroll::{custody, proof};

/// The serial the fixture's one card answers with.
const SERIAL: &str = "12345678";

/// A second card, for the backup-key run.
const BACKUP: &str = "87654321";

/// The recipient the stubbed generator prints for the first card.
///
/// Synthetic: the `age1yubikey1` prefix is the load-bearing part — it is what
/// makes the captured block a card's rather than a software key's — and nothing
/// here encrypts to it, for the reason the module head gives.
const CARD: &str = "age1yubikey1qfixture000000000000000000000000000000000000000000000000";

/// The backup card's recipient.
const BACKUP_CARD: &str = "age1yubikey1qbackup0000000000000000000000000000000000000000000000000";

/// The PIN safix generated, read out of the prompt `ykman` was answered on.
///
/// Not a literal: safix generates it, so no test can know it in advance. What the
/// stub recorded when it prompted for it is the only place it is observable — and
/// that is the point rather than an inconvenience, because it means the value
/// exists nowhere a test could have read it from a process listing.
fn generated_pin(fixture: &Fixture) -> String {
    let recorded = fixture.card_recorded("pin");
    let line = recorded.lines().next().unwrap_or_default();
    line.rsplit("-> ").next().unwrap_or_default().to_owned()
}

/// The generated PUK, read the same way.
fn generated_puk(fixture: &Fixture) -> String {
    let recorded = fixture.card_recorded("puk");
    let line = recorded.lines().next().unwrap_or_default();
    line.rsplit("-> ").next().unwrap_or_default().to_owned()
}

/// Refuse a credential that appears in any argument vector or any environment,
/// on any path the run took.
///
/// The unconditional half of the spec's custody requirement, and it is asserted
/// over every invocation the whole ceremony made rather than over the store's
/// alone: `ykman` is where a PIN used to travel as an option, so a reading scoped
/// to the stores would have passed over exactly the channel that mattered.
fn refuse_credentials_on_public_channels(fixture: &Fixture, values: &[&str]) {
    for value in values {
        assert!(
            !value.is_empty(),
            "an empty value would make this assertion vacuous"
        );
        for (channel, recorded) in [
            ("an argument vector", fixture.card_recorded("argv")),
            ("an environment", fixture.card_recorded("environ")),
        ] {
            assert!(!recorded.contains(value), "a credential reached {channel}");
        }
    }
}

/// The environment one enrollment run needs, with the card's own switches.
fn card_env(fixture: &Fixture, serials: &str, recipient: &str) -> Vec<(String, String)> {
    let mut environment = fixture.card_env();
    environment.push(("SAFIX_CARD_STUB_SERIALS".to_owned(), serials.to_owned()));
    environment.push(("SAFIX_CARD_STUB_RECIPIENT".to_owned(), recipient.to_owned()));
    // No expected PIN: safix is what generated it, so the stub accepts the first
    // answer. The one-attempt claim is drilled by a run that names one safix
    // cannot have generated.
    environment
}

fn as_pairs(environment: &[(String, String)]) -> Vec<(&str, &str)> {
    environment
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// The whole ceremony over one factory-fresh card, end to end.
#[test]
fn enrollment_provisions_generates_wires_and_commits_once() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.seed_card_custody(SERIAL);
    fixture
        .set("ana", "mail-password", "before-the-card")
        .expect_success("giving ana something to hold");
    let head = fixture.head();

    let environment = card_env(&fixture, SERIAL, CARD);
    let run = fixture.run_on_terminal(&["enroll", "ana"], "", &as_pairs(&environment));

    // The proof cannot pass with no card, and the report says so rather than
    // claiming the enrollment finished.
    assert_eq!(run.code, Some(1), "an outstanding proof is not a success");
    run.says("INCOMPLETE");
    run.says("Nothing has been undone");

    // ── the card's access ──
    let pin = generated_pin(&fixture);
    let puk = generated_puk(&fixture);
    assert_eq!(pin.len(), 8, "the generated PIN is not eight digits: {pin}");
    assert_eq!(puk.len(), 8, "the generated PUK is not eight digits: {puk}");
    assert_ne!(pin, puk, "the PUK collapsed into the PIN");
    assert_ne!(pin, "123456", "the factory PIN was left in force");
    assert!(
        fixture.card_recorded("puk").contains("12345678 ->"),
        "the PUK was not changed from the factory one"
    );
    assert!(
        fixture.card_recorded("pin").contains("123456 ->"),
        "the PIN was not changed from the factory one"
    );
    assert_eq!(
        fixture.card_recorded("management-key").trim(),
        format!("{SERIAL} protected under {pin}"),
        "the management key was not generated on the card under the PIN"
    );
    assert!(
        !fixture.card_recorded("argv").contains("--management-key"),
        "a management key was named, so one was chosen off the card"
    );

    // ── every credential travelled a prompt, and no public channel ──
    // The unconditional reading, over every invocation the ceremony made. What
    // makes it possible is that ykman's credential options are omitted: five
    // prompts answered on a pseudo-terminal, and an argument vector that carries
    // nothing but the serial and the two published factory constants.
    refuse_credentials_on_public_channels(&fixture, &[&pin, &puk]);
    assert_eq!(
        fixture.card_recorded("ykman-prompt").lines().count(),
        5,
        "the five ykman prompts were not the channel the credentials travelled: \
         two for the PUK and its confirmation, two for the PIN and its \
         confirmation, one for the management key's PIN"
    );
    let argv = fixture.card_recorded("argv");
    for names_a_credential in ["-n", "--new-pin", "--new-puk", "--pin"] {
        assert!(
            !argv
                .split_whitespace()
                .any(|word| word == names_a_credential),
            "{names_a_credential} appeared in an argument vector: {argv}"
        );
    }

    // ── the OTP applet, never ──
    assert!(
        !argv.contains(" otp"),
        "an argument vector named the OTP applet: {argv}"
    );

    // ── the one interactive step ──
    assert_eq!(
        fixture.card_recorded("pin-attempt").lines().count(),
        1,
        "the generator was answered more than once"
    );
    assert_eq!(
        fixture.card_recorded("pin-attempt").trim(),
        pin,
        "the generator was answered with something other than the PIN safix set"
    );
    assert_eq!(
        fixture.card_recorded("streams").trim(),
        "stdin=terminal stdout=pipe stderr=terminal",
        "the generator's streams were not the split its prompt and its block need"
    );
    run.says("Please touch the YubiKey");

    // ── the identity ──
    let identity = fixture.card_identity();
    assert!(
        identity.contains("AGE-PLUGIN-YUBIKEY-1"),
        "the identity file holds no stub: {identity}"
    );
    assert!(
        !identity.contains("AGE-SECRET-KEY"),
        "a private key reached the identity file"
    );
    assert!(
        identity.contains(&format!("#    Recipient: {CARD}")),
        "the block's own recipient comment is missing"
    );

    // ── the wiring ──
    let declaration = fixture.read("safix/users/ana.nix");
    assert!(
        declaration.contains(&format!("recoveryRecipients = [ \"{CARD}\" ];")),
        "the card is not in ana's recoveryRecipients: {declaration}"
    );
    assert!(
        declaration.contains(&format!("recipient = \"{}\";", fixture.ana)),
        "the primary recipient was disturbed"
    );
    assert!(
        declaration.contains(&format!("{} = {{ }};", custody::secret_name(SERIAL))),
        "the credentials' own name was not declared"
    );

    assert_ne!(fixture.head(), head, "nothing was committed");
    let subject = fixture.subject("HEAD~1");
    assert!(
        subject.contains(&format!("enroll {SERIAL} as a recovery recipient for ana")),
        "the ceremony's commit is not the one that names it: {subject}"
    );
    let ceremony = fixture.paths_in("HEAD~1");
    assert_eq!(
        ceremony,
        vec![".sops.yaml".to_owned(), "safix/users/ana.nix".to_owned()],
        "the ceremony's commit is not the record it edited and the policy that saw it"
    );

    // The policy moved because the edit moved it: the card is an anchor in the
    // regenerated document, which is only true if the edited record was staged
    // before the evaluation read it — an evaluation reads the files git tracks, so
    // regenerating first writes the policy of the declarations as they stood
    // without the card.
    let policy = fixture.read(".sops.yaml");
    assert!(
        policy.contains(CARD),
        "the regenerated policy does not carry the card just enrolled: {policy}"
    );
    assert!(
        policy.contains(&fixture.ana),
        "the regenerated policy dropped the software recipient"
    );
    assert_eq!(
        fixture.status(),
        "",
        "the ceremony left the tree dirty: {}",
        fixture.status()
    );

    // ── the credentials' custody ──
    assert_eq!(
        fixture.subject("HEAD"),
        format!("chore(safix): store the PIV access for {SERIAL} in ana's custody"),
        "the credentials were not stored through the ordinary write path"
    );
    let stored = fixture.value(ANA_FILE, &custody::secret_name(SERIAL));
    assert_eq!(
        stored,
        format!("PIN={pin}\nPUK={puk}"),
        "the stored credentials are not the ones that were set"
    );

    // ── nothing reached standard output unbidden ──
    assert!(
        run.stdout.is_empty(),
        "the run wrote to standard output: {:?}",
        String::from_utf8_lossy(&run.stdout)
    );
    assert!(
        !run.stderr.contains(&pin),
        "the PIN was echoed to the operator's terminal"
    );
    assert!(
        !run.stderr.contains(&puk),
        "the PUK was echoed to the operator's terminal"
    );
}

/// A second card is a second enrollment, and neither knows about the other.
#[test]
fn a_backup_card_sits_beside_the_first_and_changes_nothing_about_it() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    fixture
        .set("ana", "mail-password", "before-the-cards")
        .expect_success("giving ana something to hold");

    let first = card_env(&fixture, SERIAL, CARD);
    fixture.run_on_terminal(
        &["enroll", "ana", "--serial", SERIAL, "--no-store-pin"],
        "",
        &as_pairs(&first),
    );
    let after_first = fixture.read("safix/users/ana.nix");
    let identity_after_first = fixture.card_identity();

    let second = card_env(&fixture, &format!("{SERIAL} {BACKUP}"), BACKUP_CARD);
    fixture.run_on_terminal(
        &["enroll", "ana", "--serial", BACKUP, "--no-store-pin"],
        "",
        &as_pairs(&second),
    );

    let after_second = fixture.read("safix/users/ana.nix");
    assert!(
        after_second.contains(&format!("\"{CARD}\"")),
        "the first card was dropped: {after_second}"
    );
    assert!(
        after_second.contains(&format!("\"{BACKUP_CARD}\"")),
        "the second card was not added: {after_second}"
    );
    assert_eq!(
        after_second.matches("recoveryRecipients").count(),
        1,
        "a second list was written instead of the first being extended"
    );
    // Every line the first run left is still there, except the one the second run
    // extended: a list gaining an element is the only rewrite either run makes,
    // and it is an insertion into that line rather than a replacement of the file.
    assert!(
        after_first
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !line.contains("recoveryRecipients"))
            .all(|line| after_second.contains(line.trim())),
        "the second enrollment rewrote a line the first had written"
    );

    let identity = fixture.card_identity();
    assert!(
        identity.starts_with(&identity_after_first),
        "the second identity block did not append: {identity}"
    );
    assert_eq!(
        identity.matches("AGE-PLUGIN-YUBIKEY-1").count(),
        2,
        "the two cards do not have two stubs: {identity}"
    );
}

/// Every refusal the card surface produces, each leaving the tree alone.
#[test]
fn the_card_refusals_each_have_their_own_code_and_leave_the_tree_alone() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let head = fixture.head();
    let mut codes = Vec::new();

    // Two cards and nothing to choose between them.
    let two = card_env(&fixture, &format!("{SERIAL} {BACKUP}"), CARD);
    let refused = fixture.run_on_terminal(&["enroll", "ana"], "", &as_pairs(&two));
    refused.says(SERIAL);
    refused.says(BACKUP);
    refused.says("--serial");
    codes.push(refusal_code(&fixture, &["enroll", "ana"], &as_pairs(&two)));

    // No smartcard service.
    let mut absent = card_env(&fixture, SERIAL, CARD);
    absent.push(("SAFIX_CARD_STUB_NO_PCSCD".to_owned(), "yes".to_owned()));
    let refused = fixture.run_on_terminal(&["enroll", "ana"], "", &as_pairs(&absent));
    refused.says("services.pcscd.enable");
    codes.push(refusal_code(
        &fixture,
        &["enroll", "ana"],
        &as_pairs(&absent),
    ));

    // No card at all.
    let none = card_env(&fixture, "", CARD);
    fixture
        .run_on_terminal(&["enroll", "ana"], "", &as_pairs(&none))
        .says("no card is connected");
    codes.push(refusal_code(&fixture, &["enroll", "ana"], &as_pairs(&none)));

    // A touch nobody has to make.
    let touchless = card_env(&fixture, SERIAL, CARD);
    let refused = fixture.run_on_terminal(
        &["enroll", "ana", "--touch-policy", "never"],
        "",
        &as_pairs(&touchless),
    );
    refused.says("smartcard emulating a file");
    codes.push(refusal_code(
        &fixture,
        &["enroll", "ana", "--touch-policy", "never"],
        &as_pairs(&touchless),
    ));

    // An OTP slot, refused with the hazard named rather than as an unknown
    // option. Every spelling somebody would reach for gets the same refusal.
    for asked in ["--otp", "--otp-slot", "--challenge-response"] {
        let refused = fixture.run_on_terminal(&["enroll", "ana", asked], "", &as_pairs(&touchless));
        refused.says("does not write, reprogram or delete an OTP slot");
        refused.says("the database stops opening");
    }
    codes.push(refusal_code(
        &fixture,
        &["enroll", "ana", "--otp-slot"],
        &as_pairs(&touchless),
    ));

    // A run with no terminal, refused before the card is touched. This one goes
    // through the ordinary runner, which gives it pipes. The card is shown not to
    // have been reached by the spool not growing, because the runs above have
    // already written to it.
    let before = fixture.card_recorded("argv");
    fixture
        .run_env(&["enroll", "ana"], None, &as_pairs(&touchless))
        .expect_refusal("enrollment with no terminal")
        .says("enrollment needs a terminal");
    assert_eq!(
        fixture.card_recorded("argv"),
        before,
        "the card was reached before the terminal was checked"
    );

    assert_eq!(
        codes,
        vec![
            "cards_ambiguous",
            "pcscd_unavailable",
            "no_card_connected",
            "touch_policy_never",
            "otp_refused",
        ],
        "two refusals about the card share one code"
    );
    assert_eq!(fixture.head(), head, "a refusal committed something");
    assert_eq!(
        fixture.read("safix/users/ana.nix").matches(CARD).count(),
        0,
        "a refusal left a recipient behind"
    );
}

/// A card already provisioned is not re-provisioned, and its PIN is asked for.
#[test]
fn a_provisioned_card_keeps_its_access_and_the_pin_is_asked_for_once() {
    let fixture = Fixture::new();
    fixture.seed_declarations();

    let mut environment = card_env(&fixture, SERIAL, CARD);
    environment.push(("SAFIX_CARD_STUB_STATE".to_owned(), "provisioned".to_owned()));
    // The PIN the operator types, which the stub is told to expect: this is the
    // one run where safix generated nothing and the PIN comes from a person.
    environment.push((
        "SAFIX_CARD_STUB_EXPECTED_PIN".to_owned(),
        "87654321".to_owned(),
    ));

    let run = fixture.run_on_terminal(
        &["enroll", "ana", "--no-store-pin"],
        "87654321\n",
        &as_pairs(&environment),
    );
    run.says("is already provisioned, so nothing about its access is changed");

    // Nothing about the card's access was touched: the three drives that would
    // change it recorded nothing at all.
    for untouched in ["pin", "puk", "management-key"] {
        assert_eq!(
            fixture.card_recorded(untouched),
            "",
            "a provisioned card had its {untouched} changed"
        );
    }
    assert_eq!(
        fixture.card_recorded("state-asked").trim(),
        SERIAL,
        "the card's state was not probed"
    );

    // The PIN the operator typed is what answered the generator, once.
    assert_eq!(
        fixture.card_recorded("pin-attempt").trim(),
        "87654321",
        "the generator was answered with something else"
    );
    assert!(
        fixture.card_identity().contains("AGE-PLUGIN-YUBIKEY-1"),
        "no identity was generated on a card that was ready for one"
    );
    assert!(
        !run.stderr.contains("87654321\n87654321"),
        "the PIN was echoed back to the terminal it was typed on"
    );
}

/// A person whose custody record is not there to extend.
#[test]
fn a_person_with_no_custody_record_is_refused_before_the_recipient_is_wired() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.git(&["rm", "-q", "--", "safix/users/bo.nix"]);
    fixture.git(&["commit", "-q", "-m", "fixture: bo's record is elsewhere"]);
    let head = fixture.head();

    let environment = card_env(&fixture, SERIAL, CARD);
    let refused = fixture.run_on_terminal(&["enroll", "bo"], "", &as_pairs(&environment));
    refused.says("safix/users/bo.nix");
    refused.says("safix adduser bo");
    assert_eq!(fixture.head(), head, "the refusal committed something");

    // The identity was generated and appended before the record was reached,
    // which is what the refusal's own wording says: the card is enrolled and the
    // recipient is the operator's to add. Nothing was taken back out.
    assert!(
        fixture.card_identity().contains("AGE-PLUGIN-YUBIKEY-1"),
        "the refusal discarded an identity that exists on the card"
    );
}

/// A PIN the card refuses costs one retry and not three.
#[test]
fn a_rejected_pin_aborts_after_one_attempt() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let head = fixture.head();

    // The stub is told to expect a PIN safix cannot have generated, so its second
    // prompt is what the wrapper meets. A run that answered every prompt would
    // record three attempts and block the card.
    let mut wrong = card_env(&fixture, SERIAL, CARD);
    wrong.push((
        "SAFIX_CARD_STUB_EXPECTED_PIN".to_owned(),
        "000000".to_owned(),
    ));
    wrong.retain(|(name, _)| name != "SAFIX_CARD_STUB_ACCEPTS_ANY");

    let refused = fixture.run_on_terminal(&["enroll", "ana"], "", &as_pairs(&wrong));
    refused.says("refused the PIN");
    refused.says("One attempt, deliberately");
    assert_eq!(
        fixture.card_recorded("pin-attempt").lines().count(),
        1,
        "more than one attempt was made"
    );
    assert_eq!(fixture.head(), head, "a rejected PIN committed something");
    assert!(
        !fixture.read("safix/users/ana.nix").contains(CARD),
        "a rejected PIN still wired the recipient"
    );
}

/// The proof's identity source holds one line, and nothing ambient is reachable.
#[test]
fn the_proof_is_isolated_from_every_ambient_identity() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.seed_card_custody(SERIAL);
    fixture
        .set("ana", "mail-password", "a-value-the-card-would-open")
        .expect_success("giving ana something to hold");

    let spool = fixture.scratch_dir("sops-spy");
    let sops = harness::real_sops();
    let mut environment = card_env(&fixture, SERIAL, CARD);
    environment.push(("SAFIX_SOPS".to_owned(), harness::shim().to_owned()));
    environment.push(("SAFIX_SHIM_ROLE".to_owned(), "spy".to_owned()));
    environment.push(("SAFIX_SHIM_SOPS".to_owned(), sops));
    environment.push((
        "SAFIX_SHIM_SPY".to_owned(),
        spool.to_string_lossy().into_owned(),
    ));

    let run = fixture.run_on_terminal(&["enroll", "ana"], "", &as_pairs(&environment));
    assert_eq!(run.code, Some(1), "the proof passed with no card present");
    run.says("INCOMPLETE");

    // The decrypt that ran as the proof, out of what sops was handed. The last
    // invocation is the proof's: everything before it is the re-wrap and the
    // credentials' write.
    let environ = std::fs::read_to_string(spool.join("environ")).unwrap();
    let named: Vec<&str> = environ
        .lines()
        .filter(|line| line.starts_with("SOPS_AGE_KEY_FILE="))
        .collect();
    assert!(
        !named.is_empty(),
        "no invocation was given an identity file at all"
    );
    let isolated = named
        .iter()
        .filter_map(|line| line.strip_prefix("SOPS_AGE_KEY_FILE="))
        .find(|path| !path.contains("age-key.txt"))
        .expect("no invocation was given an identity file of the proof's own");

    // The fixture's own key file is ambient in every run's environment and is what
    // opens this file. The proof was handed a different one, holding exactly the
    // card's stub, so a passing proof could not have used the software key.
    //
    // Read out of what the spy recorded rather than off the disk: the source is
    // built inside the run's staging root and swept when the run ends, which is
    // the point of it — an identity file that outlived the decrypt it was made
    // for would be a file nobody removed.
    let recorded = std::fs::read_to_string(spool.join("identity-files")).unwrap();
    let source = recorded
        .split(isolated)
        .nth(1)
        .expect("the spy recorded no contents for the proof's identity file")
        .trim();
    assert_eq!(
        source.lines().count(),
        1,
        "the proof's identity source holds more than the card's stub: {source}"
    );
    assert!(
        source.starts_with("AGE-PLUGIN-YUBIKEY-1"),
        "the one line is not the card's stub: {source}"
    );
    assert!(
        !environ
            .lines()
            .any(|line| line.starts_with("SOPS_AGE_KEY=") || line.starts_with("SOPS_AGE_KEY_CMD=")),
        "another way of finding an identity was left reachable"
    );

    // The additive wiring is in place even though the proof is outstanding.
    assert!(
        fixture.read("safix/users/ana.nix").contains(CARD),
        "a failed proof took the recipient back out"
    );
    assert!(
        fixture.card_identity().contains("AGE-PLUGIN-YUBIKEY-1"),
        "a failed proof took the identity block back out"
    );
}

/// The proof machinery opens a file with the isolated source and with nothing
/// else — asserted against real sops, with a software identity standing where the
/// card's stub goes.
///
/// This is the passing half of the proof's claim, and it is a separate test for
/// the reason the module head gives: wrapping a data key to an `age1yubikey1…`
/// recipient runs the plugin, and the plugin runs the card. What stands in for the
/// card here is an ordinary age identity, which exercises every part of the
/// mechanism except the applet: the isolated file is written, `SOPS_AGE_KEY_FILE`
/// names it, every other identity variable is cleared, and the outcome is decided
/// by whether that one identity opens the file.
#[test]
fn the_proof_opens_a_file_with_the_isolated_source_alone() {
    let fixture = Fixture::new();
    let workspace = safix_core::Workspace::at(
        fixture.repo.clone(),
        safix_core::git::Git::from_environment(),
        safix_core::nix::Nix::from_environment(),
        safix_core::sops::Sops::from_environment(),
    );

    // One file encrypted to the stand-in alone, so the fixture's own key — which
    // every run of this suite has ambient — cannot be what opens it.
    let stand_in = fixture.scratch_dir("stand-in").join("identity.txt");
    harness::mint_identity(&stand_in);
    let recipient = harness::recipient_of(&stand_in);
    fixture.encrypt_to(
        ANA_FILE,
        &[&recipient],
        "mail-password: opened-by-the-stand-in\n",
    );

    let identity = std::fs::read_to_string(&stand_in).unwrap();
    let stub =
        proof::stub_of(&identity).expect("an identity file has a line that is not a comment");

    let directory = fixture.scratch_dir("proof-source");
    let source = proof::write_isolated_source(&directory, &stub).expect("it can be written");
    assert_eq!(
        std::fs::read_to_string(&source).unwrap().lines().count(),
        1,
        "the isolated source holds more than one identity"
    );

    let outcome = proof::decrypt_with(&workspace, &source, ANA_FILE).expect("sops can be run");
    assert_eq!(
        outcome,
        proof::Outcome::Proven {
            file: ANA_FILE.to_owned()
        },
        "the isolated source did not open the file it is a recipient of"
    );

    // The control: an isolated source holding an identity the file was not
    // encrypted to does not open it, which is what makes the assertion above a
    // statement about the identity rather than about sops being lenient.
    let stranger = fixture.scratch_dir("stranger").join("identity.txt");
    harness::mint_identity(&stranger);
    let stranger_stub =
        proof::stub_of(&std::fs::read_to_string(&stranger).unwrap()).expect("it has an identity");
    let other = proof::write_isolated_source(&fixture.scratch_dir("proof-other"), &stranger_stub);
    assert!(
        matches!(
            proof::decrypt_with(&workspace, &other.expect("it can be written"), ANA_FILE),
            Ok(proof::Outcome::Refused { .. })
        ),
        "a source that is not a recipient opened the file"
    );
}

/// clan learns about the recipient from its own command, and the hook gets three
/// arguments after the commit has landed.
#[test]
fn clan_and_the_hook_receive_the_enrollment_after_it_is_committed() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    let repository = fixture.repo.clone();
    fixture.clan_flake_is(&repository);
    fixture.set_enroll_hook(Some(
        "{\n  printf 'user=%s\\n' \"$1\"\n  printf 'serial=%s\\n' \"$2\"\n  \
         printf 'recipient=%s\\n' \"$3\"\n} >enroll-hook-log.txt\n",
    ));

    let mut environment = card_env(&fixture, SERIAL, CARD);
    environment.extend(fixture.clan_env());

    let run = fixture.run_on_terminal(
        &["enroll", "ana", "--no-store-pin"],
        "",
        &as_pairs(&environment),
    );
    assert_eq!(run.code, Some(1), "the proof passed with no card present");

    let registered = fixture.clan_recorded("argv");
    assert!(
        registered.contains("secrets users add --flake"),
        "clan was not asked to register the key through its own command: {registered}"
    );
    assert!(
        registered.contains(&format!("--age-key {CARD}")),
        "clan was not handed the card's recipient: {registered}"
    );
    assert_eq!(
        fixture.clan_recorded("registered").trim(),
        format!("add ana {CARD}"),
        "clan did not accept the registration it was asked for"
    );

    let log = fixture.read("enroll-hook-log.txt");
    assert_eq!(
        log,
        format!("user=ana\nserial={SERIAL}\nrecipient={CARD}\n"),
        "the hook was not given the person, the serial and the recipient"
    );
    assert!(
        !fixture
            .paths_in("HEAD")
            .contains(&"enroll-hook-log.txt".to_owned()),
        "the hook's output was swept into safix's own commit"
    );
    assert!(
        fixture.status().contains("enroll-hook-log.txt"),
        "the hook's output was not left uncommitted"
    );
}

/// A hookless run succeeds at that step, having done less, and says so.
#[test]
fn a_run_with_no_hook_and_no_clan_says_what_it_did_not_do() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let environment = card_env(&fixture, SERIAL, CARD);
    let run = fixture.run_on_terminal(
        &["enroll", "ana", "--no-store-pin"],
        "",
        &as_pairs(&environment),
    );
    run.says("no clan is declared, so nothing was registered with one");
    run.says("flake.safix.enrollHook is unset, so nothing further ran");
}

/// The credentials reach a store on standard input, round-trip, and never argv.
#[test]
fn the_mirrored_credentials_travel_standard_input_and_round_trip() {
    let mut fixture = Fixture::new();
    fixture.seed_declarations();
    fixture.seed_card_custody(SERIAL);
    fixture
        .set("ana", "mail-password", "before-the-card")
        .expect_success("giving ana something to hold");

    let environment = card_env(&fixture, SERIAL, CARD);
    fixture.run_on_terminal(
        &["enroll", "ana", "--mirror-to-store"],
        "",
        &as_pairs(&environment),
    );

    let pin = generated_pin(&fixture);
    let puk = generated_puk(&fixture);
    let expected = format!("PIN={pin}\nPUK={puk}");

    let entry = custody::entry_name(SERIAL);
    let held = fixture
        .card_holds(&format!("service-{entry}"))
        .expect("the secret service holds no entry for the card");
    assert_eq!(
        held, expected,
        "the credentials that reached the service are not the ones that were set"
    );
    assert!(
        !held.ends_with('\n'),
        "a newline nobody put there is part of the stored credential"
    );

    let recorded = fixture.card_recorded("service-entry");
    assert!(
        recorded.contains(&entry),
        "the entry was not filed under the serial: {recorded}"
    );
    // Unconditional and over every invocation, not just the store's: what the
    // credential travelled here is standard input, and what it travelled at the
    // card was a prompt, so no argument vector and no environment on any path
    // carries one.
    assert!(
        fixture
            .card_recorded("argv")
            .lines()
            .any(|line| line.starts_with("store ")),
        "the service was never asked to store"
    );
    assert!(
        fixture
            .card_recorded("environ")
            .lines()
            .any(|line| line.starts_with("[store ")),
        "the store's environment was not recorded"
    );
    refuse_credentials_on_public_channels(&fixture, &[&pin, &puk]);
}

/// A `ykman` drive that asks past its bound is not answered further.
///
/// The bounded-answer discipline at the card boundary, where the flag-driven
/// provisioning this replaced had no such question to ask. The stub is made to
/// prompt once more than the drive needs; a run that answered whatever it was
/// asked would sail past it and, on a real card, would be a run that submitted a
/// value nobody chose.
#[test]
fn a_ykman_drive_that_asks_past_its_bound_stops_the_run() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let head = fixture.head();

    let mut environment = card_env(&fixture, SERIAL, CARD);
    environment.push((
        "SAFIX_CARD_STUB_EXTRA_PROMPT".to_owned(),
        "change-puk".to_owned(),
    ));

    let refused = fixture.run_on_terminal(&["enroll", "ana"], "", &as_pairs(&environment));
    refused.says("refused the PIN");
    refused.says("One attempt, deliberately");

    // Three prompts answered, not four: the new PUK, its confirmation, and then
    // nothing for the one the stub added.
    assert_eq!(
        fixture.card_recorded("ykman-prompt").lines().count(),
        2,
        "the run answered a prompt past the bound"
    );
    assert_eq!(
        fixture.card_recorded("pin"),
        "",
        "the run went on to the next drive after a drive that asked too much"
    );
    assert_eq!(fixture.head(), head, "the aborted run committed something");
    assert!(
        !fixture.read("safix/users/ana.nix").contains(CARD),
        "the aborted run still wired a recipient"
    );
}

/// The graphical code of one refusal, which is where a code is rendered.
///
/// The harness sets the plain reporter for every run; emptying that variable
/// selects the graphical one, because only its own value selects the plain one.
fn refusal_code(fixture: &Fixture, arguments: &[&str], extra: &[(&str, &str)]) -> String {
    let mut with_graphical: Vec<(&str, &str)> = extra.to_vec();
    with_graphical.push(("SAFIX_ERROR_FORMAT", ""));
    fixture
        .run_on_terminal(arguments, "", &with_graphical)
        .refusal_code()
}
