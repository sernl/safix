# Holds the installer safix owns to the facts it is built on, before and beside
# the code that builds it. Every anchor below is a symbol name rather than a
# line, because line numbers drift and names survive.
#
# ── the entry type safix declares ──
# `safix-installer-type` reads `common.secretEntryType`'s own sub-options and
# holds the field-name set and every literal default against an accepted
# literal, so a field added, removed or re-defaulted is a failing check rather
# than a manifest that quietly grew or lost a key. Three defaults — `name`,
# `key` and `path` — are functions of the submodule's own configuration and so
# cannot be asked for a value outside an `attrsOf`; what is held for those is
# that each carries a default at all, and `path`'s value is held beside it over
# a two-entry fixture, one entry declaring a path and one not.
#
# ── the manifest safix writes ──
# `safix-installer-schema` builds `system.build.safix-manifest` over a fixture
# system configuration and diffs its whole structure — every key, every value —
# against a committed expected file, with the two values no snapshot can carry
# normalized rather than dropped: each `sopsFile` is reduced to its path below
# the store, and nothing else is touched. A field the nix half adds, removes or
# renames without the `serde` struct moving reddens this on the commit that
# makes the change; a field the `serde` struct gains without the nix half
# emitting it reddens `safix-installer-roundtrip` beside it, which is the other
# side of the same boundary.
#
# ── the manifest the program accepts, and the four it does not ──
# `safix-installer-roundtrip` runs the real `safix install` over the real
# manifest in two tiers. The schema tier accepts the built manifest and refuses
# four single-field mutations of it — an unknown schema version, a mode that is
# not octal, an unknown top-level field and an unknown entry field — each
# refusal read for the field it names. The document tier is a second fixture
# because it has to be: the fleet's sops files are paths into this flake that no
# committed file backs, so a document-mode run over the built manifest would
# refuse for want of a file rather than for want of a key. That tier encrypts a
# document in the sandbox, accepts a manifest whose key resolves in it, and
# refuses one whose key does not — which is the mutation the schema tier cannot
# express, and the evidence the two check modes are not interchangeable.
# `--ignore-passwd` is given throughout: a build sandbox has no users to
# resolve, and a mode that guessed them would be validating a resolution it
# cannot perform.
#
# ── the store and the entry default, one claim ──
# `safix-installer-store` reads the built manifest of the same fixture twice,
# once at the default roots and once with both moved through
# `safix.installer.*`, and holds one map per fixture: every entry that
# declared a path keeps it, and every entry that did not parks at
# `<symlinkPath>/<name>` under that fixture's own root. The map is one claim
# deliberately — the installer symlinks any entry path that is not
# `<symlinkPath>/<name>`, so a root moved without the entry default does not
# collide with a foreign store, it writes into it. The path-collision refusal
# is held beside it over the smallest fleet that can collide: two entries of
# one person declaring one path still refuse, and a minted default, being a
# function of the name, cannot.
#
# The expected side is read off `config.flake.safix.lib.materialize` rather
# than off the option the manifest is built from, which matters more now than
# it did: the mint lives in the entry type's own `path` default, so an
# expectation read off the type would be the mint compared with itself.
#
# ── the named entry, both mechanisms, and the consumer's ordering ──
# `safix-installer-ordering` evaluates four fixtures. One carries a foreign
# store's step under the shared `setupSecrets` name and names it through
# `safix.installer.afterActivation`: safix's step is its own node beside it,
# carries the name in its `deps`, and the foreign node's text never gains the
# installer call. One enables userborn, so the selection follows the host's
# own user-management options, and the unit carries the unit named by
# `safix.installer.afterUnits` in its `after`, plus the
# `sysinit-reactivation.target` wiring that re-runs it on a switch. The
# unadorned fixture registers the installer with no foreign dependency, so a
# host with no foreign store is a supported configuration. And one defines one
# activation step name twice, which is why safix registers a name of its own:
# `deps` is `listOf str` and `text` is `types.lines`, so two definitions become
# one node whose text carries both bodies and whose dependency set is the union
# — one node in the DAG, and a single node has no edge to state. The wrapper
# the step runs under carries an `ERR` trap and no `set -e`, which is why a
# failed half records a status and the other half still runs; that record is
# held here rather than asserted in prose.
#
# ── one installer ──
# `safix-installer-sole` holds, over the fixture whose resolution is the four
# entries `safix-consumption-system` also reads, that safix's is the only
# installer on the host and the only namespace it writes: a scan of every
# activation step's text and every unit's ExecStart finds exactly one
# invocation of an installer, safix's own, and the evaluated configuration
# carries no `sops` option tree at all, which is the mechanical form of "safix
# reads and defines no option outside its own namespace". Beside it, the two
# refusals the typed set does not carry are measured on fixtures with one
# injected entry each: a sops file outside the nix store and one that does not
# exist both refuse through the block in `modules/consume/installer.nix`, whose
# messages are declared in `common.nix` so this check can read them.
#
# ── the identity the system scope derives ──
# `safix-installer-identity` reads `ageSshKeyPaths` out of five built
# manifests rather than out of the option that fed them. A clan-shaped host
# whose keys lie under `/run/secrets` derives its ed25519 key and drops the
# rsa one; a host whose keys lie inside safix's own store derives nothing; a
# named identity survives both placements unchanged; and the switch turned
# off contributes nothing. The exclusion prefix is safix's own symlink path,
# because the catch-22 the exclusion avoids is decrypting with a key this
# installer itself deploys.
#
# ── the two refusals ──
# `safix-installer-refusals` holds both. The evaluation refusal is a throw
# while `safix.secrets` is forced, read off a configuration that resolves
# entries with nothing derivable and nothing named — nothing here forces the
# assertion collection, so what fires is safix's own message or nothing. The
# installer script's half is held as text off `system.build.safix-installer`
# without running it: it names every configured identity path, exits non-zero,
# names both ordering options as the remedy and the foreign store that has not
# run as the usual cause, and states the limit the user scope's preflight
# states — presence and readability were checked, decryption was not. The same
# text carries design I8's store-path discipline: the `sops` binary is named by
# store path and never taken from `PATH`, and the installer's own `PATH` is
# composed of the age plugins alone.
#
# ── the overrides, driven rather than declared ──
# `safix-installer-overrides` points `SAFIX_AGE_KEYGEN` and `SAFIX_SSH_TO_AGE`
# at scripts of the check's own and asserts each is invoked, which is what makes
# the override evidence rather than documentation.
#
# ── coexistence, against the binary ──
# `safix-installer-coexistence` runs the real `safix install` twice in the
# sandbox, in user mode, over ciphertext and an age identity generated there.
# Pointed at an ordinary directory holding a sentinel, the binary removes it —
# the destructive branch is measured, not assumed. Pointed at safix's own roots
# with the same foreign directory beside it, the sentinel survives, the foreign
# directory is byte-identical, the rest of the tree is unchanged by a
# before-and-after walk, and safix's own store holds the decrypted fixture
# plaintext.
#
# ── severity, each drill observed red ──
# Mutating one field of the built manifest turns `safix-installer-schema` red
# on the diff; mutating the `serde` struct instead turns
# `safix-installer-roundtrip` red on the acceptance of the real manifest, which
# is the evidence the pair holds both sides of the boundary. Making
# `--check-mode=document` accept a key absent from its document turns the
# roundtrip check's document tier green where it must refuse. Dropping the
# minted path default turns `safix-installer-type`'s two-entry fixture red on
# the path-less entry and `safix-installer-store`'s path map red on every
# path-less entry at once, which is the pair that holds the store root and the
# entry default together. Registering the installer as `setupSecrets` again
# turns the ordering check red on the own-node facts — the installer call lands
# inside the shared node's text — and dropping the `afterActivation` wiring
# turns exactly the dependency assertion red; replacing the two `setupSecrets`
# definitions with two differently-named steps turns the merge assertion red,
# the node list ceasing to be the singleton. Restoring a second installer's
# delivery turns the sole check red on the invocation scan, with both
# installers visible at once; removing the copied refusal block turns its
# refusal fixtures into the incidental failure the block pre-empts. Restoring
# the `/run/secrets` prefix turns the identity check red on the clan-shaped
# fixture, which derives nothing; dropping the exclusion entirely turns it red
# on the safix-store fixture, which derives the key safix itself deploys.
# Removing the evaluation refusal turns the refusals check red on
# `noIdentity.refuses` while every other check in this file stays green;
# dropping one identity path from the script turns exactly
# `script.namesTheIdentity` red, and replacing the script's store-path `sops`
# reference with a bare `sops` turns exactly `script.namesSopsByStorePath` red.
# Ignoring either environment override turns `safix-installer-overrides` red on
# that override's own marker. Pointing the coexistence check's second run back
# at the foreign directory turns it red on the tree walk, with the foreign
# store's paths gone.
{
  config,
  inputs,
  lib,
  ...
}:
{
  perSystem =
    {
      pkgs,
      self',
      system,
      ...
    }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      systemCommon = import ../../consume/common.nix {
        inherit lib;
        scope = "system";
      };

      # The configuration the entry type's `path` default is a function of, and
      # the only thing the type reads outside its own fields. A stub carrying
      # it is what lets the type be read here at all, outside any module
      # evaluation.
      entryCfg.installer.symlinkPath = "/run/safix";

      # `safix.installer.package` defaults to `pkgs.safix`, which this flake's
      # nixpkgs does not carry: the binary is a flake output, not an overlay.
      # Every fixture below that forces the manifest or the installer script
      # names it, and the ones that force neither do not.
      installerPackage = self'.packages.safix;

      # ── the two environment overrides ──
      # Each stub records the call in a marker file the check names in the
      # environment, then behaves as the real tool does, so a runtime that
      # ignored the variable and found the real binary on `PATH` leaves the
      # marker absent rather than failing outright.
      #
      # The stubs are built here rather than written by a heredoc inside the
      # check, because a heredoc inside a nix indented string cannot put a
      # shebang at byte zero.
      sshToAgeStub = pkgs.writeShellScript "safix-fixture-ssh-to-age" ''
        echo "ssh-to-age-was-called $*" >> "$SAFIX_OVERRIDE_MARKERS"
        for argument in "$@"; do
          case "$argument" in
            *unconvertible*)
              echo "ssh-to-age: not an ed25519 key" >&2
              exit 1
              ;;
          esac
        done
        cat "$SAFIX_OVERRIDE_CONVERTED"
      '';

      keygenStub = pkgs.writeShellScript "safix-fixture-age-keygen" ''
        echo "keygen-was-called $*" >> "$SAFIX_OVERRIDE_MARKERS"
        exec ${pkgs.age}/bin/age-keygen "$@"
      '';

      # The path the key-generation step mints into. It is baked into the
      # script at evaluation, so it names the build root a sandboxed builder
      # gives every derivation; the check verifies that assumption rather than
      # relying on it.
      generatedKeyFile = "/build/safix-fixture-minted-key.txt";

      # A user-scope profile that asks for a key file at activation. Its
      # install script is the only place in the tree `age-keygen` is reached
      # from outside the operator-facing `keygen` verb, which needs a declaring
      # repository a sandbox has none of.
      keyGenerationProfile =
        (inputs.home-manager.lib.homeManagerConfiguration {
          inherit pkgs;
          modules = [
            config.flake.homeModules.safix
            {
              home = {
                username = "alice";
                homeDirectory = "/home/alice";
                stateVersion = "24.05";
              };
              safix = {
                lib = config.flake.safix.lib;
                user = "alice";
                hostname = "workstation";
                identity = {
                  generateKey = true;
                  keyFile = generatedKeyFile;
                };
                installer = {
                  package = installerPackage;
                  validate = false;
                };
              };
            }
          ];
        }).config;

      userInstallScript = lib.head (
        lib.toList keyGenerationProfile.systemd.user.services.safix.Service.ExecStart
      );

      resolve = import ../safix/resolve.nix { inherit lib; };
      fleetTypes = import ../safix/types.nix { inherit lib; };

      typedRecords =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

      names = tokens: messages: builtins.all (t: lib.any (m: lib.hasInfix t m) messages) tokens;

      # ── the entry type ──

      entrySubOptions = lib.filterAttrs (n: _: !lib.hasPrefix "_" n) (
        (systemCommon.secretEntryType { cfg = entryCfg; }).getSubOptions [ ]
      );

      # A resolution of exactly two entries, one declaring a path and one not,
      # injected through `common.resolvedFor`'s own seam because `safix.secrets`
      # is read-only. The expectation below is written from the declarations
      # rather than from the type, so the mint is the thing measured.
      pathDefaultFixture = refusalFixtureWith {
        declares = {
          sopsFile = pkgs.writeText "safix-fixture-declares.yaml" "fixture: encrypted\n";
          path = "/var/lib/safix-fixture/declared";
        };
        mints.sopsFile = pkgs.writeText "safix-fixture-mints.yaml" "fixture: encrypted\n";
      };

      typeFacts = {
        actual = {
          fields = lib.sort (a: b: a < b) (builtins.attrNames entrySubOptions);

          # The literal defaults, read off the declaration rather than off an
          # evaluated entry, so a re-default is a failing diff even where no
          # fixture happens to exercise the field.
          defaults = lib.mapAttrs (name: _: entrySubOptions.${name}.default) {
            format = null;
            gid = null;
            group = null;
            mode = null;
            owner = null;
            reloadUnits = null;
            restartUnits = null;
            uid = null;
          };

          # `name`, `key` and `path` default from the submodule's own
          # configuration, so asking `getSubOptions` for a value would force
          # `_module.args.name` outside the `attrsOf` that supplies it. What is
          # held here is that each carries a default at all; `path`'s value is
          # held by `mintedPaths` below and by `safix-installer-store`.
          configuredDefaults = lib.mapAttrs (name: _: entrySubOptions.${name} ? default) {
            key = null;
            name = null;
            path = null;
          };

          # The one field with no default. An entry that named no document
          # would be an entry the installer could not open, so this is a
          # refusal rather than a default.
          documentIsRequired = !(entrySubOptions.sopsFile ? default);

          mintedPaths = lib.mapAttrs (_name: entry: entry.path) pathDefaultFixture.config.safix.secrets;
        };

        expected = {
          fields = [
            "format"
            "gid"
            "group"
            "key"
            "mode"
            "name"
            "owner"
            "path"
            "reloadUnits"
            "restartUnits"
            "sopsFile"
            "uid"
          ];

          defaults = {
            format = "yaml";
            gid = 0;
            group = null;
            mode = "0400";
            owner = null;
            reloadUnits = [ ];
            restartUnits = [ ];
            uid = 0;
          };

          configuredDefaults = {
            key = true;
            name = true;
            path = true;
          };

          documentIsRequired = true;

          mintedPaths = {
            declares = "/var/lib/safix-fixture/declared";
            mints = "/run/safix/mints";
          };
        };
      };

      # ── the manifest ──

      # A system configuration through the exported module, resolving bob on
      # the fixture fleet's server, the same subject `safix-consumption-system`
      # reads. `safix.installer.validate` is off because the fleet's sops files
      # are paths into this flake that no committed file backs; that selects the
      # checking branch that never reads ciphertext and leaves
      # `manifestInputHash` null, and the branch itself stays the module's
      # conditional rather than a value this check pins.
      manifestFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              installer = {
                package = installerPackage;
                validate = false;
              };
            };
          }
        ];
      };

      safixManifest = manifestFixture.config.system.build.safix-manifest;

      # The same fixture with both roots moved through the options, so the
      # settability claim and the root-and-default coupling are measured at the
      # option rather than at the literal.
      movedFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              installer = {
                package = installerPackage;
                validate = false;
                secretsMountPoint = "/run/safix-moved.d";
                symlinkPath = "/run/safix-moved";
              };
            };
          }
        ];
      };

      movedManifest = movedFixture.config.system.build.safix-manifest;

      # name -> the path the manifest must carry, computed from the
      # declarations' own resolution rather than from the option the manifest is
      # built from: `config.flake.safix.lib.materialize` is the same call
      # `common.resolvedFor` makes, read before any module minted anything, so
      # the diff below is between two independent computations of one contract
      # rather than one option against itself.
      # An entry that declared a path keeps it, and a path-less entry parks
      # under the fixture's own symlink path — read off the option so the moved
      # fixture measures the root and the entry default as one claim.
      #
      # Severity: moving the mint out of the entry type's `path` default
      # reddens this check, the diff putting the three path-less entries
      # wherever the replacement mint puts them; pointed back at
      # `fixture.config.safix.secrets` instead, the same move passes, which is
      # why the expected side is read here rather than off the option.
      entryPathContract =
        fixture:
        lib.mapAttrs (name: entry: entry.path or "${fixture.config.safix.installer.symlinkPath}/${name}") (
          config.flake.safix.lib.materialize {
            user = "bob";
            machine = null;
            hostname = "server";
            tags = [ ];
            scope = "system";
          } fixture.config
        );

      # Two entries of one person declaring one path, the smallest fleet the
      # collision refusal can fire on, so the minted default's inability to
      # collide is held beside the declared paths' refusal staying in force.
      collidingFleet = {
        users = typedRecords (lib.types.attrsOf fleetTypes.profile) {
          alice = {
            recipient = "age1fixture-alice-0000000000000000000000000000000000";
            private = {
              first.path = _cfg: "/var/lib/safix-fixture/one";
              second.path = _cfg: "/var/lib/safix-fixture/one";
            };
          };
        };
        catalogue = typedRecords (lib.types.attrsOf fleetTypes.entry) { };
        machines = typedRecords (lib.types.attrsOf fleetTypes.machine) { };
        services = typedRecords (lib.types.attrsOf fleetTypes.service) { };
        groups = typedRecords (lib.types.attrsOf fleetTypes.group) { };
        organizations = typedRecords (lib.types.attrsOf fleetTypes.organization) { };
        silos = typedRecords (lib.types.attrsOf fleetTypes.silo) { };
      };

      collisionStillRefused = fires (
        resolve.materializeFor (
          collidingFleet
          // {
            root = "";
            hostname = "somewhere";
            tags = [ ];
            user = "alice";
            scope = "system";
          }
        ) { }
      );

      # ── the ordering ──

      # A host that carries a foreign store's activation step under the name
      # both colliding packages use, and names it through safix's option.
      orderedActivationFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            system.activationScripts.setupSecrets =
              lib.stringAfter
                [
                  "specialfs"
                  "users"
                  "groups"
                ]
                ''
                  echo safix-fixture-foreign-store-half
                '';
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              installer = {
                package = installerPackage;
                validate = false;
                afterActivation = [ "setupSecrets" ];
              };
            };
          }
        ];
      };

      # A host whose user management moves secret installation into a unit,
      # so the selection is exercised from the host's own option rather than
      # from safix's override, and the ordering is a unit ordering.
      orderedUnitFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            services.userborn.enable = true;
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              installer = {
                package = installerPackage;
                validate = false;
                afterUnits = [ "age-decrypt-secrets.service" ];
              };
            };
          }
        ];
      };

      # Two definitions of one activation step name, each in the shape a
      # package registers one: `lib.stringAfter` on the same three
      # dependencies, with each body distinguishable so each half is findable
      # in the merged text. Neither is safix's — this fixture exists to hold
      # the mechanism that makes a name of one's own necessary, which is why
      # safix registers `safixInstallSecrets` and never contributes to a name
      # another package also defines.
      mergedFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          {
            nixpkgs.hostPlatform = system;
            system.stateVersion = "24.05";
          }
          (
            { config, ... }:
            {
              system.activationScripts.setupSecrets =
                lib.stringAfter
                  [
                    "specialfs"
                    "users"
                    "groups"
                  ]
                  ''
                    [ -e /run/current-system ] || echo setting up secrets...
                    echo safix-fixture-first-half
                  ''
                // lib.optionalAttrs (config.system ? dryActivationScript) {
                  supportsDryActivation = true;
                };
            }
          )
          (
            { config, ... }:
            {
              system.activationScripts.setupSecrets =
                lib.stringAfter
                  [
                    "specialfs"
                    "users"
                    "groups"
                  ]
                  ''
                    [ -e /run/current-system ] || echo setting up age secrets...
                    echo safix-fixture-second-half
                  ''
                // lib.optionalAttrs (config.system ? dryActivationScript) {
                  supportsDryActivation = true;
                };
            }
          )
        ];
      };

      mergedScripts = mergedFixture.config.system.activationScripts;

      # Read with a fallback rather than directly, so the rename drill turns
      # the assertions below red as a diff instead of an evaluation error.
      mergedStep =
        mergedScripts.setupSecrets or {
          text = "";
          deps = [ ];
        };

      orderingFacts =
        let
          # The step and the unit both reach the installer through one script
          # `system.build.safix-installer` exposes, so what is asserted here is
          # that each names that script; that the script runs `safix install`
          # is `safix-installer-refusals`' own row, read off the same text.
          # The context is discarded because these strings are used as regex
          # needles: a structural check's JSON must not name a store path as a
          # dependency of itself.
          activationInstaller = builtins.unsafeDiscardStringContext (
            toString orderedActivationFixture.config.system.build.safix-installer
          );
          unitInstaller = builtins.unsafeDiscardStringContext (
            toString orderedUnitFixture.config.system.build.safix-installer
          );
          activationScripts = orderedActivationFixture.config.system.activationScripts;
          safixStep =
            activationScripts.safixInstallSecrets or {
              deps = [ ];
              text = "";
            };
          unit =
            orderedUnitFixture.config.systemd.services.safix-install-secrets or {
              after = [ ];
              wantedBy = [ ];
              requiredBy = [ ];
              before = [ ];
              serviceConfig = { };
            };
          unorderedScripts = manifestFixture.config.system.activationScripts;
          unorderedStep =
            unorderedScripts.safixInstallSecrets or {
              deps = [ ];
            };
        in
        {
          actual = {
            activation = {
              ownNode = (activationScripts ? safixInstallSecrets) && (activationScripts ? setupSecrets);
              foreignStepNotMerged = !(lib.hasInfix activationInstaller activationScripts.setupSecrets.text);
              deps = lib.sort (a: b: a < b) (lib.unique safixStep.deps);
              runsTheInstaller = lib.hasInfix activationInstaller safixStep.text;
              noUnit = orderedActivationFixture.config.systemd.services ? safix-install-secrets;
            };

            unit = {
              exists = orderedUnitFixture.config.systemd.services ? safix-install-secrets;
              afterNamedUnit = builtins.elem "age-decrypt-secrets.service" unit.after;
              wantedBySysinit = builtins.elem "sysinit.target" unit.wantedBy;
              rerunsOnSwitch =
                builtins.elem "sysinit-reactivation.target" unit.requiredBy
                && builtins.elem "sysinit-reactivation.target" unit.before;
              runsTheInstaller = lib.any (lib.hasInfix unitInstaller) (lib.toList unit.serviceConfig.ExecStart);
              noActivationStep = orderedUnitFixture.config.system.activationScripts ? safixInstallSecrets;
            };

            # A fixture naming neither option: registered, and unordered
            # against anything foreign, so a host with no foreign store is a
            # supported configuration rather than an omission.
            unordered = {
              registered = unorderedScripts ? safixInstallSecrets;
              deps = lib.sort (a: b: a < b) (lib.unique unorderedStep.deps);
              noUnit = manifestFixture.config.systemd.services ? safix-install-secrets;
            };

            # The reason a name of one's own is not a preference: two
            # definitions of one name are one node, and a single node has no
            # edge to state.
            merge = {
              nodes = builtins.filter (n: lib.hasInfix "etupSecrets" n) (builtins.attrNames mergedScripts);
              firstHalf = lib.hasInfix "safix-fixture-first-half" mergedStep.text;
              secondHalf = lib.hasInfix "safix-fixture-second-half" mergedStep.text;
              depsUnion = lib.sort (a: b: a < b) (lib.unique mergedStep.deps);
            };

            # And the reason a merged node's failure is quiet: the wrapper
            # traps `ERR` and sets no `-e`, so a failed half records a status
            # and the other half still runs.
            wrapper = {
              trapsErr = lib.hasInfix "trap \"_status=1 _localstatus=\\$?\" ERR" mergedScripts.script;
              setELines = builtins.filter (l: builtins.match "[[:space:]]*set -e.*" l != null) (
                lib.splitString "\n" mergedScripts.script
              );
            };
          };

          expected = {
            activation = {
              ownNode = true;
              foreignStepNotMerged = true;
              deps = [
                "groups"
                "setupSecrets"
                "specialfs"
                "users"
              ];
              runsTheInstaller = true;
              noUnit = false;
            };

            unit = {
              exists = true;
              afterNamedUnit = true;
              wantedBySysinit = true;
              rerunsOnSwitch = true;
              runsTheInstaller = true;
              noActivationStep = false;
            };

            unordered = {
              registered = true;
              deps = [
                "groups"
                "specialfs"
                "users"
              ];
              noUnit = false;
            };

            merge = {
              nodes = [ "setupSecrets" ];
              firstHalf = true;
              secondHalf = true;
              depsUnion = [
                "groups"
                "specialfs"
                "users"
              ];
            };

            wrapper = {
              trapsErr = true;
              setELines = [ ];
            };
          };
        };

      # ── the identity ──

      # The identity fixtures share the bob resolution and vary only what the
      # derivation reads: where the host's keys lie, whether an identity is
      # named, and whether the switch is on. `services.openssh.enable` is on
      # because the derivation derives nothing from a host whose keys openssh
      # does not manage.
      identityFixture =
        extra:
        inputs.nixpkgs.lib.nixosSystem {
          modules = [
            config.flake.nixosModules.default
            {
              nixpkgs.hostPlatform = system;
              networking.hostName = "server";
              system.stateVersion = "24.05";
              services.openssh.enable = true;
              safix = {
                lib = config.flake.safix.lib;
                user = "bob";
                installer = {
                  package = installerPackage;
                  validate = false;
                };
              };
            }
            extra
          ];
        };

      # A clan-shaped host: every host key inside a store safix does not own,
      # with a non-ed25519 entry beside it that the type filter must drop.
      foreignHostKeys = [
        {
          path = "/run/secrets/openssh/ssh.id_ed25519";
          type = "ed25519";
        }
        {
          path = "/run/secrets/openssh/ssh.id_rsa";
          type = "rsa";
          bits = 4096;
        }
      ];

      safixHostKeys = [
        {
          path = "/run/safix/host-key.ed25519";
          type = "ed25519";
        }
      ];

      identityManifests = {
        foreignKeys =
          (identityFixture { services.openssh.hostKeys = foreignHostKeys; })
          .config.system.build.safix-manifest;
        # These two derive nothing, so each names a keyFile: without one the
        # system scope's no-identity refusal fires, which is its own check's
        # subject rather than this one's.
        safixKeys =
          (identityFixture {
            services.openssh.hostKeys = safixHostKeys;
            safix.identity.keyFile = "/var/lib/safix-fixture/key.txt";
          }).config.system.build.safix-manifest;
        namedOverForeign =
          (identityFixture {
            services.openssh.hostKeys = foreignHostKeys;
            safix.identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-named" ];
          }).config.system.build.safix-manifest;
        namedOverSafix =
          (identityFixture {
            services.openssh.hostKeys = safixHostKeys;
            safix.identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-named" ];
          }).config.system.build.safix-manifest;
        derivationOff =
          (identityFixture {
            services.openssh.hostKeys = foreignHostKeys;
            safix.identity.deriveHostKeys = false;
            safix.identity.keyFile = "/var/lib/safix-fixture/key.txt";
          }).config.system.build.safix-manifest;
      };

      # ── the refusals ──

      # An enabled configuration whose resolution is exactly one injected
      # entry, so each refusal is measured on the entry that earns it rather
      # than beside the fleet's. In pure evaluation an out-of-store path is
      # also unreadable, so the outside-store entry trips both halves of the
      # block in `modules/consume/installer.nix`; the store-membership half is
      # still the one the removal drill isolates, because without the block the
      # failure is a forced `builtins.hashFile` refusing the path — a hard
      # evaluation error that is not safix's message and that `tryEval` cannot
      # catch, which is what these fixtures would surface if the block were
      # dropped.
      #
      # The entry enters through `common.resolvedFor`'s own seam, which on this
      # path reads only `cfg.lib.violations` and `cfg.lib.materialize`, because
      # `safix.secrets` is read-only — one option per scope, and it is a
      # projection rather than a place to write. `cfg.lib.subjects` is not
      # needed: `tags`' default reads it only when `safix.machine` is set, and
      # this fixture is person-shaped. The identity is named so
      # `common.noSystemIdentityMessage` cannot pre-empt the refusal under
      # test, and `safix.installer.validate` is left at its default so the
      # refusals are reached at all.
      refusalFixtureWith =
        entry:
        inputs.nixpkgs.lib.nixosSystem {
          modules = [
            config.flake.nixosModules.default
            {
              nixpkgs.hostPlatform = system;
              system.stateVersion = "24.05";
              safix.enable = true;
              safix.lib = {
                violations = [ ];
                materialize = _args: _cfg: entry;
              };
              safix.user = "bob";
              safix.hostname = "server";
              safix.identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              safix.installer.package = installerPackage;
            }
          ];
        };

      outsideStoreFixture = refusalFixtureWith {
        outside.sopsFile = "/etc/hosts";
      };

      missingFileFixture = refusalFixtureWith {
        missing.sopsFile = "${builtins.storeDir}/00000000000000000000000000000000-safix-fixture-absent/nope.yaml";
      };

      # A configuration that resolves entries and can decrypt none of them:
      # openssh unmanaged, so nothing is derivable, and no identity named. The
      # refusal is read off `safix.secrets` directly — a throw while the
      # option is forced — which is the system scope's counterpart of reading
      # a profile without the module system's assertion collection: nothing
      # here forces `config.assertions`, so what fires is safix's own message
      # or nothing.
      identityFreeSystem = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              installer = {
                package = installerPackage;
                validate = false;
              };
            };
          }
        ];
      };

      refusalsFacts =
        let
          installerText = manifestFixture.config.system.build.safix-installer.text;
          activationText = manifestFixture.config.system.activationScripts.safixInstallSecrets.text;

          # The same fixture with one age plugin named, so the PATH claim is
          # measured against a value rather than against emptiness: the
          # activation's PATH is the plugins and nothing else, which is the one
          # thing design I8 says must be on it.
          pluginText =
            (inputs.nixpkgs.lib.nixosSystem {
              modules = [
                config.flake.nixosModules.default
                {
                  nixpkgs.hostPlatform = system;
                  networking.hostName = "server";
                  system.stateVersion = "24.05";
                  safix = {
                    lib = config.flake.safix.lib;
                    user = "bob";
                    identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
                    installer = {
                      package = installerPackage;
                      validate = false;
                      agePlugins = [ pkgs.hello ];
                    };
                  };
                }
              ];
            }).config.system.activationScripts.safixInstallSecrets.text;
        in
        {
          actual = {
            noIdentity = {
              refuses = fires identityFreeSystem.config.safix.secrets;
              namesTheOptions =
                names
                  [
                    "safix.identity.sshKeyPaths"
                    "safix.identity.keyFile"
                    "safix.identity.deriveHostKeys"
                  ]
                  [
                    (systemCommon.noSystemIdentityMessage {
                      cfg = {
                        user = "bob";
                        machine = null;
                      };
                      resolved.bob-service = { };
                    })
                  ];

              # The guard reads the identity rather than refusing every
              # configuration that resolves: the same subject with a derivable
              # identity resolves his four entries.
              withIdentity = lib.sort (a: b: a < b) (builtins.attrNames manifestFixture.config.safix.secrets);
            };

            # The installer script's own half, held as text without running
            # it: it names every configured identity path, refuses non-zero,
            # names both ordering options as the remedy and the foreign store
            # that has not run as the usual cause, and states its limit the
            # way the user scope's preflight does.
            script = {
              namesTheIdentity = lib.hasInfix "/etc/ssh/safix-fixture-identity" installerText;
              refuses = lib.hasInfix "exit 1" installerText;
              namesTheOrderingOptions =
                lib.hasInfix "safix.installer.afterActivation" installerText
                && lib.hasInfix "safix.installer.afterUnits" installerText;
              namesTheUsualCause = lib.hasInfix "has not run yet" installerText;
              statesItsLimit =
                lib.hasInfix "readability" installerText && lib.hasInfix "not a recipient" installerText;

              # Design I8's store-path discipline, held on the text rather
              # than on the option: `sops` is reached by store path, never as
              # a bare name resolved out of whatever `PATH` the activation
              # happens to carry, because a bare name is a different binary on
              # a host that has one.
              namesSopsByStorePath = lib.any (
                line: builtins.match ".*${builtins.storeDir}/[^[:space:]]+/bin/sops.*" line != null
              ) (lib.splitString "\n" installerText);
              reachesSopsByNameAlone = lib.any (
                line: builtins.match "[^#]*[^/[:alnum:]]sops[[:space:]]+(decrypt|encrypt).*" line != null
              ) (lib.splitString "\n" installerText);

              # The script reaches the installer itself by store path too,
              # which is what makes the activation step's reference to this
              # script a reference to safix's own binary rather than to
              # whatever `safix` a `PATH` resolves.
              runsTheInstaller = lib.any (
                line: builtins.match ".*${builtins.storeDir}/[^[:space:]]+/bin/safix install .*" line != null
              ) (lib.splitString "\n" installerText);
            };

            # The activation's own environment: `HOME` is the empty directory
            # that stops sops searching for ssh keys, and `PATH` is the age
            # plugins and nothing else — empty where none are named, and
            # exactly their bin directories where they are.
            environment = {
              homeIsEmpty = lib.hasInfix "export HOME='/var/empty'" activationText;
              pathIsEmptyWithNoPlugins = lib.hasInfix "export PATH=''" activationText;
              pathIsPluginsAlone = lib.hasInfix (builtins.unsafeDiscardStringContext "export PATH='${lib.makeBinPath [ pkgs.hello ]}'") pluginText;
            };
          };

          expected = {
            noIdentity = {
              refuses = true;
              namesTheOptions = true;
              withIdentity = [
                "bob-service"
                "ops-handover"
                "ops-tooling"
                "team-vault"
              ];
            };

            script = {
              namesTheIdentity = true;
              refuses = true;
              namesTheOrderingOptions = true;
              namesTheUsualCause = true;
              statesItsLimit = true;
              namesSopsByStorePath = true;
              reachesSopsByNameAlone = false;
              runsTheInstaller = true;
            };

            environment = {
              homeIsEmpty = true;
              pathIsEmptyWithNoPlugins = true;
              pathIsPluginsAlone = true;
            };
          };
        };

      # ── one installer, one namespace ──

      soleFacts =
        let
          scripts = manifestFixture.config.system.activationScripts;
          services = manifestFixture.config.systemd.services;

          # The same fleet and subject as `manifestFixture` with the gate off,
          # so a non-empty resolution is read back against an installer that
          # defined nothing.
          disabledFixture = inputs.nixpkgs.lib.nixosSystem {
            modules = [
              config.flake.nixosModules.default
              {
                nixpkgs.hostPlatform = system;
                networking.hostName = "server";
                system.stateVersion = "24.05";
                safix = {
                  enable = false;
                  lib = config.flake.safix.lib;
                  user = "bob";
                  identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
                  installer = {
                    package = installerPackage;
                    validate = false;
                  };
                };
              }
            ];
          };

          # The full script the option's `apply` assembles aggregates every
          # step, so it is excluded by name; a step registered as a bare string
          # is its own text.
          textOf = v: if lib.isAttrs v then v.text else v;

          invocationsIn =
            set: extract:
            lib.sort (a: b: a < b) (
              builtins.attrNames (lib.filterAttrs (n: v: n != "script" && extract v) set)
            );

          installerCall = text: lib.hasInfix "install-secrets" text || lib.hasInfix "/bin/safix install" text;
        in
        {
          actual = {
            # The fixture resolves the four entries `safix-consumption-system`
            # also reads, so the inertness below is evidence rather than an
            # empty resolution passing vacuously.
            established = lib.sort (a: b: a < b) (builtins.attrNames manifestFixture.config.safix.secrets);

            # safix reads and defines no option outside its own namespace, held
            # mechanically: the evaluated configuration carries no `sops`
            # option tree at all, which it could only carry if something in
            # this tree imported a module declaring one.
            noForeignNamespace = !(manifestFixture.config ? sops);

            # Exactly one invocation of an installer, and it is safix's: every
            # activation step's text and every unit's ExecStart is scanned, so
            # a second invocation appearing anywhere reddens this rather than
            # only the names one package happens to use.
            invocations = {
              activation = invocationsIn scripts (v: installerCall (textOf v));
              units = invocationsIn services (
                v: lib.any installerCall (lib.toList (v.serviceConfig.ExecStart or [ ]))
              );
            };

            refusals = {
              outsideStore = {
                refuses = fires outsideStoreFixture.config.system.build.safix-manifest;
                namesTheStore =
                  names
                    [
                      "is not in the Nix store"
                      "safix.installer.validate"
                      "/etc/hosts"
                      "outside"
                    ]
                    [
                      (systemCommon.sopsFileOutsideStoreMessage {
                        name = "outside";
                        file = "/etc/hosts";
                      })
                    ];
              };
              missingFile = {
                refuses = fires missingFileFixture.config.system.build.safix-manifest;
                namesTheFile =
                  names
                    [
                      "cannot find"
                      "nope.yaml"
                      "missing"
                    ]
                    [
                      (systemCommon.sopsFileMissingMessage {
                        name = "missing";
                        file = "${builtins.storeDir}/00000000000000000000000000000000-safix-fixture-absent/nope.yaml";
                      })
                    ];
              };
            };

            # The resolved set is reported outside the enable gate — the one
            # place the collapsed option can be defined without an evaluation
            # cycle — so what makes that safe is that nothing installs: the
            # whole installer `config` is `lib.mkIf cfg.enable`, while the
            # option still reports the resolution, minted paths and all.
            disabledOverResolution = {
              manifest = disabledFixture.config.system.build ? safix-manifest;
              activationStep = disabledFixture.config.system.activationScripts ? safixInstallSecrets;
              unit = disabledFixture.config.systemd.services ? safix-install-secrets;
              resolutionNonEmpty = disabledFixture.config.safix.secrets != { };
              paths = lib.mapAttrs (_: entry: entry.path) disabledFixture.config.safix.secrets;
            };
          };

          expected = {
            established = [
              "bob-service"
              "ops-handover"
              "ops-tooling"
              "team-vault"
            ];

            noForeignNamespace = true;

            invocations = {
              activation = [ "safixInstallSecrets" ];
              units = [ ];
            };

            refusals = {
              outsideStore = {
                refuses = true;
                namesTheStore = true;
              };
              missingFile = {
                refuses = true;
                namesTheFile = true;
              };
            };

            disabledOverResolution = {
              manifest = false;
              activationStep = false;
              unit = false;
              resolutionNonEmpty = true;
              paths = entryPathContract disabledFixture;
            };
          };
        };
    in
    {
      # Every claim here evaluates a NixOS system configuration, so the check
      # exists only where one does, following `safix-consumption-system`.
      checks = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-installer-type = mkStructuralCheck {
          name = "safix-installer-type";
          actual = typeFacts.actual;
          expected = typeFacts.expected;
        };

        # The nix half of the manifest boundary, diffed whole against a
        # committed file. Only `sopsFile` is normalized, and only by reducing
        # it to its path below the store: nothing else in the manifest carries
        # a hash, because the fixture turns `validate` off and
        # `manifestInputHash` is therefore null. That absence is recorded
        # rather than hidden — the field's emission is held by the ciphertext
        # drill in task 2.8, over a fixture whose documents exist, and what
        # this check holds is that the field is present and null when
        # validation is off.
        safix-installer-schema =
          pkgs.runCommand "safix-installer-schema"
            {
              nativeBuildInputs = [ pkgs.jq ];
              safixManifest = safixManifest;
              expected = ./installer-manifest.json;
              meta.description = "the built manifest's whole structure, against an accepted snapshot";
            }
            ''
              jq empty "$safixManifest"

              [ "$(jq '.secrets | length' "$safixManifest")" -gt 0 ] || {
                echo "safix-installer-schema: the fixture resolved nothing, so nothing below is evidence"
                exit 1
              }

              normalize() {
                jq -S '
                  .secrets |= map(.sopsFile |= sub("^/nix/store/[^/]+/"; "")) | .secrets |= sort_by(.name)
                ' "$1"
              }

              if ! diff -u <(jq -S . "$expected") <(normalize "$safixManifest"); then
                echo ""
                echo "safix-installer-schema: the built manifest no longer matches the accepted snapshot"
                echo "safix-installer-schema: if the change is deliberate, the serde struct in"
                echo "safix-installer-schema: crates/safix-core/src/install.rs moves with it and so does"
                echo "safix-installer-schema: modules/flake/checks/installer-manifest.json"
                exit 1
              fi

              touch $out
            '';

        # The program half of the same boundary, against the real binary.
        safix-installer-roundtrip =
          pkgs.runCommand "safix-installer-roundtrip"
            {
              nativeBuildInputs = [
                pkgs.age
                pkgs.jq
                pkgs.sops
                installerPackage
              ];
              safixManifest = safixManifest;
              meta.description = "the real installer accepts the built manifest and refuses four mutations of it";
            }
            ''
              export HOME="$TMPDIR"

              # No runtime directory is named, deliberately. The document
              # tier's fixture manifest is user-mode, and a check mode
              # installs nothing, so it must validate a `%r` root rather than
              # expand it: a build sandbox has no runtime directory and a
              # manifest that only ever gets checked has no business needing
              # one. This absence is what holds that.
              work="$TMPDIR/work"
              mkdir -p "$work"
              cd "$work"

              accept() {
                if ! safix install --check-mode="$1" --ignore-passwd "$2" > accepted.log 2>&1; then
                  cat accepted.log
                  echo "safix-installer-roundtrip: $3"
                  exit 1
                fi
              }

              refuse() {
                if safix install --check-mode="$1" --ignore-passwd "$2" > refused.log 2>&1; then
                  echo "safix-installer-roundtrip: $3"
                  exit 1
                fi
                if ! grep -q -- "$4" refused.log; then
                  cat refused.log
                  echo "safix-installer-roundtrip: the refusal did not name '$4'"
                  exit 1
                fi
              }

              # ── the schema tier, over the manifest the nix half built ──
              cp "$safixManifest" built.json
              accept manifest built.json \
                "the manifest this tree builds is not one this binary reads"

              jq '.version = 2' built.json > version.json
              refuse manifest version.json \
                "a schema version this binary does not know was accepted" "version"

              jq '.secrets[0].mode = "0o400"' built.json > mode.json
              refuse manifest mode.json \
                "a mode that is not octal was accepted" "$(jq -r '.secrets[0].name' built.json)"

              jq '. + { templates: [] }' built.json > top-field.json
              refuse manifest top-field.json \
                "an unknown top-level field was ignored rather than refused" "templates"

              jq '.secrets[0] += { neededForUsers: false }' built.json > entry-field.json
              refuse manifest entry-field.json \
                "an unknown entry field was ignored rather than refused" "neededForUsers"

              # ── the document tier, over ciphertext this check made ──
              # The built manifest's documents are paths into this flake that no
              # committed file backs, so a document-mode run over it would
              # refuse for want of a file rather than for want of a key. This
              # fixture is what makes the third mutation — a key absent from its
              # document — expressible at all.
              age-keygen -o identity.txt 2>/dev/null
              recipient=$(age-keygen -y identity.txt)
              printf 'present: a value\n' > plain.yaml
              sops encrypt --age "$recipient" plain.yaml > cipher.yaml

              documentManifest() {
                jq -n --arg key "$1" --arg cipher "$work/cipher.yaml" --arg identity "$work/identity.txt" '{
                  version: 1,
                  secrets: [ {
                    name: "probe", key: $key, path: "/run/safix-roundtrip/probe",
                    owner: null, group: null, uid: 0, gid: 0,
                    sopsFile: $cipher, format: "yaml", mode: "0400",
                    restartUnits: [], reloadUnits: []
                  } ],
                  secretsMountPoint: "/run/safix-roundtrip.d",
                  symlinkPath: "/run/safix-roundtrip",
                  keepGenerations: 1,
                  ageKeyFile: $identity,
                  ageSshKeyPaths: [],
                  useTmpfs: false,
                  userMode: true,
                  logging: { keyImport: false, secretChanges: false },
                  manifestInputHash: null
                }' > "$2"
              }

              documentManifest present document-present.json
              documentManifest absent document-absent.json

              accept document document-present.json \
                "a key that resolves in its document was refused"
              accept manifest document-absent.json \
                "the schema mode read a document it has no business reading"
              refuse document document-absent.json \
                "a key absent from its document was accepted" "absent"

              touch $out
            '';

        safix-installer-store =
          pkgs.runCommand "safix-installer-store"
            {
              nativeBuildInputs = [ pkgs.jq ];
              safixManifest = safixManifest;
              movedManifest = movedManifest;
              expectedPathsJson = builtins.toJSON (entryPathContract manifestFixture);
              movedPathsJson = builtins.toJSON (entryPathContract movedFixture);
              collisionRefused = builtins.toJSON collisionStillRefused;
              passAsFile = [
                "expectedPathsJson"
                "movedPathsJson"
              ];
              meta.description = "the store roots and the per-entry path default, held as one claim";
            }
            ''
              [ "$collisionRefused" = true ] || {
                echo "safix-installer-store: two entries declaring one path no longer refuse"
                exit 1
              }

              [ "$(jq -r .secretsMountPoint "$safixManifest")" = /run/safix.d ] || {
                echo "safix-installer-store: default secretsMountPoint is not safix's own"
                exit 1
              }
              [ "$(jq -r .symlinkPath "$safixManifest")" = /run/safix ] || {
                echo "safix-installer-store: default symlinkPath is not safix's own"
                exit 1
              }
              [ "$(jq -r .secretsMountPoint "$movedManifest")" = /run/safix-moved.d ] || {
                echo "safix-installer-store: secretsMountPoint did not follow its option"
                exit 1
              }
              [ "$(jq -r .symlinkPath "$movedManifest")" = /run/safix-moved ] || {
                echo "safix-installer-store: symlinkPath did not follow its option"
                exit 1
              }

              for pair in "$safixManifest=$expectedPathsJsonPath" "$movedManifest=$movedPathsJsonPath"; do
                manifest="''${pair%%=*}"
                contract="''${pair#*=}"
                if ! diff -u <(jq -S . "$contract") <(jq -S '.secrets | map({ (.name): .path }) | add' "$manifest"); then
                  echo ""
                  echo "safix-installer-store: entry paths broke the root-and-default contract in $manifest"
                  exit 1
                fi
              done

              touch $out
            '';

        safix-installer-ordering = mkStructuralCheck {
          name = "safix-installer-ordering";
          actual = orderingFacts.actual;
          expected = orderingFacts.expected;
        };

        safix-installer-sole = mkStructuralCheck {
          name = "safix-installer-sole";
          actual = soleFacts.actual;
          expected = soleFacts.expected;
        };

        safix-installer-refusals = mkStructuralCheck {
          name = "safix-installer-refusals";
          actual = refusalsFacts.actual;
          expected = refusalsFacts.expected;
        };

        # Held against the built manifests' `ageSshKeyPaths` rather than
        # against an intermediate option, so the claim is about what the
        # binary will read.
        safix-installer-identity =
          pkgs.runCommand "safix-installer-identity"
            {
              nativeBuildInputs = [ pkgs.jq ];
              foreignKeys = identityManifests.foreignKeys;
              safixKeys = identityManifests.safixKeys;
              namedOverForeign = identityManifests.namedOverForeign;
              namedOverSafix = identityManifests.namedOverSafix;
              derivationOff = identityManifests.derivationOff;
              meta.description = "the system-scope identity derivation, excluding only safix's own store";
            }
            ''
              assertIdentity() {
                if ! diff -u <(echo "$2" | jq -S .) <(jq -S .ageSshKeyPaths "$1"); then
                  echo ""
                  echo "safix-installer-identity: $3"
                  exit 1
                fi
              }

              assertIdentity "$foreignKeys" '["/run/secrets/openssh/ssh.id_ed25519"]' \
                "a host key another store deployed was not derived, or the type filter leaked"
              assertIdentity "$safixKeys" '[]' \
                "a host key inside safix's own store was derived"
              assertIdentity "$namedOverForeign" '["/etc/ssh/safix-fixture-named"]' \
                "a named identity did not survive foreign host keys"
              assertIdentity "$namedOverSafix" '["/etc/ssh/safix-fixture-named"]' \
                "a named identity did not survive safix-store host keys"
              assertIdentity "$derivationOff" '[]' \
                "the derivation contributed with its switch off"

              touch $out
            '';

        # The two environment overrides, driven rather than declared. Each
        # points at a script of this check's own that records having been
        # called and then behaves as the real tool does, so a runtime that
        # ignored the variable and found the real binary on `PATH` — or found
        # nothing and skipped — leaves the marker absent.
        #
        # `ssh-to-age`'s asymmetry is exercised in the same run: one key
        # converts, one does not, and the run still assembles an identity from
        # the one that did.
        safix-installer-overrides =
          pkgs.runCommand "safix-installer-overrides"
            {
              nativeBuildInputs = [
                pkgs.age
                pkgs.coreutils
                pkgs.jq
                pkgs.sops
                installerPackage
              ];
              sshToAgeStub = sshToAgeStub;
              keygenStub = keygenStub;
              installScript = userInstallScript;
              keyFile = generatedKeyFile;
              meta.description = "SAFIX_AGE_KEYGEN and SAFIX_SSH_TO_AGE are read rather than declared";
            }
            ''
              export HOME="$TMPDIR"
              export XDG_RUNTIME_DIR="$TMPDIR/run"
              mkdir -p "$XDG_RUNTIME_DIR"
              work="$TMPDIR/work"
              mkdir -p "$work"
              cd "$work"

              export SAFIX_OVERRIDE_MARKERS="$work/markers.txt"
              : > "$SAFIX_OVERRIDE_MARKERS"

              age-keygen -o real-identity.txt 2>/dev/null
              recipient=$(age-keygen -y real-identity.txt)
              export SAFIX_OVERRIDE_CONVERTED="$work/real-identity.txt"
              printf 'token: safix-override-fixture\n' > plain.yaml
              sops encrypt --age "$recipient" plain.yaml > cipher.yaml

              printf 'not a key\n' > unconvertible-key
              printf 'not a key either\n' > convertible-key

              jq -n --arg cipher "$work/cipher.yaml" \
                --arg convertible "$work/convertible-key" \
                --arg unconvertible "$work/unconvertible-key" '{
                version: 1,
                secrets: [ {
                  name: "token", key: "token", path: "%r/safix/token",
                  owner: null, group: null, uid: 0, gid: 0,
                  sopsFile: $cipher, format: "yaml", mode: "0400",
                  restartUnits: [], reloadUnits: []
                } ],
                secretsMountPoint: "%r/safix.d",
                symlinkPath: "%r/safix",
                keepGenerations: 1,
                ageKeyFile: null,
                ageSshKeyPaths: [ $convertible, $unconvertible ],
                useTmpfs: false,
                userMode: true,
                logging: { keyImport: false, secretChanges: false },
                manifestInputHash: null
              }' > manifest.json

              SAFIX_SSH_TO_AGE="$sshToAgeStub" safix install manifest.json 2> install.log || {
                cat install.log
                echo "safix-installer-overrides: the install refused"
                exit 1
              }

              grep -q "ssh-to-age-was-called" "$SAFIX_OVERRIDE_MARKERS" || {
                echo "safix-installer-overrides: SAFIX_SSH_TO_AGE was declared and not read"
                exit 1
              }
              [ "$(cat "$XDG_RUNTIME_DIR/safix/token")" = safix-override-fixture ] || {
                echo "safix-installer-overrides: the entry did not decrypt with the converted identity"
                exit 1
              }

              # ── the key generator's own override ──
              # Exercised through the user scope's own install script, which is
              # where `age-keygen` is reached at all: the installer mints no
              # key, and the operator-facing `keygen` verb needs a declaring
              # repository this sandbox has none of.
              #
              # The key file's path is baked into that script at evaluation, so
              # it names the sandbox's own build root. That is an assumption
              # about the sandbox rather than about safix, so it is checked
              # rather than relied on.
              [ -d "$(dirname "$keyFile")" ] && [ -w "$(dirname "$keyFile")" ] || {
                echo "safix-installer-overrides: $keyFile's directory is not writable here,"
                echo "safix-installer-overrides: so this check's assumption about the build root does not hold"
                exit 1
              }
              rm -f "$keyFile"

              # The install half of the script refuses: its manifest names the
              # fixture fleet's documents, which are paths into this flake that
              # no committed file backs. The key generation runs first and is
              # what is measured, so the exit status is deliberately not.
              SAFIX_AGE_KEYGEN="$keygenStub" "$installScript" > keygen.log 2>&1 || true

              grep -q "keygen-was-called" "$SAFIX_OVERRIDE_MARKERS" || {
                cat keygen.log
                echo "safix-installer-overrides: SAFIX_AGE_KEYGEN was declared and not read"
                exit 1
              }
              [ -s "$keyFile" ] || {
                cat keygen.log
                echo "safix-installer-overrides: no key file was minted where the profile asked for one"
                exit 1
              }

              touch $out
            '';

        # What this stands in for, and what it does not: the failure observed
        # on the pilot host was EBUSY — a RemoveAll on a live ramfs mount —
        # and a build sandbox cannot mount, so the branch demonstrated here is
        # the removal itself, which is what a mountpoint turns into an error.
        # The binary is run for real, in user mode so no privilege is needed,
        # over ciphertext and an age identity generated inside the sandbox.
        # Linux only, and absent rather than trivially green elsewhere: this
        # whole file's checks exist only where a NixOS configuration evaluates.
        safix-installer-coexistence =
          pkgs.runCommand "safix-installer-coexistence"
            {
              nativeBuildInputs = [
                pkgs.age
                pkgs.sops
                pkgs.jq
                installerPackage
              ];
              meta.description = "the destructive branch is real, and safix's roots never reach a foreign store";
            }
            ''
              export HOME="$TMPDIR"
              export XDG_RUNTIME_DIR="$TMPDIR/run"
              mkdir -p "$XDG_RUNTIME_DIR"
              work="$TMPDIR/work"
              mkdir -p "$work"
              cd "$work"

              age-keygen -o "$TMPDIR/identity.txt" 2>/dev/null
              recipient=$(age-keygen -y "$TMPDIR/identity.txt")
              printf 'token: safix-coexistence-fixture\n' > "$TMPDIR/plain.yaml"
              sops encrypt --age "$recipient" "$TMPDIR/plain.yaml" > cipher.yaml

              manifestFor() {
                jq -n --arg symlink "$1" --arg mount "$2" \
                  --arg cipher "$work/cipher.yaml" --arg key "$TMPDIR/identity.txt" '{
                  version: 1,
                  secrets: [ {
                    name: "token", key: "token", path: ($symlink + "/token"),
                    owner: null, group: null, uid: 0, gid: 0,
                    sopsFile: $cipher, format: "yaml", mode: "0400",
                    restartUnits: [], reloadUnits: []
                  } ],
                  secretsMountPoint: $mount, symlinkPath: $symlink,
                  keepGenerations: 1,
                  ageKeyFile: $key, ageSshKeyPaths: [],
                  useTmpfs: false, userMode: true,
                  logging: { keyImport: false, secretChanges: false },
                  manifestInputHash: null
                }' > "$3"
              }

              mkdir foreign-destroyed foreign-preserved
              echo sentinel > foreign-destroyed/sentinel
              echo sentinel > foreign-preserved/sentinel
              cp -r foreign-preserved "$TMPDIR/foreign-preserved.expected"

              manifestFor "$work/foreign-destroyed" "$work/foreign-destroyed.d" manifest-destructive.json
              manifestFor "$work/safix" "$work/safix.d" manifest-safix.json

              safix install --ignore-passwd manifest-destructive.json
              if [ -e foreign-destroyed/sentinel ]; then
                echo "safix-installer-coexistence: the sentinel survived, so the destructive branch did not fire"
                exit 1
              fi

              snapshot() {
                find "$work" \( -path "$work/safix" -o -path "$work/safix.d" \) -prune -o -print | sort
              }
              snapshot > "$TMPDIR/before.txt"

              safix install --ignore-passwd manifest-safix.json

              snapshot > "$TMPDIR/after.txt"
              if ! diff -u "$TMPDIR/before.txt" "$TMPDIR/after.txt"; then
                echo ""
                echo "safix-installer-coexistence: a path outside safix's store was created or removed"
                exit 1
              fi

              if [ ! -e foreign-preserved/sentinel ]; then
                echo "safix-installer-coexistence: the foreign store's sentinel did not survive safix's run"
                exit 1
              fi
              if ! diff -r "$TMPDIR/foreign-preserved.expected" foreign-preserved; then
                echo "safix-installer-coexistence: the foreign store is not byte-identical after safix's run"
                exit 1
              fi

              if [ ! -L "$work/safix" ]; then
                echo "safix-installer-coexistence: safix's symlink path was not created as a symlink"
                exit 1
              fi
              if [ "$(cat "$work/safix/token")" != safix-coexistence-fixture ]; then
                echo "safix-installer-coexistence: the installed secret did not decrypt to the fixture plaintext"
                exit 1
              fi

              touch $out
            '';
      };
    };
}
