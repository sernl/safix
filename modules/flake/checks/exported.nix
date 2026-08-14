# Instantiates the checks safix exports over the fixture fleet this repository
# declares, and drills each one.
#
# The fleet goes into `flake.safix.users` and `flake.safix.catalogue` rather than
# into a `let`, and the checks come from `config.flake.safix.lib.mkChecks` rather
# than from a direct import, so what runs here is the binding a consumer gets. A
# rename of an option, a change to a default, or a builder that stopped reading
# the records reaches this file the way it would reach theirs.
#
# ── the drills ──
# Every family below is perturbed and the perturbation is required to fail.
# Where the check is a message list, the drill asserts the list is non-empty and
# names the thing that was broken; where it is a refusal, the drill asserts the
# refusal fires. The two runCommand drills go further and execute the same
# scripts the real checks execute — `refuseScript` over a non-empty message list
# and `driftScript` over a drifted file — because a list being non-empty is only
# a failure if the shell reading it exits non-zero, and that is a claim about the
# shell.
#
# The drills' oracles are written independently of the messages. A drill that
# asserted the exact sentence the code emits would be maintained by pasting
# whatever the generator last produced, and would then hold nothing: the claim is
# that breaking a declaration is reported and names what was broken, not that a
# particular wording survives an edit.
{ config, lib, ... }:
let
  fixture = import ./fixture-fleet.nix;

  safixChecks = import ../safix/checks.nix { inherit lib; };
  policy = import ../safix/policy.nix { inherit lib; };
  resolve = import ../safix/resolve.nix { inherit lib; };
  types = import ../safix/types.nix { inherit lib; };

  # A perturbed fleet goes through the real submodules, so it cannot pass by
  # omitting a field the option system would have supplied and a rename of an
  # option breaks these drills along with everything else.
  typed =
    optionType: definition:
    (lib.evalModules {
      modules = [
        { options.value = lib.mkOption { type = optionType; }; }
        { value = definition; }
      ];
    }).config.value;

  fleetOf = raw: {
    users = typed (lib.types.attrsOf types.profile) raw.users;
    catalogue = typed (lib.types.attrsOf types.entry) raw.catalogue;
  };

  perturbed = update: fleetOf (lib.recursiveUpdate fixture.fleet update);

  fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

  namesOneOf =
    tokens: messages:
    messages != [ ] && builtins.any (t: lib.any (m: lib.hasInfix t m) messages) tokens;
in
{
  flake.safix = {
    inherit (fixture.fleet) users catalogue;
  };

  perSystem =
    { pkgs, ... }:
    let
      safix = config.flake.safix.lib;
      users = config.flake.safix.users;
      catalogue = config.flake.safix.catalogue;

      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # The configuration an entry's `path` is a function of. A fixture stands in
      # for the consumer's own config here, which is the whole reason `path` is a
      # function: safix cannot know the shape of the tree it lands in.
      fixtureCfg.home = "/home/ana";

      materializations = {
        ana-user = safix.materialize {
          user = "ana";
          hostname = "workstation";
          tags = [ ];
          scope = "user";
        } fixtureCfg;

        ana-system = safix.materialize {
          user = "ana";
          hostname = "workstation";
          tags = [ ];
          scope = "system";
        } fixtureCfg;

        bo-system = safix.materialize {
          user = "bo";
          hostname = "server";
          tags = [ ];
          scope = "system";
        } fixtureCfg;
      };

      # ── the perturbations ──
      keylessGrantee = perturbed { users.bo.recipient = null; };
      misspelledTool = perturbed {
        users.ana.private.api-token.generator.runtimeInputs = [ "opensll" ];
      };
      collidingPaths = perturbed {
        users.ana.private.api-token.path = cfg: "${cfg.home}/.config/safix-fixture/ana-alone";
      };

      plan = policy.plan users catalogue;
      audiences = resolve.audiencesOf users catalogue;

      rewriteRules = f: plan // { rules = map (r: r // { pathRegex = f r.pathRegex; }) plan.rules; };

      unanchoredPlan = rewriteRules (lib.removePrefix "^");
      greedyPlan = rewriteRules (lib.replaceStrings [ "[^/]*" ] [ ".*" ]);
      unterminatedPlan = rewriteRules (p: lib.removeSuffix "\\.yaml$" p + "[^/]*$");

      catchAllPlan = plan // {
        rules = plan.rules ++ [
          {
            pathRegex = "^.*\\.yaml$";
            audience = [ "ana" ];
            anchors = [ "ana-safix" ];
          }
        ];
      };

      # A separator that is a regex metacharacter passes an injectivity claim —
      # `+` is outside the name alphabet — and still leaves every rule matching
      # something other than the directory it names. Rewriting the plan and the
      # audiences together is what an alternative separator would have produced.
      swapSeparator = lib.replaceStrings [ resolve.audienceSeparator ] [ "+" ];
      metacharPlan = plan // {
        rules = map (r: r // { pathRegex = swapSeparator r.pathRegex; }) plan.rules;
      };
      metacharAudiences = lib.mapAttrs (_f: a: a // { dir = swapSeparator a.dir; }) audiences;

      drift = pkgs.writeText "safix-drifted-policy" (
        lib.replaceStrings [ fixture.anaKey ] [ fixture.cyKey ] safix.policyText
      );
      generated = pkgs.writeText "safix-generated-policy" safix.policyText;

      # ── the checks a consumer instantiates, instantiated ──
      exported = safix.mkChecks pkgs {
        committedPolicy = ./fixture-policy.yaml;
        inherit materializations;
      };

      drills.safix-exported-drills = mkStructuralCheck {
        name = "safix-exported-drills";
        actual = {
          # The fixture itself has to be worth judging. An emptied fleet would
          # let every claim above pass by having nothing to look at.
          fixtureRoster = {
            people = lib.sort (a: b: a < b) (builtins.attrNames users);
            files = lib.sort (a: b: a < b) (builtins.attrNames audiences);
            generators = map (g: "${g.user}/${g.name}") (safixChecks.generatorsDeclaredIn users catalogue);
          };

          # Every family is silent on the fleet as declared. This is the same
          # list each check's derivation reads, so a family that reports on a
          # well-formed fleet fails here rather than in the check that would
          # then have to be blessed.
          quietOnFixture = {
            custody = safixChecks.custodyMessages users catalogue == [ ];
            generatorTools = safixChecks.generatorToolMessages pkgs users catalogue == [ ];
            ruleShape = safixChecks.ruleShapeMessages users catalogue == [ ];
            catchAll = safixChecks.catchAllMessages users catalogue == [ ];
            separator = safixChecks.separatorMessages users catalogue == [ ];
          };

          # 8.6 — a grant to someone holding no key. The message has to name the
          # person, because the remedy is theirs.
          custodyDrill = namesOneOf [ "bo" ] (
            safixChecks.custodyMessages keylessGrantee.users keylessGrantee.catalogue
          );

          # 8.5 — a misspelled runtime tool, which is otherwise discovered at a
          # rotation. The message has to name the spelling that was written.
          generatorToolDrill = namesOneOf [ "opensll" ] (
            safixChecks.generatorToolMessages pkgs misspelledTool.users misspelledTool.catalogue
          );

          # 8.2 — the three ways a rule stops covering exactly its own
          # directory, each perturbing the plan independently of the others.
          ruleShapeDrills = {
            unanchored =
              safixChecks.ruleShapeMessagesOf {
                plan = unanchoredPlan;
                inherit audiences;
              } != [ ];
            greedy =
              safixChecks.ruleShapeMessagesOf {
                plan = greedyPlan;
                inherit audiences;
              } != [ ];
            unterminated =
              safixChecks.ruleShapeMessagesOf {
                plan = unterminatedPlan;
                inherit audiences;
              } != [ ];
          };

          # 8.3 — one added rule matching anything that ends in .yaml. Every
          # probe is a path no declaration places anything in, and an uppercase
          # element keeps them outside the name alphabet, so no fleet can turn
          # one into a real directory.
          catchAllDrill = namesOneOf [ "^.*\\.yaml$" ] (safixChecks.catchAllMessagesOf catchAllPlan);

          # 8.7 — the separator on each side of what it has to be. Inside the
          # name alphabet, two audiences reach one directory; a regex
          # metacharacter leaves the rule matching something other than the
          # directory it names.
          separatorDrills = {
            insideAlphabet =
              safixChecks.separatorMessagesOf {
                inherit plan audiences;
                separator = "-";
              } != [ ];
            regexMetacharacter =
              safixChecks.separatorMessagesOf {
                plan = metacharPlan;
                audiences = metacharAudiences;
                separator = "+";
              } != [ ];
          };

          # 8.4 — two entries onto one path, which is unrecoverable rather than
          # untidy: whichever activates second unlinks the first's output.
          pathCollisionDrill = fires (
            resolve.materializeFor {
              users = collidingPaths.users;
              catalogue = collidingPaths.catalogue;
              root = ./.;
              user = "ana";
              hostname = "workstation";
              tags = [ ];
              scope = "user";
            } fixtureCfg
          );
        };

        expected = {
          fixtureRoster = {
            people = [
              "ana"
              "bo"
              "cy"
            ];
            files = [
              "secrets/safix/shared/ana,bo/secrets.yaml"
              "secrets/safix/users/ana/secrets.yaml"
              "secrets/safix/users/bo/secrets.yaml"
            ];
            generators = [ "ana/api-token" ];
          };
          quietOnFixture = {
            custody = true;
            generatorTools = true;
            ruleShape = true;
            catchAll = true;
            separator = true;
          };
          custodyDrill = true;
          generatorToolDrill = true;
          ruleShapeDrills = {
            unanchored = true;
            greedy = true;
            unterminated = true;
          };
          catchAllDrill = true;
          separatorDrills = {
            insideAlphabet = true;
            regexMetacharacter = true;
          };
          pathCollisionDrill = true;
        };
      };

      # The shell every message-bearing check runs, run over a non-empty list.
      # Asserting a list is non-empty says nothing about whether anything fails
      # on it; this is where that is settled.
      drills.safix-drill-refusal =
        pkgs.runCommand "safix-drill-refusal"
          {
            messagesText = "first broken declaration\nsecond broken declaration\n";
            passAsFile = [ "messagesText" ];
            meta.description = "structural check: safix-drill-refusal";
          }
          ''
            if ${safixChecks.refuseScript pkgs} "$messagesTextPath" "subject" 2>report; then
              echo "safix-refuse exited 0 over a non-empty message list" >&2
              exit 1
            fi
            for expected in "subject" "first broken declaration" "second broken declaration"; do
              if ! grep -qF "$expected" report; then
                echo "safix-refuse failed without reporting: $expected" >&2
                cat report >&2
                exit 1
              fi
            done
            touch "$out"
          '';

      # The drift comparison, run over a committed file with one recipient
      # replaced. The failure has to name the command that regenerates the file,
      # because that name is the whole of what a reader does next.
      drills.safix-drill-drift =
        pkgs.runCommand "safix-drill-drift"
          {
            meta.description = "structural check: safix-drill-drift";
          }
          ''
            if ${policy.driftScript pkgs} ${drift} ${generated} >/dev/null 2>report; then
              echo "the drift comparison exited 0 over a policy with a recipient replaced" >&2
              exit 1
            fi
            if ! grep -qF ${lib.escapeShellArg policy.regenerateCommand} report; then
              echo "the drift failure does not name ${policy.regenerateCommand}" >&2
              cat report >&2
              exit 1
            fi
            touch "$out"
          '';
    in
    {
      checks = exported // drills;
    };
}
