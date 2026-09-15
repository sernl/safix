# Holds the three configurable storage roots: the `flake.safix.storage`
# option, its two evaluation refusals, and the claim that every readable path
# safix places is derived from them rather than from a literal.
#
# ── one fleet, two spellings ──
# One fleet — alice, bob, and a wireguard generator whose public half is
# declared `secret = false` — is resolved twice: `defaults`, which sets no
# `storage` at all, and `renamed`, which sets all three roots to spellings
# sharing no component with the defaults (`cipher`, `clear`, `bookkeeping`).
# Every claim below is either "the default resolution is byte-identical to the
# literal spellings this change replaced" or "the renamed resolution moved,
# tree by tree". The two together are what make "derived" a fact: a path that
# is still a literal fails the second, and a derivation that changed the
# default layout fails the first.
#
# `./storage-fixture` carries one generated public value under each spelling,
# with distinct contents, because `publicValueIn` is the one accessor whose
# following of a root is observable only by reading a different file.
#
# ── severity, proven by perturbation ──
# Each drill below is executed rather than asserted in prose: where a task
# predicts that a weakened implementation turns a claim green, the weakened
# implementation is written out beside the real one and the difference is the
# assertion.
#
# 1.15: `storageOverlap` without its `"${a}/"` suffix — `overlapAsRawPrefix`
# below — answers `true` for `secrets/safix` against `secrets/safix-public`,
# so it refuses a legal configuration; and without its `a == b` clause —
# `overlapWithoutEquality` — it answers `false` for two identical roots, so it
# accepts the one configuration that is certainly wrong. Both are asserted
# against the real function's answers, so a future edit that drops either
# clause reddens `overlapClausesAreLoadBearing`.
# 1.16: that the refusals reach evaluation rather than only existing as a
# function is held by reading `.violations` off a real `evalModules` of the
# consumer's own module — `refusals` and `overlapRefusals` below — rather than
# by calling `resolve.storageViolations` directly. Removing the concatenation
# in `default.nix` turns every one of those rows green; verified by dropping
# `++ resolve.storageViolations cfg.storage` against a scratch copy and
# observing all twelve malformed-root rows and all three overlap rows go
# silent at once, while `nonComponentPrefixAccepted` stayed `[ ]` — a check
# that reported nothing either way.
# 2.8: the ciphertext half and the public half of the renamed resolution are
# separate rows (`renamedCiphertext` and `renamedPlaintext`), so reverting
# `audienceFileOf` to a literal while leaving `publicFileOf` derived reddens
# exactly the first. Verified by reverting `audienceFileOf` alone against a
# scratch copy: `renamedCiphertext`, `renamedPolicy` and `renamedGoverned`
# went red and `renamedPlaintext` stayed green.
# 3.13: `definitionRecord` is emitted on every placement and non-null in
# readable mode (`recordsAreEmitted`). Verified by making it conditional
# again: `noneNull` went `false` and all eight record values under both roots
# went `null`, which is a `safix check` with no record path to read at all.
# 5.7: the header's two worked examples are two rows
# (`renamedPolicyHeader.sharedExample` and `.anchoringExample`), so reverting
# one paragraph reddens one row. Verified by reverting the shared-audience
# paragraph alone: `sharedExample` went `false` and `anchoringExample` stayed
# `true` (`noDefaultSpelling` went `false` too, which is the same fault seen
# from the other side).
# 6.7 (the most dangerous line in this change): `catchAllProbesAreDerived`
# runs the drill rather than describing it. A rule reaching the *renamed*
# encrypted tree is planted, and the literal probe set — which is exactly what
# this check would have had if group 6 had been skipped — is asked about it
# alongside the derived one. The literal set answers "no catch-all" and the
# derived set answers "catch-all", which is the silent-loss transition the
# task requires to be reproducible.
# 6.8: `unclaimedComponentIsLoadBearing` substitutes a really declarable name
# for `UNCLAIMED` in the derived probes and observes the legitimate fixture's
# own rules start matching them, which is why the excluded-alphabet component
# is not decoration.
# The stamp record is emitted beside the definition record under both roots
# (`stampsAreEmitted`), and `besideTheDefinitionRecord` is what holds the two
# to one derivation: a stamp path spelled from anything but the record path
# plus `.stamps` reddens that row alone.
{ lib, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      policy = import ../safix/policy.nix { inherit lib; };
      safixChecks = import ../safix/checks.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      aliceKey = "age1fixturestorageaaa000000000000000000000000000000000000000000";
      bobKey = "age1fixturestoragebbb000000000000000000000000000000000000000000";

      # The tree `publicValue` reads at evaluation, carrying one generated
      # value under each spelling so that "the read follows the root" is a
      # different file's contents rather than a different string.
      fixtureRoot = ./storage-fixture;

      renamedStorage = {
        encrypted = "cipher";
        plaintextOutputs = "clear";
        generatorRecords = "bookkeeping";
      };

      fleet = {
        users = {
          alice = {
            recipient = aliceKey;
            private = {
              solo-token = { };
              team-secret = { };
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
              wg-public.mode = "0444";
            };
            sharedWith.bob.team-secret = { };
          };
          bob.recipient = bobKey;
        };
      };

      projectionOf =
        extra:
        (lib.evalModules {
          modules = [
            ../safix
            { _module.args.self = fixtureRoot; }
            { flake.safix = fleet // extra; }
          ];
        }).config.flake.safix.lib;

      defaults = projectionOf { };
      renamed = projectionOf { storage = renamedStorage; };

      # `resolveSet` returns the `sopsFile` a consuming profile hands the
      # secret provisioner, which is `root` joined with the derived path —
      # unlike the root-independent `placements.*.file`. Reported as the tail
      # below the fixture root rather than in full, because the full value
      # carries this flake's own source store path and every row of this check
      # is serialized into its derivation: embedding it would scope cache
      # invalidation to "did any tracked file change" rather than to "did the
      # assertion target change", which is exactly what `mk-structural-check`
      # exists to avoid. `sopsFileIsRooted` holds the half this drops.
      sopsFileOf =
        projection: name:
        lib.removePrefix "${toString fixtureRoot}/" (
          toString
            (projection.resolveSet {
              user = "alice";
              machine = null;
              hostname = "h";
              tags = [ ];
            }).${name}.sopsFile
        );

      sopsFileIsRooted =
        projection: name:
        lib.hasPrefix "${toString fixtureRoot}/" (
          toString
            (projection.resolveSet {
              user = "alice";
              machine = null;
              hostname = "h";
              tags = [ ];
            }).${name}.sopsFile
        );

      regexesOf = projection: lib.sort (a: b: a < b) (map (r: r.pathRegex) projection.policyPlan.rules);

      recordsOf = projection: lib.mapAttrs (_name: p: p.definitionRecord) projection.placements.alice;

      stampsOf = projection: lib.mapAttrs (_name: p: p.stampRecord) projection.placements.alice;

      # ── the two refusals ──
      malformed = {
        empty = "";
        absolute = "/etc/safix";
        dotdot = "secrets/../safix";
        trailingSlash = "secrets/safix/";
      };

      rootNames = [
        "encrypted"
        "plaintextOutputs"
        "generatorRecords"
      ];

      namesOption =
        projection: option:
        lib.any (m: lib.hasPrefix "flake.safix.storage.${option} is " m) projection.violations;

      # option -> case -> whether evaluation refused, naming that option.
      refusals = lib.genAttrs rootNames (
        option:
        lib.mapAttrs (
          _case: value: namesOption (projectionOf { storage.${option} = value; }) option
        ) malformed
      );

      # A root that is well formed in every clause raises nothing at all.
      wellFormedAccepted = (projectionOf { storage.encrypted = "vault/documents"; }).violations;

      overlapping = {
        equalRoots = {
          encrypted = "shared/tree";
          plaintextOutputs = "shared/tree";
        };
        componentPrefix = {
          encrypted = "safix";
          plaintextOutputs = "safix/public";
        };
        nestedRecords = {
          encrypted = "secrets/safix";
          generatorRecords = "secrets/safix/state";
        };
      };

      overlapMessagesOf =
        storage: lib.filter (m: lib.hasInfix "overlap" m) (projectionOf { inherit storage; }).violations;

      overlapRefusals = lib.mapAttrs (_case: storage: overlapMessagesOf storage) overlapping;

      # The accept case: one root's string is a prefix of another's without
      # being a prefix at a component boundary. Neither directory contains the
      # other, so neither a rule nor an exclusion scoped to one reaches the
      # other, and refusing it would turn away a legal configuration.
      nonComponentPrefixAccepted =
        (projectionOf {
          storage = {
            encrypted = "secrets/safix";
            plaintextOutputs = "secrets/safix-public";
          };
        }).violations;

      # 1.15's two weakened comparisons, written out so the drill is an
      # assertion rather than a sentence.
      overlapAsRawPrefix = a: b: a == b || lib.hasPrefix a b || lib.hasPrefix b a;
      overlapWithoutEquality = a: b: lib.hasPrefix "${a}/" b || lib.hasPrefix "${b}/" a;

      # ── the derived catch-all probes ──
      matches = r: p: builtins.match r p != null;

      # A rule reaching the renamed encrypted tree, planted the way
      # `generators.nix`'s reaching-rule fixture plants one: a plan is a plain
      # value, so a hand-written rule exercises the checking function against
      # something no declaration would produce.
      reachingPlan = {
        anchors = [ ];
        rules = [
          {
            pathRegex = "^${renamedStorage.encrypted}/.*";
            audience = [ "alice" ];
            anchors = [ "alice-safix" ];
          }
        ];
      };

      # 6.8: the same probes with a name a declaration could really occupy.
      declarableProbes = map (lib.replaceStrings [ "UNCLAIMED" ] [ "alice" ]) (
        safixChecks.catchAllProbesOf renamedStorage
      );
    in
    {
      checks.safix-storage = mkStructuralCheck {
        name = "safix-storage";
        actual = {
          # An emptied fixture would let every claim below pass by having
          # nothing to judge.
          fixtureRoster = lib.sort (a: b: a < b) (builtins.attrNames defaults.placements.alice);

          # ── 1.12: the defaults preserve the layout exactly ──
          # Every surface the three literals used to spell, asserted against
          # the literal spelling rather than against the option's own value,
          # so a change to the default is a red row here rather than a silent
          # agreement between two copies of the same mistake.
          defaultLayout = {
            sopsFile = sopsFileOf defaults "solo-token";
            sharedSopsFile = sopsFileOf defaults "team-secret";
            sopsFileIsRooted = sopsFileIsRooted defaults "solo-token";
            outputPath = defaults.outputPath "alice" "wg-public";
            publicValue = defaults.publicValue "alice" "wg-public";
            publicPaths = defaults.publicPaths;
            definitionRecords = recordsOf defaults;
            pathRegexes = regexesOf defaults;
            governedFiles = defaults.governedFiles.required;
          };

          # ── 1.13: the four malformed-root refusals, per option ──
          refusals = refusals;
          wellFormedAccepted = wellFormedAccepted;

          # ── 1.14: the overlap refusals, and the accept case ──
          # Each message names both options and both values, because a
          # refusal that named one of them would leave the operator reading
          # two declarations to find out which pair it meant.
          overlapRefusals = lib.mapAttrs (_case: messages: {
            refused = messages != [ ];
            namesBothOptions = lib.all (
              m:
              lib.hasInfix "flake.safix.storage." m
              && builtins.length (lib.splitString "flake.safix.storage." m) == 3
            ) messages;
          }) overlapRefusals;
          nonComponentPrefixAccepted = nonComponentPrefixAccepted;

          # ── 1.15: both clauses of `storageOverlap` are load-bearing ──
          overlapClausesAreLoadBearing = {
            realAcceptsNonComponentPrefix = resolve.storageOverlap "secrets/safix" "secrets/safix-public";
            rawPrefixWouldRefuseIt = overlapAsRawPrefix "secrets/safix" "secrets/safix-public";
            realRefusesEqualRoots = resolve.storageOverlap "secrets/safix" "secrets/safix";
            withoutEqualityWouldAcceptIt = overlapWithoutEquality "secrets/safix" "secrets/safix";
            realRefusesNesting = resolve.storageOverlap "secrets/safix" "secrets/safix/pub";
          };

          # ── 2.1/2.6/2.7/9.3: every readable path follows a rename ──
          # Split by tree so that a site left literal reddens exactly its own
          # row (drill 2.8).
          renamedCiphertext = {
            sopsFile = sopsFileOf renamed "solo-token";
            sharedSopsFile = sopsFileOf renamed "team-secret";
            sopsFileIsRooted = sopsFileIsRooted renamed "solo-token";
            audiences = lib.sort (a: b: a < b) (builtins.attrNames renamed.audiences);
            dirs = lib.sort (a: b: a < b) (map (a: a.dir) (lib.attrValues renamed.audiences));
          };

          # `outputPathIn`, `publicValueIn` and `publicPathsIn` consume
          # `placement.public`/`placement.file` and needed no edit; this row
          # is what makes that "derived" rather than "assumed". `publicValue`
          # reads a different file's bytes under the renamed root, which is
          # the only way the accessor's following of the root is observable.
          renamedPlaintext = {
            outputPath = renamed.outputPath "alice" "wg-public";
            publicValue = renamed.publicValue "alice" "wg-public";
            publicPaths = renamed.publicPaths;
          };

          # `policy.nix:206`'s `pathRegex` derives from `audiences.<file>.dir`,
          # so every readable-mode rule follows a rename with no edit.
          renamedPolicy = regexesOf renamed;

          # `governedFiles` is `extra ∪ required` with `required` the audience
          # file names, so it follows a rename with no edit either.
          renamedGoverned = renamed.governedFiles;

          # ── 3.12: `definitionRecord` on every placement ──
          recordsAreEmitted = {
            noneNull = lib.all (p: p.definitionRecord != null) (lib.attrValues defaults.placements.alice);
            underDefaultRoot = recordsOf defaults;
            underRenamedRoot = recordsOf renamed;
          };

          # ── the stamp record is emitted beside the definition record ──
          # Under both roots, and named by appending `.stamps` to the record
          # path rather than by a second derivation of the layout: the third
          # row is what would redden if the two ever parted company.
          stampsAreEmitted = {
            noneNull = lib.all (p: p.stampRecord != null) (lib.attrValues defaults.placements.alice);
            underDefaultRoot = stampsOf defaults;
            underRenamedRoot = stampsOf renamed;
            besideTheDefinitionRecord = lib.all (
              name: (stampsOf renamed).${name} == "${(recordsOf renamed).${name}}.stamps"
            ) (builtins.attrNames renamed.placements.alice);
          };

          # ── 5.6: the committed header names the configured root ──
          renamedPolicyHeader = {
            sharedExample = lib.hasInfix "${renamedStorage.encrypted}/shared/<a>,<b>/" renamed.policyText;
            anchoringExample =
              lib.hasInfix "nested/${renamedStorage.encrypted}/users/alice/x.yaml" renamed.policyText
              && lib.hasInfix "`${renamedStorage.encrypted}/users/alice/`. `[^/]*`" renamed.policyText;
            noDefaultSpelling = !(lib.hasInfix "secrets/safix" renamed.policyText);
          };

          # ── 6.1/6.7: the catch-all probes are derived ──
          # The literal probe set is what this check would carry had group 6
          # been skipped. It answers "green" to a rule reaching the renamed
          # encrypted tree, which is the silent loss design S10 names; the
          # derived set answers "red".
          catchAllProbesAreDerived = {
            literalProbesStayGreen = safixChecks.catchAllMessagesOf resolve.defaultStorage reachingPlan == [ ];
            derivedProbesGoRed = safixChecks.catchAllMessagesOf renamedStorage reachingPlan != [ ];
            legitimateFixtureStaysGreen =
              safixChecks.catchAllMessagesOf renamedStorage renamed.policyPlan == [ ];
          };

          # ── 6.8: the `UNCLAIMED` component is load-bearing ──
          # Substituting a really declarable name makes the fixture's own
          # rules match a probe, so a probe set without it would report a
          # catch-all on a legitimate fleet.
          unclaimedComponentIsLoadBearing = {
            everyTreeProbeIsUnclaimable = lib.all (p: lib.hasInfix "UNCLAIMED" p) (
              lib.filter (
                p:
                lib.hasPrefix renamedStorage.encrypted p
                || lib.hasPrefix renamedStorage.plaintextOutputs p
                || lib.hasPrefix renamedStorage.generatorRecords p
              ) (safixChecks.catchAllProbesOf renamedStorage)
            );
            declarableProbesWouldMatch = lib.any (
              p: lib.any (r: matches r.pathRegex p) renamed.policyPlan.rules
            ) declarableProbes;
          };

          # ── 4.8/6.6: `safix-public-no-rule` follows a renamed root ──
          # Fed `resolve.publicPathsOf`, so it needed no edit; a rule planted
          # at the renamed plaintext tree has to be reported, or the deletion
          # of Rust's own prefix constants rests on nothing.
          publicRuleFollowsRename = {
            legitimateFixtureStaysGreen =
              safixChecks.publicRuleMessagesOf {
                plan = renamed.policyPlan;
                inherit (renamed) publicPaths;
              } == [ ];
            reachingRuleIsReported =
              safixChecks.publicRuleMessagesOf {
                plan = {
                  anchors = [ ];
                  rules = [
                    {
                      pathRegex = "^${renamedStorage.plaintextOutputs}/.*";
                      audience = [ "alice" ];
                      anchors = [ "alice-safix" ];
                    }
                  ];
                };
                publicPaths = renamed.publicPaths;
              } != [ ];
          };
        };

        expected = {
          fixtureRoster = [
            "solo-token"
            "team-secret"
            "wg-private"
            "wg-public"
          ];

          defaultLayout = {
            sopsFile = "secrets/safix/users/alice/secrets.yaml";
            sharedSopsFile = "secrets/safix/shared/alice,bob/secrets.yaml";
            sopsFileIsRooted = true;
            outputPath = "public/safix/users/alice/wg-public/value";
            publicValue = "default-root\n";
            publicPaths = [ "public/safix/users/alice/wg-public/value" ];
            definitionRecords = {
              solo-token = "state/safix/definitions/alice/solo-token";
              team-secret = "state/safix/definitions/alice/team-secret";
              wg-private = "state/safix/definitions/alice/wg-private";
              wg-public = "state/safix/definitions/alice/wg-public";
            };
            pathRegexes = [
              "^secrets/safix/shared/alice,bob/[^/]*\\.yaml$"
              "^secrets/safix/users/alice/[^/]*\\.yaml$"
            ];
            governedFiles = [
              "secrets/safix/shared/alice,bob/secrets.yaml"
              "secrets/safix/users/alice/secrets.yaml"
            ];
          };

          refusals = lib.genAttrs rootNames (_option: lib.mapAttrs (_case: _value: true) malformed);
          wellFormedAccepted = [ ];

          overlapRefusals = lib.mapAttrs (_case: _storage: {
            refused = true;
            namesBothOptions = true;
          }) overlapping;
          nonComponentPrefixAccepted = [ ];

          overlapClausesAreLoadBearing = {
            realAcceptsNonComponentPrefix = false;
            rawPrefixWouldRefuseIt = true;
            realRefusesEqualRoots = true;
            withoutEqualityWouldAcceptIt = false;
            realRefusesNesting = true;
          };

          renamedCiphertext = {
            sopsFile = "cipher/users/alice/secrets.yaml";
            sharedSopsFile = "cipher/shared/alice,bob/secrets.yaml";
            sopsFileIsRooted = true;
            audiences = [
              "cipher/shared/alice,bob/secrets.yaml"
              "cipher/users/alice/secrets.yaml"
            ];
            dirs = [
              "cipher/shared/alice,bob"
              "cipher/users/alice"
            ];
          };

          renamedPlaintext = {
            outputPath = "clear/users/alice/wg-public/value";
            publicValue = "renamed-root\n";
            publicPaths = [ "clear/users/alice/wg-public/value" ];
          };

          renamedPolicy = [
            "^cipher/shared/alice,bob/[^/]*\\.yaml$"
            "^cipher/users/alice/[^/]*\\.yaml$"
          ];

          renamedGoverned = {
            extra = [ ];
            managed = [
              "cipher/shared/alice,bob/secrets.yaml"
              "cipher/users/alice/secrets.yaml"
            ];
            required = [
              "cipher/shared/alice,bob/secrets.yaml"
              "cipher/users/alice/secrets.yaml"
            ];
          };

          recordsAreEmitted = {
            noneNull = true;
            underDefaultRoot = {
              solo-token = "state/safix/definitions/alice/solo-token";
              team-secret = "state/safix/definitions/alice/team-secret";
              wg-private = "state/safix/definitions/alice/wg-private";
              wg-public = "state/safix/definitions/alice/wg-public";
            };
            underRenamedRoot = {
              solo-token = "bookkeeping/alice/solo-token";
              team-secret = "bookkeeping/alice/team-secret";
              wg-private = "bookkeeping/alice/wg-private";
              wg-public = "bookkeeping/alice/wg-public";
            };
          };

          stampsAreEmitted = {
            noneNull = true;
            underDefaultRoot = {
              solo-token = "state/safix/definitions/alice/solo-token.stamps";
              team-secret = "state/safix/definitions/alice/team-secret.stamps";
              wg-private = "state/safix/definitions/alice/wg-private.stamps";
              wg-public = "state/safix/definitions/alice/wg-public.stamps";
            };
            underRenamedRoot = {
              solo-token = "bookkeeping/alice/solo-token.stamps";
              team-secret = "bookkeeping/alice/team-secret.stamps";
              wg-private = "bookkeeping/alice/wg-private.stamps";
              wg-public = "bookkeeping/alice/wg-public.stamps";
            };
            besideTheDefinitionRecord = true;
          };

          renamedPolicyHeader = {
            sharedExample = true;
            anchoringExample = true;
            noDefaultSpelling = true;
          };

          catchAllProbesAreDerived = {
            literalProbesStayGreen = true;
            derivedProbesGoRed = true;
            legitimateFixtureStaysGreen = true;
          };

          unclaimedComponentIsLoadBearing = {
            everyTreeProbeIsUnclaimable = true;
            declarableProbesWouldMatch = true;
          };

          publicRuleFollowsRename = {
            legitimateFixtureStaysGreen = true;
            reachingRuleIsReported = true;
          };
        };
      };
    };
}
