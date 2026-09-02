# A working `--entry` target: `safix --entry examples/plain-nix/entry.nix list`,
# or `SAFIX_ENTRY=examples/plain-nix/entry.nix safix list`. No flake-parts and no
# flake anywhere in this file's own evaluation — `config.flake`, `inputs.safix`
# and flake-parts appear nowhere below.
#
# `lib.mkVault` is a flake output, so a file with no flake of its own reaches it
# through `builtins.getFlake` rather than through `inputs.safix`. This
# repository is `mkVault`'s own source, hence the self-reference to `../..`; a
# consumer elsewhere names their own copy instead, for example
# `builtins.getFlake "github:you/safix"`.
let
  hooks = import ./hooks.nix;
  safix = builtins.getFlake (builtins.toString ../..);
in
{
  safix = hooks // {
    lib = safix.lib.mkVault {
      modules = [ ./fleet.nix ];
      root = ./.;
    };
  };
}
