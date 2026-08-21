# Holds the installer safix owns to the facts it is built on, before and beside
# the code that builds it. Revisions are the ones `flake.lock` pins; every line
# anchor below was read at one of them.
#
# ── the mechanism ──
# `sops-install-secrets` reads `secretsMountPoint` and `symlinkPath` from the
# manifest JSON, never from a NixOS option, and the provisioner's builder
# hardcodes both (`modules/sops/manifest-for.nix:36-38`) only to merge its
# `extraJson` argument over them (`:52`). That merge is the whole mechanism this
# change rests on, so it is held two ways: a manifest built by the provisioner's
# own `manifest-for.nix` with an `extraJson` naming two other roots carries
# those roots rather than the hardcoded ones, and the provisioner's own
# `secrets-for-users` submodule is read back doing exactly this to itself
# (`modules/sops/secrets-for-users/default.nix:24-27`), through the manifest it
# exposes as `system.build.sops-nix-users-manifest` (`:87`). Both are jq'd out
# of built manifests rather than out of the expressions that produced them.
#
# ── the option surface that is not there ──
# The NixOS option tree offers neither root: every option path under
# `options.sops` of an evaluated system configuration is enumerated, submodules
# included, and none names a secrets mount point or a symlink path. The
# home-manager scope's `sops.defaultSymlinkPath` and
# `sops.defaultSecretsMountPoint` (`modules/home-manager/sops.nix:184`, `:193`)
# are enumerated the same way and both exist, which is what makes the absence at
# system scope a fact about that scope rather than about the enumeration.
#
# ── the merge that makes ordering inexpressible ──
# A fixture system configuration defines `system.activationScripts.setupSecrets`
# twice, once in the shape the provisioner uses (`modules/sops/default.nix:497-515`)
# and once in clan's (`nixosModules/clanCore/vars/secret/age.nix:259-276`).
# `deps` is `listOf str` and `text` is `types.lines`
# (`nixos/modules/system/activation/activation-script.nix:101-107`), so the two
# definitions become one activation step whose text carries both bodies and
# whose dependency set is the union of the two lists — one node in the DAG, and
# a single node has no edge to state. The wrapper the step runs under carries an
# `ERR` trap and no `set -e` (`activation-script.nix:62-63`), which is why a
# failed half records a status and the other half still runs; that record is
# held here rather than asserted in prose.
#
# ── the manifest safix writes ──
# `safix-installer-manifest` evaluates a fixture system configuration through
# the exported module, builds `system.build.safix-manifest`, and holds it three
# ways: it parses, its two roots are safix's own rather than the provisioner's,
# and its `userMode` is false. The same fixture's entries are then run through
# the provisioner's own builder from `inputs.sops-nix` — which the flake has
# and the exported module deliberately does not — and the two manifests' JSON
# key sets are asserted equal, so a field the provisioner adds reddens this
# check on the commit that moves the pin rather than reaching a host. The
# fixture sets `sops.validateSopsFiles = false` because the fixture fleet's
# sops files are paths into this flake that no committed file backs; that
# selects the checking branch that never reads ciphertext, and the branch
# itself stays the module's conditional rather than a value this check pins.
#
# ── the store and the entry default, one claim ──
# `safix-installer-store` reads the built manifest of the same fixture twice,
# once at the default roots and once with both moved through
# `safix.installer.*`, and holds one map per fixture: every entry that
# declared a path keeps it, and every entry that did not parks at
# `<symlinkPath>/<name>` under that fixture's own root. The map is one claim
# deliberately — the installer symlinks any entry path that is not
# `<symlinkPath>/<name>` (`main.go:254-268`), so a root moved without the
# entry default does not collide with a foreign store, it writes into it. The
# path-collision refusal is held beside it over the smallest fleet that can
# collide: two entries of one person declaring one path still refuse, and a
# minted default, being a function of the name, cannot.
#
# ── the named entry, both mechanisms, and the consumer's ordering ──
# `safix-installer-ordering` evaluates three fixtures. One carries a foreign
# store's step under the shared `setupSecrets` name and names it through
# `safix.installer.afterActivation`: safix's step is its own node beside it,
# carries the name in its `deps`, and the foreign node's text never gains the
# installer call. One enables userborn, so the selection follows the host's
# own user-management options — not `sops.useSystemdActivation`, which now
# governs an installer safix does not use — and the unit carries the unit
# named by `safix.installer.afterUnits` in its `after`, plus the
# `sysinit-reactivation.target` wiring that re-runs it on a switch. And the
# unadorned fixture registers the installer with no foreign dependency, so a
# host with no foreign store is a supported configuration.
#
# ── one installer ──
# `safix-installer-sole` holds, over the fixture whose resolution is the four
# entries `safix-consumption-system` also reads, that the provisioner is
# inert: its secrets option is empty, its activation step and unit are absent,
# and a scan of every activation step's text and every unit's ExecStart finds
# exactly one invocation of the installer binary, safix's. Beside it, the two
# refusals the typed set does not carry are measured on fixtures with one
# injected entry each: a sops file outside the nix store and one that does
# not exist both refuse through the block `modules/consume/installer.nix`
# copies from the provisioner's builder, whose messages are named in
# `common.nix` so this check can read them.
#
# ── the identity the system scope derives ──
# `safix-installer-identity` reads `ageSshKeyPaths` out of five built
# manifests rather than out of the option that fed them. A clan-shaped host
# whose keys lie under `/run/secrets` derives its ed25519 key and drops the
# rsa one; a host whose keys lie inside safix's own store derives nothing; a
# named identity survives both placements unchanged; and the switch turned
# off contributes nothing. The exclusion prefix is safix's own symlink path,
# not the `/run/secrets` the provisioner hardcodes, because the catch-22 the
# exclusion avoids is decrypting with a key this installer itself deploys.
#
# ── the two refusals ──
# `safix-installer-refusals` holds both. The evaluation refusal is a throw
# while `safix.secrets` is forced, read off a configuration that resolves
# entries with nothing derivable and nothing named — nothing here forces the
# assertion collection, so what fires is safix's own message or nothing, and
# nothing else can fire: the provisioner's key-source assertion sits inside
# its `mkIf (cfg.secrets != { })`, which safix leaves empty. The installer
# script's half is held as text off `system.build.safix-installer` without
# running it: it names every configured identity path, exits non-zero, names
# both ordering options as the remedy and the foreign store that has not run
# as the usual cause, and states the limit the user scope's preflight states —
# presence and readability were checked, decryption was not.
#
# ── severity, each drill observed red ──
# Removing the `extraJson` argument from the relocated manifest turns the
# root assertion red on the hardcoded values. Replacing the two `setupSecrets`
# definitions with two differently-named steps turns the merge assertion red:
# the node list stops being the singleton and neither body reaches the shared
# text. Dropping a field from safix's manifest turns the key-set parity
# assertion red; corrupting the manifest text turns its own checkPhase into a
# build failure; and pinning the check mode to `manifest` over an entry whose
# declared key is absent from its ciphertext builds green where `sopsfile`
# mode refuses, which is the measured evidence the two modes are not
# interchangeable. Dropping the minted path default turns the store check's
# path map red on every path-less entry, at `/run/secrets/<name>` — the
# silent-write combination the map exists to forbid. Registering the installer
# as `setupSecrets` again turns the ordering check red on the own-node facts —
# the installer call lands inside the shared node's text — and dropping the
# `afterActivation` wiring turns exactly the dependency assertion red.
# Restoring the `sops.secrets` delivery turns the sole check red on the
# provisioner-step facts, with both installers visible at once; removing the
# copied refusal block turns its refusal fixtures into the incidental failure
# the block pre-empts — a hard `sopsFileHash` evaluation error that is not
# safix's message and that `tryEval` cannot catch — which is the evidence the
# option type never carried the refusal. Restoring the `/run/secrets` prefix
# turns the identity check red on the clan-shaped fixture, which derives
# nothing; dropping the exclusion entirely turns it red on the safix-store
# fixture, which derives the key safix itself deploys. Removing the
# evaluation refusal turns the refusals check red on `noIdentity.refuses`
# while every other check in this file and the consumption suite stays green,
# which is the evidence no other refusal covers it; dropping one identity
# path from the script turns exactly `script.namesTheIdentity` red.
{
  config,
  inputs,
  lib,
  ...
}:
{
  perSystem =
    { pkgs, system, ... }:
    let
      # Every option path reachable under a prefix, submodule suboptions
      # included, collecting names only: `getSubOptions` forces type structure
      # and never a default, so options whose defaults read other config (the
      # home-manager module's `defaultSymlinkPath` reads `config.xdg`) stay
      # unforced.
      optionPaths =
        prefix: set:
        lib.concatLists (
          lib.mapAttrsToList (
            name: v:
            if lib.hasPrefix "_" name then
              [ ]
            else if lib.isOption v then
              [ (lib.concatStringsSep "." (prefix ++ [ name ])) ]
              ++ optionPaths (prefix ++ [ name ]) (v.type.getSubOptions (prefix ++ [ name ]))
            else if builtins.isAttrs v then
              optionPaths (prefix ++ [ name ]) v
            else
              [ ]
          ) set
        );

      namesAStoreRoot =
        n: lib.hasInfix "symlink" (lib.toLower n) || lib.hasInfix "mountpoint" (lib.toLower n);

      # The provisioner's builder, called the way its `secrets-for-users`
      # submodule calls it, over a synthetic cfg carrying only the fields the
      # builder reads. `validateSopsFiles` is off because this manifest holds no
      # secrets and the claim is about the two roots, not about ciphertext.
      manifestFor = pkgs.callPackage "${inputs.sops-nix}/modules/sops/manifest-for.nix" {
        cfg = {
          validateSopsFiles = false;
          keepGenerations = 1;
          gnupg = {
            home = null;
            sshKeyPaths = [ ];
          };
          age = {
            keyFile = null;
            sshKeyPaths = [ ];
          };
          useTmpfs = false;
          placeholder = { };
          log = [ ];
          validationPackage = inputs.sops-nix.packages.${system}.sops-install-secrets;
        };
      };

      relocatedManifest = manifestFor "-safix-mechanism" { } { } {
        secretsMountPoint = "/run/safix-mechanism-probe.d";
        symlinkPath = "/run/safix-mechanism-probe";
      };

      # A system configuration with the provisioner's module and one
      # `neededForUsers` entry, so the `secrets-for-users` submodule builds the
      # manifest it relocates. The sops file is synthetic and never read:
      # `validateSopsFiles = false` selects the check mode that parses the
      # manifest alone (`main.go:503-505`), and the claim here is the two roots.
      providerFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          inputs.sops-nix.nixosModules.sops
          {
            nixpkgs.hostPlatform = system;
            system.stateVersion = "24.05";
            sops.validateSopsFiles = false;
            sops.secrets.fixture-for-users = {
              neededForUsers = true;
              sopsFile = pkgs.writeText "safix-fixture-users.yaml" "fixture: encrypted\n";
            };
          }
        ];
      };

      usersManifest = providerFixture.config.system.build.sops-nix-users-manifest;

      # The home-manager module evaluated alone; the walker above touches
      # declarations only, so the profile options it would need stay unforced.
      homeManagerInstrument = lib.evalModules {
        modules = [
          "${inputs.sops-nix}/modules/home-manager/sops.nix"
          {
            _module.check = false;
            _module.args.pkgs = pkgs;
          }
        ];
      };

      # Two definitions of one activation step name, each in the shape its
      # package registers: `lib.stringAfter` on the same three dependencies,
      # with the provisioner's `generate-age-key` addition off as it is on a
      # host that does not generate a key, and clan's body distinguished the
      # way clan's is, so each half is findable in the merged text.
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
                    echo safix-fixture-provisioner-half
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
                    echo safix-fixture-clan-half
                  ''
                // lib.optionalAttrs (config.system ? dryActivationScript) {
                  supportsDryActivation = true;
                };
            }
          )
        ];
      };

      scripts = mergedFixture.config.system.activationScripts;

      # Read with a fallback rather than directly, so the rename drill turns
      # the assertions below red as a diff instead of an evaluation error.
      mergedStep =
        scripts.setupSecrets or {
          text = "";
          deps = [ ];
        };

      structural = {
        actual = {
          storeRootOptions = {
            nixos = builtins.filter namesAStoreRoot (optionPaths [ "sops" ] providerFixture.options.sops);
            homeManager = builtins.filter namesAStoreRoot (
              optionPaths [ "sops" ] homeManagerInstrument.options.sops
            );
          };

          merge = {
            nodes = builtins.filter (n: lib.hasInfix "etupSecrets" n) (builtins.attrNames scripts);
            provisionerHalf = lib.hasInfix "safix-fixture-provisioner-half" mergedStep.text;
            clanHalf = lib.hasInfix "safix-fixture-clan-half" mergedStep.text;
            depsUnion = lib.sort (a: b: a < b) (lib.unique mergedStep.deps);
          };

          wrapper = {
            trapsErr = lib.hasInfix "trap \"_status=1 _localstatus=\\$?\" ERR" scripts.script;
            setELines = builtins.filter (l: builtins.match "[[:space:]]*set -e.*" l != null) (
              lib.splitString "\n" scripts.script
            );
          };
        };

        expected = {
          storeRootOptions = {
            nixos = [ ];
            homeManager = [
              "sops.defaultSecretsMountPoint"
              "sops.defaultSymlinkPath"
            ];
          };

          merge = {
            nodes = [ "setupSecrets" ];
            provisionerHalf = true;
            clanHalf = true;
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

      # A system configuration through the exported module, resolving bob on
      # the fixture fleet's server, the same subject `safix-consumption-system`
      # reads. `validateSopsFiles` is off because the fleet's sops files are
      # paths into this flake that no committed file backs, and both builders
      # below mirror the same conditional, so the comparison stays over one
      # fixture.
      manifestFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            sops.validateSopsFiles = false;
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
            };
          }
        ];
      };

      safixManifest = manifestFixture.config.system.build.safix-manifest;

      # The provisioner's own builder from `inputs.sops-nix`, over the same
      # fixture's cfg and the same typed entries, so the key-set comparison
      # below is between two builders and one input.
      parityManifest =
        (pkgs.callPackage "${inputs.sops-nix}/modules/sops/manifest-for.nix" {
          cfg = manifestFixture.config.sops;
        })
          "-safix-parity"
          manifestFixture.config.safix.installed
          { }
          { };

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
            sops.validateSopsFiles = false;
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              installer = {
                secretsMountPoint = "/run/safix-moved.d";
                symlinkPath = "/run/safix-moved";
              };
            };
          }
        ];
      };

      movedManifest = movedFixture.config.system.build.safix-manifest;

      # name -> the path the manifest must carry, read off the pre-typed
      # resolution: an entry that declared a path keeps it, and a path-less
      # entry parks under the fixture's own symlink path. Holding both against
      # the built manifest in one map is what makes the root and the entry
      # default one claim rather than two.
      entryPathContract =
        fixture:
        lib.mapAttrs (
          name: entry: entry.path or "${fixture.config.safix.installer.symlinkPath}/${name}"
        ) fixture.config.safix.secrets;

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

      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # A host that carries a foreign store's activation step under the name
      # both colliding packages use, and names it through safix's option.
      orderedActivationFixture = inputs.nixpkgs.lib.nixosSystem {
        modules = [
          config.flake.nixosModules.default
          {
            nixpkgs.hostPlatform = system;
            networking.hostName = "server";
            system.stateVersion = "24.05";
            sops.validateSopsFiles = false;
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
              installer.afterActivation = [ "setupSecrets" ];
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
            sops.validateSopsFiles = false;
            services.userborn.enable = true;
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
              identity.sshKeyPaths = [ "/etc/ssh/safix-fixture-identity" ];
              installer.afterUnits = [ "age-decrypt-secrets.service" ];
            };
          }
        ];
      };

      orderingFacts =
        let
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
              foreignStepNotMerged = !(lib.hasInfix "sops-install-secrets" activationScripts.setupSecrets.text);
              deps = lib.sort (a: b: a < b) (lib.unique safixStep.deps);
              runsTheInstaller = lib.hasInfix "safix-install-secrets" safixStep.text;
              noUnit = orderedActivationFixture.config.systemd.services ? safix-install-secrets;
            };

            unit = {
              exists = orderedUnitFixture.config.systemd.services ? safix-install-secrets;
              afterNamedUnit = builtins.elem "age-decrypt-secrets.service" unit.after;
              wantedBySysinit = builtins.elem "sysinit.target" unit.wantedBy;
              rerunsOnSwitch =
                builtins.elem "sysinit-reactivation.target" unit.requiredBy
                && builtins.elem "sysinit-reactivation.target" unit.before;
              runsTheInstaller = lib.any (lib.hasInfix "safix-install-secrets") (
                lib.toList unit.serviceConfig.ExecStart
              );
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
          };
        };

      systemCommon = import ../../consume/common.nix {
        inherit lib;
        scope = "system";
      };

      # The identity fixtures share the bob resolution and vary only what the
      # derivation reads: where the host's keys lie, whether an identity is
      # named, and whether the switch is on. `services.openssh.enable` is on
      # because the derivation mirrors the provisioner's whole rule, which
      # derives nothing from a host whose keys openssh does not manage.
      identityFixture =
        extra:
        inputs.nixpkgs.lib.nixosSystem {
          modules = [
            config.flake.nixosModules.default
            {
              nixpkgs.hostPlatform = system;
              networking.hostName = "server";
              system.stateVersion = "24.05";
              sops.validateSopsFiles = false;
              services.openssh.enable = true;
              safix = {
                lib = config.flake.safix.lib;
                user = "bob";
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

      names = tokens: messages: builtins.all (t: lib.any (m: lib.hasInfix t m) messages) tokens;

      # An enabled configuration resolving nothing, with one entry injected
      # into the typed set directly, so each refusal is measured on exactly the
      # entry that earns it rather than beside the fleet's. In pure evaluation
      # an out-of-store path is also unreadable, so the outside-store entry
      # trips both halves of the copied block; the store-membership half is
      # still the one the removal drill isolates, because without the block the
      # failure is the forced `builtins.hashFile` refusing the path — a hard
      # evaluation error that is not safix's message and that `tryEval` cannot
      # catch, which is what these fixtures would surface if the copy were
      # dropped.
      refusalFixtureWith =
        entry:
        inputs.nixpkgs.lib.nixosSystem {
          modules = [
            config.flake.nixosModules.default
            {
              nixpkgs.hostPlatform = system;
              system.stateVersion = "24.05";
              safix.enable = true;
              safix.installed = entry;
            }
          ];
        };

      outsideStoreFixture = refusalFixtureWith {
        outside.sopsFile = "/etc/hosts";
      };

      missingFileFixture = refusalFixtureWith {
        missing.sopsFile = "${builtins.storeDir}/safix-fixture-absent/nope.yaml";
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
            sops.validateSopsFiles = false;
            safix = {
              lib = config.flake.safix.lib;
              user = "bob";
            };
          }
        ];
      };

      refusalsFacts =
        let
          installerText = manifestFixture.config.system.build.safix-installer.text;
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
            };
          };
        };

      soleFacts =
        let
          scripts = manifestFixture.config.system.activationScripts;
          services = manifestFixture.config.systemd.services;

          # The full script the option's `apply` assembles aggregates every
          # step, so it is excluded by name; a step registered as a bare string
          # is its own text.
          textOf = v: if lib.isAttrs v then v.text else v;

          invocationsIn =
            set: extract:
            lib.sort (a: b: a < b) (
              builtins.attrNames (lib.filterAttrs (n: v: n != "script" && extract v) set)
            );
        in
        {
          actual = {
            # The fixture resolves the four entries `safix-consumption-system`
            # also reads, so the inertness below is evidence rather than an
            # empty resolution passing vacuously.
            established = lib.sort (a: b: a < b) (builtins.attrNames manifestFixture.config.safix.installed);

            provisionerInert = {
              secretsOption = manifestFixture.config.sops.secrets;
              activationStep = scripts ? setupSecrets;
              unit = services ? sops-install-secrets;
            };

            # Exactly one invocation of the installer binary, and it is
            # safix's: every activation step's text and every unit's ExecStart
            # is scanned, so a second invocation appearing anywhere reddens
            # this rather than only the two names the provisioner uses.
            invocations = {
              activation = invocationsIn scripts (v: lib.hasInfix "install-secrets" (textOf v));
              units = invocationsIn services (
                v: lib.any (lib.hasInfix "install-secrets") (lib.toList (v.serviceConfig.ExecStart or [ ]))
              );
            };

            refusals = {
              outsideStore = {
                refuses = fires outsideStoreFixture.config.system.build.safix-manifest;
                namesTheStore =
                  names
                    [
                      "is not in the Nix store"
                      "sops.validateSopsFiles"
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
                        file = "${builtins.storeDir}/safix-fixture-absent/nope.yaml";
                      })
                    ];
              };
            };
          };

          expected = {
            established = [
              "bob-service"
              "ops-handover"
              "ops-tooling"
              "team-vault"
            ];

            provisionerInert = {
              secretsOption = { };
              activationStep = false;
              unit = false;
            };

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
          };
        };
    in
    {
      # Every claim here evaluates a NixOS system configuration, so the check
      # exists only where one does, following `safix-consumption-system`.
      checks = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-installer-mechanism =
          pkgs.runCommand "safix-installer-mechanism"
            {
              actualJson = builtins.toJSON structural.actual;
              expectedJson = builtins.toJSON structural.expected;
              passAsFile = [
                "actualJson"
                "expectedJson"
              ];
              relocatedManifest = relocatedManifest;
              usersManifest = usersManifest;
              nativeBuildInputs = [ pkgs.jq ];
              meta.description = "the manifest-root mechanism, the absent option surface, and the setupSecrets merge";
            }
            ''
              if ! diff -u "$expectedJsonPath" "$actualJsonPath"; then
                echo ""
                echo "safix-installer-mechanism: evaluated facts differ from expected"
                exit 1
              fi

              root() { jq -r ".$2" "$1"; }

              for manifest in "$relocatedManifest" "$usersManifest"; do
                for field in secretsMountPoint symlinkPath; do
                  case "$(root "$manifest" "$field")" in
                    /run/secrets.d | /run/secrets)
                      echo "safix-installer-mechanism: $manifest carries the hardcoded $field"
                      exit 1
                      ;;
                  esac
                done
              done

              [ "$(root "$relocatedManifest" secretsMountPoint)" = /run/safix-mechanism-probe.d ] || {
                echo "safix-installer-mechanism: extraJson did not relocate secretsMountPoint"
                exit 1
              }
              [ "$(root "$relocatedManifest" symlinkPath)" = /run/safix-mechanism-probe ] || {
                echo "safix-installer-mechanism: extraJson did not relocate symlinkPath"
                exit 1
              }

              [ "$(root "$usersManifest" secretsMountPoint)" = /run/secrets-for-users.d ] || {
                echo "safix-installer-mechanism: secrets-for-users did not relocate secretsMountPoint"
                exit 1
              }
              [ "$(root "$usersManifest" symlinkPath)" = /run/secrets-for-users ] || {
                echo "safix-installer-mechanism: secrets-for-users did not relocate symlinkPath"
                exit 1
              }

              touch $out
            '';

        safix-installer-manifest =
          pkgs.runCommand "safix-installer-manifest"
            {
              nativeBuildInputs = [ pkgs.jq ];
              safixManifest = safixManifest;
              parityManifest = parityManifest;
              meta.description = "the built manifest's roots and its field-set parity with the provisioner's builder";
            }
            ''
              jq empty "$safixManifest"

              [ "$(jq '.secrets | length' "$safixManifest")" -gt 0 ] || {
                echo "safix-installer-manifest: the fixture resolved nothing, so nothing below is evidence"
                exit 1
              }

              [ "$(jq -r .secretsMountPoint "$safixManifest")" = /run/safix.d ] || {
                echo "safix-installer-manifest: secretsMountPoint is not safix's own"
                exit 1
              }
              [ "$(jq -r .symlinkPath "$safixManifest")" = /run/safix ] || {
                echo "safix-installer-manifest: symlinkPath is not safix's own"
                exit 1
              }
              [ "$(jq -r .userMode "$safixManifest")" = false ] || {
                echo "safix-installer-manifest: userMode is not false"
                exit 1
              }

              if ! diff -u <(jq -S keys "$parityManifest") <(jq -S keys "$safixManifest"); then
                echo ""
                echo "safix-installer-manifest: manifest key sets differ from the provisioner's builder"
                exit 1
              fi
              if ! diff -u <(jq -S '.logging | keys' "$parityManifest") <(jq -S '.logging | keys' "$safixManifest"); then
                echo ""
                echo "safix-installer-manifest: logging key sets differ from the provisioner's builder"
                exit 1
              fi

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
        # against `sops.age.sshKeyPaths`, so the claim is about what the
        # binary will read rather than about an intermediate option.
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
      };
    };
}
