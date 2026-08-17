# Holds every subject-model resolution and refusal over all three consumption
# shapes, and fails on a divergence between them.
#
# The claim is design decision D6's: machines, services, groups, silos and
# ownership behave identically whether a profile is a NixOS system scope, a
# home-manager profile inside NixOS, or a standalone home-manager profile on a
# non-NixOS distribution.
# It is a claim about three evaluations rather than about one function, so three
# profiles are evaluated over one fleet and their answers compared to each other
# — not to a literal. A literal would say what the answer is; the comparison says
# the three shapes agree, which is the property, and the literals beside it are
# what stop all three agreeing on the wrong answer.
#
# ── what makes the three shapes three ──
# The system shape is a real `nixosSystem`. The two home shapes differ in exactly
# one thing, which is the seam home-manager itself differs in: a home-manager
# profile evaluated as a NixOS module receives `osConfig`, and a standalone one
# receives nothing. `modules/consume/home.nix` reads that argument and only that
# argument to default a hostname, so supplying it and withholding it is the
# difference between the two shapes as far as safix is concerned.
#
# The standalone shape is the portability anchor, and its evidence is what it does
# not have: no NixOS evaluation, no `osConfig`, no host configuration of any kind,
# and a machine's granted entries still arrive. A resolution that reached for any
# of those would fail on this profile rather than resolve differently.
#
# ── the fleet, and where its projection comes from ──
# The projection each profile is bound to comes from evaluating the real flake
# module over a fleet written here, so what these profiles read is the binding a
# consumer gets rather than a hand-assembled attrset that agrees with it by
# inspection, and every fixture goes through the real option types on the way in.
# It is a fleet of its own rather than this repository's because of the refusals:
# a fleet that cannot resolve cannot be declared into `flake.safix` here, since
# every other check would fail along with it.
#
# ── severity: one drill per claim ──
# Any resolution that differs between the shapes fails `resolution.agree` and
# leaves the per-shape fields to say which shape moved.
# Dropping `machine` from the shared options fails every `machine` field on the
# home shapes and leaves the system shape green, which is the divergence this
# check exists to catch.
# Making the machine branch of `selectFor` read a hostname fails
# `standalone.machine`, and only that: the other two shapes have one.
# Deleting any subject-model refusal fails `refusals` on all three shapes at
# once, which is what says the refusal lives in the algebra rather than in a
# module.
# Requiring a hostname for a machine resolution fails
# `standalone.resolvesWithoutAHostname`.
# Keying a service's entries by the bare name fails `serviceEntry` and
# `servicePath` on every shape; leaving `sopsKey` unset for one fails
# `serviceEntry.key`, which is the severe half — the provisioner would read the
# composed name as the key inside the document.
# Dropping a service's ownership at user scope rather than refusing leaves
# `serviceOwnership.system` green and fails its two home fields, which is the
# asymmetry this pair exists to hold.
{
  config,
  inputs,
  lib,
  ...
}:
let
  hmLib = inputs.home-manager.lib;

  keyOf = name: "age1fixture-${name}-0000000000000000000000000000000000";

  # The real flake module, evaluated over a fleet written here. `self` is the
  # repository-relative root every resolved `sopsFile` is placed under, and the
  # empty string is what makes those paths read as the repository-relative strings
  # the comparison is over rather than as a store hash.
  projectionOf =
    fleet:
    (lib.evalModules {
      modules = [
        ../safix
        { _module.args.self = ""; }
        { flake.safix = fleet; }
      ];
    }).config.flake.safix.lib;

  # ── the fleet the three shapes resolve ──
  # ana holds four entries and grants one to a machine, one to a group and one to
  # a service. bo is the group's other member; deck is the machine, which ana owns
  # and the service runs on.
  #
  # The service declares no ownership, which is what lets one fleet reach all three
  # shapes: an ownership axis exists at system scope alone, so a service declaring
  # one cannot resolve at the two home shapes at all. That asymmetry is held over
  # `owningService` below rather than here, because a refusal on two of three shapes
  # is not something the agreement comparison can express.
  fleet = {
    users = {
      ana = {
        recipient = keyOf "ana";
        private = {
          fleet-token = { };
          oncall-token = { };
          laptop-token = { };
          service-token = { };
        };
        sharedWith = {
          deck.fleet-token = { };
          oncall.oncall-token = { };
          nginx.service-token = { };
        };
        perTag.portable.omit.laptop-token = { };
      };
      bo.recipient = keyOf "bo";
    };
    machines.deck = {
      recipient = keyOf "deck";
      owner = "ana";
      tags = [ "portable" ];
    };
    services.nginx = {
      machines = [ "deck" ];
      owner = "ana";
    };
    groups.oncall.members = [
      "ana"
      "bo"
    ];
    silos.corp.groups = [ "oncall" ];
  };

  # The same declaration with the service claiming an account and a group. The
  # system scope carries both to the provisioner; a user-scope profile has no axis
  # for either and refuses rather than dropping the claim.
  owningService = lib.recursiveUpdate fleet {
    services.nginx = {
      user = "nginx";
      group = "nginx";
    };
  };

  projection = projectionOf fleet;

  # ── the fleets no shape may resolve ──
  # One per subject-model refusal, each otherwise well-formed so that the rule
  # named is what stops the resolution.
  broken = {
    keylessMachine = {
      users.ana = {
        recipient = keyOf "ana";
        private.token = { };
        sharedWith.deck.token = { };
      };
      machines.deck.owner = "ana";
    };

    crossSilo = {
      users = {
        ana = {
          recipient = keyOf "ana";
          private.token = { };
          sharedWith.contractors.token = { };
        };
        bo.recipient = keyOf "bo";
      };
      groups = {
        staff.members = [ "ana" ];
        contractors.members = [ "bo" ];
      };
      silos.corp.groups = [
        "staff"
        "contractors"
      ];
    };

    groupCycle = {
      users.ana = {
        recipient = keyOf "ana";
        private.token = { };
        sharedWith.outer.token = { };
      };
      groups = {
        outer.members = [ "inner" ];
        inner.members = [ "outer" ];
      };
    };

    ownerOfUnownedMachine = {
      users.ana = {
        recipient = keyOf "ana";
        private.token = { };
        sharedWith."ownerOf.deck".token = { };
      };
      machines.deck.recipient = keyOf "deck";
    };

    collidingSubjectName = {
      users = {
        ana = {
          recipient = keyOf "ana";
          private.token = { };
        };
        deck.recipient = keyOf "deck";
      };
      machines.deck.recipient = keyOf "deck";
    };
  };

  fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;
in
{
  perSystem =
    { pkgs, system, ... }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      hostname = "server";

      # Every field of a resolved entry that safix decides, read back through
      # sops-nix's own option types on each shape so a field one shape stopped
      # emitting shows up as a divergence rather than as a missing key.
      #
      # `path` is deliberately not among them, and `owner`, `group` and
      # `sopsFileHash` are not either. The first is the provisioner's own default
      # at each scope — a system runtime path and a home one — and an entry that
      # declares its own declares it as a function of the configuration it lands
      # in, which is the one thing that legitimately differs between a home
      # directory and a system. The ownership axis exists only at system scope,
      # which `safix-consumption` holds on its own. And `sopsFileHash` reads the
      # ciphertext, which no fixture fleet has.
      decided = [
        "format"
        "key"
        "mode"
        "name"
        "sopsFile"
      ];

      viewOf = lib.mapAttrs (_name: lib.getAttrs decided);

      # A NixOS system serving one subject.
      nixosFor =
        subject: safix:
        (inputs.nixpkgs.lib.nixosSystem {
          modules = [
            config.flake.nixosModules.default
            {
              nixpkgs.hostPlatform = system;
              networking.hostName = hostname;
              system.stateVersion = "24.05";

              # A machine subject's recipient is `ssh-to-age` of a host key, so a
              # host that has one is the case this shape stands for. It is also
              # what makes sops-nix's own identity default non-empty:
              # `sops.age.sshKeyPaths` takes the ed25519 host keys of a host
              # running sshd and nothing otherwise, which is the difference
              # between a machine that opens its entries with the identity it
              # already had and one that has no identity at all.
              services.openssh.enable = true;
              safix = {
                lib = safix;
              }
              // subject;
            }
          ];
        }).config;

      # A home-manager profile serving one subject. `osConfig` is the whole of
      # what separates the two home shapes: supplied, this is home-manager
      # evaluated as a NixOS module, and withheld it is the standalone profile on
      # a distribution that has no NixOS configuration to read.
      homeFor =
        {
          subject,
          safix,
          osConfig ? null,
          extra ? { },
        }:
        (hmLib.homeManagerConfiguration {
          inherit pkgs;
          extraSpecialArgs = lib.optionalAttrs (osConfig != null) { inherit osConfig; };
          modules = [
            config.flake.homeModules.default
            {
              home = {
                username = "ana";
                homeDirectory = "/home/ana";
                stateVersion = "24.05";
              };
              safix = {
                lib = safix;
                identity.sshKeyPaths = [ "/home/ana/.ssh/agenix" ];
              }
              // subject
              // extra;
            }
          ];
        }).config;

      insideNixos = {
        networking.hostName = hostname;
      };

      # The three shapes, over the fleet that resolves. Each is asked for one
      # person's entries and one machine's, because those are the two kinds of
      # subject a profile can serve and the machine is the half that is new.
      shapeOf =
        {
          nixos,
          home,
          hostnameForHome,
        }:
        {
          person =
            if nixos then
              viewOf (nixosFor { user = "ana"; } projection).sops.secrets
            else
              viewOf
                (home {
                  subject = {
                    user = "ana";
                  }
                  // hostnameForHome;
                }).sops.secrets;
          machine =
            if nixos then
              viewOf (nixosFor { machine = "deck"; } projection).sops.secrets
            else
              viewOf
                (home {
                  subject = {
                    machine = "deck";
                  };
                }).sops.secrets;

          # The machine's arrival unstripped, which is where the path the
          # provisioner parks each entry at is readable. It is outside `decided`
          # because a system path and a home path legitimately differ; what is
          # asserted over it is that the provisioner accepts a service's composed
          # name and nests it, on each shape, rather than that the three agree.
          machineRaw =
            if nixos then
              (nixosFor { machine = "deck"; } projection).sops.secrets
            else
              (home {
                subject = {
                  machine = "deck";
                };
              }).sops.secrets;
        };

      homeShape = osConfig: args: homeFor (args // { safix = projection; } // { inherit osConfig; });

      shapes = {
        nixos = shapeOf {
          nixos = true;
          home = null;
          hostnameForHome = { };
        };
        homeInNixos = shapeOf {
          nixos = false;
          home = homeShape insideNixos;
          hostnameForHome = { };
        };
        standalone = shapeOf {
          nixos = false;
          home = homeShape null;
          hostnameForHome = {
            inherit hostname;
          };
        };
      };

      # The same three shapes over each fleet no shape may resolve. `fires` is
      # narrow on purpose: deep-forcing a whole configuration reaches options no
      # fixture profile defines and would report a refusal on every one of them.
      refusalsOf =
        fleetName:
        let
          safix = projectionOf broken.${fleetName};
        in
        {
          nixos = fires (nixosFor { user = "ana"; } safix).sops.secrets;
          homeInNixos =
            fires
              (homeFor {
                subject.user = "ana";
                inherit safix;
                osConfig = insideNixos;
              }).safix.secrets;
          standalone =
            fires
              (homeFor {
                subject = {
                  user = "ana";
                  inherit hostname;
                };
                inherit safix;
              }).safix.secrets;
        };

      allShapes = [
        "nixos"
        "homeInNixos"
        "standalone"
      ];

      agreeOn = field: lib.all (shape: shapes.${shape}.${field} == shapes.nixos.${field}) allShapes;
    in
    {
      # A system configuration only evaluates on a Linux host platform, and the
      # claim is about three shapes agreeing, so the whole comparison is Linux
      # only. `safix-consumption-system` is split the same way and for the same
      # reason.
      checks = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-portability = mkStructuralCheck {
          name = "safix-portability";
          actual = {
            # The three shapes resolve the same sets. Asserted as an agreement
            # between them and as the literal each agrees on, because agreement
            # alone would be satisfied by three shapes resolving nothing.
            resolution = {
              agree = {
                person = agreeOn "person";
                machine = agreeOn "machine";
              };
              person = lib.mapAttrs (_n: v: builtins.attrNames v.person) shapes;
              machine = lib.mapAttrs (_n: v: builtins.attrNames v.machine) shapes;
              placement = lib.mapAttrs (
                _n: v: lib.mapAttrs (_e: secret: secret.sopsFile) (v.person // v.machine)
              ) shapes;
            };

            # The whole of what safix decided about one machine-granted entry, on
            # each shape: the file its audience picked, the key inside it, and the
            # mode its owner declared.
            machineEntry = lib.mapAttrs (_n: v: v.machine.fleet-token) shapes;

            # The identity a machine's system scope opens those entries with is
            # the one it already had. safix names none — the profile sets no
            # `safix.identity.*` — and sops-nix's own default stands: the host's
            # ed25519 keys, whose age form is what
            # `flake.safix.machines.<m>.recipient` is. That is the whole of why
            # declaring a machine needs no enrollment step, and it is a claim
            # about the system scope alone, which is the one scope whose
            # provisioner has a host identity to default to.
            systemIdentity =
              let
                system = nixosFor { machine = "deck"; } projection;
              in
              {
                keyFile = system.sops.age.keyFile;
                sshKeyPaths = system.sops.age.sshKeyPaths;
                hostKeys = map (key: key.path) (
                  lib.filter (key: key.type == "ed25519") system.services.openssh.hostKeys
                );
              };

            # Every subject-model refusal, over every shape. A refusal that lived
            # in a module rather than in the algebra would fire on one shape and
            # not the others.
            refusals = lib.genAttrs (builtins.attrNames broken) refusalsOf;

            # ── the service, across the three shapes ──
            # The entry a service was granted arrives on the machine the service
            # runs on, under the service's own composed name, and the provisioner
            # takes that name and nests the file under it. Read off each shape's own
            # arrival rather than from safix, because the claim is about what
            # sops-nix does with the name safix hands it.
            serviceEntry = lib.mapAttrs (_n: v: v.machine."nginx/service-token") shapes;
            servicePath = lib.mapAttrs (_n: v: v.machineRaw."nginx/service-token".path) shapes;

            # The ownership asymmetry, over the one capability that is scope-specific.
            # The system scope carries the service's account and group to the
            # provisioner; the two home shapes have no axis for either and refuse.
            # An ownerless service resolves at every shape, which the fleet above is
            # the whole of.
            serviceOwnership =
              let
                owning = projectionOf owningService;
              in
              {
                system = lib.getAttrs [
                  "owner"
                  "group"
                  "mode"
                ] (nixosFor { machine = "deck"; } owning).sops.secrets."nginx/service-token";
                homeInNixos =
                  fires
                    (homeFor {
                      subject.machine = "deck";
                      safix = owning;
                      osConfig = insideNixos;
                    }).safix.secrets;
                standalone =
                  fires
                    (homeFor {
                      subject.machine = "deck";
                      safix = owning;
                    }).safix.secrets;
              };

            # ── the standalone shape ──
            # It resolves a machine's entries with no `osConfig`, no NixOS
            # configuration and no hostname of its own. Nothing about the
            # resolution reaches for a host configuration, which on this profile
            # would be reaching for null.
            standalone = {
              machine = builtins.attrNames shapes.standalone.machine;
              resolvesWithoutAHostname =
                (homeFor {
                  subject.machine = "deck";
                  safix = projection;
                }).safix.hostname == null;

              # The tags a machine's declaration carries reach the resolution:
              # `laptop-token` is omitted by a perTag layer selecting on the tag
              # `deck` declares, so its absence from the machine's set and its
              # presence in the person's is the tag being read.
              tagsComeFromTheDeclaration =
                (homeFor {
                  subject.machine = "deck";
                  safix = projection;
                }).safix.tags;
            };

            # The person's own resolution is unchanged by any of it: the entries
            # they granted outward are still theirs, in the files their audiences
            # picked.
            grantsStayWithTheirOwner = lib.mapAttrs (_e: secret: secret.sopsFile) shapes.nixos.person;
          };

          expected = {
            resolution = {
              agree = {
                person = true;
                machine = true;
              };
              person = lib.genAttrs allShapes (_: [
                "fleet-token"
                "laptop-token"
                "oncall-token"
                "service-token"
              ]);
              machine = lib.genAttrs allShapes (_: [
                "fleet-token"
                "nginx/service-token"
              ]);
              placement = lib.genAttrs allShapes (_: {
                fleet-token = "/secrets/safix/shared/ana,deck/secrets.yaml";
                laptop-token = "/secrets/safix/users/ana/secrets.yaml";
                oncall-token = "/secrets/safix/shared/@oncall,ana/secrets.yaml";
                service-token = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
                "nginx/service-token" = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
              });
            };

            machineEntry = lib.genAttrs allShapes (_: {
              format = "yaml";
              key = "fleet-token";
              mode = "0400";
              name = "fleet-token";
              sopsFile = "/secrets/safix/shared/ana,deck/secrets.yaml";
            });

            systemIdentity = {
              keyFile = null;
              sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
              hostKeys = [ "/etc/ssh/ssh_host_ed25519_key" ];
            };

            refusals = lib.genAttrs (builtins.attrNames broken) (_: lib.genAttrs allShapes (_: true));

            serviceEntry = lib.genAttrs allShapes (_: {
              format = "yaml";
              key = "service-token";
              mode = "0400";
              name = "nginx/service-token";
              sopsFile = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
            });

            servicePath = {
              nixos = "/run/secrets/nginx/service-token";
              homeInNixos = "/home/ana/.config/sops-nix/secrets/nginx/service-token";
              standalone = "/home/ana/.config/sops-nix/secrets/nginx/service-token";
            };

            serviceOwnership = {
              system = {
                owner = "nginx";
                group = "nginx";
                mode = "0400";
              };
              homeInNixos = true;
              standalone = true;
            };

            standalone = {
              machine = [
                "fleet-token"
                "nginx/service-token"
              ];
              resolvesWithoutAHostname = true;
              tagsComeFromTheDeclaration = [ "portable" ];
            };

            grantsStayWithTheirOwner = {
              fleet-token = "/secrets/safix/shared/ana,deck/secrets.yaml";
              laptop-token = "/secrets/safix/users/ana/secrets.yaml";
              oncall-token = "/secrets/safix/shared/@oncall,ana/secrets.yaml";
              service-token = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
            };
          };
        };
      };
    };
}
