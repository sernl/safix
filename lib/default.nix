# The single definition of `mkVault`. `modules/flake/lib.nix` republishes this
# exact value at `flake.lib.mkVault` for a flake-parts consumer who already
# has `inputs.safix`; a flakeless consumer imports this file directly and
# supplies `lib` itself, since nothing here is a flake output. See design
# decision D1 in `openspec/changes/support-plain-nix-consumers/design.md` for
# why `mkVault` returns a resolver projection rather than a namespace, and D4
# for the boundary between what it returns and what an entry file declares
# beside it.
{ lib }:
{
  mkVault =
    { modules, root }:
    (lib.evalModules {
      modules = [
        ../modules/flake/safix
        { _module.args.self = root; }
      ]
      ++ modules;
    }).config.flake.safix.lib;
}
