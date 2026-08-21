# Holds the consumption modules to the wiring they replace, judged by evaluating
# real profiles rather than the modules on their own.
#
# ── the equivalence ──
# The claim is that a profile naming a person and a host establishes exactly what
# the hand-written wiring establishes. So two home-manager configurations are
# evaluated over one fixture fleet: one in the consumer form — import, `safix.lib`,
# `safix.user`, `safix.hostname`, `safix.identity.sshKeyPaths` — and one wiring the
# resolver into sops-nix directly, in the shape of the file the module replaces.
# Both are read back through sops-nix's own option types, entry by entry and field
# by field, so a field safix stopped emitting or started emitting shows up.
#
# ── the ordering ──
# The preflight's failure message claims that nothing was linked. That claim is
# entirely a claim about where the entry sorts, so the activation DAG of a real
# profile is topologically sorted and the index held: safix's entry before
# `checkLinkTargets`, sops-nix's own entry after it. The second half is the reason
# the first exists and is asserted rather than described.
#
# ── which person appears at which scope ──
# The system-scope configuration resolves bob rather than alice. alice's
# `alice-alone` declares its path as a function of a home-manager
# configuration — `cfg.home.homeDirectory` — so her set is materializable in a
# profile and not in a system configuration. That is a property of that one
# declaration and the `path`-is-a-function-of-the-consuming-configuration
# contract, not of safix, and it is not assertable here: it surfaces as a
# missing attribute rather than a `throw`, and `builtins.tryEval` catches only
# thrown and asserted errors.
# `selectionIsScopeFree` carries the claim that survives — that what arrives at
# either scope is exactly what the scope-free resolver selected — on each side.
#
# ── severity, one drill per claim, each observed red ──
# A module that establishes anything other than what the resolver selected — the
# drill filters one name out of `sops.secrets` — fails `equivalence`, `established`
# and `selectionIsScopeFree` together.
# `equivalence` alone is deliberately not the whole of it, and the drill that shows
# why is dropping `key` from `materializeFor`: both forms call that function, so
# they still agree, and what goes red is the `entry` literal beside them, which was
# written independently and reads sops-nix's own default for the field that
# vanished. The pair is the claim; either alone is half of it.
# Replacing `entryBefore [ "checkLinkTargets" ]` with a bare string fails
# `safixBeforeCheckLinkTargets`; the sops-nix half of that pair fails if sops-nix
# ever pins its own entry, which is the day this guard stops being needed.
# `inert` is held by two independent gates — `mkIf cfg.enable` outside and
# `sopsCfg.secrets != { }` on the preflight — so removing either alone leaves
# `inert.preflight` green and removing both turns it red. That is the drill, and
# the redundancy is deliberate: the inner gate is what covers a consumer who sets
# `safix.enable = true` by hand over an empty resolution. `inert.identity`
# separates them: it reads the ssh key path the fixture profile names, which only
# the outer gate withholds, so dropping that gate alone turns it red.
# Rewording `missingLibMessage` so it stops naming an option fails
# `flakeWithoutLib.namesTheOptions` while `refuses` stays green, which is the
# point of holding the two apart.
# Dropping the user-scope identity refusal fails `noIdentity.refuses` and
# nothing else, and only because that field is read off a profile evaluated
# without home-manager's assertion wrapper: a wrapped profile refuses under the
# drill too, on sops-nix's key-source assertion, which is the defect the refusal
# exists to pre-empt rather than evidence of it. Making the refusal
# unconditional instead — throwing whenever anything resolved — leaves
# `noIdentity.refuses` green and fails `noIdentity.withIdentity`.
# Dropping the user-scope ownership refusal fails `userScopeRefusesOwnership`, and
# dropping the ownership fields from the system materialization fails
# `systemCarriesOwnership`.
# Moving the wiring assertions back inside the enable gate fails
# `unaddressed.refuses`, which is the whole reason they sit outside it.
# Dropping the flakeless refusal fails `flakeless.refuses` and
# `flakeless.namesTheOption`. Widening it — conditioning it on `cfg.user` rather
# than on a definition of it — leaves both green and fails `unwired`, since the
# user scope defaults that option to the profile's own username and every
# imported-but-unconfigured profile would then be refused. The pair is the
# claim: a state that is silent and a state that must stay silent.
# Removing the membership guard from `selectFor` fails
# `undeclaredUser.refuses`, and only that field: the profile still fails
# either way, but as `attribute 'zed' missing` against a line of resolve.nix,
# which `fires` over safix's own option does not catch and no message names.
# Pointing the second collision copy at the first's path fails `twoPaths`, which
# is the drill for the check that the export shape rests on: it is only evidence
# while the two paths really are two.
{
  config,
  inputs,
  lib,
  self,
  ...
}:
let
  hmLib = inputs.home-manager.lib;

  # Two distinct store paths holding byte-identical content. `builtins.path`
  # names them differently, which is the whole of what makes them distinct: the
  # module system keys a path module on its path, so this is the minimal
  # construction of "the same module, imported from two places".
  copyOf =
    name: path:
    builtins.path {
      inherit name path;
    };

  collisionA = copyOf "safix-collision-a" ./collision-fixture;
  collisionB = copyOf "safix-collision-b" ./collision-fixture;

  # sops-nix's own home-manager module, copied to a second store path. This is
  # what a consumer pinning a different sops-nix revision produces, and it is
  # measured against the real module rather than against the synthetic one above.
  sopsHomeCopy = copyOf "sops-nix-home-copy" "${inputs.sops-nix}/modules/home-manager";

in
{
  perSystem =
    { pkgs, system, ... }:
    let
      safix = config.flake.safix.lib;
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # Whether a set of modules declares its options exactly once, judged by
      # forcing one option's declaration. The forcing is what makes the claim: a
      # duplicate declaration is not detected when the module list is built, only
      # when the option it collides on is merged, so a probe that stopped at
      # `evalModules` would report every list as fine.
      #
      # The modules under test are evaluated alone rather than inside their host
      # module system, so `_module.check = false` admits their definitions of
      # options their host would have declared. None of those are forced.
      declaresOnce =
        { modules, option }:
        (builtins.tryEval (
          let
            evaluated = lib.evalModules {
              modules = modules ++ [
                { _module.check = false; }
                { _module.args.pkgs = pkgs; }
                { _module.args.utils = { }; }
              ];
            };
          in
          builtins.seq (lib.getAttrFromPath option evaluated.options).type true
        )).success;

      hostname = "workstation";

      # Everything a home-manager profile needs before it is a profile at all,
      # and nothing else. The username is the fixture person's, so
      # `safix.user`'s default is exercised by omission in the module form.
      baseProfile = user: {
        home = {
          username = user;
          homeDirectory = "/home/${user}";
          stateVersion = "24.05";
        };
      };

      mkHome =
        user: modules:
        hmLib.homeManagerConfiguration {
          inherit pkgs;
          modules = [ (baseProfile user) ] ++ modules;
        };

      # The consumer form. `safix.user` is deliberately left to its default so
      # that the default is under test alongside the rest.
      moduleForm =
        user:
        mkHome user [
          config.flake.homeModules.default
          {
            safix = {
              lib = safix;
              inherit hostname;
              identity.sshKeyPaths = [ "/home/${user}/.ssh/agenix" ];
            };
          }
        ];

      # The wiring the module replaces, in the shape of the file it replaces:
      # sops-nix imported directly, the identity set on sops-nix's own options,
      # and the resolver called and assigned into `sops.secrets`.
      handForm =
        user:
        mkHome user [
          inputs.sops-nix.homeManagerModules.sops
          (
            { config, ... }:
            {
              sops = {
                age.keyFile = null;
                age.sshKeyPaths = [ "/home/${user}/.ssh/agenix" ];
                secrets = safix.materialize {
                  inherit user hostname;
                  tags = [ ];
                  scope = "user";
                } config;
              };
            }
          )
        ];

      # Every field of every entry as sops-nix's own option type resolved it,
      # with the encrypted file reduced to its path within this flake's source
      # so the comparison is not against a store hash.
      viewOf =
        secrets:
        lib.mapAttrs (
          _name: secret:
          lib.filterAttrs (n: _: n != "_module" && n != "sopsFile") secret
          // {
            sopsFile = lib.removePrefix (toString self) (toString secret.sopsFile);
          }
        ) secrets;

      moduleView = viewOf (moduleForm "alice").config.sops.secrets;
      handView = viewOf (handForm "alice").config.sops.secrets;

      # A person who resolves nothing on this host: carol records a recipient
      # and holds no entry, so every audience excludes them.
      inertProfile = moduleForm "carol";

      activationOrder =
        profile:
        let
          sorted = hmLib.hm.dag.topoSort profile.config.home.activation;
        in
        map (entry: entry.name) sorted.result;

      aliceOrder = activationOrder (moduleForm "alice");

      indexOf = order: name: lib.lists.findFirstIndex (n: n == name) null order;

      before =
        order: a: b:
        let
          ia = indexOf order a;
          ib = indexOf order b;
        in
        ia != null && ib != null && ia < ib;

      fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

      # bob declares ownership fields, which the user scope has no axis for.
      bobUserProfile = (moduleForm "bob").config.sops.secrets;

      names = tokens: messages: builtins.all (t: lib.any (m: lib.hasInfix t m) messages) tokens;

      # A profile bound to declarations and given no host. Standalone
      # home-manager cannot derive one, so this is the mistake a consumer
      # actually makes.
      unaddressedProfile = mkHome "alice" [
        config.flake.homeModules.default
        { safix.lib = safix; }
      ];

      # A profile that imports the module and says nothing else, which must stay
      # a no-op rather than become a demand.
      unwiredProfile = mkHome "alice" [ config.flake.homeModules.default ];

      # A profile that names a person and is bound to nothing, which is what
      # omitting `safix.flake` produces. It is the state that used to be silent:
      # `safix.lib` null makes every other assertion vacuously true and the
      # resolved set empty, so the profile built and established nothing.
      flakelessProfile = mkHome "alice" [
        config.flake.homeModules.default
        { safix.user = "alice"; }
      ];

      # The messages a mis-wired profile prints. Read off the pure function
      # rather than off an evaluated profile, because home-manager's
      # `homeManagerConfiguration` collects failed assertions and throws the
      # whole configuration, so a profile whose assertion fires has no readable
      # `config.assertions` left. The evaluated profile carries the other half of
      # the claim — that it refuses at all — through `fires`.
      failedMessages =
        common: args: map (a: a.message) (lib.filter (a: !a.assertion) (common.assertionsFor args));

      # The module's own view of a projection that reports violations. The list
      # is substituted rather than produced by breaking the fleet, because the
      # claim under test belongs to the module — that it reports all of them,
      # itself — and not to the resolver, which has its own drills.
      brokenBinding = {
        lib = safix // {
          violations = [
            "flake.safix.users.alice.sharedWith names 'dz', which is not a declared user"
            "flake.safix.users.bob.recipient is null"
          ];
        };
        user = "alice";
        machine = null;
        inherit hostname;
        tags = [ ];
      };

      # safix's own module and sops-nix's own module, evaluated without
      # home-manager's assertion wrapper.
      #
      # The wrapper is what makes this instrument necessary rather than
      # fussy. `homeManagerConfiguration` forces `config.assertions` on any
      # access to `config` and throws every failed one together, so a profile
      # evaluated through it reports that something refused and never which
      # module refused — sops-nix's key-source assertion and safix's identity
      # refusal are one observation there, and `builtins.tryEval` reports
      # neither's text. Forcing safix's own option here makes the refusal
      # safix's by construction.
      #
      # `_module.check = false` admits the home-manager option paths the module
      # defines and this evaluation does not declare; none of them are forced.
      bareProfile =
        user: extra:
        lib.evalModules {
          modules = [
            ../../consume/home.nix
            inputs.sops-nix.homeManagerModules.sops
            {
              options.home = {
                username = lib.mkOption { type = lib.types.str; };
                homeDirectory = lib.mkOption { type = lib.types.str; };
              };
              config._module = {
                check = false;
                args = {
                  inherit pkgs;

                  # Standalone home-manager has no host configuration, which is
                  # the case this instrument stands in for. It is named because
                  # the module system supplies every formal argument of a path
                  # module from `_module.args`, so a formal with a default is
                  # still an error when forced and unnamed.
                  osConfig = null;
                };
              };
            }
            {
              home = {
                username = user;
                homeDirectory = "/home/${user}";
              };
              safix = {
                lib = safix;
                inherit hostname;
              };
            }
          ]
          ++ extra;
        };

      # The state the README's three-line user-scope form used to produce: a
      # person whose declarations resolve, on a profile that names no identity.
      identityFreeProfile = bareProfile "alice" [ ];

      identityGivenProfile = bareProfile "alice" [
        { safix.identity.sshKeyPaths = [ "/home/alice/.ssh/agenix" ]; }
      ];

      # A person nobody declared. `safix.user` defaults to the profile's own
      # username, so this is what any operating-system account whose name
      # differs from its declaration key produces, and the identity is named so
      # that this fixture tests one thing.
      undeclaredUserProfile = bareProfile "zed" [
        { safix.identity.sshKeyPaths = [ "/home/zed/.ssh/agenix" ]; }
      ];

      resolve = import ../safix/resolve.nix { inherit lib; };

      homeCommon = import ../../consume/common.nix {
        inherit lib;
        scope = "user";
      };

      systemCommon = import ../../consume/common.nix {
        inherit lib;
        scope = "system";
      };

      nixosFor =
        user:
        inputs.nixpkgs.lib.nixosSystem {
          modules = [
            config.flake.nixosModules.default
            {
              nixpkgs.hostPlatform = system;
              networking.hostName = "server";
              system.stateVersion = "24.05";

              # The identity the fixture decrypts with is the derived one: with
              # openssh managing host keys outside safix's store, the system
              # scope needs no named identity, which is the arrangement the
              # README documents.
              services.openssh.enable = true;
              safix = {
                lib = safix;
                inherit user;
              };
            }
          ];
        };

      # Read off `safix.installed` rather than `sops.secrets`: the system scope
      # no longer delivers through the provisioner's option, which safix leaves
      # empty so that exactly one installer — safix's own — acts on the
      # resolved set.
      systemView = viewOf (nixosFor "bob").config.safix.installed;

      sortNames = lib.sort (a: b: a < b);
    in
    {
      # The NixOS half evaluates a system configuration, which only resolves on a
      # Linux host platform; the home-manager half holds on every system safix
      # supports.
      checks =
        lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          # Scope is a property of the module and of nothing a consumer declares,
          # so one fleet reaches both, and the ownership axis the system scope has
          # and the user scope does not is read off the arrival rather than
          # restated.
          safix-consumption-system = mkStructuralCheck {
            name = "safix-consumption-system";
            actual = {
              established = sortNames (builtins.attrNames systemView);
              systemCarriesOwnership = lib.getAttrs [
                "owner"
                "group"
                "mode"
              ] systemView.bob-service;
              hostnameFromTheHost = (nixosFor "bob").config.safix.hostname;

              # Selection is custody and custody has no scope: what arrives at the
              # system scope is exactly what the scope-free resolver selected.
              # The home side of the same claim is in `safix-consumption`.
              selectionIsScopeFree =
                sortNames (
                  safix.resolveNames {
                    user = "bob";
                    hostname = "server";
                    tags = [ ];
                  }
                ) == sortNames (builtins.attrNames systemView);
            };
            expected = {
              established = [
                "bob-service"
                "ops-handover"
                "ops-tooling"
                "team-vault"
              ];
              systemCarriesOwnership = {
                owner = "bob";
                group = "staff";
                mode = "0400";
              };
              hostnameFromTheHost = "server";
              selectionIsScopeFree = true;
            };
          };
        }
        // {
          # The fact both export forms exist for. Asserted about the module system
          # rather than about safix, because that is where it lives, and about
          # sops-nix's real module as well as a synthetic one, because the
          # synthetic one could agree with a module system that had grown a
          # special case for large modules.
          safix-module-collision = mkStructuralCheck {
            name = "safix-module-collision";
            actual = {
              samePathTwice = declaresOnce {
                option = [
                  "safixCollisionFixture"
                  "thing"
                ];
                modules = [
                  "${collisionA}/declaring-module.nix"
                  "${collisionA}/declaring-module.nix"
                ];
              };
              twoPaths = declaresOnce {
                option = [
                  "safixCollisionFixture"
                  "thing"
                ];
                modules = [
                  "${collisionA}/declaring-module.nix"
                  "${collisionB}/declaring-module.nix"
                ];
              };

              provisionerSamePathTwice = declaresOnce {
                option = [
                  "sops"
                  "defaultSopsFormat"
                ];
                modules = [
                  inputs.sops-nix.homeManagerModules.sops
                  inputs.sops-nix.homeManagerModules.sops
                ];
              };
              provisionerTwoPaths = declaresOnce {
                option = [
                  "sops"
                  "defaultSopsFormat"
                ];
                modules = [
                  inputs.sops-nix.homeManagerModules.sops
                  "${sopsHomeCopy}/sops.nix"
                ];
              };
            };
            expected = {
              samePathTwice = true;
              twoPaths = false;
              provisionerSamePathTwice = true;
              provisionerTwoPaths = false;
            };
          };

          safix-consumption = mkStructuralCheck {
            name = "safix-consumption";
            actual = {
              # The consumer form and the wiring it replaces, entry by entry and
              # field by field, through sops-nix's own option types.
              equivalence = moduleView == handView;
              established = sortNames (builtins.attrNames moduleView);
              entry = moduleView.alice-alone;

              # Selection is custody and custody has no scope: what arrived in the
              # profile is exactly what the scope-free resolver selected.
              selectionIsScopeFree =
                sortNames (
                  safix.resolveNames {
                    user = "alice";
                    inherit hostname;
                    tags = [ ];
                  }
                ) == sortNames (builtins.attrNames moduleView);

              # A person who resolves nothing defines nothing.
              #
              # `identity` is the severe half. The fixture profile names an ssh
              # key path, and sops-nix's own defaults for both fields are the
              # empty ones, so these read as untouched only while the enable gate
              # holds — dropping it puts the named path here.
              inert = {
                secrets = inertProfile.config.sops.secrets;
                preflight = inertProfile.config.home.activation ? safixIdentityPreflight;
                unit = inertProfile.config.systemd.user.services ? sops-nix;
                evaluates = builtins.isAttrs inertProfile.config.home.activation;
                identity = {
                  keyFile = inertProfile.config.sops.age.keyFile;
                  sshKeyPaths = inertProfile.config.sops.age.sshKeyPaths;
                };
              };

              # The user-scope half of the ownership asymmetry. The system half is
              # in `safix-consumption-system`, which only a Linux builder can
              # evaluate.
              userScopeRefusesOwnership = fires bobUserProfile;
            };

            expected = {
              equivalence = true;
              established = [
                "alice-alone"
                "api-token"
                "corp-handover"
                "corp-token"
                "ops-handover"
                "ops-tooling"
                "team-vault"
                "web-token"
                "wg-private"
              ];
              selectionIsScopeFree = true;
              entry = {
                format = "yaml";
                key = "alice_alone";
                mode = "0440";
                name = "alice-alone";
                path = "/home/alice/.config/safix-fixture/alice-alone";
                sopsFile = "/secrets/safix/users/alice/secrets.yaml";
              };

              inert = {
                secrets = { };
                preflight = false;
                unit = false;
                evaluates = true;
                identity = {
                  keyFile = null;
                  sshKeyPaths = [ ];
                };
              };

              userScopeRefusesOwnership = true;
            };
          };

          # Every refusal a mis-wired profile can produce, held as a message rather
          # than as a failure: what a consumer sees is the whole value of these,
          # and `builtins.tryEval` reports that something threw and never what it
          # said.
          safix-consumption-refusals = mkStructuralCheck {
            name = "safix-consumption-refusals";
            actual = {
              # Bound and unaddressed: the profile refuses, and the message names
              # the option that is unset. The assertion has to sit outside the
              # enable gate for this to happen at all — an unset host is exactly
              # what produces the empty resolution that turns `enable` off.
              unaddressed = {
                # The access is narrow on purpose. `fires` over the whole
                # configuration would be tautologically true: deep-forcing a
                # home-manager configuration reaches options no fixture profile
                # defines, and would report a refusal on every profile.
                refuses = fires unaddressedProfile.config.sops.secrets;
                namesTheOption = names [ "safix.hostname" ] (
                  failedMessages homeCommon {
                    configured = true;
                    cfg = {
                      lib = safix;
                      user = "alice";
                      machine = null;
                      hostname = null;
                    };
                  }
                );
              };

              # The system scope's own unset-person case, which the home scope
              # cannot have: there `safix.user` defaults from the profile's
              # username, and here there is no person to default from.
              unnamedPerson = names [ "safix.user" ] (
                failedMessages systemCommon {
                  configured = true;
                  cfg = {
                    lib = safix;
                    user = null;
                    machine = null;
                    hostname = "server";
                  };
                }
              );

              # A profile that imports the module and says nothing is a no-op,
              # not a demand. This is the half the flakeless refusal below must
              # not swallow: both have a null `safix.lib`, and only a definition
              # of `safix.user` or `safix.hostname` separates them.
              unwired = {
                quiet =
                  failedMessages homeCommon {
                    configured = false;
                    cfg = {
                      lib = null;
                      user = null;
                      machine = null;
                      hostname = null;
                    };
                  } == [ ];
                establishesNothing = unwiredProfile.config.sops.secrets == { };
                enable = unwiredProfile.config.safix.enable;
              };

              # Configured and bound to nothing: `safix.flake` omitted. The
              # profile refuses rather than building an empty resolution in
              # silence, and the message names the option that supplies the
              # binding.
              flakeless = {
                refuses = fires flakelessProfile.config.sops.secrets;
                namesTheOption =
                  names
                    [
                      "safix.flake"
                      "safix.lib"
                    ]
                    (
                      failedMessages homeCommon {
                        configured = true;
                        cfg = {
                          lib = null;
                          user = "alice";
                          machine = null;
                          hostname = null;
                        };
                      }
                    );
              };

              # A correctly bound profile asserts nothing.
              boundProfileIsQuiet =
                failedMessages homeCommon {
                  configured = true;
                  cfg = {
                    lib = safix;
                    user = "alice";
                    machine = null;
                    inherit hostname;
                  };
                } == [ ];

              # safix.flake pointed at something carrying no projection. The
              # message is held beside the refusal, and read off the named
              # string rather than off the throw, because `builtins.tryEval`
              # reports that something fired and never what it said.
              flakeWithoutLib = {
                refuses =
                  fires
                    (mkHome "alice" [
                      config.flake.homeModules.default
                      { safix.flake = { }; }
                    ]).config.safix.lib;
                namesTheOptions =
                  names
                    [
                      "safix.flake"
                      "safix.lib"
                    ]
                    [ homeCommon.missingLibMessage ];
              };

              # A profile whose declarations resolve and which names no
              # identity. `refuses` is read off the bare instrument rather than
              # off a home-manager profile deliberately: the profile refuses
              # either way — sops-nix's key-source assertion fires when safix's
              # throw does not — so a `fires` over the wrapped profile is green
              # under the drill that removes safix's guard. Forcing safix's own
              # option is what attributes the refusal, and `withIdentity` is
              # what says the guard reads the identity rather than refusing
              # every profile that resolves anything.
              noIdentity = {
                refuses = fires identityFreeProfile.config.safix.secrets;
                namesTheOptions =
                  names
                    [
                      "safix.identity.keyFile"
                      "safix.identity.sshKeyPaths"
                    ]
                    [
                      (homeCommon.noIdentityMessage {
                        cfg = {
                          user = "alice";
                          machine = null;
                          inherit hostname;
                        };
                        resolved.alice-alone = { };
                      })
                    ];
                withIdentity = sortNames (builtins.attrNames identityGivenProfile.config.safix.secrets);
              };

              # A person nobody declared. Held here as well as in
              # `safix-custody` because the two say different things: that one
              # says the resolver refuses, and this one says the refusal reaches
              # a profile through the module rather than surfacing as a missing
              # attribute against a line of resolve.nix.
              undeclaredUser = {
                refuses = fires undeclaredUserProfile.config.safix.secrets;
                namesTheDeclaredUsers =
                  names
                    [
                      "'zed' is not a declared user of flake.safix.users"
                      "safix.user"
                      "  - alice\n"
                      "  - bob\n"
                      "  - carol\n"
                    ]
                    [ (resolve.unknownUserMessage config.flake.safix.users "zed") ];
              };

              # Violations are reported together, by safix, naming the namespace
              # they belong to.
              violations = {
                refuses = fires (
                  homeCommon.resolvedFor {
                    cfg = brokenBinding;
                    target = { };
                  }
                );
                message =
                  names
                    [
                      "safix"
                      "flake.safix.users.alice.sharedWith names 'dz'"
                      "flake.safix.users.bob.recipient is null"
                    ]
                    [ (homeCommon.violationMessage brokenBinding) ];
              };
            };

            expected = {
              unaddressed = {
                refuses = true;
                namesTheOption = true;
              };
              unnamedPerson = true;
              unwired = {
                quiet = true;
                establishesNothing = true;
                enable = false;
              };
              flakeless = {
                refuses = true;
                namesTheOption = true;
              };
              boundProfileIsQuiet = true;
              flakeWithoutLib = {
                refuses = true;
                namesTheOptions = true;
              };
              noIdentity = {
                refuses = true;
                namesTheOptions = true;
                withIdentity = [
                  "alice-alone"
                  "api-token"
                  "corp-handover"
                  "corp-token"
                  "ops-handover"
                  "ops-tooling"
                  "team-vault"
                  "web-token"
                  "wg-private"
                ];
              };
              undeclaredUser = {
                refuses = true;
                namesTheDeclaredUsers = true;
              };
              violations = {
                refuses = true;
                message = true;
              };
            };
          };

          safix-consumption-ordering = mkStructuralCheck {
            name = "safix-consumption-ordering";
            actual = {
              present = builtins.elem "safixIdentityPreflight" aliceOrder;

              # The guarantee the preflight's own message rests on.
              safixBeforeCheckLinkTargets = before aliceOrder "safixIdentityPreflight" "checkLinkTargets";

              # The reason it has to exist: sops-nix's entry is registered as a
              # bare string, so it sorts wherever the DAG puts it, which is after
              # the point at which a refusal would still be atomic.
              provisionerAfterCheckLinkTargets = before aliceOrder "checkLinkTargets" "sops-nix";

              # The stronger fact home.nix's non-atomicity prose rests on: the
              # provisioner runs after the generation is linked, so a failure
              # there is loud but late.
              provisionerAfterLinkGeneration = before aliceOrder "linkGeneration" "sops-nix";

              # Read, do not decrypt: the script names each configured identity and
              # exits non-zero, and it invokes no decryptor. `runsTheDecryptor`
              # matches the binary in the closure rather than the program's name,
              # because the name appears in the remediation prose — which is the
              # other claim here: the message narrows itself to what it checked
              # rather than implying that a readable identity can open the files.
              script =
                let
                  text = (moduleForm "alice").config.home.activation.safixIdentityPreflight.data;
                in
                {
                  namesTheIdentity = lib.hasInfix "/home/alice/.ssh/agenix" text;
                  refuses = lib.hasInfix "exit 1" text;
                  runsTheDecryptor = lib.hasInfix "bin/sops-install-secrets" text;
                  statesItsLimit = lib.hasInfix "readability" text && lib.hasInfix "not a recipient" text;
                };
            }
            # The same claim about the systemd daemon reload is linux's alone.
            # home-manager registers `reloadSystemd` from its `systemd.user`
            # module, and the DAG this check reads on aarch64-darwin is
            # `checkFilesChanged checkLinkTargets writeBoundary installPackages
            # linkGeneration onFilesChange setupLaunchAgents` — read off the
            # pinned home-manager on that platform — and carries no such entry.
            # `before` answers false for a name it cannot find, so asserting the
            # ordering there is asserting a step's absence rather than its
            # position. darwin's analogue is `setupLaunchAgents`, and the claim is
            # not moved onto it, because where sops-nix's entry sorts against that
            # one has not been established.
            // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
              provisionerAfterReloadSystemd = before aliceOrder "reloadSystemd" "sops-nix";
            };

            expected = {
              present = true;
              safixBeforeCheckLinkTargets = true;
              provisionerAfterCheckLinkTargets = true;
              provisionerAfterLinkGeneration = true;
              script = {
                namesTheIdentity = true;
                refuses = true;
                runsTheDecryptor = false;
                statesItsLimit = true;
              };
            }
            // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
              provisionerAfterReloadSystemd = true;
            };
          };
        };
    };
}
