# Holds the bridge refusals of ../safix/bridge.nix against fleets built to break
# each one, and holds the boundary of what evaluation is allowed to claim.
#
# Every fixture is synthetic and every mapping names a machine that does not
# exist. That is deliberate rather than incidental: the clan half of a mapping
# lives in another flake, and nothing here may resolve it. A fixture whose clan
# side were real would make this file's passing depend on a clan being present,
# which is precisely the property the surface is designed not to have.
#
# Each refusal is asserted as the message it produces, against a literal, and
# the well-formed fleet is asserted to produce none. The pair is what binds
# them: a refusal that stopped firing empties its own field, and a refusal that
# fired naming the wrong party fails the literal.
#
# The drill runs `refuseScript` — the same bytes `mkMessageCheck` runs — over a
# perturbed fleet, so the severity claim is executed rather than described.
#
# ── what this cannot check ──
# That a mapping's clan side resolves, that the machine exists, that the
# generator declares the file, or that clan is installed. All four are run-time
# facts about another flake, and a build that asserted any of them would be
# asserting something about whatever clan happened to be in the check closure.
#
# And whether a safix entry holds a value at all. That one is not about the far
# side: it is unanswerable here because an entry is a declaration of where a
# value lives rather than that one is there. `handSetExportMessages` below is
# the assertion that evaluation stays silent about it, and the refusal lives in
# the runtime where the question can be asked.
#
# ── severity: proven by perturbation, one drill per claim ──
# Dropping `unresolvableSafixSide` from the list `violationsOf` returns empties
# `unknownUserMessages` and `unknownNameMessages` and moves no other field.
# Dropping `twoProducers` empties `twoProducersMessages` and, since the rule was
# broadened to two-way, `twoWayTwoProducersMessages` as well; each fixture's
# mapping is otherwise sound, so nothing else covers either.
# Dropping `twoMappingsOneTarget` empties `oneTargetMessages` and
# `sharedDuplicateTargetMessages`. `bothDirections` does not move under that
# drill and must not: opposite directions over one pair of endpoints have
# *different* targets, so the duplicate-target rule never saw them and the two
# rules are independent.
# Dropping `bothDirections` empties `bothDirectionsMessages` alone;
# `singleTwoWayMessages` stays empty either way, which is what proves the
# narrowed rule accepts a single two-way declaration rather than merely never
# firing.
# Dropping the `resolve.violations` short-circuit fills `brokenCustody` with
# bridge sentences about a fleet whose custody has not resolved, which is one
# fault producing two unrelated messages.
# Removing the direction enum lets `badDirection` evaluate rather than throw.
# Dropping `reservedId` from the list empties `reservedIdMessages`' three
# fields, one per reserved word a mapping id may collide with.
# Dropping `placementConsistency` empties `perMachineNoMachineMessages` and
# `sharedMachineSetMessages` independently, one per branch of the `if`.
# Dropping the placement-aware branch of `clanAddressOf` (reverting a shared
# mapping's key to `m.clan.machine`, always null) collapses every shared
# mapping's target to the same string regardless of generator or file, which
# would turn `soundSharedMessages` non-empty (two unrelated shared mappings in
# the well-formed fixture would collide) rather than catching the genuine
# collision `sharedDuplicateTargetMessages` asserts.
# Dropping `sharePlacementMismatch` empties `sharedGeneratorPerMachinePlacementMessages`
# and `perUserGeneratorSharedPlacementMessages` independently, one per branch;
# `sharedGeneratorSharedPlacementMessages` and `handSetSharedPlacementMessages`
# stay empty either way, which is what proves the rule accepts rather than
# merely never firing.
# Dropping `reservedCompanionName` empties `reservedCompanionNameMessages`.
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

      # Typed through the real option types, so a fixture cannot pass by omitting
      # a field the option system would have supplied, and an option rename
      # breaks this file along with the rest.
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

      # One fleet for every fixture, so a message that names the wrong person is
      # a failure rather than a coincidence. alice holds a hand-set entry and a
      # generated one; bob holds a hand-set entry alone.
      fleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          private.minted.generator.script = ''printf '%s' x > "$out/minted"'';
        };
        bob = {
          recipient = "age1fixturebbb00000000000000000000000000000000000000000000000";
          private.tok = { };
        };
      };

      # A fleet whose custody does not resolve: a grant to nobody. Used to hold
      # the short-circuit — while custody is broken the bridge says nothing.
      brokenFleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          sharedWith.nobody.tok = { };
        };
      };

      mapping = direction: user: name: generator: {
        inherit direction;
        clan = {
          machine = "nonexistent";
          inherit generator;
          file = "token";
        };
        safix = { inherit user name; };
      };

      sharedMapping = direction: user: name: generator: {
        inherit direction;
        clan = {
          placement = "shared";
          machine = null;
          inherit generator;
          file = "token";
        };
        safix = { inherit user name; };
      };

      violations = fleet': record: bridge.violationsOf { users = fleet'; } (bridgeOf record);

      violationsWith =
        fleet': catalogue': record:
        bridge.violationsOf { users = fleet'; catalogue = catalogue'; } (bridgeOf record);

      # A second catalogue and fleet, used only by the share/placement fixtures
      # below: `shared-tok` is a generator whose derived `share` is true because
      # its one output is carried from a `shared = true` catalogue entry, so it
      # is a distinct fixture from `fleet`'s `minted` (a per-user generator,
      # `share = false`).
      shareCatalogue = typed (lib.types.attrsOf types.entry) {
        shared-tok = {
          shared = true;
          generator.script = ''printf '%s' x > "$out/shared-tok"'';
        };
      };

      shareFleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          carries.shared-tok = { };
          private.tok = { };
          private.minted.generator.script = ''printf '%s' x > "$out/minted"'';
        };
      };

      shareViolations = record: violationsWith shareFleet shareCatalogue record;

      # A fleet whose alice has hand-declared the exact name a two-way mapping
      # of `tok` would reserve for its companion, used only by
      # `reservedCompanionNameMessages`.
      reservedCompanionFleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          private."tok-safix-bridge-sync-state" = { };
        };
      };

      sound = {
        clanFlake = ".";
        mappings = {
          down = mapping "clan-to-safix" "alice" "tok" "ntfy";
          up = mapping "safix-to-clan" "bob" "tok" "other";
        };
      };

      # `tryEval` catches the enum's refusal, which is a throw rather than a
      # message: an out-of-range direction is refused by the type before any
      # rule in `violationsOf` could look at it.
      badDirection =
        (builtins.tryEval (
          builtins.deepSeq (bridgeOf {
            clanFlake = ".";
            mappings.a = mapping "import" "alice" "tok" "ntfy";
          }) "resolved"
        )).success;

      drill =
        pkgs.runCommand "safix-bridge-drill" { meta.description = "severity drill: safix-bridge-refusals"; }
          ''
            messages=$(mktemp)
            printf '%s\n' ${
              lib.escapeShellArg (
                builtins.head (
                  violations fleet {
                    clanFlake = ".";
                    mappings.a = mapping "clan-to-safix" "carol" "tok" "ntfy";
                  }
                )
              )
            } > "$messages"

            if ${safixChecks.refuseScript pkgs} "$messages" "subject" 2> refused; then
              echo "the refusal script accepted a non-empty message list" >&2
              exit 1
            fi
            grep -q "which flake.safix.users does not declare" refused
            grep -q "subject" refused

            : > "$messages"
            ${safixChecks.refuseScript pkgs} "$messages" "subject"
            touch "$out"
          '';
    in
    {
      checks.safix-bridge = mkStructuralCheck {
        name = "safix-bridge";
        actual = {
          directions = bridge.directions;

          soundMessages = violations fleet sound;

          unknownUserMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "carol" "tok" "ntfy";
          };

          unknownNameMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "alice" "absent" "ntfy";
          };

          twoProducersMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "alice" "minted" "ntfy";
          };

          # The same target reached by two mappings that differ on the clan
          # side, so the duplicate is the safix half and nothing else.
          oneTargetMessages = violations fleet {
            clanFlake = ".";
            mappings = {
              a = mapping "clan-to-safix" "alice" "tok" "ntfy";
              b = mapping "clan-to-safix" "alice" "tok" "other";
            };
          };

          bothDirectionsMessages = violations fleet {
            clanFlake = ".";
            mappings = {
              down = mapping "clan-to-safix" "alice" "tok" "ntfy";
              up = mapping "safix-to-clan" "alice" "tok" "ntfy";
            };
          };

          noClanFlakeMessages = violations fleet {
            mappings.a = mapping "clan-to-safix" "alice" "tok" "ntfy";
          };

          reservedIdMessages = {
            clan = violations fleet {
              clanFlake = ".";
              mappings.clan = mapping "clan-to-safix" "alice" "tok" "ntfy";
            };
            keepassxc = violations fleet {
              clanFlake = ".";
              mappings.keepassxc = mapping "clan-to-safix" "alice" "tok" "ntfy";
            };
            all = violations fleet {
              clanFlake = ".";
              mappings.all = mapping "clan-to-safix" "alice" "tok" "ntfy";
            };
          };

          # A mapping-free bridge with no clan named is the default a consumer
          # who has never heard of clan evaluates, and it must be silent.
          emptyBridgeMessages = violations fleet { };

          # Exporting a hand-set entry is the ordinary case and is not refused.
          # A generator on the safix side is not required to send a value.
          #
          # This is the surviving half of a rule that used to have an
          # evaluation-time sibling: the spec once required evaluation to refuse
          # a safix-to-clan mapping "whose source entry has neither a generator
          # nor a declared value". That requirement had no referent. An entry
          # declares where a value lives, not that one is there, so at
          # evaluation a hand-set entry before its first write and one after it
          # are the same declaration — and refusing on it would refuse the
          # ordinary export.
          #
          # Its replacement is a run-time refusal, and the two are siblings
          # rather than a move: this asserts that the mapping produces no
          # evaluation message, and `bridge.rs`'s
          # `an_export_whose_source_holds_no_value_is_refused` asserts that a
          # transfer reaching the same mapping over an unwritten entry refuses
          # and names both remedies. Neither is redundant, and this one would be
          # vacuous without it.
          handSetExportMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "safix-to-clan" "alice" "tok" "ntfy";
          };

          brokenCustody = violations brokenFleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "carol" "tok" "ntfy";
          };

          # Without this the field above is vacuous: an empty bridge message
          # list proves the short-circuit only if the fleet it was computed over
          # is one custody actually refuses.
          brokenCustodyIsBroken = resolve.violations { users = brokenFleet; } != [ ];

          # ── the placement model ──
          perMachineNoMachineMessages = violations fleet {
            clanFlake = ".";
            mappings.a = {
              direction = "clan-to-safix";
              clan = {
                placement = "per-machine";
                machine = null;
                generator = "ntfy";
                file = "token";
              };
              safix = {
                user = "alice";
                name = "tok";
              };
            };
          };

          sharedMachineSetMessages = violations fleet {
            clanFlake = ".";
            mappings.a = {
              direction = "clan-to-safix";
              clan = {
                placement = "shared";
                machine = "nonexistent";
                generator = "ntfy";
                file = "token";
              };
              safix = {
                user = "alice";
                name = "tok";
              };
            };
          };

          soundSharedMessages = violations fleet {
            clanFlake = ".";
            mappings.a = sharedMapping "clan-to-safix" "alice" "tok" "ntfy";
          };

          # A single two-way mapping over one pair of endpoints, accepted:
          # `bothDirections` groups by pair and only fires when a group holds
          # more than one distinct direction, so one mapping never can.
          singleTwoWayMessages = violations fleet {
            clanFlake = ".";
            mappings.rel = mapping "two-way" "alice" "tok" "ntfy";
          };

          # Two shared mappings of the same generator and file collide by
          # generator/file alone, regardless of the (absent) machine — the
          # defect D2 exists to close.
          sharedDuplicateTargetMessages = violations fleet {
            clanFlake = ".";
            mappings = {
              a = sharedMapping "safix-to-clan" "alice" "tok" "ntfy";
              b = sharedMapping "safix-to-clan" "bob" "tok" "ntfy";
            };
          };

          # ── two-producers broadened to two-way ──
          twoWayTwoProducersMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "two-way" "alice" "minted" "ntfy";
          };

          # ── the share/placement comparison ──
          sharedGeneratorSharedPlacementMessages = shareViolations {
            clanFlake = ".";
            mappings.a = sharedMapping "safix-to-clan" "alice" "shared-tok" "ntfy";
          };

          sharedGeneratorPerMachinePlacementMessages = shareViolations {
            clanFlake = ".";
            mappings.a = mapping "safix-to-clan" "alice" "shared-tok" "ntfy";
          };

          perUserGeneratorSharedPlacementMessages = shareViolations {
            clanFlake = ".";
            mappings.a = sharedMapping "safix-to-clan" "alice" "minted" "ntfy";
          };

          handSetSharedPlacementMessages = shareViolations {
            clanFlake = ".";
            mappings.a = sharedMapping "safix-to-clan" "alice" "tok" "ntfy";
          };

          # ── the companion reservation ──
          reservedCompanionNameMessages = violations reservedCompanionFleet {
            clanFlake = ".";
            mappings.a = mapping "two-way" "alice" "tok" "ntfy";
          };

          badDirection = badDirection;
        };
        expected = {
          directions = [
            "clan-to-safix"
            "safix-to-clan"
            "two-way"
          ];

          soundMessages = [ ];

          unknownUserMessages = [
            "flake.safix.bridge.mappings.a names the user 'carol', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.bridge.mappings.a names the secret 'absent', which flake.safix.users.alice does not hold"
          ];

          twoProducersMessages = [
            "flake.safix.bridge.mappings.a imports into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          oneTargetMessages = [
            "flake.safix.bridge.mappings a and b both write flake.safix.users.alice.tok"
          ];

          bothDirectionsMessages = [
            "flake.safix.bridge.mappings down and up declare nonexistent:ntfy/token <-> flake.safix.users.alice.tok in both directions, which is a two-way relationship and is declared once, as a single mapping whose direction is \"two-way\""
          ];

          noClanFlakeMessages = [
            "flake.safix.bridge declares 1 mapping(s) and no clanFlake, so there is no clan for them to reach"
          ];

          reservedIdMessages = {
            clan = [
              "flake.safix.bridge.mappings.clan is named 'clan', which sync and audit read as a target keyword rather than a mapping name"
            ];
            keepassxc = [
              "flake.safix.bridge.mappings.keepassxc is named 'keepassxc', which sync and audit read as a target keyword rather than a mapping name"
            ];
            all = [
              "flake.safix.bridge.mappings.all is named 'all', which sync and audit read as a target keyword rather than a mapping name"
            ];
          };

          emptyBridgeMessages = [ ];
          handSetExportMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;
          badDirection = false;

          perMachineNoMachineMessages = [
            "flake.safix.bridge.mappings.a has placement = \"per-machine\" and declares no machine"
          ];
          sharedMachineSetMessages = [
            "flake.safix.bridge.mappings.a has placement = \"shared\" and declares a machine, which a shared placement does not take: the machine that answers for it is discovered at run time"
          ];
          soundSharedMessages = [ ];
          singleTwoWayMessages = [ ];
          sharedDuplicateTargetMessages = [
            "flake.safix.bridge.mappings a and b both write shared:ntfy/token"
          ];

          twoWayTwoProducersMessages = [
            "flake.safix.bridge.mappings.a imports into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          sharedGeneratorSharedPlacementMessages = [ ];
          sharedGeneratorPerMachinePlacementMessages = [
            "flake.safix.bridge.mappings.a exports from a generator whose derived share is true into placement = \"per-machine\", which clan would derive as shared"
          ];
          perUserGeneratorSharedPlacementMessages = [
            "flake.safix.bridge.mappings.a exports from a generator whose derived share is false into placement = \"shared\", which clan would derive as per-machine"
          ];
          handSetSharedPlacementMessages = [ ];

          reservedCompanionNameMessages = [
            "flake.safix.users.alice declares 'tok-safix-bridge-sync-state', and '-safix-bridge-sync-state' is the suffix flake.safix.bridge.mappings.a reserves for the entry its two-way convergence records its last agreement in"
          ];
        };
      };

      checks.safix-bridge-drill = drill;
    };
}
