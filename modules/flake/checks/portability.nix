# Holds every subject-model resolution and refusal over all three consumption
# shapes, and fails on a divergence between them.
#
# The claim is design decision D6's: machines, services, groups, silos,
# ownership and organizations behave identically whether a profile is a NixOS
# system scope, a home-manager profile inside NixOS, or a standalone
# home-manager profile on a non-NixOS distribution.
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
# Resolving an organization only where the flake module binds the record — rather
# than in the algebra the three shapes share — fails `organizationEntry` and
# `organizationOwnedEntry` on every shape at once, which is the same evidence the
# refusal rows carry: the model lives in `resolve.nix` and not in a module.
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
  # alice holds six entries and grants one to a machine, one to a group, one to a
  # service, one to an organization and one to the owner of the machine that
  # organization owns. bob is the group's other member; deck is the machine alice
  # owns and the service runs on; rack is acme's, which is what makes the `ownerOf`
  # grant resolve to custody keys rather than to a person's recipient.
  #
  # alice also consents to acme's escrow, so every file she holds carries acme's
  # custody on every shape. That is the half with no element of its own: it widens
  # who can open her files and moves nothing, so a shape that resolved it
  # differently would differ in a `sopsFile` rather than in a name.
  #
  # The service declares no ownership, which is what lets one fleet reach all three
  # shapes: an ownership axis exists at system scope alone, so a service declaring
  # one cannot resolve at the two home shapes at all. That asymmetry is held over
  # `owningService` below rather than here, because a refusal on two of three shapes
  # is not something the agreement comparison can express.
  fleet = {
    users = {
      alice = {
        recipient = keyOf "alice";
        escrowedTo = [ "acme" ];
        private = {
          fleet-token = { };
          oncall-token = { };
          laptop-token = { };
          service-token = { };
          corp-token = { };
          corp-handover = { };
        };
        sharedWith = {
          deck.fleet-token = { };
          oncall.oncall-token = { };
          nginx.service-token = { };
          acme.corp-token = { };
          "ownerOf.rack".corp-handover = { };
        };
        perTag.portable.omit.laptop-token = { };
      };
      bob.recipient = keyOf "bob";
    };
    machines.deck = {
      recipient = keyOf "deck";
      owner = "alice";
      tags = [ "portable" ];
    };
    machines.rack = {
      recipient = keyOf "rack";
      owner = "acme";
    };
    services.nginx = {
      machines = [ "deck" ];
      owner = "alice";
    };
    groups.oncall.members = [
      "alice"
      "bob"
    ];
    organizations.acme.custody.acme-escrow.key = keyOf "acme-escrow";
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
      users.alice = {
        recipient = keyOf "alice";
        private.token = { };
        sharedWith.deck.token = { };
      };
      machines.deck.owner = "alice";
    };

    crossSilo = {
      users = {
        alice = {
          recipient = keyOf "alice";
          private.token = { };
          sharedWith.contractors.token = { };
        };
        bob.recipient = keyOf "bob";
      };
      groups = {
        staff.members = [ "alice" ];
        contractors.members = [ "bob" ];
      };
      silos.corp.groups = [
        "staff"
        "contractors"
      ];
    };

    groupCycle = {
      users.alice = {
        recipient = keyOf "alice";
        private.token = { };
        sharedWith.outer.token = { };
      };
      groups = {
        outer.members = [ "inner" ];
        inner.members = [ "outer" ];
      };
    };

    ownerOfUnownedMachine = {
      users.alice = {
        recipient = keyOf "alice";
        private.token = { };
        sharedWith."ownerOf.deck".token = { };
      };
      machines.deck.recipient = keyOf "deck";
    };

    emptyOrganizationCustody = {
      users.alice = {
        recipient = keyOf "alice";
        escrowedTo = [ "acme" ];
        private.token = { };
      };
      organizations.acme = { };
    };

    organizationInAGroup = {
      users.alice = {
        recipient = keyOf "alice";
        private.token = { };
        sharedWith.oncall.token = { };
      };
      groups.oncall.members = [ "acme" ];
      organizations.acme.custody.acme-escrow.key = keyOf "acme-escrow";
    };

    collidingSubjectName = {
      users = {
        alice = {
          recipient = keyOf "alice";
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
              # what safix's identity derivation reads: the ed25519 host keys of
              # a host running sshd, excluding only safix's own store, and
              # nothing otherwise — the difference between a machine that opens
              # its entries with the identity it already had and one whose
              # resolution refuses for want of any.
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
                username = "alice";
                homeDirectory = "/home/alice";
                stateVersion = "24.05";
              };
              safix = {
                lib = safix;
                identity.sshKeyPaths = [ "/home/alice/.ssh/agenix" ];
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
              viewOf (nixosFor { user = "alice"; } projection).safix.installed
            else
              viewOf
                (home {
                  subject = {
                    user = "alice";
                  }
                  // hostnameForHome;
                }).sops.secrets;
          machine =
            if nixos then
              viewOf (nixosFor { machine = "deck"; } projection).safix.installed
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
              (nixosFor { machine = "deck"; } projection).safix.installed
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
          nixos = fires (nixosFor { user = "alice"; } safix).safix.secrets;
          homeInNixos =
            fires
              (homeFor {
                subject.user = "alice";
                inherit safix;
                osConfig = insideNixos;
              }).safix.secrets;
          standalone =
            fires
              (homeFor {
                subject = {
                  user = "alice";
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

      # Shared by both checks below so neither recomputes the projection
      # `serviceOwnership` reads or re-evaluates the refusal fixtures.
      owning = projectionOf owningService;

      refusalsResults = lib.genAttrs (builtins.attrNames broken) refusalsOf;
    in
    {
      # The system shape needs a real `nixosSystem`, which only evaluates on a
      # Linux host platform, so `safix-portability-system` stays gated the way
      # `safix-consumption-system` is. The two home shapes need nothing
      # platform-specific — `homeFor`'s `osConfig` argument is either the
      # synthetic `insideNixos` attrset or nothing at all — so
      # `safix-portability-home` holds the `homeInNixos`-versus-`standalone`
      # half of the same claims, ungated, on every system.
      checks =
        lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          safix-portability-system = mkStructuralCheck {
            name = "safix-portability-system";
            actual = {
              # The nixos shape's own resolved sets, and its agreement with
              # the two home shapes — the one comparison that legitimately
              # needs a `nixosSystem` on both sides.
              resolution = {
                agree = {
                  person = agreeOn "person";
                  machine = agreeOn "machine";
                };
                person = {
                  nixos = builtins.attrNames shapes.nixos.person;
                };
                machine = {
                  nixos = builtins.attrNames shapes.nixos.machine;
                };
                placement = {
                  nixos = lib.mapAttrs (_e: secret: secret.sopsFile) (shapes.nixos.person // shapes.nixos.machine);
                };
              };

              # The whole of what safix decided about one machine-granted entry,
              # on the nixos shape: the file its audience picked, the key inside
              # it, and the mode its owner declared.
              machineEntry = {
                nixos = shapes.nixos.machine.fleet-token;
              };

              # The same, over the two entries an organization is the audience
              # of: one granted to acme directly and one granted to the owner of
              # the machine acme owns.
              organizationEntry = {
                nixos = shapes.nixos.person.corp-token;
              };
              organizationOwnedEntry = {
                nixos = shapes.nixos.person.corp-handover;
              };

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

              # Every subject-model refusal, on the nixos shape. A refusal that
              # lived in a module rather than in the algebra would fire on one
              # shape and not the others.
              refusals = lib.mapAttrs (_fleetName: result: { nixos = result.nixos; }) refusalsResults;

              # ── the service, on the nixos shape ──
              # The entry a service was granted arrives on the machine the
              # service runs on, under the service's own composed name, and the
              # store's path default takes that name and nests the file under it.
              serviceEntry = {
                nixos = shapes.nixos.machine."nginx/service-token";
              };
              servicePath = {
                nixos = shapes.nixos.machineRaw."nginx/service-token".path;
              };

              # The ownership asymmetry's system half: the system scope carries
              # the service's account and group to the provisioner. The two home
              # shapes have no axis for either and refuse — held by
              # `safix-portability-home`.
              serviceOwnership.system = lib.getAttrs [
                "owner"
                "group"
                "mode"
              ] (nixosFor { machine = "deck"; } owning).safix.installed."nginx/service-token";

              # The person's own resolution is unchanged by any of it: the
              # entries they granted outward are still theirs, in the files
              # their audiences picked.
              grantsStayWithTheirOwner = lib.mapAttrs (_e: secret: secret.sopsFile) shapes.nixos.person;
            };

            expected = {
              resolution = {
                agree = {
                  person = true;
                  machine = true;
                };
                person = {
                  nixos = [
                    "corp-handover"
                    "corp-token"
                    "fleet-token"
                    "laptop-token"
                    "oncall-token"
                    "service-token"
                  ];
                };
                machine = {
                  nixos = [
                    "fleet-token"
                    "nginx/service-token"
                  ];
                };
                placement = {
                  nixos = {
                    corp-handover = "/secrets/safix/shared/@~rack,alice/secrets.yaml";
                    corp-token = "/secrets/safix/shared/=acme,alice/secrets.yaml";
                    fleet-token = "/secrets/safix/shared/alice,deck/secrets.yaml";
                    laptop-token = "/secrets/safix/users/alice/secrets.yaml";
                    oncall-token = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
                    service-token = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
                    "nginx/service-token" = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
                  };
                };
              };

              machineEntry = {
                nixos = {
                  format = "yaml";
                  key = "fleet-token";
                  mode = "0400";
                  name = "fleet-token";
                  sopsFile = "/secrets/safix/shared/alice,deck/secrets.yaml";
                };
              };

              organizationEntry = {
                nixos = {
                  format = "yaml";
                  key = "corp-token";
                  mode = "0400";
                  name = "corp-token";
                  sopsFile = "/secrets/safix/shared/=acme,alice/secrets.yaml";
                };
              };

              organizationOwnedEntry = {
                nixos = {
                  format = "yaml";
                  key = "corp-handover";
                  mode = "0400";
                  name = "corp-handover";
                  sopsFile = "/secrets/safix/shared/@~rack,alice/secrets.yaml";
                };
              };

              systemIdentity = {
                keyFile = null;
                sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
                hostKeys = [ "/etc/ssh/ssh_host_ed25519_key" ];
              };

              refusals = lib.genAttrs (builtins.attrNames broken) (_: {
                nixos = true;
              });

              serviceEntry = {
                nixos = {
                  format = "yaml";
                  key = "service-token";
                  mode = "0400";
                  name = "nginx/service-token";
                  sopsFile = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
                };
              };

              servicePath = {
                nixos = "/run/safix/nginx/service-token";
              };

              serviceOwnership.system = {
                owner = "nginx";
                group = "nginx";
                mode = "0400";
              };

              grantsStayWithTheirOwner = {
                corp-handover = "/secrets/safix/shared/@~rack,alice/secrets.yaml";
                corp-token = "/secrets/safix/shared/=acme,alice/secrets.yaml";
                fleet-token = "/secrets/safix/shared/alice,deck/secrets.yaml";
                laptop-token = "/secrets/safix/users/alice/secrets.yaml";
                oncall-token = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
                service-token = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
              };
            };
          };
        }
        // {
          safix-portability-home = mkStructuralCheck {
            name = "safix-portability-home";
            actual = {
              # The two home shapes' own resolved sets, compared to each other
              # by both agreeing with the same literal below rather than by an
              # explicit boolean flag — the same technique every other
              # per-field comparison in this check already uses.
              resolution = {
                person = {
                  homeInNixos = builtins.attrNames shapes.homeInNixos.person;
                  standalone = builtins.attrNames shapes.standalone.person;
                };
                machine = {
                  homeInNixos = builtins.attrNames shapes.homeInNixos.machine;
                  standalone = builtins.attrNames shapes.standalone.machine;
                };
                placement = {
                  homeInNixos = lib.mapAttrs (_e: secret: secret.sopsFile) (
                    shapes.homeInNixos.person // shapes.homeInNixos.machine
                  );
                  standalone = lib.mapAttrs (_e: secret: secret.sopsFile) (
                    shapes.standalone.person // shapes.standalone.machine
                  );
                };
              };

              machineEntry = {
                homeInNixos = shapes.homeInNixos.machine.fleet-token;
                standalone = shapes.standalone.machine.fleet-token;
              };

              organizationEntry = {
                homeInNixos = shapes.homeInNixos.person.corp-token;
                standalone = shapes.standalone.person.corp-token;
              };
              organizationOwnedEntry = {
                homeInNixos = shapes.homeInNixos.person.corp-handover;
                standalone = shapes.standalone.person.corp-handover;
              };

              refusals = lib.mapAttrs (_fleetName: result: {
                homeInNixos = result.homeInNixos;
                standalone = result.standalone;
              }) refusalsResults;

              serviceEntry = {
                homeInNixos = shapes.homeInNixos.machine."nginx/service-token";
                standalone = shapes.standalone.machine."nginx/service-token";
              };
              servicePath = {
                homeInNixos = shapes.homeInNixos.machineRaw."nginx/service-token".path;
                standalone = shapes.standalone.machineRaw."nginx/service-token".path;
              };

              # The ownership asymmetry's home half: neither home shape has an
              # axis for a service's account or group, so both refuse rather
              # than silently dropping the claim.
              serviceOwnership = {
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
              # resolution reaches for a host configuration, which on this
              # profile would be reaching for null.
              standalone = {
                machine = builtins.attrNames shapes.standalone.machine;
                resolvesWithoutAHostname =
                  (homeFor {
                    subject.machine = "deck";
                    safix = projection;
                  }).safix.hostname == null;

                # The tags a machine's declaration carries reach the resolution:
                # `laptop-token` is omitted by a perTag layer selecting on the
                # tag `deck` declares, so its absence from the machine's set and
                # its presence in the person's is the tag being read.
                tagsComeFromTheDeclaration =
                  (homeFor {
                    subject.machine = "deck";
                    safix = projection;
                  }).safix.tags;
              };
            };

            expected = {
              resolution = {
                person = lib.genAttrs [ "homeInNixos" "standalone" ] (_: [
                  "corp-handover"
                  "corp-token"
                  "fleet-token"
                  "laptop-token"
                  "oncall-token"
                  "service-token"
                ]);
                machine = lib.genAttrs [ "homeInNixos" "standalone" ] (_: [
                  "fleet-token"
                  "nginx/service-token"
                ]);
                placement = lib.genAttrs [ "homeInNixos" "standalone" ] (_: {
                  corp-handover = "/secrets/safix/shared/@~rack,alice/secrets.yaml";
                  corp-token = "/secrets/safix/shared/=acme,alice/secrets.yaml";
                  fleet-token = "/secrets/safix/shared/alice,deck/secrets.yaml";
                  laptop-token = "/secrets/safix/users/alice/secrets.yaml";
                  oncall-token = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
                  service-token = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
                  "nginx/service-token" = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
                });
              };

              machineEntry = lib.genAttrs [ "homeInNixos" "standalone" ] (_: {
                format = "yaml";
                key = "fleet-token";
                mode = "0400";
                name = "fleet-token";
                sopsFile = "/secrets/safix/shared/alice,deck/secrets.yaml";
              });

              organizationEntry = lib.genAttrs [ "homeInNixos" "standalone" ] (_: {
                format = "yaml";
                key = "corp-token";
                mode = "0400";
                name = "corp-token";
                sopsFile = "/secrets/safix/shared/=acme,alice/secrets.yaml";
              });

              organizationOwnedEntry = lib.genAttrs [ "homeInNixos" "standalone" ] (_: {
                format = "yaml";
                key = "corp-handover";
                mode = "0400";
                name = "corp-handover";
                sopsFile = "/secrets/safix/shared/@~rack,alice/secrets.yaml";
              });

              refusals = lib.genAttrs (builtins.attrNames broken) (_: {
                homeInNixos = true;
                standalone = true;
              });

              serviceEntry = lib.genAttrs [ "homeInNixos" "standalone" ] (_: {
                format = "yaml";
                key = "service-token";
                mode = "0400";
                name = "nginx/service-token";
                sopsFile = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
              });

              servicePath = {
                homeInNixos = "/home/alice/.config/sops-nix/secrets/nginx/service-token";
                standalone = "/home/alice/.config/sops-nix/secrets/nginx/service-token";
              };

              serviceOwnership = {
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
            };
          };
        };
    };
}
