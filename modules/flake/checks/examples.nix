# Holds ../../../examples/'s two declaration-side consumers of the identical
# fleet: `plain-nix` through `lib.mkVault` with no flake-parts and no flake,
# `dendritic` through `flakeModules.default` with one declaration per file. An
# example nobody evaluates is documentation that rots, so this check reaches
# every file under both.
#
# The third example directory, `examples/profiles/`, is not read here.
# What a profile resolves is a materialization under a scope, which none of the
# fields compared below carries, and evaluating one needs a host module system
# this check's sandbox deliberately does not hold — `./examples-profiles.nix`
# is that evaluation.
#
# ── how each half is read ──
# `examples/plain-nix/entry.nix` is executed for real, as written: a sandboxed
# derivation reproduces `examples/plain-nix/`, the top-level `lib/` directory
# `entry.nix` imports, and the `modules/flake/safix` resolver module
# `lib/default.nix` imports in turn, at their real relative paths, then runs
# `nix eval --file examples/plain-nix/entry.nix <attr> --json` for each
# `safix.lib.*`/`safix.*` attribute spelling, with
# `NIX_PATH=nixpkgs=${pkgs.path}` supplying the `<nixpkgs>` `entry.nix` itself
# resolves `lib` from — the same source `NIX_PATH` supplies at a real
# `--entry` invocation. `entry.nix` no longer self-references this
# repository's own flake through `builtins.getFlake`, so the obstacle that
# once stood between this check and the file as written — resolving that
# self-reference means re-evaluating this flake's own transitive input
# closure a second time, which a network-less build sandbox can do for
# neither the network fetch nor the private store registration that would
# take — no longer exists, now that `mkVault` is a plain function of `{ lib
# }` rather than a flake output.
#
# `examples/dendritic`'s declarations are read by directory rather than by a
# count, through `lib.evalModules` directly: the same mechanism flake-parts'
# own `imports` is sugar over (design decision D3), not
# `examples/dendritic/flake.nix` evaluated as a flake, which would need a
# self-reference of its own. The example's own `flake.nix` reads the same
# directory the same way, and `noHandList` below is what holds that.
#
# ── what is compared, and what a comparison cannot hold ──
# Every attribute `crates/safix-core/src/nix.rs`'s `Attribute` enum names and
# that serializes. The criterion is serializability rather than a chosen
# number, so a new runtime-read attribute is compared the day it is added.
#
# A field-for-field diff holds that the two consumers agree, which is a
# different proposition from coverage: deleting one feature from *both*
# examples keeps the diff green. `coverageProbes` is the second half, one
# literal row per feature family asserted against the dendritic projection.
#
# ── severity: nine drills, all observed ──
# Changing one field in the dendritic fleet without changing the plain-nix one
# fails `safix-examples` on exactly that field, which is the evidence the two
# are compared rather than merely both evaluated.
# Deleting `examples/dendritic/modules/hooks.nix` fails the hook rows: the
# expected hooks come from the dendritic evaluation and the actual ones from
# the executed `entry.nix`, so the two sides are compared. Pointing
# `dendriticHooks` back at `plain-nix/hooks.nix` makes that deletion green
# again, which is the pre-change condition and what `hooksAreNotEmpty` exists
# beside.
# Widening `elide` to replace every `/nix/store/…` path makes a deliberately
# divergent `bridge.clanFlake` subdirectory in one example stop being
# detected; the narrow form detects it.
# Deleting `sharedWith.acme.escrow-note` from both examples at once leaves the
# field-for-field diff green and reddens `coverageProbes`, which is the whole
# reason that row exists.
# Putting a `./modules/one-file.nix` path back into
# `examples/dendritic/flake.nix` fails `noHandList`, naming the line; adding a
# declaration file under `modules/` with no other edit is seen by both the
# example's own evaluation and this check.
# Changing the keepassxc mapping's `kdbx.path` in the dendritic example alone
# fails on that field of `keepassxc` and nothing else; deleting its whole
# `fields` block reddens `coverageProbes.keepassxcFields` instead, because the
# far-side fields are what that row reads.
# Declaring `fields.tags` on the keepassxc mapping does not redden anything
# here, and that is where the refusal lives rather than a gap: the message
# comes from `../safix/keepassxc.nix`'s own violation list, which
# `flake.safix.lib.violations` does not fold in, so `safix-keepassxc` is what
# holds it. Observed by evaluating that list over this fleet with the field
# added, which names the mapping and the field.
# The same two shapes hold for the `pass` mapping, which is the target the
# drill was written against: changing its `pass.path` in the dendritic example
# alone fails on that one field of `pass` and nothing else, and deleting its
# whole `fields` block reddens `coverageProbes.passFields` instead.
# Declaring `fields.tags` on the bitwarden mapping does not redden anything
# here either, for the reason the keepassxc row gives: evaluating
# `../safix/bitwarden.nix`'s own violation list over this fleet returns nothing
# as declared and, with the field added, the one sentence naming the mapping
# and the field, which `safix-bitwarden` is what holds.
#
# The coverage rows are a build input of the entry diff, so a coverage failure
# arrives first and the diff does not run. Either half red is the check red;
# what it means for a drill is that the two halves report in that order.
{ lib, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      examplesRoot = ../../../examples;
      plainNixRoot = examplesRoot + "/plain-nix";
      dendriticRoot = examplesRoot + "/dendritic";
      libRoot = ../../../lib;
      safixModule = ../safix;

      # Every `flake.safix.lib.*` attribute `crates/safix-core/src/nix.rs`'s
      # `Attribute` enum names, minus the two hooks, which are siblings of
      # `.lib` rather than fields inside it and are added to both operands
      # below.
      #
      # The criterion is serializability. `mkChecks`, `resolveSet`,
      # `resolveNames`, `materialize`, `publicValue`, `outputPath`,
      # `mkDriftCheck` and `policyPlan` are excluded because each is a function
      # or outside the runtime's own contract, and no function is
      # JSON-serializable — so a field's absence here is that exclusion and
      # never a choice about how much to compare.
      comparedFields = proj: {
        inherit (proj)
          placements
          audiences
          governedFiles
          recipients
          delegation
          policyText
          generatorPlan
          nameRegex
          bridge
          keepassxc
          pass
          bitwarden
          onepassword
          subjects
          vaultDeclared
          vaultCreationRulesText
          ;
      };

      dendriticModules = builtins.filter (p: lib.hasSuffix ".nix" (toString p)) (
        lib.filesystem.listFilesRecursive (dendriticRoot + "/modules")
      );

      # One evaluation, read twice: the projection and the two hooks come out of
      # the same `flake.safix`, so the expected side of every compared value is
      # this consumer's own declaration.
      dendriticSafix =
        (lib.evalModules {
          modules = [
            ../safix
            { _module.args.self = dendriticRoot; }
          ]
          ++ dendriticModules;
        }).config.flake.safix;

      dendriticVault = dendriticSafix.lib;
      dendriticHooks = {
        inherit (dendriticSafix) onboardingHook enrollHook;
      };

      # `safix.lib.<name>` for the data fields, `safix.<name>` for the two
      # hooks — every spelling `crates/safix-core/src/nix.rs`'s `Attribute` enum
      # names, and the same set `--entry` and a flake target both resolve
      # identically (README, "Fitting safix to a tree you already have").
      entryAttrs =
        (map (name: "safix.lib." + name) (builtins.attrNames (comparedFields dendriticVault)))
        ++ [
          "safix.onboardingHook"
          "safix.enrollHook"
        ];

      entryExpected = comparedFields dendriticVault // dendriticHooks;
      entryExpectedFile = pkgs.writeText "safix-examples-entry-expected.json" (
        builtins.toJSON entryExpected
      );

      # `_module.args.self` differs between the two halves by construction —
      # the dendritic store path here, `root = ./.` inside the sandbox there —
      # and `bridge.clanFlake` is a `lib.types.path` that reaches the projection
      # as a string, so a path each example declares relative to its own root
      # stringifies to two different absolute strings. The elision replaces a
      # leading example root with one marker and compares the rest, so a
      # divergence in the tail — one example naming a subdirectory the other
      # does not — still fails, and the field stays in the comparison rather
      # than being dropped out of it.
      #
      # It replaces those two prefixes and nothing else. A store path appearing
      # where one is not expected is itself a finding, and a general store-path
      # elision would hide it.
      #
      # Written as a jq filter rather than as a nix function because the
      # executed side exists only as JSON inside the build: one definition,
      # applied to both operands.
      elide = ''
        elide() {
          jq -S --arg dendritic "${toString dendriticRoot}" --arg plain "$repo/examples/plain-nix" '
            def elideRoots:
              walk(
                if type == "string" then
                  if startswith($dendritic) then "<example-root>" + ltrimstr($dendritic)
                  elif startswith($plain) then "<example-root>" + ltrimstr($plain)
                  else . end
                else . end
              );
            elideRoots
          ' "$@"
        }
      '';

      # The example's own entry point must name no path under its module
      # directory: a hand list beside a discovered one drifts on the next added
      # file, and the drift is silent because nothing evaluates that flake.
      # Asserting the absence of a list rather than its equality with the
      # discovered set is deliberate — comparing them means extracting an
      # import list from nix source, which is either a parser with one caller
      # or a text match that breaks when the formatter moves a path onto
      # another line.
      handListLines = lib.filter (line: lib.hasInfix "./modules/" line) (
        lib.splitString "\n" (builtins.readFile (dendriticRoot + "/flake.nix"))
      );

      placements = dendriticVault.placements;
      audiences = builtins.attrValues dendriticVault.audiences;
      generators = name: placements.alice.${name}.generator;
      audienceCarries = subject: builtins.any (a: builtins.elem subject a.audience) audiences;

      # One row per feature family. Each is a value the feature produces rather
      # than a restatement of the declaration, so a probe cannot be satisfied by
      # the declaration being present and inert.
      #
      # Four features of the declaration surface are absent here because no
      # compared field carries them: an entry's `mode`, an entry's `path`, a
      # `perHost.<h>.add` override and a `perTag` `force`. All four are
      # materializations under a scope, and `./examples-profiles.nix`'s
      # `homeSecrets` and `entryPath` rows are where they are held.
      probes = {
        hooksAreNotEmpty = {
          # Null-guarded, so a dendritic side that declares no hook at all
          # fails as a false row rather than as a coercion error naming
          # nothing.
          onboardingCarriesFixtureText =
            dendriticHooks.onboardingHook != null
            && lib.hasInfix "onboarded %s (%s)" dendriticHooks.onboardingHook;
          enrollHookIsNull = dendriticHooks.enrollHook == null;
        };

        noHandList = {
          offendingLines = handListLines;
        };

        coverageProbes = {
          organizationAudience = audienceCarries "=acme";
          ownerOfAudience = audienceCarries "@~rack";
          organizationGrantFile = placements.alice.escrow-note.file;
          ownerOfGrantFile = placements.alice.corp-handover.file;
          recoveryRecipient = builtins.elem "age1examplemaster00000000000000000000000000000000000000000000" dendriticVault.recipients.alice;
          extraGovernedFile = dendriticVault.governedFiles.extra;
          delegationManagers = dendriticVault.delegation.managers.acme;
          delegationManagedBy = dendriticVault.delegation.managedBy.bob;
          generatorPrompt = dendriticVault.generatorPlan.alice.inputs.prompted-token.passphrase;
          generatorDependencyEdge = dendriticVault.generatorPlan.alice.inputs.derived-token.generated-token;
          generatorMultiOutput = dendriticVault.generatorPlan.alice.outputs.wg-key;
          generatorValidation = (generators "validated-token").validation != null;
          generatorNetwork = (generators "fetched-token").network;
          publicOutputPath = placements.alice.wg-public.public;
          secretOutputIsNotPublic = placements.alice.wg-private.public == null;
          subjectMachines = builtins.attrNames dendriticVault.subjects.machines;
          subjectServices = builtins.attrNames dendriticVault.subjects.services;
          subjectGroups = builtins.attrNames dendriticVault.subjects.groups;
          nestedGroupMembers = dendriticVault.subjects.groups.infra.members;
          bridgeMappings = map (m: m.id) dendriticVault.bridge.mappings;
          keepassxcMappings = map (m: m.id) dendriticVault.keepassxc.mappings;
          keepassxcFields = (lib.head dendriticVault.keepassxc.mappings).kdbx.fields;
          passMappings = map (m: m.id) dendriticVault.pass.mappings;
          passFields = (lib.head dendriticVault.pass.mappings).pass.fields;
          bitwardenMappings = map (m: m.id) dendriticVault.bitwarden.mappings;
          bitwardenFields = (lib.head dendriticVault.bitwarden.mappings).bitwarden.fields;
          onepasswordMappings = map (m: m.id) dendriticVault.onepassword.mappings;
          onepasswordFields = (lib.head dendriticVault.onepassword.mappings).onepassword.fields;
        };
      };

      expected = {
        hooksAreNotEmpty = {
          onboardingCarriesFixtureText = true;
          enrollHookIsNull = true;
        };

        noHandList = {
          offendingLines = [ ];
        };

        coverageProbes = {
          organizationAudience = true;
          ownerOfAudience = true;
          organizationGrantFile = "secrets/safix/shared/=acme,alice/secrets.yaml";
          ownerOfGrantFile = "secrets/safix/shared/@~rack,alice/secrets.yaml";
          recoveryRecipient = true;
          extraGovernedFile = [ "secrets/safix/users/alice/legacy.yaml" ];
          delegationManagers = [ "alice" ];
          delegationManagedBy = "acme";
          generatorPrompt = {
            kind = "prompt";
            name = "passphrase";
          };
          generatorDependencyEdge = {
            kind = "dependency";
            name = "generated-token";
          };
          generatorMultiOutput = [
            "wg-key"
            "wg-private"
            "wg-public"
          ];
          generatorValidation = true;
          generatorNetwork = true;
          publicOutputPath = "public/safix/users/alice/wg-public/value";
          secretOutputIsNotPublic = true;
          subjectMachines = [
            "deck"
            "rack"
          ];
          subjectServices = [ "web" ];
          subjectGroups = [
            "contractors"
            "infra"
            "oncall"
          ];
          nestedGroupMembers = [
            "deck"
            "web"
            "oncall"
          ];
          bridgeMappings = [ "ntfy-token" ];
          keepassxcMappings = [ "grafana" ];
          keepassxcFields = {
            username = "alice@example.com";
            url = "https://grafana.example";
            notes = "example mapping — no real database";
            tags = [ ];
          };
          passMappings = [ "deploy" ];
          passFields = {
            username = {
              entry = "deploy-username";
            };
            url = "https://deploy.example";
            notes = "example mapping";
            tags = [ "example" ];
          };
          bitwardenMappings = [ "vpn" ];
          bitwardenFields = {
            username = "alice";
            url = "https://vpn.example";
            notes = "example mapping";
            tags = [ ];
          };
          onepasswordMappings = [ "registry" ];
          onepasswordFields = {
            username = {
              entry = "deploy-username";
            };
            url = "https://registry.example";
            notes = "example mapping";
            tags = [ "example" ];
          };
        };
      };

      coverage = mkStructuralCheck {
        name = "examples-coverage";
        actual = probes;
        expected = expected;
      };
    in
    {
      checks.safix-examples =
        pkgs.runCommand "safix-examples"
          {
            nativeBuildInputs = [
              pkgs.nix
              pkgs.jq
            ];
            env.NIX_CONFIG = "experimental-features = nix-command flakes";
            meta.description = "flakeless-entry check: safix-examples";
          }
          ''
            set -eu
            export HOME="$TMPDIR"
            export NIX_PATH="nixpkgs=${pkgs.path}"

            repo="$PWD/repo"
            mkdir -p "$repo/examples" "$repo/modules/flake"
            cp -r ${plainNixRoot} "$repo/examples/plain-nix"
            cp -r ${libRoot} "$repo/lib"
            cp -r ${safixModule} "$repo/modules/flake/safix"
            cd "$repo"

            ${elide}

            actual='{}'
            for attr in ${lib.concatStringsSep " " entryAttrs}; do
              key="''${attr##*.}"
              value="$(nix eval --file examples/plain-nix/entry.nix "$attr" --json)"
              actual="$(jq --argjson v "$value" ". + {\"$key\": \$v}" <<<"$actual")"
            done

            if ! diff -u <(elide ${entryExpectedFile}) <(elide <<<"$actual"); then
              echo ""
              echo "safix-examples: entry.nix, evaluated as written, diverges from the dendritic fixture"
              exit 1
            fi

            # The coverage and hand-list rows, built beside the diff so one
            # check name answers both halves: that the two consumers agree, and
            # that what they agree on is the whole declaration surface.
            test -e ${coverage}
            touch $out
          '';
    };
}
