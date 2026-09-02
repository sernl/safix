# Holds `flake.lib.mkVault`'s contract: the flakeless-projection spec's claim
# that it returns exactly what a flake-parts consumer's `flake.safix.lib`
# holds for the same declarations, that its `modules` argument merges the way
# `imports` does, and design D4's boundary between what it returns and what
# an entry file declares beside it.
#
# ── three mechanisms, one fleet ──
# Mechanism A evaluates the real `../safix` module directly with the fleet
# declared as `flake.safix.*`, mirroring `portability.nix:76-83`'s own
# construction — the shape a flake-parts consumer's own module gets merged
# into. Mechanism B extracts `mkVault` from the actual published
# `modules/flake/lib.nix`, through its own `lib.evalModules` call, rather
# than a second copy of D1's formula, so what this check exercises is the
# function a consumer calls. Mechanism C imports `../../../lib` directly —
# the plain, flake-agnostic file `modules/flake/lib.nix` republishes rather
# than redefines — so mechanism B and mechanism C resolve identically by
# construction whenever the publisher stays thin. `routesAgree` below holds
# that; because `vaultProjection` (mechanism B) is what most other
# assertions in this file already use as ground truth, its own severity
# drill diverges `plainImportMkVault` (mechanism C) alone, which is what
# `routesAgree` reads and nothing else in this file does.
#
# `flake.safix.lib` carries seven fields that are resolver accessors bound to
# the registry — `resolveSet`, `resolveNames`, `materialize`, `publicValue`,
# `outputPath`, `mkDriftCheck`, `mkChecks` — which a JSON diff cannot compare.
# Both mechanisms bind them to the identical registry either way, so the
# comparison below is restricted to the data fields; a divergence in how
# either mechanism composes its modules would already show up there.
#
# ── the namespace refusal, and where D3's substitution happens ──
# `namespace.nix`'s scan is a private script local to its own `perSystem`
# closure, and its target is always the literal directory `${../safix}`
# (`namespace.nix:83`) — a filesystem tree grepped for source text, never a
# value reachable through `lib.evalModules`. Task 1.5 asks to point that scan
# at a fixture reached through `lib.evalModules` directly, and that is what
# is structurally impossible: a grep script takes a directory argument, not
# an evaluated module list, and `namespace.nix` exports no reusable binding
# this file could call instead.
#
# The claim scenario 4 of the flakeless-projection spec makes still holds,
# by a different and closer mechanism than the one named: a module in
# `mkVault`'s `modules` that sets `config.*` outside `flake.safix` is refused
# by the plain module system's own undeclared-option check, because neither
# `../safix` nor `mkVault`'s bare `lib.evalModules` composition declares
# anything freeform for it to land in. That refusal does not exist under
# flake-parts, whose own `flake` submodule is freeform — which is the reason
# `namespace.nix`'s separate static scan exists for that path in the first
# place — so the mkVault path is refused by a mechanism strictly its own
# rather than by namespace.nix's mechanism reused. Held below by forcing the
# offending option through a nested evaluation and asserting the refusal
# names it, the same evidence namespace.nix's own drill holds for its scan.
{ lib, ... }:
let
  keyOf = name: "age1fixture-${name}-000000000000000000000000000000000";

  # ── the fleet mechanism A and mechanism B both resolve ──
  # alice carries a catalogue entry, holds a private secret of her own, and
  # grants a second private secret to the machine she owns; bob holds nothing
  # beyond a recipient and belongs to the group. Small enough to read here,
  # real enough that a resolution divergence between the two mechanisms shows
  # up as two non-empty sets disagreeing rather than two empty ones agreeing.
  fleet = {
    users = {
      alice = {
        recipient = keyOf "alice";
        private = {
          laptop-token = { };
          fleet-token = { };
        };
        sharedWith.deck.fleet-token = { };
        carries.payment-key = { };
      };
      bob.recipient = keyOf "bob";
    };
    machines.deck = {
      recipient = keyOf "deck";
      owner = "alice";
    };
    groups.oncall.members = [
      "alice"
      "bob"
    ];
    catalogue.payment-key.owner = "alice";
  };

  flakePartsProjection =
    fleet:
    (lib.evalModules {
      modules = [
        ../safix
        { _module.args.self = ""; }
        { flake.safix = fleet; }
      ];
    }).config.flake.safix.lib;

  mkVault = (lib.evalModules { modules = [ ../lib.nix ]; }).config.flake.lib.mkVault;

  vaultProjection =
    fleet:
    mkVault {
      modules = [ { flake.safix = fleet; } ];
      root = "";
    };

  # Mechanism C: the plain import route `modules/flake/lib.nix` republishes
  # rather than redefines. `../../../lib` is the same `lib/default.nix`
  # `config.flake.lib.mkVault` (mechanism B, above) delegates to, so
  # `routesAgree` below holds by construction; a divergence can only come
  # from `modules/flake/lib.nix` no longer delegating to this file.
  plainImportMkVault = (import ../../../lib { inherit lib; }).mkVault;

  plainImportProjection =
    fleet:
    plainImportMkVault {
      modules = [ { flake.safix = fleet; } ];
      root = "";
    };

  dataFieldsOf = projection: lib.filterAttrs (_name: v: !(builtins.isFunction v)) projection;

  # ── 1.4: a fleet scattered across fixture modules ──
  # `travel-key` is declared by two module fixtures rather than one: one
  # names it in the catalogue, the other has alice carry it. Real files are
  # not needed to hold this claim — the module system draws no distinction
  # between a path-imported file and an inline attrset for merge purposes,
  # which is D3's own point, so each fixture module below stands for what
  # would be a separate file in a consumer's tree.
  scatteredFileA = {
    flake.safix.catalogue.travel-key.owner = "alice";
  };
  scatteredFileB = {
    flake.safix.users.alice = {
      recipient = keyOf "alice";
      carries.travel-key = { };
    };
  };
  scatteredOneFile = {
    flake.safix.catalogue.travel-key.owner = "alice";
    flake.safix.users.alice = {
      recipient = keyOf "alice";
      carries.travel-key = { };
    };
  };

  scatteredProjection = mkVault {
    modules = [
      scatteredFileA
      scatteredFileB
    ];
    root = "";
  };
  reorderedProjection = mkVault {
    modules = [
      scatteredFileB
      scatteredFileA
    ];
    root = "";
  };
  oneFileProjection = mkVault {
    modules = [ scatteredOneFile ];
    root = "";
  };
in
{
  perSystem =
    { pkgs, ... }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # A module-system error is not comparable with `builtins.tryEval` alone
      # — it gives success or failure, never the message — so the refusal
      # itself is forced inside a nested evaluation and its stderr is what is
      # asserted against, the same shape `integration.nix` already uses `nix`
      # inside a build for. `${pkgs.path}` is this flake's own pinned
      # nixpkgs, already realized, so nothing is fetched or built to reach it.
      nestedRefusal =
        {
          name,
          extraModules,
          forcedPath,
          mustMention,
        }:
        let
          exprText = ''
            let
              lib = (import <nixpkgs> {}).lib;
            in
            (lib.evalModules {
              modules = [
                ${toString ../safix}
                { _module.args.self = ""; }
                ${lib.concatStringsSep "\n      " extraModules}
              ];
            }).config.${forcedPath}
          '';
          exprFile = pkgs.writeText "safix-vault-projection-${name}.nix" exprText;
        in
        pkgs.runCommand "safix-vault-projection-${name}" { nativeBuildInputs = [ pkgs.nix ]; } ''
          export HOME="$TMPDIR"
          export NIX_PATH="nixpkgs=${pkgs.path}"
          if nix-instantiate --eval --strict ${exprFile} >out.log 2>err.log; then
            echo "expected evaluation of '${name}' to be refused; it produced:" >&2
            cat out.log >&2
            exit 1
          fi
          if ! grep -qF ${lib.escapeShellArg mustMention} err.log; then
            echo "the refusal for '${name}' did not name '${mustMention}'" >&2
            cat err.log >&2
            exit 1
          fi
          touch $out
        '';

      # 1.4 — two files declaring `travel-key` with different `owner`s.
      collisionRefusal = nestedRefusal {
        name = "collision";
        extraModules = [
          ''{ flake.safix.catalogue.travel-key.owner = "alice"; }''
          ''{ flake.safix.catalogue.travel-key.owner = "bob"; }''
        ];
        forcedPath = "flake.safix.catalogue.travel-key.owner";
        mustMention = "flake.safix.catalogue.travel-key.owner";
      };

      # 1.5 — a module in `modules` setting an option outside `flake.safix`.
      namespaceRefusal = nestedRefusal {
        name = "namespace-refusal";
        extraModules = [ ''{ notUnderFlakeSafix.value = "x"; }'' ];
        forcedPath = "notUnderFlakeSafix.value";
        mustMention = "notUnderFlakeSafix";
      };

      structural = mkStructuralCheck {
        name = "safix-vault-projection";
        actual = {
          # An emptied fleet would let `agree` and `scatterMatchesOneFile`
          # pass by comparing two empty sets to each other.
          fixtureIsReal = {
            audiences = lib.sort (a: b: a < b) (builtins.attrNames (vaultProjection fleet).audiences);
            alicePlacements = lib.sort (a: b: a < b) (
              builtins.attrNames (vaultProjection fleet).placements.alice
            );
          };

          # 1.3 — the same fleet, declared once as `flake.safix.*` and once
          # through `mkVault`'s `modules`, resolves to the same data.
          agree = dataFieldsOf (vaultProjection fleet) == dataFieldsOf (flakePartsProjection fleet);

          # 1.4 — `travel-key` scattered across two fixture modules resolves
          # identically to the same declaration in one.
          scatterMatchesOneFile = dataFieldsOf scatteredProjection == dataFieldsOf oneFileProjection;

          # 1.7 drill — reordering the same two fixture modules turns nothing
          # red, which is the evidence merge order does not matter.
          orderIndependent = dataFieldsOf scatteredProjection == dataFieldsOf reorderedProjection;

          # 1.6 — the returned value carries neither hook key.
          hookBoundary = {
            onboardingHook = vaultProjection fleet ? onboardingHook;
            enrollHook = vaultProjection fleet ? enrollHook;
          };

          # Two-routes-agree — the published route (`config.flake.lib.mkVault`,
          # extracted via mechanism B above) and the plain-import route
          # (`import ../../../lib { inherit lib; }`, mechanism C) resolve the
          # same fleet to the same data fields.
          routesAgree = dataFieldsOf (vaultProjection fleet) == dataFieldsOf (plainImportProjection fleet);
        };
        expected = {
          fixtureIsReal = {
            audiences = [
              "secrets/safix/shared/alice,deck/secrets.yaml"
              "secrets/safix/users/alice/secrets.yaml"
            ];
            alicePlacements = [
              "fleet-token"
              "laptop-token"
              "payment-key"
            ];
          };
          agree = true;
          scatterMatchesOneFile = true;
          orderIndependent = true;
          hookBoundary = {
            onboardingHook = false;
            enrollHook = false;
          };
          routesAgree = true;
        };
      };
    in
    {
      checks.safix-vault-projection =
        pkgs.runCommand "safix-vault-projection-suite"
          {
            meta.description = "structural check: safix-vault-projection";
          }
          ''
            : ${structural}
            : ${collisionRefusal}
            : ${namespaceRefusal}
            touch $out
          '';
    };
}
