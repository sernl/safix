//! What `-h` prints, for the subcommands this binary implements.
//!
//! One text per ported subcommand, word for word from
//! `modules/flake/safix/safix.sh`, because the help is part of what the
//! differential harness compares and the shell runtime is the oracle. It goes
//! to standard error and the process exits zero, which is also the shell
//! runtime's shape.
//!
//! [`SCAFFOLD`] is the shell's own general usage, unabridged, because this
//! binary now implements every subcommand it lists.

/// `safix set -h`.
pub const SET: &str = "\
safix set [<user>] <name>

Prompt for a value twice without echoing it, write it into the file the
declarations place <name> in, then stage and commit that file alone.

Values are single-line and stored exactly as typed, with no trailing newline. A
multi-line value is `sops <file>`, or a generator — `generate` stores what a
script produces, newlines and all.

This is the hand-typed case, and it stays separate from `generate` on purpose: a
credential some server already knows cannot be minted, only transcribed.
";

/// `safix fix -h`.
pub const FIX: &str = "\
safix fix [--yes]

Regenerate .sops.yaml from the declarations, then re-wrap each governed file's
data key to the audience that policy declares. --yes answers sops' confirmation.

The order is not interchangeable: re-wrapping first re-wraps to a policy that is
about to change.

The governed set is the union of the files the declarations imply and the ones
named in flake.safix.extraGovernedFiles. A file left out of it is a file a change
of audience reaches for every other file and not for that one.

It does not commit: re-wrapping every governed file is a diff worth reading
first. It does not revoke either. A person removed from an audience has already
read every value in the file, and re-wrapping the data key does not unread it;
revoking means a new value, which is `generate --regenerate` or `sops <file>`.
";

/// `safix get -h`.
pub const GET: &str = "\
safix get [<user>] <name>

Decrypt that one key to stdout. The output is plaintext by design and is meant
for piping. It needs an identity that opens the file, which is the owner's or a
recovery identity theirs names.
";

/// `safix list -h`.
pub const LIST: &str = "\
safix list [<user>]

Every name <user> holds, where it came from, whether it has a generator, the key
it is read under, and the file serving it.

The GENERATOR column shows a generator's own description when it has one, `yes`
when it has a generator and no description, and `-` when the value can only be
typed or transcribed.
";

/// `safix check -h`.
pub const CHECK: &str = "\
safix check [<user>]

Report drift and change nothing. Exits non-zero when there is any, and each
finding prints the command that resolves it. Four classes:

  - the committed .sops.yaml against the policy the declarations imply
  - each governed file's recipients against the audience declared for it
  - declared names with no value, saying which have a generator
  - values in a governed file that no declaration claims

`fix` handles the first two. The last two are decisions rather than
convergences — a value is minted or typed, an unclaimed one is declared or
deleted — so nothing here does them for you.

It needs no identity for any file it examines: every question above is answered
from the document's structure, and nothing on this path decrypts.
";

/// `safix generate -h`.
pub const GENERATE: &str = "\
safix generate [--regenerate] [--yes] [<user>] [<name>]

Run <user>'s generators, in the dependency order the declarations compute, for
every declared secret with no value yet. --regenerate re-runs over values that
already exist, which is the rotation affordance.

With no <name>, every generator that has something to mint runs. Naming a secret
runs the one generator that writes it; naming either half of a multi-output
generator runs the generator that mints both, and both land in one commit.

A single argument that names a declared user selects that user and runs all of
their generators; anything else is read as a secret's name.

\u{2500}\u{2500} --regenerate cascades \u{2500}\u{2500}
Rotating a named generator also re-runs every generator that reads what it
writes, transitively, in the same dependency order. Otherwise a rotation would
leave values derived from the value it replaced, and nothing afterwards can tell
a hash of a retired password from a hash of the current one \u{2014} the tree records
no run that a value came from.

The set is listed before anything runs and confirmed, because each re-run
commits as it goes and declining afterwards takes nothing back out of history.
--yes answers that confirmation in advance. A generator nothing reads is not a
cascade and asks nothing.

\u{2500}\u{2500} what a generator script sees \u{2500}\u{2500}
Each prompt and each dependency is `$in_<name>`, holding the path of a
read-only file descriptor carrying that value. A hyphen in the name becomes an
underscore. Nothing reaches argv, the environment or a file, and a descriptor is
read once \u{2014} read it into a variable if the script needs it twice.

That describes how the value arrives, not a sandbox it stays inside. The script
runs with the caller's filesystem and network: one that redirects `$in_<name>`
into a file, or echoes it to standard error, has put plaintext somewhere this
command does not know about and cannot shred. What the script does with a value
is the script author's to get right.

`runtimeInputs` is prepended to PATH. Name every tool the script runs, or it
works for whoever wrote it and fails for everyone else.

One output: the script's standard output is the value, and one trailing newline
comes off a single-line one. Several outputs: the script prints a JSON object
keyed by output name, and nothing is stripped from a value read out of it.

Standard error reaches you, so diagnostics go there and never into the value.
";

/// `safix keygen -h`.
pub const KEYGEN: &str = "\
safix keygen [--for-someone-else] [<user>]

Mint an age identity and append it to ${XDG_CONFIG_HOME:-$HOME/.config}/sops/age/keys.txt,
then print the public half and what to do with it. The private half is never
printed.

It appends and never truncates: sops tries every identity in that file, so a
second identity beside a first is a working state, and overwriting is how
someone loses the key to everything they hold.

Run it on your own machine, as yourself. Minting another person's identity here
means you hold their private key, which is the opposite of the independent
custody this package rests on, so it takes an explicit --for-someone-else.

An existing ssh key works instead: `ssh-to-age < ~/.ssh/id_ed25519.pub` prints
the age recipient for it, and sops.age.sshKeyPaths names the private half.
";

/// `safix adduser -h`.
pub const ADDUSER: &str = "\
safix adduser <name> <age-recipient> [--host <hostname>]... [--yes]

Declare a person who holds nothing yet: write safix/users/<name>.nix,
regenerate .sops.yaml from the policy that declaration implies, commit the two,
and then hand the name and the recipient to flake.safix.onboardingHook.

  --host H    passed through to the hook, repeatable. Refused when no hook is
              configured, because attaching an account on a host is a property
              of a consumer's module tree and safix has none.
  --yes       skip the confirmation.

<age-recipient> is theirs, minted by them, and only its SHAPE is checked here \u{2014}
whether anyone holds the private half is not knowable from this machine. A
recipient that needs a physical interaction to decrypt is refused for this field:
activation decrypts non-interactively and a card needs a touch, so it belongs in
that person's recoveryRecipients instead, where it is additive.

\u{2500}\u{2500} what this does not do \u{2500}\u{2500}
Mint anything. No age key (that is `keygen`, run by them on their machine), no
password material, and no secret value.

Give them anything to hold. The scaffold declares no secret, so no audience is
computed for them and the regenerated .sops.yaml carries their key as an anchor
with no creation rule yet. Their first secret is a name under `private` or
`carries`, then `safix fix` to write the rule, then `safix set`.

Anything about hosts, accounts, identifiers or groups. Those are one consumer's
module tree, reached through the hook and nowhere else. A hook receives:

    $1  the new person's name
    $2  their recipient
    $3\u{2026} every --host given, in order

and runs after the scaffold and the policy are committed, so whatever it writes
is its own to stage. Its absence is a supported configuration: onboarding
without a hook succeeds, having done less.
";

/// What a bare invocation says, and what `safix -h` prints.
///
/// The shell runtime's own general usage, word for word: this binary implements
/// every subcommand it lists.
pub const SCAFFOLD: &str = "\
safix \u{2014} the whole lifecycle of one secret, by name and never by file.

  safix set      [<user>] <name>                    write a value you type
  safix get      [<user>] <name>                    decrypt one key to stdout
  safix list     [<user>]                           every name a user holds
  safix generate [--regenerate] [--yes] [<user>] [<name>]
                                                    mint values from generators
  safix check    [<user>]                           report drift, change nothing
  safix fix      [--yes]                            converge policy and ciphertext
  safix keygen   [--for-someone-else] [<user>]      an age identity for a person
  safix adduser  <name> <age-recipient> [...]       declare a person who holds none

<user> defaults to $USER when flake.safix.users declares them, and otherwise to
the sole declared holder when there is exactly one.

The file, the key inside it and the recipients all come from
flake.safix.lib.placements. A name no declaration covers is refused rather than
given a destination.

`safix <subcommand> -h` explains one of them.

\u{2500}\u{2500} narrowing an audience is not revocation \u{2500}\u{2500}
Removing someone from an audience stops future encryptions reaching them. It
takes nothing back: they have already read every value in every file they could
open, and only minting a new value revokes it. `safix fix` re-wraps each
governed file's data key to the audience now declared, which aligns ciphertext
with policy and is explicitly not revocation.

\u{2500}\u{2500} verbs that do not exist here, and why \u{2500}\u{2500}
  upload   a tool that pushes generated values to a machine over ssh exists
           because the machine does not evaluate the flake holding them. A
           profile served from this repository does: activation is what delivers
           a value, through sops-nix reading the committed file. There is
           nothing for an upload to do that a rebuild does not already do.

  export   writing every value out as a plaintext tree serves migrating between
  import   backends. Both directions exist here as `get` and `set`, one name at
           a time and never as a tree \u{2014} there is one backend and the migration
           those two serve is the one this does not have. A plaintext tree is
           also a thing that outlives the migration that made it, on a disk,
           which is the shape this command exists to avoid.
";
