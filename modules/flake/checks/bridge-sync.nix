# Holds the two-way companion mechanism ../safix/bridge.nix mints: that a
# two-way mapping's companion resolves to the same file and audience as the
# entry it mirrors, distinguished only by the reserved key suffix; that a
# mapping with no two-way declaration mints no companion at all; and that a
# hand-declared entry colliding with a reserved companion name is refused.
#
# What ../safix/bridge.nix's own checks already prove — the placement model,
# the direction refusals, the share/placement comparison — belongs in
# ./bridge.nix, not here. This file's claims are specifically about
# `companionsOf`: what it mints, when it mints nothing, and the refusal that
# keeps a consumer from declaring the name it reserves.
#
# ── severity: proven by perturbation ──
# The drill runs `refuseScript` over a fleet whose alice has hand-declared the
# exact name a two-way mapping of `tok` reserves for its companion. Dropping
# `reservedCompanionName` from `bridge.nix`'s `violationsOf` list empties that
# message, which is what the drill's `grep` would then fail to find, turning
# the drill red. The structural check's `oneWayCompanions = { }` and
# `placementsUnchangedByOneWay = true` fields hold the companion minting on
# the other side: a mapping declared `clan-to-safix` or `safix-to-clan` reaches
# no branch of `companionsOf` that ever writes an entry, so dropping that
# filter (minting a companion for every mapping regardless of direction) would
# turn `oneWayCompanions` non-empty.
{
  perSystem =
    {
      pkgs,
      lib,
      ...
    }:
    let
      bridge = import ../safix/bridge.nix { inherit lib; };
      resolve = import ../safix/resolve.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      safixChecks = import ../safix/checks.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      typed =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      fleetOf = users: typed (lib.types.attrsOf types.profile) users;

      bridgeOf =
        record:
        typed (lib.types.submodule {
          options = {
            clanFlake = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
            };
            mappings = lib.mkOption {
              type = lib.types.attrsOf bridge.mapping;
              default = { };
            };
          };
        }) record;

      mapping = direction: user: name: generator: {
        inherit direction;
        clan = {
          placement = "per-machine";
          machine = "nonexistent";
          inherit generator;
          file = "token";
        };
        safix = { inherit user name; };
      };

      fleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
        };
        bob = {
          recipient = "age1fixturebbb00000000000000000000000000000000000000000000000";
          private.tok = { };
        };
      };

      registry = { users = fleet; };

      oneWayBridge = bridgeOf {
        clanFlake = ".";
        mappings.a = mapping "clan-to-safix" "alice" "tok" "ntfy";
      };

      twoWayBridge = bridgeOf {
        clanFlake = ".";
        mappings.a = mapping "two-way" "alice" "tok" "ntfy";
      };

      basePlacements = resolve.placementsOf registry;
      mapped = basePlacements.alice.tok;

      companions = bridge.companionsOf registry twoWayBridge;
      companion = companions.alice."tok-safix-bridge-sync-state";

      # A fleet whose alice has hand-declared the exact name a two-way
      # mapping of `tok` reserves for its companion, used only by the drill.
      reservedCompanionFleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          private."tok-safix-bridge-sync-state" = { };
        };
      };

      drill =
        pkgs.runCommand "safix-bridge-sync-drill"
          { meta.description = "severity drill: safix-bridge-sync companion reservation"; }
          ''
            messages=$(mktemp)
            printf '%s\n' ${
              lib.escapeShellArg (
                builtins.head (
                  bridge.violationsOf { users = reservedCompanionFleet; } twoWayBridge
                )
              )
            } > "$messages"

            if ${safixChecks.refuseScript pkgs} "$messages" "subject" 2> refused; then
              echo "the refusal script accepted a non-empty message list" >&2
              exit 1
            fi
            grep -q "reserves for the entry its two-way convergence records its last agreement in" refused
            grep -q "subject" refused

            : > "$messages"
            ${safixChecks.refuseScript pkgs} "$messages" "subject"
            touch "$out"
          '';
    in
    {
      checks.safix-bridge-sync = mkStructuralCheck {
        name = "safix-bridge-sync";
        actual = {
          # The companion shares the mapped entry's file, audience-derived
          # origin, ownership and shared-ness, and reuses no generator or
          # public output of its own: it is minted, never generated.
          companionFile = companion.file;
          companionOwner = companion.owner;
          companionShared = companion.shared;
          companionOrigin = companion.origin;
          companionKey = companion.key;
          companionGenerator = companion.generator;
          companionPublic = companion.public;

          # A one-way bridge mints no companion at all: `companionsOf` filters
          # to `direction == "two-way"` before it ever looks at a mapping's
          # safix side.
          oneWayCompanions = bridge.companionsOf registry oneWayBridge;

          # A one-way declaration leaves the resolved placement set exactly as
          # it was before this change: minting is additive, keyed under a name
          # no one-way mapping's own entry ever collides with.
          placementsUnchangedByOneWay =
            lib.mapAttrs (
              user: named: named // (bridge.companionsOf registry oneWayBridge).${user} or { }
            ) basePlacements
            == basePlacements;
        };
        expected = {
          companionFile = mapped.file;
          companionOwner = mapped.owner;
          companionShared = mapped.shared;
          companionOrigin = mapped.origin;
          companionKey = "${mapped.key}-safix-bridge-sync-state";
          companionGenerator = null;
          companionPublic = null;

          oneWayCompanions = { };
          placementsUnchangedByOneWay = true;
        };
      };

      checks.safix-bridge-sync-drill = drill;
    };
}
