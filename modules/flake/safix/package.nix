# Builds ./safix.sh and exposes it as `packages.safix-sh`.
#
# This is no longer what an operator runs — `packages.safix` is the rust binary —
# and it is kept because it is the oracle every `safix-differential-*` mode
# compares that binary against. Retiring it would retire the evidence that the
# two agree, so it stays in the tree, built and linted, and is what those checks
# drive as `SAFIX_SH`.
#
# A package rather than a script declared inline: the runtime dependencies are
# pinned into the closure instead of inherited from whatever PATH the caller
# happens to have, which is how a backend goes missing from under a tool. The
# build also runs shellcheck over the script, so a check that builds this package
# is also the one that lints it.
#
# The content half is deliberately unable to alter the policy: `set` and
# `generate` write through sops, which reads recipients from the file's own
# metadata or from the creation rules, so no run of either can grant anyone a
# key. `fix` is the only path that moves the policy, and it regenerates before
# it re-wraps.
{ ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      readers = import ./readers.nix { inherit pkgs; };

      safix-sh = pkgs.writeShellApplication {
        # Installed under its own name rather than as `safix`, so that having
        # both in one profile is not a collision over one path.
        name = "safix-sh";
        runtimeInputs = [
          # `keygen` mints an identity with it; nothing else here runs age.
          pkgs.age
          pkgs.coreutils
          # `check` compares the committed .sops.yaml with the generated one.
          pkgs.diffutils
          pkgs.git
          pkgs.gnugrep
          pkgs.gnused
          pkgs.jq
          # The write half. Pinned here rather than taken from the devshell, for
          # the reason in the header.
          pkgs.sops
          readers.sops-recipients-of
          readers.sops-keys-of
          # `column`, for the `list` table.
          pkgs.util-linux
        ];
        text = builtins.readFile ./safix.sh;
        meta.description = "The shell runtime the rust one is compared against; not what ships";
      };
    in
    {
      packages.safix-sh = safix-sh;
    };
}
