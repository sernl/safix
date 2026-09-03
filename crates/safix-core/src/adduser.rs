//! Declaring a person, which is an edit to the declarations and nothing else.
//!
//! This writes the one file that says who somebody is, regenerates the recipient
//! policy that declaration implies, and commits the two together. Nothing is
//! encrypted, nothing is decrypted, and no key material is minted or read —
//! which is what lets it run before its subject holds anything at all.
//!
//! The recipient is an argument and is never generated here, for the reason
//! [`keygen`](crate::keygen) refuses at length: minting it on this machine would
//! mean this operator held the private half, which is the custody inversion the
//! package exists to avoid.
//!
//! Everything beyond that is a consumer's business and reaches it through the
//! hook. Attaching an account on a host, allocating an identifier, editing a
//! host's module imports: each is a property of one consumer's module tree, so
//! safix passes the name and the recipient to `flake.safix.onboardingHook` and
//! makes no assumption about what it does. No hook configured is a supported
//! configuration; onboarding simply does less.
//!
//! # Why no delegation check
//!
//! Because there is nothing here for one to read. A delegation over a person is
//! that person's own [`managedBy`](crate::delegation) consent, and this verb's
//! target is by construction a name the declarations do not carry: a name they do
//! carry is refused as already declared before anything else happens, and
//! `flake.safix.lib.placements` carries a row for every declared person whether
//! they hold a secret or not. So a `managedBy` for this target cannot exist, and a
//! check here would be a branch no input reaches. Once the person is declared,
//! every later edit to their record goes through [`enroll`](crate::enroll), which
//! is where the check lives.

use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::nix::Attribute;
use crate::progress::{Progress, log, note};
use crate::workspace::Workspace;
use crate::{git, scratch};

/// The prefix of an age recipient that needs a card, a PIN and a touch.
const HARDWARE_PREFIX: &str = "age1yubikey1";

/// The alphabet bech32 draws from: everything but `1`, `b`, `i` and `o`.
const BECH32: &str = "023456789acdefghjklmnpqrstuvwxyz";

/// How many bech32 characters follow `age1` in an age X25519 recipient.
const RECIPIENT_BODY: usize = 58;

/// Where the operator answers the one question this asks.
///
/// Read from this process's standard input rather than from a terminal, which is
/// what the shell runtime does here and is not what it does for a value: the
/// answer is not a secret, and a scaffold driven from a pipe is how the checks
/// drive one.
pub trait Confirm {
    /// Whether to write the scaffold. Anything but yes is no.
    ///
    /// # Errors
    ///
    /// Whatever reading the answer failed with. A stream that ends without one
    /// is not an error — it is a no.
    fn scaffold(&mut self) -> Result<bool>;
}

/// What an invocation asked for.
#[derive(Debug, Clone)]
pub struct Request {
    /// The person's name, which becomes a path and a `path_regex` fragment.
    pub name: String,
    /// Their age public key, handed over by them.
    pub recipient: String,
    /// Every `--host` given, in order, passed through to the hook.
    pub hosts: Vec<String>,
    /// Skip the confirmation.
    pub assume_yes: bool,
}

/// The file a scaffold is written to.
///
/// safix imposes no layout on declarations — an attrset option merges from
/// anywhere, so this file resolves the same wherever it sits — but a scaffold has
/// to choose a path, and it chooses one under a directory of safix's own rather
/// than guessing at the consumer's.
#[must_use]
pub fn scaffold_path(name: &str) -> String {
    format!("safix/users/{name}.nix")
}

/// Declare a person, regenerate the policy, commit the two, then run the hook.
///
/// # Errors
///
/// [`Error::BadUserName`], [`Error::HardwareRecipient`], [`Error::BadRecipient`],
/// [`Error::AlreadyDeclared`], [`Error::ScaffoldExists`], [`Error::HostWithoutHook`],
/// [`Error::ScaffoldDeclined`], [`Error::Unparsable`],
/// [`Error::PolicyEvalAfterScaffold`] and [`Error::HookFailed`], in that order of
/// reachability.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    confirm: &mut dyn Confirm,
    request: &Request,
) -> Result<()> {
    scratch::set_floor(workspace.root());
    scratch::set_floor(workspace.vault_root());
    let _guard = scratch::Guard;

    refuse_bad_name(workspace, &request.name)?;
    refuse_bad_recipient(&request.recipient)?;
    refuse_existing(workspace, &request.name)?;

    let hook = workspace.onboarding_hook()?;
    if !request.hosts.is_empty() && hook.is_empty() {
        return Err(Error::HostWithoutHook);
    }

    let relative = scaffold_path(&request.name);
    let absolute = workspace.absolute(&relative);

    progress.write(&plan(&relative, request));
    if !request.assume_yes {
        progress.write("  scaffold this? [y/N] ");
        if !confirm.scaffold()? {
            return Err(Error::ScaffoldDeclined);
        }
    }

    if let Some(directory) = absolute.parent()
        && !directory.is_dir()
    {
        scratch::register_dir(directory);
        std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
            path: directory.display().to_string(),
            cause,
        })?;
    }
    std::fs::write(&absolute, declaration(&request.name, &request.recipient)).map_err(|cause| {
        Error::FileUnwritable {
            path: absolute.display().to_string(),
            cause,
        }
    })?;

    // Cleared once the file is written: from here it is the command's output
    // rather than scratch, and the guard must not reclaim the directory it has
    // just filled.
    scratch::keep_dirs();

    // Every generated file is parsed before anything is staged. A scaffold that
    // does not parse would be committed alongside a regenerated .sops.yaml and
    // found at the next evaluation, with the recipient policy already moved.
    if !workspace.nix().parses(&absolute) {
        return Err(Error::Unparsable {
            path: absolute.display().to_string(),
        });
    }

    // Staged before the policy is regenerated rather than after, because a flake
    // evaluation reads the files git knows about and nothing else. An untracked
    // scaffold is invisible to `flake.safix.lib.policyText`, so regenerating
    // first would write the policy of the declarations as they stood WITHOUT
    // this person — a .sops.yaml that looks freshly generated, carries no anchor
    // for them, and disagrees with the tree it was committed beside.
    let written = vec![relative.clone(), String::from(".sops.yaml")];
    workspace
        .git()
        .stage(workspace.root(), std::slice::from_ref(&relative))?;

    let root = workspace.root();
    let staging = root.join(".sops.yaml.new");
    workspace
        .nix()
        .eval_raw_to(root, Attribute::PolicyText, &staging)
        .map_err(|_| Error::PolicyEvalAfterScaffold {
            root: root.display().to_string(),
        })?;
    let policy = root.join(".sops.yaml");
    std::fs::rename(&staging, &policy).map_err(|cause| Error::FileUnwritable {
        path: policy.display().to_string(),
        cause,
    })?;

    git::commit_written_files(
        workspace.git(),
        root,
        progress,
        &format!(
            "feat(safix): declare {} and regenerate the recipient policy",
            request.name
        ),
        &written,
    )?;

    progress.write(&done(&relative, &request.name));
    run_hook(workspace, progress, &hook, request)?;
    progress.write(&remaining(&relative, &request.name));
    Ok(())
}

/// The name is interpolated into a path and into a rule's `path_regex`, so the
/// alphabet excludes everything that could act as a separator or a metacharacter.
fn refuse_bad_name(workspace: &Workspace, name: &str) -> Result<()> {
    let pattern = workspace.name_regex()?;
    // `builtins.match` anchors the whole string; the pattern the nix half
    // publishes does not carry the anchors, so they are added here rather than
    // assumed of it.
    let anchored = regex_lite::Regex::new(&format!("^(?:{pattern})$")).map_err(|cause| {
        Error::NixSchemaMismatch {
            attribute: "flake.safix.lib.nameRegex",
            cause: cause.to_string(),
        }
    })?;
    if anchored.is_match(name) {
        return Ok(());
    }
    Err(Error::BadUserName {
        name: name.to_owned(),
        pattern,
    })
}

/// Shape only. Nothing here can tell whether anyone holds the private half, and a
/// recipient no one can decrypt with is the one error this command cannot catch.
fn refuse_bad_recipient(recipient: &str) -> Result<()> {
    if recipient.starts_with(HARDWARE_PREFIX) {
        return Err(Error::HardwareRecipient {
            recipient: recipient.to_owned(),
        });
    }
    let well_formed = recipient.strip_prefix("age1").is_some_and(|body| {
        body.len() == RECIPIENT_BODY && body.chars().all(|letter| BECH32.contains(letter))
    });
    if well_formed {
        return Ok(());
    }
    Err(Error::BadRecipient {
        recipient: recipient.to_owned(),
    })
}

/// Editing an existing person is not what this does, and scaffolding over a file
/// that declares nobody is not either.
fn refuse_existing(workspace: &Workspace, name: &str) -> Result<()> {
    if workspace.placements()?.declares(name) {
        return Err(Error::AlreadyDeclared {
            user: name.to_owned(),
        });
    }
    let relative = scaffold_path(name);
    if workspace.absolute(&relative).exists() {
        return Err(Error::ScaffoldExists { file: relative });
    }
    Ok(())
}

/// What is about to happen, before it does.
fn plan(relative: &str, request: &Request) -> String {
    let mut block = format!(
        "\nsafix: declare {name}\n\n  {relative}   custody record, holds nothing yet\n  \
        .sops.yaml                regenerated from the above\n\n  recipient {recipient}\n  \
        no value is written, no key is minted.\n",
        name = request.name,
        recipient = request.recipient,
    );
    if !request.hosts.is_empty() {
        block.push_str("\n  then flake.safix.onboardingHook, with:\n");
        for host in &request.hosts {
            block.push_str("    --host ");
            block.push_str(host);
            block.push('\n');
        }
    }
    block.push('\n');
    block
}

/// The custody record itself.
fn declaration(name: &str, recipient: &str) -> String {
    format!(
        "# {path} — {name}'s custody record.\n\
        #\n\
        # Scaffolded by `safix adduser`. This file holds who can read what and nothing\n\
        # else: no account, no identifier, no profile. Move it anywhere this flake\n\
        # imports and it resolves the same — declarations merge, so where one is written\n\
        # is not something safix knows or cares about.\n\
        {{\n\
        \x20 flake.safix.users.{name} = {{\n\
        \x20   # The age public key this person's secrets are encrypted to, handed over by\n\
        \x20   # them. A recipient, never an identity: the private half stays on their\n\
        \x20   # machine, nothing here can decrypt anything, and this file names no private\n\
        \x20   # key.\n\
        \x20   #\n\
        \x20   # recoveryRecipients is deliberately absent. With this key alone their\n\
        \x20   # custody is independent — no one else can open what they own — and the cost\n\
        \x20   # is that losing it makes those files unopenable by everyone, because adding\n\
        \x20   # a recipient to a file requires decrypting it first. The mitigation that\n\
        \x20   # keeps independence is a second recipient THEY hold, listed here before\n\
        \x20   # their first secret is committed.\n\
        \x20   recipient = \"{recipient}\";\n\
        \n\
        \x20   # Both empty, which is what a person who holds nothing yet looks like. The\n\
        \x20   # first name added to `private` is declared and selected in one stroke;\n\
        \x20   # regenerating the policy is what writes the creation rule their file is\n\
        \x20   # made through, so `safix fix` comes before `safix set` for the first one.\n\
        \x20   #\n\
        \x20   # Catalogue selection is by explicit name rather than every entry in\n\
        \x20   # flake.safix.catalogue, so an entry added for someone else does not\n\
        \x20   # silently join this user.\n\
        \x20   carries = {{ }};\n\
        \x20   private = {{ }};\n\
        \x20 }};\n\
        }}\n",
        path = scaffold_path(name),
    )
}

/// What was done, and what was deliberately not.
fn done(relative: &str, name: &str) -> String {
    format!(
        "\nsafix: {name} is declared.\n\
        \n\
        What was done:\n\
        \x20 - {relative}, holding their recipient and nothing else\n\
        \x20 - .sops.yaml regenerated, carrying their key as an anchor\n\
        \x20 - both committed together\n\
        \n\
        What was NOT done, because it is not safix's:\n\
        \x20 - no key was minted. They run safix keygen on THEIR machine.\n\
        \x20 - no account, identifier, group or password hash anywhere.\n\
        \x20 - no creation rule for them yet: they hold nothing, so no audience\n\
        \x20   includes them and no rule is emitted.\n\
        \n"
    )
}

/// What is left, and why none of it is this command's.
fn remaining(relative: &str, name: &str) -> String {
    format!(
        "What remains, and none of it is something this command may do for you:\n\
        \n\
        \x20 the recipient — it has to be a key {name} themselves holds the private\n\
        \x20   half of. Nothing here checked that, because nothing here can: only\n\
        \x20   the shape was verified. If that string did not come from them,\n\
        \x20   every file it is added to is one they cannot open.\n\
        \n\
        \x20 their first secret — add a name to {relative} under private or carries,\n\
        \x20   then safix fix to write the rule, then safix set {name} <name>.\n"
    )
}

/// The consumer-supplied invocation, after the safix-owned scaffolding is
/// committed.
///
/// After, so that whatever the hook does is its own to stage and commit and this
/// command's single-intent commit stays single-intent. safix makes no assumption
/// about what it does and reports its exit status without interpreting it.
fn run_hook(
    workspace: &Workspace,
    progress: &dyn Progress,
    hook: &str,
    request: &Request,
) -> Result<()> {
    if hook.is_empty() {
        note(
            progress,
            "flake.safix.onboardingHook is unset, so nothing further ran.",
        );
        return Ok(());
    }

    log(progress, "safix: running flake.safix.onboardingHook");
    let status = Command::new("bash")
        .arg("-euo")
        .arg("pipefail")
        .arg("-c")
        .arg(hook)
        .arg("safix-onboarding-hook")
        .arg(&request.name)
        .arg(&request.recipient)
        .args(&request.hosts)
        .current_dir(workspace.root())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|_| Error::HookFailed { status: 127 })?
        .code()
        .unwrap_or(1);

    if status != 0 {
        return Err(Error::HookFailed { status });
    }
    note(
        progress,
        "the hook ran; anything it wrote is uncommitted and yours to review.",
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic throughout: 58 bech32 characters, minted by nobody.
    const WELL_FORMED: &str = "age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq";

    #[test]
    fn a_well_formed_recipient_is_age1_and_fifty_eight_bech32_characters() {
        assert!(refuse_bad_recipient(WELL_FORMED).is_ok());
    }

    #[test]
    fn a_card_is_refused_for_this_field_rather_than_for_its_shape() {
        let card = format!("age1yubikey1{}", "q".repeat(RECIPIENT_BODY));
        assert!(matches!(
            refuse_bad_recipient(&card),
            Err(Error::HardwareRecipient { .. })
        ));
    }

    #[test]
    fn the_characters_bech32_leaves_out_are_left_out() {
        for excluded in ['1', 'b', 'i', 'o'] {
            let mut body = "q".repeat(RECIPIENT_BODY.saturating_sub(1));
            body.push(excluded);
            assert!(matches!(
                refuse_bad_recipient(&format!("age1{body}")),
                Err(Error::BadRecipient { .. })
            ));
        }
    }

    #[test]
    fn a_recipient_of_the_wrong_length_or_the_wrong_prefix_is_refused() {
        for wrong in [
            "age1",
            "age2qqqq",
            &WELL_FORMED[..40],
            &format!("{WELL_FORMED}q"),
        ] {
            assert!(matches!(
                refuse_bad_recipient(wrong),
                Err(Error::BadRecipient { .. })
            ));
        }
    }

    #[test]
    fn the_scaffold_names_the_recipient_and_no_private_key() {
        let text = declaration("alice", WELL_FORMED);
        assert!(text.contains(&format!("recipient = \"{WELL_FORMED}\";")));
        assert!(text.contains("flake.safix.users.alice = {"));
        assert!(text.contains("carries = { };"));
        assert!(text.starts_with("# safix/users/alice.nix — alice's custody record."));
    }
}
