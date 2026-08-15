//! What `-h` prints, for the subcommands this binary implements.
//!
//! One text per ported subcommand, word for word from
//! `modules/flake/safix/safix.sh`, because the help is part of what the
//! differential harness compares and the shell runtime is the oracle. It goes
//! to standard error and the process exits zero, which is also the shell
//! runtime's shape.
//!
//! There is deliberately no general usage here. The shell's `usage` lists eight
//! subcommands, and printing that list from a binary implementing three would
//! advertise five it refuses. A bare invocation of this binary says which three
//! it has instead.

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

/// What a bare invocation of this binary says.
///
/// Not the shell runtime's usage. This binary is `safix-rs`, it is not what
/// ships, and it implements the read paths only; saying so is more useful than
/// reproducing a menu of subcommands it would refuse.
pub const SCAFFOLD: &str = "\
safix-rs — the rust runtime, mid-port. Not what ships.

  safix set   [<user>] <name>   write a value you type
  safix get   [<user>] <name>   decrypt one key to stdout
  safix list  [<user>]          every name a user holds
  safix check [<user>]          report drift, change nothing
  safix fix   [--yes]           converge policy and ciphertext

Every other subcommand is still the shell runtime's: run the flake's `safix`
package for those. A subcommand appears here only once the differential harness
has compared it against the shell on standard output, standard error, exit code
and effect on the repository.
";
