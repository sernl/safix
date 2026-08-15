# The gate that will permit retiring the shell runtime, run as a check.
#
# Every other check in this tree judges one runtime against a claim. These judge
# two runtimes against each other, which is a different kind of evidence and the
# only kind that can end the port: a claim can be written to match whatever the
# code does, and a comparison against the runtime that ships cannot.
#
# The read-path modes are fixtures — a repository in step, a declared name with
# no value, recipients drifted from their audience, a value no declaration
# claims, names nobody declared, and a path no creation rule covers — and each
# runs the same list of `list`, `get` and `check` invocations against both
# runtimes. The write-path modes drive `set` and `fix` over the same fleet.
# `safix-differential-drills` is the one that keeps the rest honest: it mutates
# the rust side on purpose, once per channel, and fails unless each mutation is
# caught by the channel that exists to catch it.
#
# Two of them are not comparisons. `abort` interrupts a write in each window it
# has and holds the run to what it must leave behind, which is nothing; `pipes`
# observes the sops process itself and holds the value to travelling down a pipe
# and no other way. Both also assert the shell runtime's own behaviour where it
# differs, so a divergence stays recorded rather than becoming folklore.
#
# The generator graph, `keygen` and `adduser` are compared here too, which is
# what completed the gate: every subcommand the shell runtime has is judged
# against it, and `packages.safix` is the rust binary as a result. The shell
# runtime stays in the tree as `packages.safix-sh`, the oracle these modes
# compare against.
{ ... }:
{
  perSystem =
    { config, pkgs, ... }:
    let
      readers = import ../safix/readers.nix { inherit pkgs; };

      # Real sops, age and git, and the real python readers the shell runtime
      # calls: standing a stub in for any of them is what would let a comparison
      # stay green over a runtime calling something the tree no longer contains.
      # `column` is util-linux's, because the shell's `list` pipes through it and
      # the rust table is judged against what it produces.
      harness = [
        pkgs.age
        pkgs.bash
        pkgs.coreutils
        pkgs.diffutils
        pkgs.findutils
        pkgs.git
        pkgs.gnugrep
        pkgs.gnused
        pkgs.jq
        # For `nix-instantiate --parse` alone, which is what holds the nix
        # `adduser` generates to being nix. Parsing needs no store and no
        # daemon, so it is available where an evaluation is not; the `nix` both
        # runtimes reach for stays the stub, because it is named through
        # SAFIX_NIX rather than found on PATH.
        pkgs.nix
        pkgs.sops
        readers.sops-recipients-of
        readers.sops-keys-of
        pkgs.util-linux
      ];

      differential =
        name: mode:
        pkgs.runCommand name { nativeBuildInputs = harness; } ''
          export HOME="$PWD"
          SAFIX_SH=${../safix/safix.sh} \
          SAFIX_RS=${config.packages.safix-rs}/bin/safix \
            bash ${../safix/safix-differential.sh} ${mode}
          touch "$out"
        '';
    in
    {
      # Every value set, the policy in step, nothing anywhere it should not be.
      # Two runtimes reporting no drift agree as much as two reporting a page of
      # it, and this is the fixture the others are perturbations of.
      checks.safix-differential-clean = differential "safix-differential-clean" "clean";

      # Declared names with no value, one with a generator and one without,
      # which is the distinction the report exists to draw and the reason a
      # valueless name is not simply an error.
      checks.safix-differential-missing = differential "safix-differential-missing" "missing";

      # A file whose stanzas disagree with the audience declared for it, drifted
      # in both directions at once, so that the two halves of the finding — who
      # can open it and should not, who should and cannot — are both compared.
      checks.safix-differential-drift = differential "safix-differential-drift" "drift";

      # A value in a governed file that no declaration claims, which is the
      # direction of the question whose remedy is to declare it or delete it.
      checks.safix-differential-orphan = differential "safix-differential-orphan" "orphan";

      # Names nobody declared, argument lists no subcommand takes, and the help
      # each ported subcommand prints. This is the refusal surface, and it is
      # where the plain reporter is doing the work.
      checks.safix-differential-unknown = differential "safix-differential-unknown" "unknown";

      # A governed file no rule's directory covers and a placement outside the
      # suffix every rule ends in. Neither is repairable by `fix`, and both
      # runtimes have to say so in the same words.
      checks.safix-differential-norule = differential "safix-differential-norule" "norule";

      # The write path that lands. A value replaced in a file that exists, a
      # value re-entered unchanged and committing nothing, a file created
      # through the creation rules, and a staged change to a path `set` does not
      # name surviving in the index rather than being swept into the commit.
      checks.safix-differential-write = differential "safix-differential-write" "write";

      # What `set` refuses about how it was asked and about what was typed: an
      # empty value, two entries that differ, and a stream that ended before
      # either arrived.
      checks.safix-differential-refuse = differential "safix-differential-refuse" "refuse";

      # The states a write is refused in because of what the repository or the
      # declarations are. Recipients drifted from the declared audience is the
      # one this exists for: `sops set` takes an existing file's recipients from
      # that file, so a value minted into a drifted file would be wrapped for
      # the audience that used to be, and committed.
      checks.safix-differential-guard = differential "safix-differential-guard" "guard";

      # `fix` over a drifted fixture, compared as an invocation and then
      # asserted as a convergence: run once, `check` has nothing left to report.
      # Both bounds of the re-wrap fan-out are exercised, because the bound is
      # what decides whether sops holds the operator's own streams or a pipe.
      checks.safix-differential-converge = differential "safix-differential-converge" "converge";

      # An interrupted write, in each of the three windows it has. Not a
      # comparison: the shell runtime does not act on SIGINT in any of them, and
      # the two assertions that record why are what keep this from silently
      # becoming one.
      checks.safix-differential-abort = differential "safix-differential-abort" "abort";

      # The value's path into sops, read off the sops process itself. This is
      # the claim `safix.sh` carries in a comment — never argv, never the
      # environment — made checkable for both runtimes.
      checks.safix-differential-pipes = differential "safix-differential-pipes" "pipes";

      # Every generator with something to mint, run in the plan's order: the
      # bulk form, the named form, the skip a second run makes, and a
      # multi-output generator whose two halves land in one commit.
      checks.safix-differential-generate = differential "safix-differential-generate" "generate";

      # The rotation, and the cascade a rotation carries. A generator nothing
      # reads asks nothing; one whose output another reads announces the whole
      # downstream set before the first commit, and the answer is drilled both
      # ways as well as given in advance.
      checks.safix-differential-regenerate =
        differential "safix-differential-regenerate" "regenerate";

      # What `generate` refuses about a declaration and about what a script
      # printed: no generator, an empty value, a script that exited non-zero, a
      # validation that rejected the candidate, a multi-output script that
      # printed something other than a JSON object, a prompt nobody answered,
      # and a dependency with no value yet.
      checks.safix-differential-genrefuse =
        differential "safix-differential-genrefuse" "genrefuse";

      # Minting an identity. Not a byte comparison of the value — two correct
      # runs mint two different identities — so each side is held to the
      # property and only the rendering is compared, with the recipient
      # normalized away.
      checks.safix-differential-keygen = differential "safix-differential-keygen" "keygen";

      # Declaring a person who holds nothing yet: the scaffold, the policy
      # regenerated from a tree that includes it, the two committed together,
      # the hook, and every refusal about the name and the recipient.
      checks.safix-differential-adduser = differential "safix-differential-adduser" "adduser";

      # The harness shown to fail. One mutation per channel — a line added to
      # standard output, a line added to standard error, an exit code changed, a
      # file left in the repository, a value left in the temporary directory —
      # each of which must be caught, and caught by its own channel rather than
      # incidentally by another.
      checks.safix-differential-drills = differential "safix-differential-drills" "drills";
    };
}
