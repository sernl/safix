# A working `--entry` target: `safix --entry examples/plain-nix/entry.nix list`,
# or `SAFIX_ENTRY=examples/plain-nix/entry.nix safix list`. No flake-parts and no
# flake anywhere in this file's own evaluation — `config.flake`, `inputs.safix`,
# flake-parts and `builtins.getFlake` appear nowhere below.
#
# `lib.mkVault` lives at `../../lib`, a plain function of `{ lib }` rather than
# a flake output, so reaching it needs no flake reference. A flakeless entry
# file supplies its own `lib` the same way any other non-flake nix expression
# does: from `NIX_PATH`, which `nix eval --file` (what `--entry` runs) already
# requires to resolve `<nixpkgs>`.
let
  hooks = import ./hooks.nix;
  lib = (import <nixpkgs> { }).lib;
  mkVault = (import ../../lib { inherit lib; }).mkVault;
in
{
  safix = hooks // {
    lib = mkVault {
      modules = [ ./fleet.nix ];
      root = ./.;
    };
  };
}
