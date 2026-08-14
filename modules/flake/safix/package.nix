# Builds ./safix.sh and exposes it as a package and as a devshell command.
#
# A package rather than a script declared inline in the devshell: the runtime
# dependencies are pinned into the closure instead of inherited from whatever
# PATH the caller happens to have, which is how a backend goes missing from
# under a tool. The build also runs shellcheck over the script, so a check that
# builds this package is also the one that lints it.
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

      safix = pkgs.writeShellApplication {
        name = "safix";
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
        meta.description = "The whole lifecycle of one secret, by name and never by file (set | get | list | generate | check | fix | keygen | adduser)";
      };
    in
    {
      # A package and not a devshell entry, because this module is the one a
      # consumer imports and their devshell is theirs. It is exposed here so
      # that the command they run is built from the same declarations it reads.
      packages.safix = safix;
    };
}
