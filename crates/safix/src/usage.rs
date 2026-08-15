//! What `-h` prints, for the subcommands this binary implements.
//!
//! One text per subcommand, word for word from the retired shell runtime,
//! because the help was part of what the differential harness compared while
//! both existed and is a contract an operator's habits are built on. It goes to
//! standard error and the process exits zero, which was that runtime's shape
//! too.
//!
//! [`SCAFFOLD`] is that runtime's general usage, unabridged, because this binary
//! implements every subcommand it lists.

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

/// `safix import -h`.
pub const IMPORT: &str = "\
safix import [<mapping>]

Move every clan-to-safix mapping declared in flake.safix.bridge.mappings, or the
one named, from clan into safix. With no mapping named it acts on all of them.

The value is read by running clan's own command and arrives on a pipe. safix
reads, writes and parses none of clan's stored files, in either direction, so
the bridge works whatever backend that clan uses.

Both sides are read and compared before either is written. A mapping whose two
sides already agree is not written and not committed, so a second run
immediately after a first writes nothing. Each mapping that is written goes
through the same path `set` uses — the same recipient-drift refusal, the same
staged write and rename — and lands as its own commit naming the mapping.

Each mapping is reported as unchanged, updated, absent at source, or refused.
Absent at source is not a failure: a clan var that has not been generated yet is
the ordinary state during bootstrap. A refused mapping does not stop the others,
and the run exits non-zero.
";

/// `safix export -h`.
pub const EXPORT: &str = "\
safix export [<mapping>]

Move every safix-to-clan mapping declared in flake.safix.bridge.mappings, or the
one named, from safix into clan. With no mapping named it acts on all of them.

The value is written by running clan's own command with the value on its
standard input. Nothing here writes into clan's store directly, and clan commits
what it wrote, in its own repository.

Both sides are read and compared before either is written, and here the
comparison is load-bearing rather than an optimisation: clan's write is
unconditional and a re-encrypting backend produces fresh ciphertext for an
unchanged value, so without it every run would commit in the clan repository for
every mapping.

Two refusals are this direction's own. A source entry that holds no value is
refused rather than exported as nothing. A mapping whose clan-side generator
clan already considers outdated is refused, because clan's next routine
generation would replace whatever was exported without saying so; there is no
option that exports anyway.
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
safix generate [--regenerate] [--yes] [--allow-disk-staging] [<user>] [<name>]

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
A staging directory, with the script's working directory at its root:

  $out/<name>              write each declared output here
  $prompts/<name>          one answered prompt per file, when any are declared
  $in/<generator>/<name>   a dependency's plaintext, keyed by its producer

This is the interface clan's own generators are written against, so a script
written for either system runs under the other. Only the dependencies a
generator declares appear under $in \u{2014} clan places every file of the dependency
generator, which would hand a script depending on a keypair's public half the
private half as well.

Every declared output must exist when the script exits. A missing one refuses
the whole run and lists what $out did contain, and nothing is written until all
of them are present. Bytes are stored exactly as written: `echo` leaves a
trailing newline and `printf` does not, and nothing here removes one.

`runtimeInputs` is prepended to PATH. Name every tool the script runs, or it
works for whoever wrote it and fails for everyone else.

An output declared `files.<name>.secret = false` is written to the repository in
the clear under public/, is given no creation rule, and is readable at
evaluation. That is what a public key or a fingerprint is for.

\u{2500}\u{2500} where the plaintext is \u{2500}\u{2500}
The staging directory is mode 0700 on a filesystem this command asks the kernel
about rather than infers from its name, and it is overwritten and removed
however the run ends \u{2014} on return, on error, on panic, and on interrupt or
terminate. There is no fallback to /tmp: on a host whose /tmp is disk-backed a
silent fallback would put plaintext in free blocks under a code path that looks
like it succeeded. Where no memory-backed filesystem is available the run
refuses, and --allow-disk-staging accepts a disk-backed one. SAFIX_STAGING_DIR
names the mount to use instead of the conventional ones \u{2014} it replaces them
rather than being tried first, so a mount you named and this rejects is a
refusal rather than a silent fall back to somewhere else \u{2014} and it is
verified like any one of them.

What that bounds, and what it does not. Overwriting a page of a memory-backed
filesystem does not reach a copy already written to swap. A mode-0700 directory
is readable by every process running as you for the length of the run, where the
pipe this replaced was readable by neither a third process nor a shell.

And it is not a sandbox. The script runs with the caller's filesystem and
network: one that copies $in/dep/name elsewhere, or writes an output outside
$out, has put plaintext somewhere this command does not look. What the script
does with a value is the script author's to get right.

Standard error and standard output both reach you, so diagnostics are free and
neither is a value.
";

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
pub const EDIT: &str = "\
safix edit [--allow-disk-staging] [<user>] <name>

Open $VISUAL, or $EDITOR when that is unset, on <name>'s value. Neither set is a
refusal naming both: this command opens no editor of its own choosing, because
dropping you into one you did not pick with a secret in the buffer produces
either an accidental write or an accidental abandonment, and nothing here can
tell those apart.

The command is split on whitespace and run directly rather than through a shell,
so EDITOR=\"code --wait\" works. The staged file's path is an argument; the value
is not.

An entry that holds no value yet opens on an empty buffer, so this is an
authoring verb as well as an amending one.

\u{2500}\u{2500} what each outcome writes \u{2500}\u{2500}
  editor exits non-zero   nothing written, nothing committed
  buffer unchanged        nothing written, nothing committed
  buffer emptied          refused \u{2014} an empty value is what a truncated write
                          leaves behind
  buffer changed          written through the same path `safix set` writes
                          through, and committed

\u{2500}\u{2500} where the buffer is \u{2500}\u{2500}
Inside the same private staging directory `safix generate` uses: mode 0700 on a
filesystem verified to be memory-backed, removed however the run ends.
--allow-disk-staging accepts a disk-backed one where none is available.

What the editor leaves beside the buffer \u{2014} swap files, backups, undo history
\u{2014} goes with it, because what is removed is the directory and not the one file
this command made. An editor configured to write undo history or backups to a
directory of its own has put plaintext where this command does not look. That is
the limit of the containment, and it is stated here rather than left to be
discovered.

A public output is not editable here: it is already plaintext in the repository,
and the generator declaring it is what mints it.
";

pub const SCAFFOLD: &str = "\
safix \u{2014} the whole lifecycle of one secret, by name and never by file.

  safix set      [<user>] <name>                    write a value you type
  safix edit     [<user>] <name>                    author a value in your editor
  safix get      [<user>] <name>                    decrypt one key to stdout
  safix list     [<user>]                           every name a user holds
  safix generate [--regenerate] [--yes] [<user>] [<name>]
                                                    mint values from generators
  safix check    [<user>]                           report drift, change nothing
  safix fix      [--yes]                            converge policy and ciphertext
  safix import   [<mapping>]                        clan-to-safix declared mappings
  safix export   [<mapping>]                        safix-to-clan declared mappings
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

\u{2500}\u{2500} what import and export are, and are not \u{2500}\u{2500}
These two move values across the clan boundary, one declared mapping at a time.
They are not a plaintext dump and restore. Writing every value out as a tree
serves migrating between backends, and that tree outlives the migration that
made it, on a disk \u{2014} which is the shape this command exists to avoid. There
is nothing here that writes one.

\u{2500}\u{2500} one verb that does not exist here, and why \u{2500}\u{2500}
  upload   a tool that pushes generated values to a machine over ssh exists
           because the machine does not evaluate the flake holding them. A
           profile served from this repository does: activation is what delivers
           a value, through sops-nix reading the committed file. There is
           nothing for an upload to do that a rebuild does not already do.
";
