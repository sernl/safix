# The gate that will permit retiring the shell runtime, run as a check.
#
# Every other check in this tree judges one runtime against a claim. These judge
# two runtimes against each other, which is a different kind of evidence and the
# only kind that can end the port: a claim can be written to match whatever the
# code does, and a comparison against the runtime that ships cannot.
#
# The modes are read-path fixtures — a repository in step, a declared name with
# no value, recipients drifted from their audience, a value no declaration
# claims, names nobody declared, and a path no creation rule covers — and each
# runs the same list of `list`, `get` and `check` invocations against both
# runtimes. `safix-differential-drills` is the one that keeps the rest honest:
# it mutates the rust side on purpose, once per channel, and fails unless each
# mutation is caught by the channel that exists to catch it.
#
# Only the read paths are compared, because only the read paths are ported. The
# write paths and the generator graph reach this file as they land, and the
# shell runtime stays `packages.safix` until the last of them has.
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

      # The harness shown to fail. One mutation per channel — a line added to
      # standard output, a line added to standard error, an exit code changed, a
      # file left in the repository, a value left in the temporary directory —
      # each of which must be caught, and caught by its own channel rather than
      # incidentally by another.
      checks.safix-differential-drills = differential "safix-differential-drills" "drills";
    };
}
