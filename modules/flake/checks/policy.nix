# Holds the recipient policy to the declarations that are supposed to have
# produced it.
#
# The generated file is judged as a plan rather than as prose. The whole text is
# what sops reads, so a consumer's own drift check compares the whole text; the
# claims that matter here are the anchors and the one rule per audience, and
# those have to survive an edit to the header. A severity check written against
# the rendered text would fail on every wording change and would be maintained by
# pasting whatever the generator emitted, which is the failure mode of deriving
# an expectation through the code under test.
#
# ── the fixture ──
# Two people, one of whom shares one secret with the other, and three throwaway
# recipient strings that decrypt nothing and have no private half anywhere.
# Encrypting needs the public key alone, so nothing here is a key: they are
# literals chosen to be distinguishable in a diff. alice escrows to a second
# identity she holds and bob does not, so the same fixture carries both custody
# postures and the rules below show each.
#
# Severity: proven by perturbation, one drill per claim, each recorded with the
# projection it moves.
# Dropping `bob` from the granted fixture's `sharedWith` fails `grantedRules`,
# whose shared rule disappears, and `sharedOrphaned.granted`, which flips: the
# shared file stops having an audience and so stops having any rule, which is
# what makes a file left behind on disk fail closed. `grantedAnchors` does not
# move, and should not — anchors are the registry's key list rather than a
# per-rule one, and bob still holds a key.
# Setting the audience separator to `+` passes the injectivity assertion in
# resolve.nix — `+` is outside the name alphabet, so the join stays injective —
# and fails `sharedRuleMatches` on `ownFile` and `elidedSeparator`. The rule for
# `alice+bob` matches `alicebob` and never `alice+bob`, so every file in that
# directory would fail closed under a rule that reads as if it covered them.
# That drill is why the separator has to be inert in a regex as well as absent
# from names, and it is the one an injectivity claim alone would miss.
# Removing the `^` from the emitted pattern, or replacing `[^/]*` with `.*`,
# fails `rulesWellFormed` and — for the second — `sharedRuleMatches.nestedFile`,
# which is the file that must not be covered.
# Removing the command name from the drift message fails `driftNamesCommand`,
# which is what binds the header a reader meets to the failure that sends them to
# it.
# Emptying the fixture fails `fixtureRoster`, which is what stops every claim
# below from passing by having nothing to judge.
{
  perSystem =
    { pkgs, lib, ... }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      policy = import ../safix/policy.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      sorted = lib.sort (a: b: a < b);

      typed =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      fixtureA = "age1fixtureaaa00000000000000000000000000000000000000000000000";
      fixtureB = "age1fixturebbb00000000000000000000000000000000000000000000000";
      fixtureVault = "age1fixturevault0000000000000000000000000000000000000000000000";

      mkUser =
        {
          recipient ? null,
          recoveryRecipients ? { },
          custody ? { },
        }:
        typed types.profile (
          custody
          // {
            inherit recipient;
            recoveryRecipients = lib.mapAttrs (_n: key: {
              inherit key;
              note = null;
            }) recoveryRecipients;
          }
        );

      fleetOf = lib.mapAttrs (_name: mkUser);

      granted = fleetOf {
        alice = {
          recipient = fixtureA;
          recoveryRecipients.vault = fixtureVault;
          custody = {
            private.alice-alone = { };
            private.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        bob = {
          recipient = fixtureB;
          custody.private.bob-alone = { };
        };
      };

      revoked = fleetOf {
        alice = {
          recipient = fixtureA;
          recoveryRecipients.vault = fixtureVault;
          custody = {
            private.alice-alone = { };
            private.shared-token = { };
          };
        };
        bob = {
          recipient = fixtureB;
          custody.private.bob-alone = { };
        };
      };

      # A third person who records a recipient and holds nothing. They earn an
      # anchor and no rule, since no audience includes them.
      withBystander = granted // {
        carol = mkUser { recipient = "age1fixtureccc00000000000000000000000000000000000000000000000"; };
      };

      sharedFile = "secrets/safix/shared/alice,bob/secrets.yaml";

      # These fixtures declare every secret under `private`, so no catalogue
      # entry can be shared and the audiences are the grant-derived ones alone.
      grantedAudiences = resolve.audiencesOf { users = granted; };
      revokedAudiences = resolve.audiencesOf { users = revoked; };

      planOf = users: policy.plan { inherit users; };

      rulesOf =
        users:
        map (r: {
          inherit (r) pathRegex audience anchors;
        }) (planOf users).rules;

      rendered = policy.render { users = granted; };
    in
    {
      checks.safix-policy = mkStructuralCheck {
        name = "safix-policy";
        actual = {
          # An emptied fixture would otherwise let every claim below pass by
          # having nothing to judge.
          fixtureRoster = sorted (builtins.attrNames granted);

          # Anchors are registry-wide and ordered: recovery identities first,
          # sorted, then one <user>-safix per person that records a recipient. A
          # rule's recipients are emitted in the same order.
          grantedAnchors = map (a: a.anchor) (planOf granted).anchors;
          grantedRules = rulesOf granted;
          revokedRules = rulesOf revoked;

          # A declared person holding nothing gets an anchor and no rule.
          bystanderAnchors = map (a: a.anchor) (planOf withBystander).anchors;
          bystanderRuleAudiences = map (r: r.audience) (planOf withBystander).rules;

          # The shared file has no audience once the grant is gone, so it has no
          # rule either. A file left behind on disk therefore matches nothing and
          # fails closed rather than inheriting anyone's custody.
          sharedOrphaned = {
            granted = grantedAudiences ? ${sharedFile};
            revoked = revokedAudiences ? ${sharedFile};
          };

          # Every rule is start-anchored, extension-terminated and one directory
          # level, and no rule is a catch-all.
          rulesWellFormed = builtins.all (
            r: lib.hasPrefix "^secrets/" r.pathRegex && lib.hasSuffix "/[^/]*\\.yaml$" r.pathRegex
          ) (planOf granted).rules;

          # The audience separator is interpolated into a generated path_regex,
          # so it has to be inert there as well as outside the name alphabet. A
          # regex metacharacter would leave the rule matching something other
          # than the directory it names — `alice+bob` matches `alicebob` and
          # never `alice+bob` — and every file in that directory would fail
          # closed under a rule that reads as if it covered them.
          sharedRuleMatches =
            let
              rule = lib.head (lib.filter (r: builtins.length r.audience > 1) (planOf granted).rules);
              matches = p: builtins.match rule.pathRegex p != null;
            in
            {
              ownFile = matches sharedFile;
              elidedSeparator = matches (lib.replaceStrings [ "alice,bob" ] [ "alicebob" ] sharedFile);
              siblingDirectory = matches "secrets/safix/users/alice/secrets.yaml";
              nestedFile = matches "secrets/safix/shared/alice,bob/deeper/secrets.yaml";
              prefixedPath = matches "nested/${sharedFile}";
            };

          # The rendered text is judged only on the properties a consumer's file
          # depends on structurally: that it is a comment-headed YAML document
          # with both blocks, that every anchor it defines is referenced, and
          # that the header names the regenerating command.
          renderedShape = {
            headed = lib.hasPrefix "# .sops.yaml — generated by safix." rendered;
            hasKeys = lib.hasInfix "\nkeys:\n" rendered;
            hasRules = lib.hasInfix "\ncreation_rules:\n" rendered;
            namesCommand = lib.hasInfix policy.regenerateCommand rendered;
            definesEveryAnchor = builtins.all (
              a: lib.hasInfix "  - &${a.anchor} ${a.key}\n" rendered
            ) (planOf granted).anchors;
          };

          # The header a reader meets and the failure that sends them to it name
          # one command.
          driftNamesCommand = lib.hasInfix policy.regenerateCommand policy.driftMessage;
        };
        expected = {
          fixtureRoster = [
            "alice"
            "bob"
          ];

          grantedAnchors = [
            "vault"
            "alice-safix"
            "bob-safix"
          ];
          grantedRules = [
            {
              pathRegex = "^secrets/safix/shared/alice,bob/[^/]*\\.yaml$";
              audience = [
                "alice"
                "bob"
              ];
              anchors = [
                "vault"
                "alice-safix"
                "bob-safix"
              ];
            }
            {
              pathRegex = "^secrets/safix/users/alice/[^/]*\\.yaml$";
              audience = [ "alice" ];
              anchors = [
                "vault"
                "alice-safix"
              ];
            }
            {
              pathRegex = "^secrets/safix/users/bob/[^/]*\\.yaml$";
              audience = [ "bob" ];
              anchors = [ "bob-safix" ];
            }
          ];
          revokedRules = [
            {
              pathRegex = "^secrets/safix/users/alice/[^/]*\\.yaml$";
              audience = [ "alice" ];
              anchors = [
                "vault"
                "alice-safix"
              ];
            }
            {
              pathRegex = "^secrets/safix/users/bob/[^/]*\\.yaml$";
              audience = [ "bob" ];
              anchors = [ "bob-safix" ];
            }
          ];

          bystanderAnchors = [
            "vault"
            "alice-safix"
            "bob-safix"
            "carol-safix"
          ];
          bystanderRuleAudiences = [
            [
              "alice"
              "bob"
            ]
            [ "alice" ]
            [ "bob" ]
          ];

          sharedOrphaned = {
            granted = true;
            revoked = false;
          };

          rulesWellFormed = true;

          sharedRuleMatches = {
            ownFile = true;
            elidedSeparator = false;
            siblingDirectory = false;
            nestedFile = false;
            prefixedPath = false;
          };

          renderedShape = {
            headed = true;
            hasKeys = true;
            hasRules = true;
            namesCommand = true;
            definesEveryAnchor = true;
          };

          driftNamesCommand = true;
        };
      };
    };
}
