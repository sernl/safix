# Holds ../../../examples/: two complete, self-contained consumers of the
# identical fleet, `plain-nix` through `lib.mkVault` with no flake-parts and no
# flake, `dendritic` through `flakeModules.default` with one declaration per
# file. An example nobody evaluates is documentation that rots, so this check
# reaches every file under both.
#
# ── how each half is read ──
# `examples/plain-nix/entry.nix` is executed for real, as written: a sandboxed
# derivation reproduces `examples/plain-nix/`, the top-level `lib/` directory
# `entry.nix` imports, and the `modules/flake/safix` resolver module
# `lib/default.nix` imports in turn, at their real relative paths, then runs
# `nix eval --file examples/plain-nix/entry.nix <attr> --json` for each of the
# twelve `safix.lib.*`/`safix.*` attribute spellings, with
# `NIX_PATH=nixpkgs=${pkgs.path}` supplying the `<nixpkgs>` `entry.nix` itself
# resolves `lib` from — the same source `NIX_PATH` supplies at a real
# `--entry` invocation. `entry.nix` no longer self-references this
# repository's own flake through `builtins.getFlake`, so the obstacle that
# once stood between this check and the file as written — resolving that
# self-reference means re-evaluating this flake's own transitive input
# closure a second time, which a network-less build sandbox can do for
# neither the network fetch nor the private store registration that would
# take — no longer exists, now that `mkVault` is a plain function of `{ lib
# }` rather than a flake output.
#
# `examples/dendritic`'s twenty-two scattered files are read through `lib.evalModules`
# directly: the same mechanism flake-parts' own `imports` is sugar over (design
# decision D3), not `examples/dendritic/flake.nix` evaluated as a flake, which
# would need a self-reference of its own.
#
# ── severity: one drill ──
# Changing one field in the dendritic fleet without changing the plain-nix one
# fails `safix-examples` on exactly that field, which is the evidence the two are
# compared rather than merely both evaluated.
{ lib, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      examplesRoot = ../../../examples;
      plainNixRoot = examplesRoot + "/plain-nix";
      dendriticRoot = examplesRoot + "/dendritic";
      libRoot = ../../../lib;
      safixModule = ../safix;

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

      # The two remaining attribute spellings — `safix.onboardingHook` and
      # `safix.enrollHook` — are declared in ./plain-nix/hooks.nix rather
      # than under `.lib`.
      plainNixHooks = import (plainNixRoot + "/hooks.nix");

      # `safix.lib.<name>` for the ten data fields, `safix.<name>` for the
      # two hooks — the twelve spellings `crates/safix-core/src/nix.rs`'s
      # `Attribute` enum names, and the same twelve `--entry` and a flake
      # target both resolve identically (README, "Without flake-parts, or
      # without a flake").
      entryAttrs = (map (name: "safix.lib." + name) (builtins.attrNames (tenFields dendriticVault))) ++ [
        "safix.onboardingHook"
        "safix.enrollHook"
      ];

      entryExpected = tenFields dendriticVault // {
        onboardingHook = plainNixHooks.onboardingHook;
        enrollHook = plainNixHooks.enrollHook;
      };
      entryExpectedFile = pkgs.writeText "safix-examples-entry-expected.json" (
        builtins.toJSON entryExpected
      );
    in
    {
      checks.safix-examples =
        pkgs.runCommand "safix-examples"
          {
            nativeBuildInputs = [
              pkgs.nix
              pkgs.jq
            ];
            env.NIX_CONFIG = "experimental-features = nix-command flakes";
            meta.description = "flakeless-entry check: safix-examples";
          }
          ''
            set -eu
            export HOME="$TMPDIR"
            export NIX_PATH="nixpkgs=${pkgs.path}"

            repo="$PWD/repo"
            mkdir -p "$repo/examples" "$repo/modules/flake"
            cp -r ${plainNixRoot} "$repo/examples/plain-nix"
            cp -r ${libRoot} "$repo/lib"
            cp -r ${safixModule} "$repo/modules/flake/safix"
            cd "$repo"

            actual='{}'
            for attr in ${lib.concatStringsSep " " entryAttrs}; do
              key="''${attr##*.}"
              value="$(nix eval --file examples/plain-nix/entry.nix "$attr" --json)"
              actual="$(jq --argjson v "$value" ". + {\"$key\": \$v}" <<<"$actual")"
            done

            if ! diff -u <(jq -S . ${entryExpectedFile}) <(jq -S . <<<"$actual"); then
              echo ""
              echo "safix-examples: entry.nix, evaluated as written, diverges from the dendritic fixture"
              exit 1
            fi
            touch $out
          '';
    };
}
