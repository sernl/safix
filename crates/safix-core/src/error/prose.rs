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

use crate::delegation::Refused;

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

/// A run order carrying a cycle, which is an order no walk can start.
///
/// The path is spelled as the resolver spells its own — each generator quoted,
/// joined by arrows, closing on the name it re-enters — so an operator who has
/// seen one refusal reads the other without translating it.
pub(super) fn generator_cycle(user: &str, cycle: &[String]) -> String {
    let path = cycle
        .iter()
        .map(|node| format!("'{node}'"))
        .collect::<Vec<_>>()
        .join(" -> ");
    format!(
        "the run order for flake.safix.users.{user} carries a cycle of generators:\n\
        {path}. Nothing can run first, so nothing runs.\n\
        \n\
        Nothing was written. The order is flake.safix.lib.generatorPlan's and\n\
        this runtime derives none of its own: a cycle is refused at evaluation,\n\
        and the generators inside one are left out of the order rather than\n\
        placed in it. An order carrying one therefore did not come from that\n\
        refusal — a stand-in for nix, or a program that built the plan itself.\n\
        \n\
        Refused before the first generator rather than at the one that cannot\n\
        read its input, because this command commits each generator as it goes: a\n\
        cycle met part-way through a run has already committed values and cannot\n\
        finish, and a committed value is a distributed one."
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

/// `--show` was asked and no identity has been minted on this machine yet.
pub(super) fn keygen_no_identity_yet(file: &str) -> String {
    format!(
        "{file} holds no identity yet, so there is no public recipient to\n\
        show.\n\
        \n\
        Mint one first:\n\
        \n\
        \x20   safix keygen"
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

/// clan is not there, and both verbs stop before touching anything.
pub(super) fn clan_unavailable(program: &str) -> String {
    format!(
        "could not run {program}, and clan is the authority on its own store.\n\
        \n\
        Nothing was transferred. safix does not read, write, encrypt or decrypt\n\
        a file clan placed: every value crosses that boundary through clan's own\n\
        command, in both directions, so a run without it does none of its\n\
        mappings rather than some of them. A run that did some would report the\n\
        rest as unchanged, which is a claim about a side it never looked at.\n\
        \n\
        Install clan, or name the one to use in SAFIX_CLAN."
    )
}

/// The clan half of a mapping names something clan does not have.
pub(super) fn clan_var_unknown(
    mapping: &str,
    machine: &str,
    generator: &str,
    file: &str,
) -> String {
    format!(
        "the mapping '{mapping}' names a var clan does not have.\n\
        \n\
        \x20   machine     {machine}\n\
        \x20   generator   {generator}\n\
        \x20   file        {file}\n\
        \n\
        Evaluation could not have caught this. The clan half of a mapping lives\n\
        in another flake, and the only thing that can answer whether it resolves\n\
        is clan itself.\n\
        \n\
        Check the three names against clan's own list:\n\
        \n\
        \x20   clan vars list {machine}"
    )
}

/// clan refused, and what it said is carried rather than reworded.
pub(super) fn clan_command_failed(
    mapping: &str,
    machine: &str,
    var_id: &str,
    output: &str,
) -> String {
    format!(
        "clan refused {machine} {var_id}, transferring the mapping '{mapping}'.\n\
        \n\
        clan said:\n\
        \n\
        {output}"
    )
}

/// A mapping name nothing declares.
pub(super) fn unknown_mapping(mapping: &str, declared: &str) -> String {
    if declared.is_empty() {
        return format!(
            "'{mapping}' is not a declared mapping, and no mapping is declared.\n\
            \n\
            A mapping is a declaration rather than an argument. Declare one under\n\
            flake.safix.bridge.mappings, naming both endpoints and a direction."
        );
    }
    format!(
        "'{mapping}' is not a declared mapping.\n\
        \n\
        Declared: {declared}"
    )
}

/// A mirror mapping name nothing declares.
///
/// Separate from [`unknown_mapping`] rather than sharing it, because the empty
/// case has to name the option the operator would declare one under and the two
/// surfaces have different ones.
pub(super) fn unknown_sync_mapping(mapping: &str, declared: &str) -> String {
    if declared.is_empty() {
        return format!(
            "'{mapping}' is not a declared mapping, and no mapping is declared.\n\
            \n\
            A mapping is a declaration rather than an argument. Declare one under\n\
            flake.safix.keepassxc.mappings, naming both endpoints and a mode."
        );
    }
    format!(
        "'{mapping}' is not a declared mapping.\n\
        \n\
        Declared: {declared}"
    )
}

/// A named mapping is declared with a direction the `--direction` filter does
/// not accept.
pub(super) fn mapping_wrong_direction(mapping: &str, actual: &str, filter: &str) -> String {
    format!(
        "the mapping '{mapping}' is declared {actual}, not {filter}.\n\
        \n\
        --direction {filter} narrows the run to mappings declared with that\n\
        value; '{mapping}' is declared {actual} instead. Drop --direction to\n\
        act on it, or narrow to its own direction with --direction {actual}."
    )
}

/// A word `sync` and `audit` read as a target keyword was given where a
/// mapping name belongs.
pub(super) fn reserved_mapping_word(word: &str) -> String {
    format!(
        "'{word}' is a target keyword, not a mapping name.\n\
        \n\
        sync and audit read clan and keepassxc as target keywords, never as\n\
        mapping names — no declared mapping may be named either, or 'all'.\n\
        Name the mapping you meant, or drop it to act on every mapping of the\n\
        target already named."
    )
}

/// A mapping name was given to `sync` or `audit` before any target was named.
pub(super) fn mapping_name_needs_target(verb: &str, name: &str) -> String {
    format!(
        "'{name}' was read as a mapping name, and no target was named first.\n\
        \n\
        A mapping name may follow clan or keepassxc on {verb}, never neither: a\n\
        mapping id may be declared under both targets' namespaces, so guessing\n\
        which one a bare name belongs to would be ambiguous exactly when it\n\
        matters. Name a target first:\n\
        \n\
        \x20   safix {verb} clan {name}\n\
        \x20   safix {verb} keepassxc {name}"
    )
}

/// `--direction` was given to a target other than `clan`.
pub(super) fn direction_on_wrong_target(target: &str) -> String {
    format!(
        "--direction is refused on {target}.\n\
        \n\
        Only the clan target accepts --direction, because a keepassxc mapping\n\
        declares a mode rather than a direction, and a mode narrows a run by\n\
        being named rather than by a run-time flag. Drop --direction, or name\n\
        the clan target instead."
    )
}

/// An export whose source entry has nothing in it.
///
/// The remedy that leads is the one that applies: naming `safix set` first for
/// an entry a generator mints would be telling the operator to type a value the
/// declarations say is minted.
pub(super) fn source_has_no_value(
    mapping: &str,
    user: &str,
    name: &str,
    file: &str,
    generated: bool,
) -> String {
    let remedy = if generated {
        format!(
            "\x20   safix generate {user} {name}\n\
            \n\
            or, if this value is not to be minted after all, set it by hand:\n\
            \n\
            \x20   safix set {user} {name}"
        )
    } else {
        format!(
            "\x20   safix set {user} {name}\n\
            \n\
            or, if a generator should mint it, declare one and run:\n\
            \n\
            \x20   safix generate {user} {name}"
        )
    };
    format!(
        "the mapping '{mapping}' exports {name} for {user}, which holds no value yet.\n\
        \n\
        {file} does not carry the key. Evaluation could not have caught this: an\n\
        entry declares where a value lives, not that one is there, and the two\n\
        are the same declaration until something reads the file.\n\
        \n\
        Put a value there first:\n\
        \n\
        {remedy}"
    )
}

/// An export whose source the operator cannot open.
pub(super) fn source_unreadable(mapping: &str, user: &str, name: &str, file: &str) -> String {
    format!(
        "the mapping '{mapping}' exports {name} for {user}, and {file} did not\n\
        decrypt. sops has said why, above this.\n\
        \n\
        The mapping is refused rather than transferred. A value that cannot be\n\
        read cannot be verified, and pushing an unverifiable value into another\n\
        store is worse than not pushing it."
    )
}

/// An export into a generator clan already considers stale.
///
/// Both remedies are named because the second is the right one often enough to
/// belong here: a mapping whose clan-side generator keeps being edited is a
/// mapping declared in the wrong direction, and clan is the producer.
pub(super) fn generator_definition_drifted(
    mapping: &str,
    machine: &str,
    generator: &str,
) -> String {
    format!(
        "clan considers the generator '{generator}' on {machine} outdated, so exporting\n\
        the mapping '{mapping}' would write a value clan's next generation replaces.\n\
        \n\
        clan records a validation for each generator and regenerates when the\n\
        recorded one no longer matches the definition. That has already happened\n\
        here, so the next routine\n\
        \n\
        \x20   clan vars generate {machine}\n\
        \n\
        would discard whatever this export wrote, without saying so.\n\
        \n\
        Two ways out. Either bring clan's side back into agreement, by running\n\
        that generation now and accepting the value it mints; or declare this\n\
        mapping clan-to-safix instead, which is the right shape when clan's\n\
        generator is what produces the value.\n\
        \n\
        There is no option that exports anyway. safix has nowhere to record that\n\
        this var is externally supplied, so proceeding would be a silent loss\n\
        rather than an accepted risk."
    )
}

/// A transfer with no clan to transfer with.
pub(super) fn no_clan_flake() -> String {
    "no clan is declared, so there is nothing to transfer with.\n\
    \n\
    Set flake.safix.bridge.clanFlake to the clan this consumer bridges to. It is\n\
    declared once for the consumer rather than once per mapping."
        .to_owned()
}

/// An OTP slot was asked for, and no flag will ever accept the ask.
///
/// The one refusal in this package whose reason is a property of the machine it
/// runs on rather than of the declarations: the fleet's password database is
/// opened by a challenge-response secret on OTP slot 2 of both keys, and writing
/// that slot replaces the secret with one the database has never seen.
pub(super) const OTP_REFUSED: &str = "\
safix enroll does not write, reprogram or delete an OTP slot, under any flag.

A programmed challenge-response slot is what opens a password database, and the
database has no record of the secret it was built with. Writing that slot
replaces the factor and the database stops opening — permanently, for every copy
of it, with no recovery that does not already require what was in it.

The two applets are disjoint. safix drives PIV: a PIN, a PUK, a protected
management key and an age identity in a retired slot, none of which touches OTP.

Extending a challenge-response factor to a second card is a deliberate manual
act with the database's life at stake, and KeePassXC's own enrollment of one is
GUI-only. It is not automated here, and that is a decision rather than an
omission.";

/// Two cards and nothing to choose between them.
pub(super) fn cards_ambiguous(serials: &[String]) -> String {
    format!(
        "more than one card is connected, so there is nothing to enroll unambiguously.\n\
        \n\
        Connected:{}\n\
        \n\
        Name the one you mean:\n\
        \n\
        \x20   safix enroll --serial <serial> [<user>]\n\
        \n\
        Guessing is the one guess with a provisioning at the end of it: the card\n\
        that was not meant would have its PIN, PUK and management key replaced.",
        bulleted(serials)
    )
}

/// No smartcard service, so no reader answers.
pub(super) const PCSCD_UNAVAILABLE: &str = "\
no smartcard service answered, so no card can be reached.

A YubiKey's PIV applet is reached over PC/SC, and on this fleet that means
pcscd. Nothing was touched.

    services.pcscd.enable = true;

Then re-insert the card and re-run. A card held exclusively by another agent —
a running ssh-agent, a browser, a second enrollment — presents the same way, so
check for one before concluding the service is absent.";

/// A run with no terminal, refused before the card is touched.
pub(super) const NO_TERMINAL: &str = "\
enrollment needs a terminal, and there is none. Nothing was touched.

The card is generated with touch-policy cached, so somebody has to touch it, and
somebody has to be told when. Both of those need a terminal: the instruction goes
to standard error and the generator's own PIN prompt is answered on a
pseudo-terminal this run opens.

An enrollment that could run unattended would be one whose card was generated
with touch-policy never, which is a smartcard emulating a file. That is refused
separately, and for the same reason.";

/// A choice was needed and there is no terminal to offer it on.
///
/// Deliberately not [`NO_TERMINAL`]: that one is enrollment's, down to its
/// touch-policy paragraph, and printing it at an operator who asked to read a
/// secret would answer a question they did not ask.
pub(super) const PICKER_NEEDS_TERMINAL: &str = "\
choosing needs a terminal, and there is none to offer one on. Nothing was
decrypted.

A picker draws a list, reads single keystrokes, and redraws — so it needs
/dev/tty and it needs to put it in raw mode. Where either is unavailable there
is no reduced picker to fall back to: a list that cannot be moved through is a
list, which is safix list, and a prompt that reads a line is not what was asked
for. So this refuses rather than degrading.

Two ways on. Name the entry, which never needs a terminal and is what a
pipeline wants:

\x20   safix view <name>

Or see what is there first, and then name it:

\x20   safix list [<user>]";

/// A choice was needed and the user holds nothing to choose from.
pub(super) fn nothing_to_pick(user: &str) -> String {
    format!(
        "flake.safix.users.{user} holds no secret, so there is nothing to choose\n\
        from. Nothing was decrypted.\n\
        \n\
        This is a state of the declarations rather than of the session: the same\n\
        run on a terminal, with a preview, over a user who holds something would\n\
        have offered it. safix list {user} prints the same emptiness without\n\
        refusing.\n\
        \n\
        safix edit additionally offers nothing for a user who holds only public\n\
        outputs: a public output is refused for editing, so a choice that offered\n\
        one would be offering a refusal."
    )
}

/// The operator left the choice without making one.
///
/// The one refusal in this file that is not a sentence, and deliberately: every
/// other one describes something that went wrong and names the way out, where
/// leaving a picker is the operator doing exactly what they meant to do. The
/// paragraph this used to carry — the terminal is back, nothing was written,
/// nothing decrypted was kept — described the picker's own guarantees to
/// somebody who had just pressed escape and was reading their prompt again. So
/// what is left is the machine-readable name of the outcome and nothing else;
/// the guarantees are stated where they belong, in `README.md` and in
/// `safix view -h`.
pub(super) const SELECTION_CANCELLED: &str = "safix::selection_cancelled";

/// A PIN this run generated, or was given, that the card refused.
pub(super) fn card_pin_rejected(serial: &str) -> String {
    format!(
        "{serial} refused the PIN, and the generator asked again. Nothing further was\n\
        attempted.\n\
        \n\
        One attempt, deliberately: a card allows three and a run that answered\n\
        every prompt would spend all three on the same wrong PIN and block the\n\
        card. The counter is at two rather than at zero.\n\
        \n\
        If safix provisioned this card, its PIN is in custody — safix get {serial}'s\n\
        access entry — or in the password store beside it. If somebody else did,\n\
        the PIN is theirs to supply.\n\
        \n\
        A blocked PIN is unblocked with the PUK; a blocked PUK leaves the PIV\n\
        applet resettable and every identity on it gone."
    )
}

/// A person with no custody record to add a recipient to.
pub(super) fn no_declaration_file(user: &str, file: &str) -> String {
    format!(
        "{file} is not a custody record this can extend, so {user}'s recovery\n\
        recipients were not touched.\n\
        \n\
        Enrollment adds a recipient to a record that already declares one. The\n\
        record is written by:\n\
        \n\
        \x20   safix adduser {user} <age-recipient>\n\
        \n\
        A record living somewhere else is supported — declarations merge, so where\n\
        one is written is not something safix knows — but the edit has to have a\n\
        file to make, so move it to that path or add the recipient by hand and\n\
        re-run for the rest of the ceremony."
    )
}

/// A re-wrap that took an existing reader's stanza away.
pub(super) fn recipients_lost(file: &str, lost: &[String]) -> String {
    format!(
        "{file} lost a recipient it had before this run, so nothing was committed.\n\
        \n\
        No longer able to open it:{}\n\
        \n\
        Enrollment is additive and only additive: it appends an identity, appends a\n\
        recipient and re-wraps every governed file to the policy those imply. A\n\
        recipient that disappeared in the re-wrap means the policy narrowed for a\n\
        reason this run did not ask for — most often a declaration edited between\n\
        the last `safix fix` and now.\n\
        \n\
        The card's identity and its recipient are written and are correct. Review\n\
        what changed, converge deliberately, then re-run:\n\
        \n\
        \x20   safix check\n\
        \x20   git diff",
        bulleted(lost)
    )
}

/// A mirror mapping whose safix side holds nothing to mirror.
///
/// Separate from [`source_has_no_value`] rather than sharing it: that one says
/// the mapping exports the entry, and this mapping does not export anything.
pub(super) fn sync_source_empty(
    mapping: &str,
    user: &str,
    name: &str,
    file: &str,
    generated: bool,
) -> String {
    let remedy = if generated {
        format!("\x20   safix generate {user} {name}")
    } else {
        format!("\x20   safix set {user} {name}")
    };
    format!(
        "the mapping '{mapping}' mirrors {name} for {user} into the database, and it holds\n\
        no value yet. Nothing was written.\n\
        \n\
        {file} does not carry the key. Evaluation could not have caught this: an\n\
        entry declares where a value lives, not that one is there.\n\
        \n\
        Give it a value, then re-run:\n\
        \n\
        {remedy}"
    )
}

/// A mirror mapping whose database side holds no entry to read.
pub(super) fn store_entry_absent(mapping: &str, entry: &str, mode: &str) -> String {
    format!(
        "the mapping '{mapping}' is {mode}, so the database is where its value comes\n\
        from, and the database holds no entry at '{entry}'. Nothing was written.\n\
        \n\
        safix does not author that entry, and this is the one place the asymmetry\n\
        shows: a value the operator types into their own database is theirs to\n\
        create, where a value safix mints is safix's. Create it, and the next run\n\
        converges safix onto it.\n\
        \n\
        If safix is the producer after all, the mapping is declared the wrong way\n\
        round: mode = \"safix-to-keepassxc\" makes the database follow safix."
    )
}

/// Mappings declared and no database for them to reach.
pub(super) fn no_store_database(mappings: usize) -> String {
    format!(
        "flake.safix.keepassxc declares {mappings} mapping(s) and no database, so there\n\
        is nothing for them to converge against.\n\
        \n\
        There is no default and there cannot be one: which database holds a\n\
        person's credentials is a fact about their machine. Name it as a string\n\
        rather than a nix path — a path is copied into the store when it is\n\
        interpolated, and the store is world-readable:\n\
        \n\
        \x20   flake.safix.keepassxc.database = \"/home/<you>/.keys/master.kdbx\";"
    )
}

/// A run with no terminal to ask for the database's password on.
pub(super) fn store_locked(database: &str) -> String {
    format!(
        "{database} needs its password and there is no terminal to ask on. Nothing\n\
        was read.\n\
        \n\
        The refusal is here rather than after the first mapping deliberately: a run\n\
        that prompted into the void would have decrypted safix's side of every\n\
        mapping first, and a run that treated the database as empty would report\n\
        every mapping as one-sided and, in backup mode, write.\n\
        \n\
        There is one way to provide the database, and it is the declared path plus\n\
        the password on a terminal. The session's secret service is not a second\n\
        way: the collection it publishes is KeePassXC's exposed group, so it cannot\n\
        address the group and path a mapping declares.\n\
        \n\
        Run this where you can type, or leave the mapping to a run that can."
    )
}

/// The store's own command would not open the database.
pub(super) fn database_unreadable(database: &str, output: &str) -> String {
    format!(
        "{database} did not open, so no mapping was judged and nothing was written.\n\
        \n\
        A wrong password and an unreadable file present the same way here, and the\n\
        store's own message below is what tells them apart.\n\
        \n\
        {output}"
    )
}

/// The store's own command refused over one entry.
pub(super) fn store_command_failed(entry: &str, arguments: &str, output: &str) -> String {
    format!(
        "the store's own command refused over the entry '{entry}'.\n\
        \n\
        It was run as:\n\
        \n\
        \x20   keepassxc-cli {arguments}\n\
        \n\
        No value is in that line: the database's password and the entry's both\n\
        travel standard input. The command said:\n\
        \n\
        {output}"
    )
}

/// A value the store's own command cannot carry whole.
pub(super) fn value_spans_lines(entry: &str) -> String {
    format!(
        "the value for '{entry}' carries a newline, and the store's own command reads\n\
        an entry's password as one line. Nothing was written.\n\
        \n\
        What would land is the bytes before the first newline, which is a mirror\n\
        that lies about what it holds — and the comparison that decides whether a\n\
        run has anything to do is byte-exact, so a value ending in a newline would\n\
        differ from the stored one on every run and rewrite the whole database each\n\
        time, forever.\n\
        \n\
        Nothing here strips the byte for you. `echo` appends a newline and `printf`\n\
        does not, so a value minted or typed with one is re-established without:\n\
        \n\
        \x20   printf '%s' \"$VALUE\" | safix set <user> <name>\n\
        \n\
        or, where a generator minted it, change the generator's last write to\n\
        `printf` and re-run `safix generate --regenerate <user> <name>`."
    )
}

/// A declared field the target cannot carry at all.
pub(super) fn field_unsupported(target: &str, field: &str) -> String {
    format!(
        "the mapping declares the field '{field}', and {target} cannot carry it. The\n\
        store's own command has no way to write that field, so nothing was written:\n\
        accepting the declaration would mean converging a mapping while doing less\n\
        than it says.\n\
        \n\
        Remove the field from the mapping, or move the mapping to a target whose\n\
        own command can write it."
    )
}

/// A declared field read out of another entry, on a field the target carries in
/// an argument vector.
pub(super) fn field_source_in_argv(target: &str, field: &str, entry: &str) -> String {
    format!(
        "the mapping sources the field '{field}' from the entry '{entry}', and {target}\n\
        carries that field in an argument vector. The entry's value is a secret, and a\n\
        secret value travels standard input or a pipe — never an argument vector, where\n\
        every process on the host can read it. Nothing was written.\n\
        \n\
        Either write the field as a literal in the declaration, which is evaluated into\n\
        the world-readable store and is therefore not a secret, or declare it on a\n\
        target whose channel for that field is a pipe."
    )
}

/// The store's own command could not be run at all.
pub(super) fn pass_unavailable(program: &str) -> String {
    format!(
        "could not run {program}, so no pass mapping was judged and nothing was\n\
        written on either side.\n\
        \n\
        safix reaches the store only through the store's own command; it never\n\
        reads or decrypts the files itself, because how a store encrypts an entry\n\
        is the whole of what the store is. Install it, or point safix at the one\n\
        you have:\n\
        \n\
        \x20   SAFIX_PASS=/path/to/pass safix sync pass"
    )
}

/// The operator's own agent declined to decrypt one entry.
pub(super) fn pass_locked(entry: &str, output: &str) -> String {
    format!(
        "the entry '{entry}' did not decrypt, so this mapping was not judged and\n\
        nothing was written for it.\n\
        \n\
        There is no safix option for this and there should not be: `pass` shells to\n\
        gpg, and the unlock belongs to your own agent — it may answer from its\n\
        cache, from a pinentry on a terminal this run does not have, or not at all.\n\
        safix does not interpose on that, because interposing would defeat a\n\
        hardware-backed key and would mean holding a passphrase safix has no use\n\
        for.\n\
        \n\
        This is not the absent-entry refusal: absence is answered from the store's\n\
        own listing of names, so a backup mapping never writes over an entry it\n\
        merely could not read.\n\
        \n\
        gpg said:\n\
        \n\
        {output}"
    )
}

/// The store's own command refused over one entry.
pub(super) fn pass_command_failed(entry: &str, arguments: &str, output: &str) -> String {
    format!(
        "the store's own command refused over the entry '{entry}'.\n\
        \n\
        It was run as:\n\
        \n\
        \x20   pass {arguments}\n\
        \n\
        No value and no field is in that line: the whole record body travels\n\
        standard input, and what the argument vector carries is the entry path.\n\
        The store's location is in the child's environment, which is a location\n\
        rather than a value. The command said:\n\
        \n\
        {output}"
    )
}

/// The declared store root is not a store.
pub(super) fn no_pass_store(store: &str, mappings: usize) -> String {
    format!(
        "{store} is not a pass store, so none of the {mappings} mapping(s) declared\n\
        against it was read and nothing was written.\n\
        \n\
        The refusal is here rather than after the first mapping deliberately: a run\n\
        that treated an absent store as an empty one would report every mapping as\n\
        one-sided and, in backup mode, write.\n\
        \n\
        A store is a directory carrying a .gpg-id, which is the store's own\n\
        declaration of who can read it. Either the location is wrong:\n\
        \n\
        \x20   flake.safix.pass.store = \"/home/<you>/.password-store\";\n\
        \n\
        or there is no store there yet, and creating one is yours rather than\n\
        safix's — writing a .gpg-id would be safix deciding who can read your\n\
        secrets:\n\
        \n\
        \x20   pass init <your-gpg-id>"
    )
}

/// A mapping whose store side holds no entry to read.
pub(super) fn pass_entry_absent(mapping: &str, entry: &str, mode: &str) -> String {
    format!(
        "the mapping '{mapping}' is {mode}, so the store is where its value comes\n\
        from, and the store holds no entry at '{entry}'. Nothing was written.\n\
        \n\
        safix does not author that entry, and this is the one place the asymmetry\n\
        shows: a value the operator puts in their own store is theirs to create,\n\
        where a value safix mints is safix's. Create it, and the next run converges\n\
        safix onto it.\n\
        \n\
        If safix is the producer after all, the mapping is declared the wrong way\n\
        round: mode = \"safix-to-pass\" makes the store follow safix."
    )
}

/// The vault's own client could not be run at all.
pub(super) fn bitwarden_unavailable(program: &str) -> String {
    format!(
        "could not run {program}, so no bitwarden mapping was judged and nothing was\n\
        written on either side.\n\
        \n\
        safix drives the vault through the client an operator already logged in\n\
        with; it speaks no vault protocol of its own. Install the client, or point\n\
        safix at the one you have:\n\
        \n\
        \x20   SAFIX_BW=/path/to/bw safix sync bitwarden"
    )
}

/// The client is locked, or not logged in at all.
pub(super) fn bitwarden_locked(state: &str) -> String {
    if state == "unauthenticated" {
        return String::from(
            "the vault's client is not logged in, so no side of any mapping was read and\n\
            nothing was written.\n\
            \n\
            safix unlocks a vault; it never logs one in. Logging in registers a device\n\
            against an account and is the operator's own act, exactly as creating the\n\
            vault is — a tool that did it for you would be authenticating as you\n\
            without being asked.\n\
            \n\
            Log in yourself, then re-run:\n\
            \n\
            \x20   bw login\n\
            \x20   safix sync bitwarden",
        );
    }
    String::from(
        "the vault's client is locked and there is no terminal to ask its master\n\
        password on. Nothing was read.\n\
        \n\
        The refusal is here rather than after the first mapping deliberately: a run\n\
        that prompted into the void would have decrypted safix's side of every\n\
        mapping first, and a run that treated a locked vault as an empty one would\n\
        report every mapping as one-sided and, in backup mode, write.\n\
        \n\
        Run this where you can type, or unlock the client first and let safix use\n\
        the session it finds:\n\
        \n\
        \x20   bw unlock",
    )
}

/// The client refused one invocation.
pub(super) fn bitwarden_command_failed(address: &str, arguments: &str, output: &str) -> String {
    format!(
        "the vault's own client refused over the item '{address}'.\n\
        \n\
        It was run as:\n\
        \n\
        \x20   bw {arguments}\n\
        \n\
        No value is in that line: the master password and every item payload travel\n\
        standard input. The client said:\n\
        \n\
        {output}"
    )
}

/// The declared server is not the one the unlocked client reached.
pub(super) fn bitwarden_server_mismatch(declared: &str, reached: &str) -> String {
    format!(
        "flake.safix.bitwarden.server declares {declared} and the unlocked client\n\
        reports reaching {reached}. Nothing was read and nothing was written.\n\
        \n\
        The refusal arrives before any side is read because a write against the\n\
        wrong vault is not correctable by a later run: the secret is in somebody\n\
        else's store the moment it lands, and removing it there does not unsend it.\n\
        \n\
        safix never repoints the client — which server it reaches is the operator's\n\
        own configuration. Either point the client at the declared server, or\n\
        declare the one it reaches:\n\
        \n\
        \x20   bw config server {declared}"
    )
}

/// The client's refresh of its own local copy failed.
pub(super) fn bitwarden_stale(output: &str) -> String {
    format!(
        "the vault's client could not refresh its local copy, so every bitwarden\n\
        mapping was refused: none was read, judged or written.\n\
        \n\
        The client's read commands answer from that local copy, and it may predate\n\
        another device's change. Comparing against it would report agreement that is\n\
        not there, and a backup mapping would write a value into an item that\n\
        already holds one — which is the exact outcome backup's non-overwriting rule\n\
        exists to prevent.\n\
        \n\
        The client said:\n\
        \n\
        {output}"
    )
}

/// One declared address matching more than one item.
pub(super) fn bitwarden_item_ambiguous(mapping: &str, address: &str, matched: usize) -> String {
    format!(
        "the mapping '{mapping}' addresses '{address}', and {matched} items in the vault\n\
        answer to it. Nothing was read and nothing was written for it.\n\
        \n\
        Nothing picks between them — not recency, not an identifier's order. A vault\n\
        legitimately holds two items with one name, and a mirror that chose would\n\
        converge a person's secret with whichever one happened to sort first.\n\
        \n\
        Rename one of the items, or move it to another folder, so that the declared\n\
        address names exactly one."
    )
}

/// A mapping whose vault side holds no item to read.
pub(super) fn bitwarden_item_absent(mapping: &str, address: &str, mode: &str) -> String {
    format!(
        "the mapping '{mapping}' is {mode}, so the vault is where its value comes from,\n\
        and the vault holds no item at '{address}'. Nothing was written.\n\
        \n\
        safix does not author that item, and this is the one place the asymmetry\n\
        shows: a value the operator types into their own vault is theirs to create,\n\
        where a value safix mints is safix's. Create it, and the next run converges\n\
        safix onto it.\n\
        \n\
        If safix is the producer after all, the mapping is declared the wrong way\n\
        round: mode = \"safix-to-bitwarden\" makes the vault follow safix."
    )
}

/// The 1Password command could not be run at all.
pub(super) fn onepassword_unavailable(program: &str) -> String {
    format!(
        "could not run {program}, so no 1password mapping was judged and nothing was\n\
        written on either side.\n\
        \n\
        safix reaches the service only through its own command, under whatever\n\
        session the operator already established; it speaks no 1Password protocol\n\
        and signs nothing in. No check of this repository runs the real command\n\
        either — the package is unfree, there is no self-hostable server, and every\n\
        authentication path needs the network. Install it, or point safix at the one\n\
        you have:\n\
        \n\
        \x20   SAFIX_OP=/path/to/op safix sync 1password"
    )
}

/// The session preflight did not answer.
pub(super) fn onepassword_signed_out(account: Option<&str>, output: &str) -> String {
    let named = match account {
        Some(account) => format!("the declared account '{account}'"),
        None => String::from("the account the command resolves for itself"),
    };
    format!(
        "the 1Password command would not answer for {named}, so no side of any mapping\n\
        was read and nothing was written.\n\
        \n\
        The session is yours rather than safix's: safix never signs in, mints, holds\n\
        or prints a session token, and it never puts one in an argument vector. What\n\
        the child process inherits is whatever your own environment carries — a\n\
        service-account token in OP_SERVICE_ACCOUNT_TOKEN, or a session your own\n\
        `op signin` established in this shell.\n\
        \n\
        This refusal precedes the first read, which is why safix's own side of no\n\
        mapping has been decrypted.\n\
        \n\
        The command said:\n\
        \n\
        {output}"
    )
}

/// The 1Password command refused over one item.
pub(super) fn onepassword_command_failed(item: &str, arguments: &str, output: &str) -> String {
    format!(
        "the 1Password command refused over the item '{item}'. That mapping is refused\n\
        and the rest of the run went on, so the report is complete over the\n\
        declarations.\n\
        \n\
        It was run as:\n\
        \n\
        \x20   op {arguments}\n\
        \n\
        No value and no field is in that line, by construction: the whole item\n\
        travels standard input as one JSON object, and safix never spells a\n\
        field=value assignment — the service's own documentation says such a\n\
        statement is recorded in shell history and can be visible to other\n\
        processes. The command said:\n\
        \n\
        {output}"
    )
}

/// A mapping whose 1Password side holds no item to read.
pub(super) fn onepassword_item_absent(
    mapping: &str,
    vault: &str,
    item: &str,
    mode: &str,
) -> String {
    format!(
        "the mapping '{mapping}' is {mode}, so the item is where its value comes from,\n\
        and the vault '{vault}' holds no item '{item}'. Nothing was written.\n\
        \n\
        safix does not author that item, and this is the one place the asymmetry\n\
        shows: a value the operator keeps in their own vault is theirs to create,\n\
        where a value safix mints is safix's. Create it, and the next run converges\n\
        safix onto it.\n\
        \n\
        If safix is the producer after all, the mapping is declared the wrong way\n\
        round: mode = \"safix-to-1password\" makes the item follow safix."
    )
}

/// The declared vault is not one this session can see.
pub(super) fn onepassword_vault_absent(mapping: &str, vault: &str, output: &str) -> String {
    format!(
        "the mapping '{mapping}' names the vault '{vault}', and this session cannot see\n\
        a vault by that name. Nothing was read and nothing was written for it.\n\
        \n\
        This is not the absent-item refusal and its remedy is not the same one: an\n\
        item that is not there is written by a pushing mode, where a vault that is\n\
        not reachable is a permission. A service account reaches exactly the vaults\n\
        it was granted and cannot reach a built-in Private, Personal or Employee\n\
        vault at all, so grant it that vault or name one it already has.\n\
        \n\
        The command said:\n\
        \n\
        {output}"
    )
}

/// A person whose audience covers no file the proof could use.
pub(super) fn no_file_to_prove_with(user: &str) -> String {
    format!(
        "{user}'s audience covers no file that exists, so there is nothing for the\n\
        card to open and the proof cannot run.\n\
        \n\
        The card's identity and its recipient are written and are correct: the\n\
        enrollment is additive and complete except for the one claim it exists to\n\
        make. A canary encrypted for the occasion is deliberately not what runs\n\
        here — it would prove that a fresh file made from a fresh rule opens, which\n\
        is not the question.\n\
        \n\
        Give {user} a secret, then re-run this verb:\n\
        \n\
        \x20   safix set {user} <name>\n\
        \x20   safix enroll {user}"
    )
}

/// A name no declaration of any subject kind covers.
///
/// The list is every subject rather than every person, because a group's members
/// are subjects: a machine, a service and another group each belong in one, so a
/// list of people would read as a narrower rule than the option has.
pub(super) fn unknown_subject(subject: &str, declared: &[String]) -> String {
    format!(
        "'{subject}' is not a declared subject, so no membership may name it.\n\
        \n\
        A subject is what can hold a key and appear in an audience: a person of\n\
        flake.safix.users, a machine, a service, or another group. An organization is\n\
        not one — a principal is not a member, and an audience wanting its custody\n\
        names the organization.\n\
        \n\
        A membership naming an undeclared subject is refused at the next evaluation,\n\
        so nothing was written: an edit that wrote one would commit a tree that no\n\
        longer resolves.\n\
        \n\
        Declared subjects:{}",
        bulleted(declared)
    )
}

/// A group declared somewhere this verb cannot edit.
pub(super) fn no_group_declaration(group: &str, file: &str) -> String {
    format!(
        "{file} is not a group declaration this can edit, so {group}'s membership\n\
        was not touched.\n\
        \n\
        This edits one `members` list as text: a line inserted or a line removed,\n\
        parsed before anything is staged. What it needs is that file, holding a\n\
        `members` list of names it can read — a list written by hand as one line or\n\
        as one name per line is both.\n\
        \n\
        A declaration living somewhere else is supported, and so is a `members` value\n\
        computed rather than written: declarations merge, so where one is written is\n\
        not something safix knows. But the edit has to have a file to make, so move\n\
        the declaration to that path or edit the membership by hand.\n\
        \n\
        A hand edit owes the same disclosure this verb prints: removing a subject\n\
        narrows every file the group's audience names and takes nothing back."
    )
}

/// A list of option paths, joined as every refusal here joins one.
fn joined(paths: &[String]) -> String {
    match paths.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::from("nothing"),
    }
}

/// The organizations a delegation names, as their option paths.
fn organizations(named: &[String]) -> String {
    joined(
        &named
            .iter()
            .map(|organization| format!("flake.safix.organizations.{organization}"))
            .collect::<Vec<_>>(),
    )
}

/// Where those organizations declare their managers, which is where a name joins
/// them.
fn manager_sites(named: &[String]) -> String {
    joined(
        &named
            .iter()
            .map(|organization| format!("flake.safix.organizations.{organization}.managers"))
            .collect::<Vec<_>>(),
    )
}

/// The sentence both delegation refusals open the record with.
fn delegated_by(delegation: &Refused) -> String {
    format!(
        "{} is delegated to {} by {}.",
        delegation.through.subject(),
        organizations(&delegation.organizations),
        delegation.through.site()
    )
}

/// A commit identity no declaration corresponds to, met where a delegation asked
/// who is acting.
///
/// The boundary sentence is reached through [`crate::delegation::BOUNDARY`] rather
/// than written here, so that no surface can carry a second wording of it.
pub(super) fn actor_undeclared(
    name: &str,
    email: &str,
    delegation: &Refused,
    declared: &[String],
) -> String {
    format!(
        "a commit made here would be authored by '{name} <{email}>', and\n\
        flake.safix.users declares nobody of that name.\n\
        \n\
        {}\n\
        A delegated scaffold is judged against the identity its own commit will\n\
        carry, so this run stops before editing anything.\n\
        \n\
        That identity is user.name and user.email as this repository resolves them,\n\
        because the commit is the act: there is no flag naming somebody else, since a\n\
        flag would let the check and the attribution disagree. It is matched to a\n\
        declared person by name and by nothing else — no declaration maps a git\n\
        identity to a person, and taking one from an address's local part is how the\n\
        wrong name ends up in history.\n\
        \n\
        Declared people:{}\n\
        \n\
        Set the identity this repository commits under to a name the fleet declares,\n\
        or declare the person these commits already name:\n\
        \n\
        \x20   git config user.name <name>\n\
        \n\
        {}",
        delegated_by(delegation),
        bulleted(declared),
        crate::delegation::BOUNDARY
    )
}

/// A declared person acting outside the delegation covering what they were about
/// to edit.
pub(super) fn scaffold_out_of_scope(
    actor: &str,
    delegation: &Refused,
    managers: &[String],
) -> String {
    format!(
        "{}\n\
        {actor} is not among the managers named there, so nothing about it was\n\
        edited: the check ran before any file was written, and the identity it read\n\
        is the one a commit made here would carry.\n\
        \n\
        Declared managers:{}\n\
        \n\
        A manager runs this, or this name joins them — one line under\n\
        {}, committed first.\n\
        \n\
        {}",
        delegated_by(delegation),
        bulleted(managers),
        manager_sites(&delegation.organizations),
        crate::delegation::BOUNDARY
    )
}

/// clan refused to list its machines, addressing a shared mapping.
pub(super) fn clan_machines_list_failed(output: &str) -> String {
    format!(
        "clan refused to list its machines.\n\
        \n\
        clan said:\n\
        \n\
        {output}"
    )
}

/// No machine clan has resolves a shared mapping's generator.
pub(super) fn clan_address_unresolved(mapping: &str, generator: &str, file: &str) -> String {
    format!(
        "no machine in clan's own fleet resolves the mapping '{mapping}'.\n\
        \n\
        \x20   placement   shared\n\
        \x20   generator   {generator}\n\
        \x20   file        {file}\n\
        \n\
        The machine that addresses a shared var on clan's command line is\n\
        discovered by asking clan which machines it has and trying each in turn,\n\
        rather than declared, so there is no second field on the mapping to check\n\
        against clan's own list.\n\
        \n\
        Check the generator and file against what any machine in the fleet sees:\n\
        \n\
        \x20   clan vars list <machine>"
    )
}

/// clan refused to list one machine's vars, enumerating them for the audit's
/// lingering report.
pub(super) fn clan_machine_list_failed(machine: &str, output: &str) -> String {
    format!(
        "clan refused to list {machine}'s vars.\n\
        \n\
        clan said:\n\
        \n\
        {output}"
    )
}

/// A vault-root commit landed with no matching declaration-root commit —
/// design V4's half-landed state.
pub(super) fn vault_commit_half_landed(vault_commit: &str, pending: &[String]) -> String {
    format!(
        "the vault committed {vault_commit}, but the declaration-root commit that was\n\
        to follow it failed, so nothing there is committed yet.\n\
        \n\
        Still staged at the declaration root:{}\n\
        \n\
        The vault content is correct and already committed; re-running the same\n\
        command completes the operation and will not repeat the vault commit,\n\
        because the content it would stage there already matches HEAD.",
        bulleted(pending)
    )
}

/// A stamp record that exists and does not parse.
///
/// The remedy names both moves and says what each costs, because a record
/// this refusal is about is one somebody edited by hand: restoring it from
/// history keeps the dates, and removing it keeps none of them.
pub(super) fn stamp_record_unparsable(path: &str) -> String {
    format!(
        "{path} is not a stamp record this version of safix can read.\n\
        \n\
        A stamp record is one line: v1 created=<unix seconds> updated=<unix\n\
        seconds>. Restore it from git history to keep the dates it held, or\n\
        remove it — an entry with no record reads as having no dates, which is\n\
        what every value written before the record existed reads as."
    )
}
