//! The refusals whose message is a page rather than a sentence.
//!
//! Every function here renders one variant of [`Error`](super::Error), and each
//! is the text the retired shell runtime printed for the same refusal, less
//! the `safix: ` prefix the command's reporter adds and less the trailing
//! newline it adds after it. They live beside the enum rather than inside its
//! attributes because a fifteen-line format string is a fifteen-line format
//! string wherever it is written, and the variant list is easier to read as a
//! list.
//!
//! `safix` is spelled out rather than taken from the command: this is the
//! library, and the shell runtime spells its own `$PROG` into the same
//! sentences.

/// The one-per-line bulleted continuation the shell writes with `sed 's/^/  - /'`.
///
/// Empty for an empty list, and leading rather than trailing with its newline,
/// so that a message ending in a list and a message ending in a heading with no
/// list under it both end without one.
pub(super) fn bulleted(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("\n  - {item}"))
        .collect::<Vec<_>>()
        .concat()
}

/// The recipient-drift refusal, whose two lists are each present or absent.
pub(super) fn drifted(file: &str, extra: &[String], missing: &[String]) -> String {
    let mut message = format!("{file} is not encrypted to the audience declared for it.\n\n");
    if !extra.is_empty() {
        message.push_str("Can open it and is not in its audience:");
        message.push_str(&bulleted(extra));
        message.push_str("\n\n");
    }
    if !missing.is_empty() {
        message.push_str("Is in its audience and cannot open it:");
        message.push_str(&bulleted(missing));
        message.push_str("\n\n");
    }
    message.push_str(
        "Nothing was written. A value set now would be wrapped for the recipients\n\
        above rather than for the declared audience, and this command commits what\n\
        it writes, so a reader the audience no longer names would read a value\n\
        minted after their removal straight out of git history.\n\
        \n\
        Re-wrap the file to its declared audience, review the diff, then re-run:\n\
        \n\
        \x20   safix fix\n",
    );
    message.push_str("    git diff -- ");
    message.push_str(file);
    message
}

/// A name with nothing to mint it.
pub(super) fn no_generator(user: &str, name: &str) -> String {
    format!(
        "'{name}' has no generator, so there is nothing to run.\n\
        \n\
        A generator is declared on the entry, beside its mode and its path:\n\
        \n\
        \x20   generator.script = \"openssl rand -base64 32\";\n\
        \x20   generator.runtimeInputs = [ \"openssl\" ];\n\
        \n\
        Only a value you are free to choose can have one. A credential some\n\
        server already knows is set by hand:\n\
        \n\
        \x20   safix set {user} {name}"
    )
}

/// Minting an identity for somebody else, on this machine.
pub(super) fn keygen_for_someone_else(user: &str) -> String {
    format!(
        "'{user}' is not you, and this writes a private key into your own\n\
        identity file.\n\
        \n\
        An age identity is custody: everything encrypted to its public half is\n\
        readable by whoever holds this private half. Minting one for another\n\
        person on your machine means you hold their key, which is the opposite of\n\
        the independent custody this package is built on. They should run this\n\
        themselves and hand you the public half.\n\
        \n\
        If you have decided otherwise anyway, say so:\n\
        \n\
        \x20   safix keygen --for-someone-else {user}"
    )
}

/// A name outside the alphabet a path and a `path_regex` are built from.
pub(super) fn bad_user_name(name: &str, pattern: &str) -> String {
    format!(
        "{name} is not a well-formed user name.\n\
        \n\
        A user name is interpolated into the path of the file their secrets are\n\
        placed in and into the path_regex of a .sops.yaml creation rule, so the\n\
        alphabet excludes everything that could act as a path separator or as a\n\
        regex metacharacter. A widened rule is how a sops updatekeys sweep\n\
        reaches a file it was never meant to touch.\n\
        \n\
        Names match {pattern}, anchored: lowercase letters and digits, then any of\n\
        those plus underscore and hyphen."
    )
}

/// A recipient nothing can decrypt to without a person present.
pub(super) fn hardware_recipient(recipient: &str) -> String {
    let shown: String = recipient.chars().take(20).collect();
    format!(
        "a recipient that needs a physical interaction cannot be the\n\
        primary one.\n\
        \n\
        Decrypting to {shown}...\n\
        requires the card present, a PIN and a touch, once per file. Activation\n\
        decrypts non-interactively, with the identity sops.age.sshKeyPaths\n\
        names, so a profile whose only recipient is a hardware key cannot\n\
        activate at all — and the failure lands at switch time on their\n\
        machine, not here.\n\
        \n\
        A card belongs in the same person's recoveryRecipients, which is\n\
        additive: every file their audience names is encrypted to it as well as\n\
        to recipient, so the card opens their files after the activation key is\n\
        lost and is needed at no other time.\n\
        \n\
        Pass their software recipient here, then add the card by hand."
    )
}

/// A recipient that is not shaped like an age X25519 key.
pub(super) fn bad_recipient(recipient: &str) -> String {
    format!(
        "{recipient} is not a well-formed age recipient.\n\
        \n\
        An age X25519 recipient is age1 followed by 58 bech32 characters\n\
        (no 1, b, i or o). This checks the shape and nothing else: whether anyone\n\
        holds the private half is not knowable from here.\n\
        \n\
        They mint one with safix keygen on their own machine, or convert an\n\
        ed25519 ssh key they already hold:\n\
        \n\
        \x20   ssh-to-age -i ~/.ssh/id_ed25519.pub"
    )
}

/// Declaring somebody the declarations already name.
pub(super) fn already_declared(user: &str) -> String {
    format!(
        "{user} is already a declared user.\n\
        \n\
        Editing an existing person is not what this does.\n\
        \n\
        What they hold is safix list {user}; changing their recipient is an edit to\n\
        flake.safix.users.{user}.recipient followed by safix fix, which re-wraps every\n\
        file their audience names and is explicitly not revocation."
    )
}

/// `--host` with nothing configured to receive it.
pub(super) const HOST_WITHOUT_HOOK: &str = "\
--host was given and flake.safix.onboardingHook is unset.

safix scaffolds a person's custody declaration and regenerates the recipient
policy. Attaching an account on a host is not one of those: allocating an
identifier, writing a per-host account module and editing that host's imports
are all properties of one consumer's module tree, and safix has no way to know
its shape.

Set the hook, which receives the name, the recipient and every --host given,
and runs after the scaffolding is committed:

    flake.safix.onboardingHook = ''
      name=\"$1\"
      recipient=\"$2\"
      shift 2
      for host in \"$@\"; do ... ; done
    '';

Or drop --host: onboarding without it succeeds, having done less.";
