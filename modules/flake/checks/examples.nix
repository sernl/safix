# Holds ../../../examples/: two complete, self-contained consumers of the
# identical fleet, `plain-nix` through `lib.mkVault` with no flake-parts and no
# flake, `dendritic` through `flakeModules.default` with one declaration per
# file. An example nobody evaluates is documentation that rots, so this check
# reaches every file under both.
#
# ── how each half is read ──
# `examples/plain-nix/fleet.nix` is read through the real `config.flake.lib.mkVault`
# — the same function `examples/plain-nix/entry.nix` reaches through
# `builtins.getFlake (toString ../..)`, a self-reference to this repository's own
# flake. That indirection is correct and works for a real, unsandboxed
# `nix eval --file`, verified by hand against this tree; it cannot be driven from
# inside this check, because resolving it means re-evaluating this flake's own
# transitive inputs (flake-parts, nixpkgs, sops-nix, …) a second time, and a build
# sandbox has neither the network nor the private store registration that would
# take. So this check reaches the identical `fleet.nix` and `hooks.nix` — the same
# two files `entry.nix` itself imports — through the direct route to the same
# `mkVault` value, rather than by executing the self-reference.
#
# `examples/dendritic`'s twenty-two scattered files are read through `lib.evalModules`
# directly: the same mechanism flake-parts' own `imports` is sugar over (design
# decision D3), not `examples/dendritic/flake.nix` evaluated as a flake, which
# would need the identical nested fetch `entry.nix`'s self-reference does.
#
# ── severity: one drill ──
# Changing one field in the dendritic fleet without changing the plain-nix one
# fails `safix-examples` on exactly that field, which is the evidence the two are
# compared rather than merely both evaluated.
{
  config,
  lib,
  ...
}:
{
  perSystem =
    { pkgs, ... }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      examplesRoot = ../../../examples;
      plainNixRoot = examplesRoot + "/plain-nix";
      dendriticRoot = examplesRoot + "/dendritic";

      # The ten `flake.safix.lib.*` fields `crates/safix-core/src/nix.rs:52-67`
      # reads, which is what "resolves the same fleet field for field" is a claim
      # about. `mkChecks`, `resolveSet`, `resolveNames`, `materialize`,
      # `publicValue`, `outputPath` and `policyPlan` are functions or outside the
      # runtime's own contract, and none of them is JSON-serializable.
      tenFields = proj: {
        inherit (proj)
          placements
          audiences
          governedFiles
          recipients
          delegation
          policyText
          generatorPlan
          nameRegex
          bridge
          keepassxc
          ;
      };

      plainNixVault = config.flake.lib.mkVault {
        modules = [ (plainNixRoot + "/fleet.nix") ];
        root = plainNixRoot;
      };

      dendriticModules = builtins.filter (p: lib.hasSuffix ".nix" (toString p)) (
        lib.filesystem.listFilesRecursive (dendriticRoot + "/modules")
      );

      dendriticVault =
        (lib.evalModules {
          modules = [
            ../safix
            { _module.args.self = dendriticRoot; }
          ]
          ++ dendriticModules;
        }).config.flake.safix.lib;
    in
    {
      checks.safix-examples = mkStructuralCheck {
        name = "safix-examples";
        actual = {
          dendritic = tenFields dendriticVault;

          # The two remaining attribute spellings — `safix.onboardingHook` and
          # `safix.enrollHook` — are declared in ./plain-nix/hooks.nix rather
          # than under `.lib`; forcing them here, through the same import
          # entry.nix performs, is what proves they resolve rather than merely
          # parse.
          plainNixHooks = import (plainNixRoot + "/hooks.nix");
        };
        expected = {
          dendritic = tenFields plainNixVault;
          plainNixHooks = import (plainNixRoot + "/hooks.nix");
        };
      };
    };
}
