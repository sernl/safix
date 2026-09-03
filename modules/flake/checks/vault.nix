# Holds the secrets-vault capability: the `flake.safix.vault` option, its
# naming-key refusals, the root flip, `vaultDeclared`, the opaque physical
# layout groups 4 and 10.1 add to `placementsIn`, and the disposable vault
# creation-rules rendering groups 5.2-5.3 add to `policy.nix`.
#
# ── one fixture, three registries ──
# One fleet — alice, bob, carol, and the machine `deck` alice owns — carries a
# sole-owner secret, a three-member shared secret (alice, bob and the machine
# `deck`, so an audience element that is a person and one that is a machine
# both exercise opacity), and a generator with a public output (`wg-public`),
# which is what gives `placements.<u>.<n>.public` something to be opaque
# about. `noVault` resolves it with no `vault` declared; `withVault` resolves
# the same fleet with a well-formed one; `withNoKey`/`withShortKey`/
# `withNonHexKey` each resolve it with one of the three malformed naming
# keys the option refuses.
#
# ── the oracle ──
# Every expected opaque value below is derived from `noVault`'s own readable
# output — `noVault.placements.<u>.<n>.file` *is* `audienceFileOf audience`,
# by the readable-mode identity groups 1 and 4 hold — fed through
# `resolve.opaqueOf`/`secretsFileOf`/`publicFileOfVault`, imported directly
# rather than read back off `withVault`. That is what makes the comparison a
# check on wiring — did the right call site apply the right tag to the right
# logical path — rather than a restatement of `opaqueOf`'s own one-line
# formula, which nothing here disputes.
#
# ── severity, proven by perturbation ──
# 1.9: reverting the root flip in `default.nix`'s `bound` (`root = self;`
# unconditionally) while leaving `vaultDeclared` computed from `cfg.vault`
# turns `rootFlip.withVault` red — `sopsFile` keeps joining against the
# declaration root — while `vaultDeclared.withVault` stays `true`, which is
# the evidence the two are held independently rather than one standing in
# for the other. Verified by patching a scratch copy and re-evaluating.
# 1.10: dropping any one of `vaultViolations`' three clauses from
# `resolve.nix` turns exactly the corresponding case in `namingKeyRefusals`
# green (no message) and leaves the other two red as before — verified by
# dropping the short-key clause and observing `shortKey` alone go silent.
# 4.10: `opaqueOf`'s four tags are load-bearing rather than decorative —
# collapsing two of its four call sites onto one tag makes a public output
# and a ciphertext file that happen to share one readable identity hash to
# the same string; the real function's distinct tags keep them apart.
# Verified directly against `builtins.hashString` with the tags collapsed.
# 4.9, as stated in `tasks.md`, does not reproduce against this
# implementation: the task predicts that reverting `materializeFor`'s
# unconditional `key` emission (line ~2384, kept as written) while leaving
# `opaqueKeyOf` routing in place turns `consumptionResolves` red for an
# entry with no declared `sopsKey`. It does not, because `forUser` (not one
# of the two sites `tasks.md` names) was also changed to set `sopsKey`
# unconditionally in vault mode — required so a user-scope activation can
# find the opaque key `placementsIn` wrote into the document at all, not
# only to preserve opacity for an undeclared key. That addition already
# guarantees `secret.sopsKey` is non-null in vault mode, so
# `materializeFor`'s own guard is redundant in this implementation; it is
# kept as the second, independent guarantee the task asks for rather than
# removed, on the same "belt and braces" reasoning `scratch.rs`'s own two
# sweep mechanisms use. Verified by reverting only the `materializeFor`
# guard against a scratch copy and observing `consumptionResolves` stay
# green.
{ lib, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      policy = import ../safix/policy.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # Minted the way the option's own description tells an operator to:
      # `openssl rand -hex 32`. Fixed here rather than re-minted per
      # evaluation so a diff against a previous run's expectations is
      # meaningful.
      namingKey = "fc84b416cd03fedc2c02116b068d75c543d327361f19e3d18d9e8aa3de0f4ad2";
      shortKey = builtins.substring 0 63 namingKey;
      nonHexKey = "g" + builtins.substring 1 63 namingKey;

      # Two distinct, always-present paths rather than one reused for both:
      # `declarationRoot` stands in for `self` and `vaultRootPath` for
      # `flake.safix.vault.root`, so a check that the root actually flips
      # (rather than merely accepting a value it never joins against) can
      # tell the two apart. Neither is ever read from; `toString` below is
      # the only operation performed on either.
      declarationRoot = /dev/null;
      vaultRootPath = /dev;

      aliceKey = "age1fixturevaultaaaa000000000000000000000000000000000000000000";
      bobKey = "age1fixturevaultbbbb000000000000000000000000000000000000000000";
      carolKey = "age1fixturevaultcccc000000000000000000000000000000000000000000";
      deckKey = "age1fixturevaultdddd000000000000000000000000000000000000000000";

      fleet = {
        users = {
          alice = {
            recipient = aliceKey;
            private = {
              solo-token = { };
              team-secret = { };

              # clan's wireguard keypair, ported the same way
              # `fixture-fleet.nix` ports it: one generator, two registry
              # entries, so `wg-public`'s opacity has a real public output to
              # be about rather than a bare option name.
              wg-private = {
                mode = "0400";
                generator = {
                  runtimeInputs = [ "wireguard-tools" ];
                  script = ''
                    wg genkey > "$out/wg-private"
                    wg pubkey < "$out/wg-private" > "$out/wg-public"
                  '';
                  files.wg-public.secret = false;
                };
              };
              wg-public = {
                mode = "0444";
              };
            };

            # Granted to a person and to a machine, so the shared file's
            # audience carries both an unmarked person element and an
            # unmarked machine element.
            sharedWith.bob.team-secret = { };
            sharedWith.deck.team-secret = { };
          };
          bob.recipient = bobKey;
          # Holds nothing; earns a policy anchor and no rule, exactly as
          # `fixture-fleet.nix`'s carol does.
          carol.recipient = carolKey;
        };
        machines.deck = {
          recipient = deckKey;
          owner = "alice";
        };
      };

      projectionOf =
        extra:
        (lib.evalModules {
          modules = [
            ../safix
            { _module.args.self = declarationRoot; }
            { flake.safix = fleet // extra; }
          ];
        }).config.flake.safix.lib;

      noVault = projectionOf { };
      withVault = projectionOf {
        vault = {
          root = vaultRootPath;
          namingKey = namingKey;
        };
      };
      withNoKey = projectionOf {
        vault = {
          root = vaultRootPath;
        };
      };
      withShortKey = projectionOf {
        vault = {
          root = vaultRootPath;
          namingKey = shortKey;
        };
      };
      withNonHexKey = projectionOf {
        vault = {
          root = vaultRootPath;
          namingKey = nonHexKey;
        };
      };

      # 1.6's own claim, over `sopsFile` itself rather than the
      # root-independent `placements.*.file` — `resolveSet` is what
      # `selectFor` returns once `root` is joined in. `toString` avoids
      # forcing a store copy of a `path` value that is never meant to exist.
      soloSopsFile =
        projection:
        toString
          (projection.resolveSet {
            user = "alice";
            machine = null;
            hostname = "h";
            tags = [ ];
          }).solo-token.sopsFile;

      # The readable file identity for one of alice's names, read off the
      # no-vault projection — `audienceFileOf audience` itself, by the
      # readable-mode identity.
      logicalFileOf = name: noVault.placements.alice.${name}.file;
      logicalKeyOf = name: noVault.placements.alice.${name}.key;
      logicalPublicOf = name: noVault.placements.alice.${name}.public;
      audienceOfFile = file: noVault.audiences.${file}.audience;

      logicalRecordOf =
        name:
        let
          placement = noVault.placements.alice.${name};
        in
        if placement.shared then
          "shared/${lib.concatStringsSep resolve.audienceSeparator (audienceOfFile placement.file)}/${name}"
        else
          "${placement.owner}/${name}";

      expectedOpaqueFile =
        name: "secrets/${resolve.opaqueOf namingKey "secrets" (logicalFileOf name)}.yaml";
      expectedOpaqueKey = name: resolve.opaqueKeyOf namingKey (logicalFileOf name) (logicalKeyOf name);
      expectedOpaquePublic = name: "public/${resolve.opaqueOf namingKey "public" (logicalPublicOf name)}";
      expectedDefinitionRecord =
        name: "state/${resolve.opaqueOf namingKey "state" (logicalRecordOf name)}";

      names = [
        "solo-token"
        "team-secret"
        "wg-private"
        "wg-public"
      ];

      # Every opaque field a vault-mode fixture produces, flattened into one
      # list of strings, for the "names nothing readable" scan below.
      opaqueStrings =
        lib.concatMap (
          name:
          let
            p = withVault.placements.alice.${name};
          in
          [
            p.file
            p.key
            p.definitionRecord
          ]
          ++ lib.optional (p.public != null) p.public
        ) names
        ++ [ withVault.vaultCreationRulesText ];

      readableFragments = [
        "alice"
        "bob"
        "carol"
        "deck"
        "solo-token"
        "team-secret"
        "wg-private"
        "wg-public"
      ];

      leakedFragments = lib.filter (
        fragment: lib.any (s: lib.hasInfix fragment s) opaqueStrings
      ) readableFragments;

      # ── 5.2/5.3: the disposable creation-rules rendering ──
      rulesLines = lib.splitString "\n" withVault.vaultCreationRulesText;
      pathRegexLines = lib.filter (l: lib.hasPrefix "  - path_regex: " l) rulesLines;
      renderedRegexes = map (l: lib.removePrefix "  - path_regex: " l) pathRegexLines;

      expectedRegexOf =
        name: "^secrets/${resolve.opaqueOf namingKey "secrets" (logicalFileOf name)}\\.yaml$";

      expectedRegexes = lib.sort (a: b: a < b) (
        map expectedRegexOf [
          "solo-token"
          "team-secret"
        ]
      );

      # Parsed as YAML by a parser that is not this codebase's own renderer:
      # `yq-go` converts the rendered text to JSON, and the assertions below
      # read that JSON rather than the text `renderVaultRules` produced,
      # which is what makes "parses as YAML" a claim about the bytes rather
      # than about `renderVaultRules`'s own string-concatenation agreeing
      # with itself.
      vaultRulesText = pkgs.writeText "safix-vault-rules-fixture.yaml" withVault.vaultCreationRulesText;

      vaultRulesParse =
        pkgs.runCommand "safix-vault-rules-parse"
          {
            nativeBuildInputs = [
              pkgs.yq-go
              pkgs.jq
            ];
          }
          ''
            yq -o=json '.' ${vaultRulesText} > rules.json

            keys="$(jq -r 'keys | join(",")' rules.json)"
            if [ "$keys" != "creation_rules" ]; then
              echo "vault creation rules: expected exactly one top-level key 'creation_rules', got: $keys" >&2
              exit 1
            fi

            count="$(jq '.creation_rules | length' rules.json)"
            if [ "$count" != "2" ]; then
              echo "vault creation rules: expected 2 rules, got $count" >&2
              cat rules.json >&2
              exit 1
            fi

            touch $out
          '';

      # ── the naming-key refusal messages ──
      hasMessage = result: needle: lib.any (m: lib.hasInfix needle m) result.violations;

      structural = mkStructuralCheck {
        name = "safix-vault";
        actual = {
          # 1.6/4.8 — no vault declared resolves exactly today's readable
          # formula: independently recomputed via `audienceFileOf`/
          # `publicFileOf` rather than trusted from `placementsIn`'s own
          # output, so a routing bug on the readable side would show up
          # here too.
          readableMatchesFormula = {
            soloFile = noVault.placements.alice.solo-token.file == resolve.audienceFileOf [ "alice" ];
            teamFile =
              noVault.placements.alice.team-secret.file
              == resolve.audienceFileOf (audienceOfFile noVault.placements.alice.team-secret.file);
            wgPublicValue =
              noVault.placements.alice.wg-public.public == resolve.publicFileOf [ "alice" ] "wg-public";
            noneOpaque = {
              definitionRecord = noVault.placements.alice.solo-token.definitionRecord;
              logicalFile = noVault.placements.alice.solo-token.logicalFile;
              logicalKey = noVault.placements.alice.solo-token.logicalKey;
              logicalPublic = noVault.placements.alice.wg-public.logicalPublic;
            };
          };

          # 1.6 — the root itself flips: `sopsFile` joins against `self`
          # with no vault declared and against `flake.safix.vault.root`
          # with one, and never the other way around.
          rootFlip = {
            noVault =
              soloSopsFile noVault == "${toString declarationRoot}/${resolve.audienceFileOf [ "alice" ]}";
            withVault =
              soloSopsFile withVault == "${toString vaultRootPath}/${expectedOpaqueFile "solo-token"}";
          };

          # 1.7 — vaultDeclared
          vaultDeclared = {
            noVault = noVault.vaultDeclared;
            withVault = withVault.vaultDeclared;
          };

          # 1.8 — the three naming-key refusals, and the well-formed accept
          namingKeyRefusals = {
            noKey = hasMessage withNoKey "namingKey is not set";
            shortKey = hasMessage withShortKey "short of the 64 lowercase hexadecimal";
            nonHexKey = hasMessage withNonHexKey "outside [0-9a-f]";
            wellFormedAccepted = withVault.violations;
          };

          # 4.6/10.1/12.2 — every vault-rooted name is opaque, agrees with
          # `opaqueOf` applied to the readable identity, and names nothing
          # readable. `team-secret`'s file is the "multi-member audience in
          # vault mode" scenario 12.2 names: three elements, one of them a
          # machine, and the file is a hash rather than a sorted member list.
          opaque = lib.genAttrs names (
            name:
            let
              p = withVault.placements.alice.${name};
            in
            {
              file = p.file == expectedOpaqueFile name;
              key = p.key == expectedOpaqueKey name;
              definitionRecord = p.definitionRecord == expectedDefinitionRecord name;
              logicalFile = p.logicalFile == logicalFileOf name;
              logicalKey = p.logicalKey == logicalKeyOf name;
            }
          );
          publicOpaque = withVault.placements.alice.wg-public.public == expectedOpaquePublic "wg-public";
          publicLogical = withVault.placements.alice.wg-public.logicalPublic == logicalPublicOf "wg-public";
          noPublicStaysNull = {
            public = withVault.placements.alice.solo-token.public;
            logicalPublic = withVault.placements.alice.solo-token.logicalPublic;
          };
          leakedFragments = leakedFragments;

          # 4.7 — `placementsIn`'s key and `selectFor.forMachine`'s
          # independently recomputed `sopsKey` agree bit for bit for the
          # entry that resolves at both scopes: alice's own placement of
          # `team-secret`, and the machine `deck`'s grant of the same name.
          machineKeyAgreement =
            withVault.placements.alice.team-secret.key == (withVault.materialize {
              machine = "deck";
              hostname = "deck";
              tags = [ ];
              scope = "system";
            } { }).team-secret.key;

          # 12.1 — installation and consumption resolve under vault mode at
          # both scopes: the shape `materializeFor` hands a home-manager or
          # NixOS profile — `mode`, `sopsFile`, `key` — is present and the
          # key is the same opaque value `placements` carries, for a
          # user-scope materialization of alice's own name and a
          # system-scope materialization of the machine's grant.
          consumptionResolves = {
            userScopeHasOpaqueKey =
              (withVault.materialize {
                user = "alice";
                machine = null;
                hostname = "deck";
                tags = [ ];
                scope = "user";
              } { }).solo-token.key == expectedOpaqueKey "solo-token";
            systemScopeHasOpaqueKey =
              (withVault.materialize {
                machine = "deck";
                hostname = "deck";
                tags = [ ];
                scope = "system";
              } { }).team-secret.key == expectedOpaqueKey "team-secret";
          };

          # 12.3 — a vault-mode public leaf is a single opaque file, not a
          # `<name>/value` directory, and the two top-level prefixes stay
          # disjoint the way `public-outputs`' unamended requirement already
          # holds.
          publicLeafIsAFile = !(lib.hasSuffix "/value" withVault.placements.alice.wg-public.public);
          prefixesStayDisjoint =
            !(lib.hasPrefix "secrets/" withVault.placements.alice.wg-public.public)
            && !(lib.hasPrefix "public/" withVault.placements.alice.solo-token.file);

          # 12.4 — a vault does not move the committed policy: the text
          # `.sops.yaml` is rendered from is identical whether or not a
          # vault is declared, and the vault-mode rendering is a separate
          # attribute entirely.
          committedPolicyUnmoved = withVault.policyText == noVault.policyText;
          noVaultHasNoRulesText = noVault.vaultCreationRulesText;

          # 5.2/5.3/12.5 — the rendered rules: no header, no `keys:` block,
          # every `path_regex` a literal anchored opaque filename matching
          # `secretsFileOf` bit for bit, no wildcard character among them.
          rules = {
            noHeader = !(lib.hasInfix "generated by safix" withVault.vaultCreationRulesText);
            noKeysBlock = !(lib.hasInfix "\nkeys:\n" withVault.vaultCreationRulesText);
            noAudienceComment = !(lib.hasInfix "Audience:" withVault.vaultCreationRulesText);
            regexesMatchFormula = lib.sort (a: b: a < b) renderedRegexes == expectedRegexes;
            regexesAreLiteral = !(lib.any (r: lib.hasInfix "[^/]" r) renderedRegexes);
            oneRegexPerFile = builtins.length renderedRegexes == builtins.length (lib.unique renderedRegexes);
          };
        };
        expected = {
          readableMatchesFormula = {
            soloFile = true;
            teamFile = true;
            wgPublicValue = true;
            noneOpaque = {
              definitionRecord = null;
              logicalFile = null;
              logicalKey = null;
              logicalPublic = null;
            };
          };
          rootFlip = {
            noVault = true;
            withVault = true;
          };
          vaultDeclared = {
            noVault = false;
            withVault = true;
          };
          namingKeyRefusals = {
            noKey = true;
            shortKey = true;
            nonHexKey = true;
            wellFormedAccepted = [ ];
          };
          opaque = lib.genAttrs names (_name: {
            file = true;
            key = true;
            definitionRecord = true;
            logicalFile = true;
            logicalKey = true;
          });
          publicOpaque = true;
          publicLogical = true;
          noPublicStaysNull = {
            public = null;
            logicalPublic = null;
          };
          leakedFragments = [ ];
          machineKeyAgreement = true;
          consumptionResolves = {
            userScopeHasOpaqueKey = true;
            systemScopeHasOpaqueKey = true;
          };
          publicLeafIsAFile = true;
          prefixesStayDisjoint = true;
          committedPolicyUnmoved = true;
          noVaultHasNoRulesText = null;
          rules = {
            noHeader = true;
            noKeysBlock = true;
            noAudienceComment = true;
            regexesMatchFormula = true;
            regexesAreLiteral = true;
            oneRegexPerFile = true;
          };
        };
      };
    in
    {
      checks.safix-vault =
        pkgs.runCommand "safix-vault-suite"
          {
            meta.description = "structural check: safix-vault";
          }
          ''
            : ${structural}
            : ${vaultRulesParse}
            touch $out
          '';
    };
}
