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
# Dropping `twoProducers` empties `twoProducersMessages`; the fixture's mapping
# is otherwise sound, so nothing else covers it.
# Dropping `twoMappingsOneTarget` empties `oneTargetMessages`. `bothDirections`
# does not move under that drill and must not: opposite directions over one pair
# of endpoints have *different* targets, so the duplicate-target rule never saw
# them and the two rules are independent or the two-way refusal is unreachable.
# Dropping `bothDirections` empties `bothDirectionsMessages` alone.
# Dropping the `resolve.violations` short-circuit fills `brokenCustody` with
# bridge sentences about a fleet whose custody has not resolved, which is one
# fault producing two unrelated messages.
# Removing the direction enum lets `badDirection` evaluate rather than throw.
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
      # a failure rather than a coincidence. ana holds a hand-set entry and a
      # generated one; bo holds a hand-set entry alone.
      fleet = fleetOf {
        ana = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          private.minted.generator.script = ''printf '%s' x > "$out/minted"'';
        };
        bo = {
          recipient = "age1fixturebbb00000000000000000000000000000000000000000000000";
          private.tok = { };
        };
      };

      # A fleet whose custody does not resolve: a grant to nobody. Used to hold
      # the short-circuit — while custody is broken the bridge says nothing.
      brokenFleet = fleetOf {
        ana = {
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

      violations = fleet': record: bridge.violationsOf { users = fleet'; } (bridgeOf record);

      sound = {
        clanFlake = ".";
        mappings = {
          down = mapping "clan-to-safix" "ana" "tok" "ntfy";
          up = mapping "safix-to-clan" "bo" "tok" "other";
        };
      };

      # `tryEval` catches the enum's refusal, which is a throw rather than a
      # message: an out-of-range direction is refused by the type before any
      # rule in `violationsOf` could look at it.
      badDirection =
        (builtins.tryEval (
          builtins.deepSeq (bridgeOf {
            clanFlake = ".";
            mappings.a = mapping "import" "ana" "tok" "ntfy";
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
                    mappings.a = mapping "clan-to-safix" "cy" "tok" "ntfy";
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
            mappings.a = mapping "clan-to-safix" "cy" "tok" "ntfy";
          };

          unknownNameMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "ana" "absent" "ntfy";
          };

          twoProducersMessages = violations fleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "ana" "minted" "ntfy";
          };

          # The same target reached by two mappings that differ on the clan
          # side, so the duplicate is the safix half and nothing else.
          oneTargetMessages = violations fleet {
            clanFlake = ".";
            mappings = {
              a = mapping "clan-to-safix" "ana" "tok" "ntfy";
              b = mapping "clan-to-safix" "ana" "tok" "other";
            };
          };

          bothDirectionsMessages = violations fleet {
            clanFlake = ".";
            mappings = {
              down = mapping "clan-to-safix" "ana" "tok" "ntfy";
              up = mapping "safix-to-clan" "ana" "tok" "ntfy";
            };
          };

          noClanFlakeMessages = violations fleet {
            mappings.a = mapping "clan-to-safix" "ana" "tok" "ntfy";
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
            mappings.a = mapping "safix-to-clan" "ana" "tok" "ntfy";
          };

          brokenCustody = violations brokenFleet {
            clanFlake = ".";
            mappings.a = mapping "clan-to-safix" "cy" "tok" "ntfy";
          };

          # Without this the field above is vacuous: an empty bridge message
          # list proves the short-circuit only if the fleet it was computed over
          # is one custody actually refuses.
          brokenCustodyIsBroken = resolve.violations { users = brokenFleet; } != [ ];

          badDirection = badDirection;
        };
        expected = {
          directions = [
            "clan-to-safix"
            "safix-to-clan"
          ];

          soundMessages = [ ];

          unknownUserMessages = [
            "flake.safix.bridge.mappings.a names the user 'cy', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.bridge.mappings.a names the secret 'absent', which flake.safix.users.ana does not hold"
          ];

          twoProducersMessages = [
            "flake.safix.bridge.mappings.a imports into flake.safix.users.ana.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          oneTargetMessages = [
            "flake.safix.bridge.mappings a and b both write flake.safix.users.ana.tok"
          ];

          bothDirectionsMessages = [
            "flake.safix.bridge.mappings down and up declare nonexistent:ntfy/token <-> flake.safix.users.ana.tok in both directions, which is a two-way synchronisation and has no conflict resolution"
          ];

          noClanFlakeMessages = [
            "flake.safix.bridge declares 1 mapping(s) and no clanFlake, so there is no clan for them to reach"
          ];

          emptyBridgeMessages = [ ];
          handSetExportMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;
          badDirection = false;
        };
      };

      checks.safix-bridge-drill = drill;
    };
}
