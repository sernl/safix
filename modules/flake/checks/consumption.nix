# Holds the consumption modules to the wiring they replace, judged by evaluating
# real profiles rather than the modules on their own.
#
# ── the equivalence ──
# The claim is that a profile naming a person and a host establishes exactly what
# the hand-written wiring establishes. So two home-manager configurations are
# evaluated over one fixture fleet: one in the consumer form — import, `safix.lib`,
# `safix.user`, `safix.hostname`, `safix.identity.sshKeyPaths` — and one calling
# the resolver by hand against the same profile, in the shape a consumer would
# write it. Both are read back through safix's own entry type, entry by entry and
# field by field, so a field safix stopped emitting or started emitting shows up,
# and so does a default the type stopped carrying.
#
# The premise moved with the dependency. It used to be "the module form equals
# the sops-nix wiring", read back through that framework's option types; it is
# now "the module form equals the resolver output read back through safix's own
# entry type", which is the only type either side passes through.
#
# ── the ordering ──
# The preflight's failure message claims that nothing was linked. That claim is
# entirely a claim about where the entry sorts, so the activation DAG of a real
# profile is topologically sorted and the index held: safix's preflight before
# `checkLinkTargets`, and safix's own install entry after `writeBoundary` and so
# after `checkLinkTargets`. The second half is the reason the first exists and is
# asserted rather than described — the install is late, and a refusal that waited
# for it would be a refusal after the profile had been linked.
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
# drill filters one name out of `safix.secrets`' definition — fails
# `equivalence`, `established` and `selectionIsScopeFree` together.
# `equivalence` alone is deliberately not the whole of it, and the drill that shows
# why is dropping `key` from `materializeFor`: both forms call that function, so
# they still agree, and what goes red is the `entry` literal beside them, which was
# written independently and reads safix's own declared default for the field that
# vanished. The pair is the claim; either alone is half of it. That is also the
# drill the re-point had to survive: the hand form now goes through the same type
# the module form does, so `equivalence` is vacuous under a `materializeFor`
# mutation by construction, and the literal is what is not.
# Replacing `entryBefore [ "checkLinkTargets" ]` with a bare string fails
# `safixBeforeCheckLinkTargets`; registering `safixInstall` as a bare string
# rather than `entryAfter [ "writeBoundary" ]` fails `installAfterWriteBoundary`,
# which is the half that says the install is late on purpose.
# `inert` is held by two independent gates — `mkIf cfg.enable` outside and
# `cfg.secrets != { }` on the preflight — so removing either alone leaves
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
# drill too, on the module's own assertion collection, which is the defect the
# refusal exists to pre-empt rather than evidence of it. Making the refusal
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
# Pointing the second collision copy at the first's path fails `twoPaths` and
# `declaringModule.twoPaths`, which is the drill for the check that the export
# shape rests on: it is only evidence while the two paths really are two.
# The system-scope inert probes have one drill per mechanism, each moving the
# real definition outside the enable gate while keeping its own selection
# condition: the ungated activation entry fails `inert.activationHost.step`
# alone — the userborn host stays green because its selection would pick the
# unit — and the ungated unit fails `inert.systemdHost.unit` alone, and
# neither reddens anything else in the suite, which is the evidence the gate
# is what holds the requirement rather than an accident of the fixture.
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

  # safix's own consumption directory, copied to a second store path. This is
  # what a consumer reaching one declaring module by two routes produces — a
  # vendored copy beside a flake input, two flake inputs at different
  # revisions — and it is the module the collision fact now has to be about,
  # since safix is the only declaring module either published name carries.
  # The whole directory is copied rather than the one file, because
  # `nixos.nix` imports `./installer.nix` relatively.
  consumeCopyA = copyOf "safix-consume-a" ../../consume;
  consumeCopyB = copyOf "safix-consume-b" ../../consume;

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

      # The wiring the module replaces, in the shape a consumer would write by
      # hand: a plain home-manager profile with no module of safix's in it at
      # all, handed to the resolver directly, with the result left as the
      # resolver produced it. The profile is real rather than an attrset,
      # because `alice-alone` declares its path as a function of
      # `cfg.home.homeDirectory` and that is the configuration it is a
      # function of.
      #
      # There is no second framework to import any more, and the point of the
      # arm is that the module form adds nothing to the resolver's answer
      # rather than that two modules agree.
      handForm =
        user:
        safix.materialize {
          inherit user hostname;
          tags = [ ];
          scope = "user";
        } (mkHome user [ ]).config;

      # The cfg safix's entry type's `path` default is a function of, and the
      # only thing it reads outside its own fields.
      entryCfg.installer.symlinkPath = "/run/safix";

      # Every field of every entry as safix's own entry type resolved it — the
      # type `modules/consume/nixos.nix` hands `common.sharedOptions` as its
      # `secretsType` argument — with the encrypted file reduced to its path
      # within this flake's source so the comparison is not against a store
      # hash.
      #
      # The whole set goes through one `attrsOf` rather than each entry
      # through the submodule alone, because `name`'s default is the attribute
      # key and only `attrsOf` supplies it.
      viewOf =
        secrets:
        lib.mapAttrs
          (
            _name: secret:
            lib.filterAttrs (n: _: n != "_module" && n != "sopsFile") secret
            // {
              sopsFile = lib.removePrefix (toString self) (toString secret.sopsFile);
            }
          )
          (lib.evalModules {
            modules = [
              {
                options.secrets = lib.mkOption {
                  type = lib.types.attrsOf (systemCommon.secretEntryType { cfg = entryCfg; });
                };
              }
              { inherit secrets; }
            ];
          }).config.secrets;

      moduleView = viewOf (moduleForm "alice").config.safix.secrets;
      handView = viewOf (handForm "alice");

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
      bobUserProfile = (moduleForm "bob").config.safix.secrets;

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

      # safix's own module, evaluated without home-manager's assertion
      # wrapper.
      #
      # The wrapper is what makes this instrument necessary rather than
      # fussy. `homeManagerConfiguration` forces `config.assertions` on any
      # access to `config` and throws every failed one together, so a profile
      # evaluated through it reports that something refused and never which
      # assertion refused, and `builtins.tryEval` reports none of their text.
      # Forcing safix's own option here makes the refusal safix's by
      # construction. That was the instrument's reason before the dependency
      # went — where the other observation was a second framework's
      # key-source assertion — and it is its reason still, since safix's own
      # wiring assertions sit outside the enable gate and would be collected
      # together with the identity refusal.
      #
      # `_module.check = false` admits the home-manager option paths the module
      # defines and this evaluation does not declare; none of them are forced.
      bareProfile =
        user: extra:
        lib.evalModules {
          modules = [
            ../../consume/home.nix
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

      nixosWith =
        user: extra:
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
          ]
          ++ extra;
        };

      nixosFor = user: nixosWith user [ ];

      # The system-scope half of the inert claim: carol resolves nothing on
      # this host, and the requirement that a profile resolving nothing
      # defines no activation entry and no unit now governs two new surfaces,
      # `system.activationScripts.safixInstallSecrets` and
      # `systemd.services.safix-install-secrets`. Both hosts are probed
      # because each mechanism only registers where its selection holds: a
      # drill that ungates the unit can only redden a host that would select
      # the unit form, which is what the userborn variant is for.
      inertSystem = nixosFor "carol";
      inertSystemdSystem = nixosWith "carol" [ { services.userborn.enable = true; } ];

      # The system scope delivers through safix's own single option, which is
      # the whole of what it reports arrived, and it is the only option any
      # installer on the host is built from: safix writes no secrets option of
      # any other framework's at all.
      systemView = viewOf (nixosFor "bob").config.safix.secrets;

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

              inert = {
                activationHost = {
                  step = inertSystem.config.system.activationScripts ? safixInstallSecrets;
                  unit = inertSystem.config.systemd.services ? safix-install-secrets;
                };
                systemdHost = {
                  step = inertSystemdSystem.config.system.activationScripts ? safixInstallSecrets;
                  unit = inertSystemdSystem.config.systemd.services ? safix-install-secrets;
                };

                # The identity fields the user-scope probe also reads. Limited
                # severity here, recorded rather than implied: an ungated
                # identity definition writes the same values these defaults
                # already hold — a null keyFile, and the list safix's own
                # host-key derivation produces — so the two mechanism probes
                # above are what carry the requirement. `derivedHostKeys` is
                # where that derivation lives now, and `sshKeyPaths` is the
                # consumer's own list, which this fixture leaves empty.
                identity = {
                  keyFile = inertSystem.config.safix.identity.keyFile;
                  named = inertSystem.config.safix.identity.sshKeyPaths;
                  derived = inertSystem.config.safix.identity.derivedHostKeys;
                };
              };

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

              inert = {
                activationHost = {
                  step = false;
                  unit = false;
                };
                systemdHost = {
                  step = false;
                  unit = false;
                };
                identity = {
                  keyFile = null;
                  named = [ ];
                  derived = [ "/etc/ssh/ssh_host_ed25519_key" ];
                };
              };

              selectionIsScopeFree = true;
            };
          };
        }
        // {
          # The fact the export shape rests on. Asserted about the module system
          # rather than about safix, because that is where it lives, and about
          # safix's own declaring module as well as a synthetic one, because the
          # synthetic one could agree with a module system that had grown a
          # special case for small modules — and because safix's own module is
          # now the only declaring module either published name carries, so it
          # is the one a consumer can import twice.
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

              # safix's own declaring module, reached by two routes. The option
              # forced is one `modules/consume/nixos.nix` declares through
              # `common.sharedOptions`, so the collision is on safix's own
              # declaration rather than on a fixture's.
              declaringModuleSamePathTwice = declaresOnce {
                option = [
                  "safix"
                  "lib"
                ];
                modules = [
                  "${consumeCopyA}/nixos.nix"
                  "${consumeCopyA}/nixos.nix"
                ];
              };
              declaringModuleTwoPaths = declaresOnce {
                option = [
                  "safix"
                  "lib"
                ];
                modules = [
                  "${consumeCopyA}/nixos.nix"
                  "${consumeCopyB}/nixos.nix"
                ];
              };
            };
            expected = {
              samePathTwice = true;
              twoPaths = false;
              declaringModuleSamePathTwice = true;
              declaringModuleTwoPaths = false;
            };
          };

          safix-consumption = mkStructuralCheck {
            name = "safix-consumption";
            actual = {
              # The consumer form and the hand-written wiring it replaces, entry
              # by entry and field by field, through safix's own entry type.
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
              # `install` is the severe half, and it is safix's own definition
              # rather than another framework's option left empty: the entry
              # exists exactly where the resolution is non-empty, so dropping
              # the enable gate puts it on carol's profile. The identity fields
              # are not probed here any more, because safix defines none — they
              # are the consumer's own `safix.identity.*` values and reading
              # them back would assert what the fixture wrote.
              inert = {
                secrets = inertProfile.config.safix.secrets;
                preflight = inertProfile.config.home.activation ? safixIdentityPreflight;
                install = inertProfile.config.home.activation ? safixInstall;
                unit = inertProfile.config.systemd.user.services ? safix;
                evaluates = builtins.isAttrs inertProfile.config.home.activation;
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
              # Safix's own declared defaults, every one of them. The values are
              # unchanged from what a deleted dependency's type used to supply,
              # and their source is not: `format`, `mode`, `owner`, `group`,
              # `uid`, `gid`, `restartUnits` and `reloadUnits` are now
              # `common.secretEntryType`'s own defaults, so a literal that
              # silently agreed with somebody else's default cannot go
              # unnoticed here.
              entry = {
                format = "yaml";
                gid = 0;
                group = null;
                key = "alice_alone";
                mode = "0440";
                name = "alice-alone";
                owner = null;
                path = "/home/alice/.config/safix-fixture/alice-alone";
                reloadUnits = [ ];
                restartUnits = [ ];
                sopsFile = "/secrets/safix/users/alice/secrets.yaml";
                uid = 0;
              };

              inert = {
                secrets = { };
                preflight = false;
                install = false;
                unit = false;
                evaluates = true;
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
                refuses = fires unaddressedProfile.config.safix.secrets;
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
                establishesNothing = unwiredProfile.config.safix.secrets == { };
                enable = unwiredProfile.config.safix.enable;
              };
              # Configured and bound to nothing: `safix.flake` omitted. The
              # profile refuses rather than building an empty resolution in
              # silence, and the message names the option that supplies the
              # binding.
              flakeless = {
                refuses = fires flakelessProfile.config.safix.secrets;
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
              # off a home-manager profile deliberately: `homeManagerConfiguration`
              # collects every failed assertion and throws them together, so a
              # `fires` over the wrapped profile is green under the drill that
              # removes safix's guard — safix's own wiring assertions fire
              # instead. Forcing safix's own option is what attributes the
              # refusal, and `withIdentity` is what says the guard reads the
              # identity rather than refusing every profile that resolves
              # anything.
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

              # The reason it has to exist: safix's own install entry is
              # registered `entryAfter [ "writeBoundary" ]`, and `writeBoundary`
              # is itself after `checkLinkTargets`, so the install is on the far
              # side of the point at which a refusal would still be atomic.
              # Registering it as a bare string — which becomes `entryAnywhere`
              # and gives home-manager no edge to sort on — is what
              # `installAfterWriteBoundary` refuses.
              installAfterWriteBoundary = before aliceOrder "writeBoundary" "safixInstall";
              installAfterCheckLinkTargets = before aliceOrder "checkLinkTargets" "safixInstall";

              # The pair the preflight's own message rests on, stated as one
              # fact rather than left to be inferred from the two above.
              preflightBeforeInstall = before aliceOrder "safixIdentityPreflight" "safixInstall";

              # Read, do not decrypt: the script names each configured identity and
              # exits non-zero, and it invokes no installer. `runsTheInstaller`
              # matches a store path ending in the installer's own binary rather
              # than the program's name, because the name appears in the
              # remediation prose — which is the other claim here: the message
              # narrows itself to what it checked rather than implying that a
              # readable identity can open the files.
              script =
                let
                  text = (moduleForm "alice").config.home.activation.safixIdentityPreflight.data;
                in
                {
                  namesTheIdentity = lib.hasInfix "/home/alice/.ssh/agenix" text;
                  refuses = lib.hasInfix "exit 1" text;
                  runsTheInstaller = lib.any (
                    line: builtins.match ".*${builtins.storeDir}/[^[:space:]]+/bin/safix.*" line != null
                  ) (lib.splitString "\n" text);
                  statesItsLimit = lib.hasInfix "readability" text && lib.hasInfix "not a recipient" text;
                };
            }
            # The user unit is linux's alone: `systemd.user.services.safix` is
            # what safix registers where the platform has a user manager, and
            # darwin's home-manager DAG — `checkFilesChanged checkLinkTargets
            # writeBoundary installPackages linkGeneration onFilesChange
            # setupLaunchAgents`, read off the pinned home-manager there —
            # carries no `reloadSystemd` for a unit to be ordered against.
            # `before` answers false for a name it cannot find, so asserting
            # that ordering on darwin would be asserting a step's absence
            # rather than its position.
            // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
              unitExists = (moduleForm "alice").config.systemd.user.services ? safix;
            };

            expected = {
              present = true;
              safixBeforeCheckLinkTargets = true;
              installAfterWriteBoundary = true;
              installAfterCheckLinkTargets = true;
              preflightBeforeInstall = true;
              script = {
                namesTheIdentity = true;
                refuses = true;
                runsTheInstaller = false;
                statesItsLimit = true;
              };
            }
            // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
              unitExists = true;
            };
          };
        };
    };
}
