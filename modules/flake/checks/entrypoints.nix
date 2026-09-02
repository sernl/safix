# Holds the module entrypoints a consumer reaches with no flake: that
# `homeManagerModules` never drifts from `homeModules` (D7), and that
# `nixosModules.safix` and `homeModules.safix` are genuinely importable with
# no flake anywhere in the importing tree, unlike their `.default` siblings.
#
# ── homeManagerModules is a plain alias ──
# `flake.nix` binds `homeManagerModules = homeModules;` in one `let`, so the
# two names can only ever disagree if a future edit points one of them at a
# second definition. `==` over the published values is what would catch
# that — a fresh copy of the module compares unequal to the shared binding
# even when its content is byte-identical, because paths compare by where
# they point rather than by what they contain, which is the property this
# check needs and a content diff would not give it.
#
# ── the bare-fixture mechanism ──
# `modules/consume/home.nix` and `modules/consume/nixos.nix` are each
# evaluated through a plain `lib.evalModules` fixture whose only other
# module turns `_module.check` off. That option gates the module system's
# whole-tree pass matching every `config` definition against a declared
# option, a pass forced merely by touching `.options` at all — and without
# it, evaluating either file bare throws long before `options.safix.lib`
# is reached: `nixos.nix`'s `installed` option reads `options.sops.secrets.type`
# off a real secrets provisioner module, and both files' `config` blocks
# write into NixOS's or home-manager's own base surface (`assertions`,
# `home.activation`, and — through `./installer.nix` — `system.build`,
# `system.activationScripts`, `systemd.services`, `services.openssh`,
# `networking.hostName`) that neither file declares and that no consumer's
# entrypoint file is expected to reconstruct by hand. Declaring
# `options.safix.lib` needs none of that: building the `options` tree only
# needs each option's own fields to stay lazy thunks, and `_module.check =
# false` is what lets `.options` come back without forcing the unrelated
# ones. No module beyond each file's own path is named in the fixture — no
# `inputs`, no sops-nix, no nixpkgs base modules — so this measures the
# zero-flake claim directly rather than standing in for it.
#
# ── the import-list asymmetry ──
# `lib.evalModules`'s own `.graph` records every module it collected through
# `imports`, each carrying its own file path; flattened recursively, it
# gives one predicate — does any reachable file's path fall under
# `inputs.sops-nix`'s own store path — applicable uniformly to all four
# values under test, `.safix` and `.default` on both files alike. That
# uniformity is not optional here:
# flake-parts declares `flake.nixosModules` as `lazyAttrsOf deferredModule`,
# and does not know `flake.homeModules` at all (safix imports no
# home-manager flake module), so `nixosModules.default` arrives already
# normalized by `lib.types.deferredModule`'s own `merge` — a set wrapping
# the original `{ imports = [ sops-nix... nixos.nix ]; }` one level deeper,
# `{ imports = [ <that set> ]; }` — while `homeModules.default` arrives
# exactly as `flake.nix` wrote it. A mechanism that reads `.imports`
# structurally by hand sees two different shapes and has to special-case
# one of them; `filesReachedFrom` does not care how many `imports` hops a
# file is behind, because `.graph` is already the fully recursively
# collected tree, so the same call finds `nixos.nix` pulling in
# `installer.nix` and nothing else, `home.nix` pulling in nothing, and each
# `.default` form reaching sops-nix's real module through however many
# wrapping layers flake-parts' typing added.
#
# ── anti-vacuity ──
# Two independent probes, because two different things could silently stop
# being tested. A fixture module that declares an unrelated option and
# never touches `safix.lib` is run through the same `declaresLib`
# mechanism the two real files are; it answering `false` proves the `true`
# answers for `home.nix` and `nixos.nix` come from a mechanism that can
# fail to find the option, not from a probe that always says yes. Separately,
# a marker path is run through the real `lib.types.deferredModule.merge` —
# the identical function flake-parts applies to `nixosModules` — and then
# through `filesReachedFrom`; finding it proves the flatten still sees
# through that specific normalization, independent of whether sops-nix
# itself is where it is expected. A future nixpkgs or flake-parts change to
# how `deferredModule` wraps its definitions would redden this probe rather
# than silently turning `nixosDefaultNamesSopsNix` vacuous.
{
  config,
  inputs,
  lib,
  ...
}:
{
  perSystem =
    { pkgs, ... }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      homeModules = config.flake.homeModules;
      homeManagerModules = config.flake.homeManagerModules;
      nixosModules = config.flake.nixosModules;

      bareFixtureBase = {
        config = {
          _module.check = false;
        };
      };

      bareFixtureArgs =
        { modules }:
        lib.evalModules {
          modules = [ bareFixtureBase ] ++ modules;
          # Only `home.nix`'s function signature requires `pkgs`; nothing
          # `declaresLib` or `filesReachedFrom` reads ever forces it.
          specialArgs.pkgs = { };
        };

      declaresLib =
        modulePath: ((bareFixtureArgs { modules = [ modulePath ]; }).options.safix or { }) ? lib;

      # Flattens `lib.evalModules`'s recursively collected import graph, for
      # any module value — a path, an attrset, or a `deferredModule`-wrapped
      # set — into the flat list of every reachable file's path.
      filesReachedFrom =
        moduleValue:
        let
          flatten = node: [ node.file ] ++ lib.concatMap flatten node.imports;
        in
        lib.concatMap flatten (bareFixtureArgs { modules = [ moduleValue ]; }).graph;

      namesSopsNix =
        moduleValue:
        lib.any (file: lib.hasPrefix (toString inputs.sops-nix) (toString file)) (
          filesReachedFrom moduleValue
        );

      vacuousFixture = {
        options.safix.somethingElse = lib.mkOption {
          type = lib.types.bool;
          default = true;
        };
      };

      # The same `lib.types.deferredModule.merge` flake-parts runs over
      # `flake.nixosModules`'s definitions, applied by hand to one marker
      # path so the probe is independent of `nixosModules` itself.
      deferredModuleMarker = nixosModules.safix;
      deferredModuleProbe =
        (bareFixtureArgs {
          modules = [
            {
              options.probe = lib.mkOption { type = lib.types.deferredModule; };
              config.probe = {
                imports = [ deferredModuleMarker ];
              };
            }
          ];
        }).config.probe;
    in
    {
      checks.safix-module-entrypoints = mkStructuralCheck {
        name = "safix-module-entrypoints";
        actual = {
          alias = {
            safixEqual = homeManagerModules.safix == homeModules.safix;
            defaultEqual = homeManagerModules.default == homeModules.default;
          };

          bareEvaluation = {
            home = declaresLib homeModules.safix;
            nixos = declaresLib nixosModules.safix;
          };

          importAsymmetry = {
            homeSafixNamesSopsNix = namesSopsNix homeModules.safix;
            nixosSafixNamesSopsNix = namesSopsNix nixosModules.safix;
            homeDefaultNamesSopsNix = namesSopsNix homeModules.default;
            nixosDefaultNamesSopsNix = namesSopsNix nixosModules.default;
          };

          antiVacuity = {
            declaresLibDetectsAbsence = declaresLib vacuousFixture;
            seesThroughDeferredModule = lib.any (file: lib.hasInfix "consume/nixos.nix" (toString file)) (
              filesReachedFrom deferredModuleProbe
            );
          };
        };
        expected = {
          alias = {
            safixEqual = true;
            defaultEqual = true;
          };

          bareEvaluation = {
            home = true;
            nixos = true;
          };

          importAsymmetry = {
            homeSafixNamesSopsNix = false;
            nixosSafixNamesSopsNix = false;
            homeDefaultNamesSopsNix = true;
            nixosDefaultNamesSopsNix = true;
          };

          antiVacuity = {
            declaresLibDetectsAbsence = false;
            seesThroughDeferredModule = true;
          };
        };
      };
    };
}
