# The claims that were never comparisons, held to one runtime.
#
# These attributes are what survived `modules/flake/checks/differential.nix`,
# whose nineteen modes ran the shell runtime and the rust one against each other
# on standard output, standard error, exit status and effect on the repository.
# Fifteen of those modes asserted that two runtimes said the same thing about
# something, which is a claim with no successor once there is one runtime; the
# thing itself is asserted against a literal by the checks in `./cli.nix`.
#
# Four of them asserted something about the rust runtime in its own right and
# were only ever incidentally comparative, so they keep their claims and lose the
# `differential-` infix: `safix-abort-residue`, `safix-value-pipe`,
# `safix-syscall-proof` and `safix-channel-drills`. `safix-memory-backing`,
# `safix-bridge-transfer` and `safix-bridge-audit` have no differential ancestor
# and are here because they are the same shape of claim rather than because
# anything was retired into them.
#
# Each is one test target of the integration suite, because each is a single
# claim made of several windows or several channels and splitting it across
# attributes would leave an attribute asserting a fragment.
#
# `safix-channel-drills` is the one that keeps every other check on this flake
# honest, and is the last thing that should ever be deleted rather than the
# first: without it the suite is a page of assertions nobody has watched fail,
# which is the failure mode a green suite is best at hiding.
{ ... }:
{
  perSystem =
    { config, pkgs, ... }:
    let
      integration = import ./integration.nix { inherit pkgs; };

      claim = name: target: integration.runOne config.checks.safix-integration name target "";
    in
    {
      # An interrupted write, in each of the four windows it has: waiting for the
      # value, waiting for the confirmation, under a signal that is not SIGINT,
      # and waiting for sops while it holds the candidate document open. Each is
      # held to the status the signal implies, a repository identical to the one
      # the run found, no candidate document beside the target, and the value in
      # no file under the repository or the temporary directory.
      #
      # The fourth window is a fixture rather than a race: sops signals the
      # runtime from inside it and then finishes normally, which is what the
      # retired mode did through a pidfile.
      checks.safix-abort-residue = claim "safix-abort-residue" "abort_residue";

      # The value's path into sops, read off the sops process itself: never argv,
      # so never a process listing, and never the environment, so never
      # /proc/<pid>/environ. The run has to succeed and the value has to come
      # back out, or the silence would hold just as well over a runtime that sent
      # sops nothing.
      checks.safix-value-pipe = claim "safix-value-pipe" "value_pipe";

      # Every plaintext byte a `set` and a `generate` write, observed at the
      # system call and held to going down a pipe. `safix-value-pipe` shows the
      # two routes the value did not take; this shows the one it did, and carries
      # its own drill — a runtime writing a plaintext value to a regular file has
      # to be caught, and caught by the pipe assertion rather than incidentally
      # by the residue sweep.
      #
      # `strace` needs ptrace, which is a linux capability and has no darwin
      # equivalent: `dtruss` needs system integrity protection disabled, which a
      # build sandbox cannot do. The check exists on both platforms and the
      # suite's non-linux half says what it did not do, because a claim that
      # quietly stops being made on a platform is a claim nobody decided to stop
      # making.
      checks.safix-syscall-proof = claim "safix-syscall-proof" "syscall_proof";

      # The suite shown to fail. One mutation per channel — a line added to
      # standard output, a line added to standard error, a changed exit status, a
      # file left in the repository, a value left in the temporary directory —
      # each of which must be caught, and caught by its own channel rather than
      # incidentally by another.
      #
      # The exit status is the channel this form has to assert deliberately: two
      # runtimes that exit differently differ whether or not anybody names the
      # channel, and with one runtime there is nothing to differ from unless the
      # status is recorded and compared.
      checks.safix-channel-drills = claim "safix-channel-drills" "channel_drills";

      # The tmpfs rule, held against the kernel's own mount table rather than
      # against the probe that enforces it. The drill that exercises the refusal
      # used to select its disk-backed directory by asking that probe which
      # candidate was disk-backed, so a probe answering "memory-backed" for
      # everything made the selection find nothing and the drill report itself
      # skipped — a check that stopped asserting without failing. The two
      # readings are now compared mount by mount, and each direction has to be
      # present or a probe stuck at either answer would agree with the machine.
      checks.safix-memory-backing = claim "safix-memory-backing" "memory_backing";

      # Both bridge directions end to end against a clan that records what it
      # was handed: the value on a pipe and in no argument vector, the read
      # taking raw bytes rather than a terminal rendering, a second run writing
      # nothing, and each refusal for its own reason. The stub states why
      # stubbing clan is permitted where stubbing sops is not.
      checks.safix-bridge-transfer = claim "safix-bridge-transfer" "bridge";

      # The report over the same declarations, which is a separate claim from
      # the transfer rather than more of it: a mapping whose two sides hold
      # different values is a finding naming the mapping and neither value, one
      # whose sides agree is not, and a mapping the caller cannot decrypt is a
      # finding rather than a mapping quietly left out. That last one is what
      # keeps a clean report meaning the mappings agree instead of meaning the
      # ones this operator could open agree.
      checks.safix-bridge-audit = claim "safix-bridge-audit" "audit";
    };
}
