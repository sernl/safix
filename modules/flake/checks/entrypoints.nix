# Holds the module entrypoints a consumer reaches with no flake: that
# `homeManagerModules` never drifts from `homeModules` (D7), that each scope's
# two published names are one value, and that all four are genuinely importable
# with no flake anywhere in the importing tree.
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
# is reached: both files' `config` blocks write into NixOS's or
# home-manager's own base surface (`assertions`, `home.activation`, and —
# through `./installer.nix` — `system.build`,
# `system.activationScripts`, `systemd.services`, `services.openssh`,
# `networking.hostName`) that neither file declares and that no consumer's
# entrypoint file is expected to reconstruct by hand. Declaring
# `options.safix.lib` needs none of that: building the `options` tree only
# needs each option's own fields to stay lazy thunks, and `_module.check =
# false` is what lets `.options` come back without forcing the unrelated
# ones. No module beyond each file's own path is named in the fixture — no
# `inputs`, no flake input of any kind, no nixpkgs base modules — so this
# measures the zero-flake claim directly rather than standing in for it.
#
# ── no published form names an input ──
# `lib.evalModules`'s own `.graph` records every module it collected through
# `imports`, each carrying its own file path; flattened recursively, it
# gives one predicate — does any reachable file's path fall under any
# declared flake input's own store path — applicable uniformly to all four
# values under test, `.safix` and `.default` on both files alike. That
# uniformity is not optional here:
# flake-parts declares `flake.nixosModules` as `lazyAttrsOf deferredModule`,
# and does not know `flake.homeModules` at all (safix imports no
# home-manager flake module), so `nixosModules.default` arrives already
# normalized by `lib.types.deferredModule`'s own `merge` — a set wrapping
# the original definition one level deeper, `{ imports = [ <that set> ]; }` —
# while `homeModules.default` arrives exactly as `flake.nix` wrote it. A
# mechanism that reads `.imports` structurally by hand sees two different
# shapes and has to special-case one of them; `filesReachedFrom` does not
# care how many `imports` hops a file is behind, because `.graph` is already
# the fully recursively collected tree, so the same call finds `nixos.nix`
# pulling in `installer.nix` and nothing else and `home.nix` pulling in
# nothing, through however many wrapping layers flake-parts' typing added.
#
# The predicate is over every declared input rather than over one, because
# the asymmetry it used to measure has no second side: the `.default` forms
# imported a secrets provisioner and the `.safix` forms did not, and now
# both names are one value that imports nothing. `self` is excluded from the
# input set deliberately — it is this repository, so every consumption
# module is inside it by construction and counting it would answer true for
# every value under test.
#
# ── anti-vacuity ──
# Three independent probes, because three different things could silently stop
# being tested. A fixture module that declares an unrelated option and
# never touches `safix.lib` is run through the same `declaresLib`
# mechanism the two real files are; it answering `false` proves the `true`
# answers for `home.nix` and `nixos.nix` come from a mechanism that can
# fail to find the option, not from a probe that always says yes. A fixture
# whose import list names a file inside a declared input is run through
# `namesAnyFlakeInput`; it answering `true` is what makes the empty list of
# offending published names evidence rather than the answer a predicate
# stuck at `false` would also give. And separately, a marker path is run
# through the real `lib.types.deferredModule.merge` — the identical function
# flake-parts applies to `nixosModules` — and then through
# `filesReachedFrom`; finding it proves the flatten still sees through that
# specific normalization. A future nixpkgs or flake-parts change to how
# `deferredModule` wraps its definitions would redden that probe rather than
# silently turning the published-name row vacuous.
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

      # The consumption files a module value reaches, which is what "one value
      # under both names" has to be measured over rather than `==`.
      # flake-parts types `flake.nixosModules` as `lazyAttrsOf
      # deferredModule`, and `lib.types.deferredModule`'s own merge wraps each
      # attribute with a per-attribute `_file` breadcrumb naming the attribute
      # path it was defined at, so two attributes bound to one `let` value in
      # `flake.nix` still compare unequal there. `flake.homeModules` is
      # untyped and its two names do compare equal, which is exactly the
      # asymmetry that makes `==` the wrong instrument: it would report a
      # difference that is flake-parts' bookkeeping rather than safix's.
      consumeFilesOf =
        moduleValue:
        lib.sort (a: b: a < b) (
          lib.unique (
            builtins.filter (file: file != null) (
              map (
                file:
                let
                  matched = builtins.match ".*(modules/consume/.*)" (toString file);
                in
                if matched == null then null else builtins.head matched
              ) (filesReachedFrom moduleValue)
            )
          )
        );

      # `self` is excluded because it is this repository: every consumption
      # module is a file inside it, so a predicate counting it would answer
      # true for every value under test and the row below would say nothing.
      declaredInputs = builtins.removeAttrs inputs [ "self" ];

      namesAnyFlakeInput =
        moduleValue:
        let
          prefixes = lib.mapAttrsToList (_name: input: toString input) declaredInputs;
        in
        lib.any (file: lib.any (prefix: lib.hasPrefix prefix (toString file)) prefixes) (
          filesReachedFrom moduleValue
        );

      # The published names, each paired with its own spelling, so the row
      # below reports which form regained a dependency rather than only that
      # one did.
      publishedForms = {
        "homeModules.safix" = homeModules.safix;
        "homeModules.default" = homeModules.default;
        "nixosModules.safix" = nixosModules.safix;
        "nixosModules.default" = nixosModules.default;
      };

      vacuousFixture = {
        options.safix.somethingElse = lib.mkOption {
          type = lib.types.bool;
          default = true;
        };
      };

      # A fixture whose import list names a file inside a declared input.
      # Nothing in it is forced: `filesReachedFrom` reads the collected graph
      # and never an option's value, so any real module file under an input
      # serves, and this one declares options and defines nothing.
      inputNamingFixture = {
        imports = [ "${inputs.nixpkgs}/nixos/modules/misc/meta.nix" ];
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

          # One symmetric row: no published form names any flake input. It
          # reports the offending spellings rather than a boolean, because a
          # form that regains a dependency is worth naming in the diff.
          publishedFormsNamingAnInput = builtins.attrNames (
            lib.filterAttrs (_name: namesAnyFlakeInput) publishedForms
          );

          # Each scope's two names reach one module, which is what makes the
          # row above a statement about two modules rather than four. Measured
          # over the consumption files each name reaches rather than by `==`,
          # for the reason `consumeFilesOf` records.
          collapsed = {
            home = consumeFilesOf homeModules.default == consumeFilesOf homeModules.safix;
            nixos = consumeFilesOf nixosModules.default == consumeFilesOf nixosModules.safix;
            homeReaches = consumeFilesOf homeModules.default;
            nixosReaches = consumeFilesOf nixosModules.default;
          };

          antiVacuity = {
            declaresLibDetectsAbsence = declaresLib vacuousFixture;
            namesAnyFlakeInputDetectsOne = namesAnyFlakeInput inputNamingFixture;
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

          publishedFormsNamingAnInput = [ ];

          collapsed = {
            home = true;
            nixos = true;
            homeReaches = [ "modules/consume/home.nix" ];
            nixosReaches = [
              "modules/consume/installer.nix"
              "modules/consume/nixos.nix"
            ];
          };

          antiVacuity = {
            declaresLibDetectsAbsence = false;
            namesAnyFlakeInputDetectsOne = true;
            seesThroughDeferredModule = true;
          };
        };
      };
    };
}
