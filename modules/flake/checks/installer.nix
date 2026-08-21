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
# silent-write combination the map exists to forbid.
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
      };
    };
}
