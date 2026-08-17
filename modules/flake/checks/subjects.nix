# Holds the subject model of ../safix/resolve.nix against fleets built to break
# each of its rules.
#
# Every fixture is synthetic and typed, on the same terms as ./custody.nix: a
# synthetic machine goes through the real `machine` submodule and a synthetic
# group through the real `group` one, so a fixture cannot pass by omitting a field
# the option system would have supplied.
#
# Each rule is asserted twice where it is a refusal — the message `violations`
# produces, against a literal, and that a resolution of that fleet actually
# throws — and once where it is a resolution, against the file and the recipients
# it derives. The recipients are what make a re-wrap observable: a growth, a
# shrink and an ownership change each leave the file where it was and change the
# list, which is the difference between a re-wrap and a migration.
#
# Every subject's recipient is a distinct literal, which ./custody.nix does not
# need and this cannot do without: one key shared between two people collapses
# under `lib.unique`, and a claim about an audience gaining a recipient would
# then be a claim about nothing.
#
# Severity: proven by perturbation, one drill per claim.
# Dropping the machine leg from `subjectRecipientsOf` empties
# `machineGrant.recipients` and fails that field alone.
# Having `elementOf` render a group as its own name rather than marking it fails
# `groupGrant.file` and `markedElementsAreDistinct`, and the second is the severe
# half: an unmarked group element is a directory a person of that name would also
# reach, which is one rule over two audiences.
# Naming a group audience's file for its expansion rather than for the group —
# which is what dropping the marker and expanding in `audienceOf` would do —
# leaves `groupGrant` green and fails `growthIsARewrap` and `shrinkIsARewrap`,
# because the file then moves when the membership does. That pair is the whole
# reason the marker exists.
# Resolving `ownerOf.<machine>` to the machine rather than to its owner fails
# `ownerOf.recipients` and `ownerOf.resolvedBy`.
# Naming an `ownerOf` audience's file for the resolved owner fails
# `ownerChangeIsARewrap`, and only that: the audience and the recipients are
# right either way, and what breaks is that a change of owner becomes a
# migration nothing re-wraps.
# Dropping the owner filter from `reachesOf` fails `ownGroupIsNotACollision`:
# sharing with a group one is a member of is the ordinary case and would be
# reported as a collision with itself.
# Making the silo refusal transitive over ownership fails
# `machinesInTwoSilos.violations`, which is what D3 says must stay legal, while
# `machinesInTwoSilos.spanningFileRefused` is what says the refusal still fires
# on the file. Deleting the refusal fails the second and leaves the first green.
# Bounding `expandGroups` at fewer rounds than there are groups fails
# `nestedGroup.recipients`; removing the bound entirely does not fail anything
# here and instead does not terminate on `groupCycle`, which is why the cycle
# refusal is asserted on its message as well as on the throw.
# Expanding a service to its machines in `expandGroups` rather than in
# `leafRowsOf` loses which service reached a machine and fails
# `twoServicesOneMachine`, which is the whole of what the composed key buys.
# Keying a service's entries by the bare name fails `twoServicesOneMachine.keys`
# and `serviceGrant.resolvedByTheMachine`; leaving `sopsKey` unset on a
# service-granted entry fails `serviceGrant.sopsKeys`, which is the severe half —
# the provisioner would look for the composed name inside the file and find
# nothing.
# Dropping the service marker from `elementOf` fails `serviceGrant.file` and both
# service rows of `markedElementsAreDistinct`, and the second is the severe half
# for the reason the group one is.
# Naming a service audience's file for its machines rather than for the service
# leaves `serviceGrant` green and fails `serviceGrowthIsARewrap` and
# `serviceShrinkIsARewrap`.
# Applying a service's ownership to the entry and then dropping it at user scope
# rather than refusing fails `serviceOwnershipAtUserScope.refused`; refusing an
# ownerless service there fails `ownerlessResolves`.
# Emptying any fixture fleet fails `fixtureRosters`.
{
  perSystem =
    { pkgs, lib, ... }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      safixChecks = import ../safix/checks.nix { inherit lib; };
      keepassxcLib = import ../safix/keepassxc.nix { inherit lib; };
      bridgeLib = import ../safix/bridge.nix { inherit lib; };
      policy = import ../safix/policy.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      fixture = import ./fixture-fleet.nix;
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      sorted = lib.sort (a: b: a < b);

      typed =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      # A literal shaped like a recipient and distinct per subject. Nothing here
      # has a private half, nothing encrypts anything, and no ciphertext exists
      # for any of them to open.
      keyOf = name: "age1fixture-${name}-0000000000000000000000000000000000";

      # A fleet, as the six records the resolver reads. `machines`, `services` and
      # `groups` go through their own submodules for the same reason `users` does.
      fleetOf =
        {
          users ? { },
          catalogue ? { },
          machines ? { },
          services ? { },
          groups ? { },
          silos ? { },
        }:
        {
          users = typed (lib.types.attrsOf types.profile) users;
          catalogue = typed (lib.types.attrsOf types.entry) catalogue;
          machines = typed (lib.types.attrsOf types.machine) machines;
          services = typed (lib.types.attrsOf types.service) services;
          groups = typed (lib.types.attrsOf types.group) groups;
          silos = typed (lib.types.attrsOf types.silo) silos;
        };

      # A person who holds one private entry and records their own key, which is
      # the shortest fleet a grant can be made from.
      holder =
        name: grants:
        {
          recipient = keyOf name;
          private.token = { };
        }
        // grants;

      keyholder = name: { recipient = keyOf name; };

      machine =
        name: owner:
        {
          recipient = keyOf name;
        }
        // lib.optionalAttrs (owner != null) { inherit owner; };

      fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

      audienceOfToken = fleet: (resolve.audiencesOf fleet).${fileOfToken fleet};

      fileOfToken = fleet: (resolve.placementsOf fleet).ana.token.file;

      # What one subject resolves, as name -> the file it reads, over either kind
      # of subject: a person selects with `user` and a machine with `machine`, and
      # the answer is the same map either way.
      filesFor =
        fleet: subject:
        lib.mapAttrs (_n: s: s.sopsFile) (
          resolve.selectFor (
            fleet
            // {
              root = "";
              hostname = "somewhere";
              tags = [ ];
            }
            // subject
          )
        );

      # The same selection materialized into the provisioner's shape, which is
      # where a declared path becomes a literal one and where the ownership axis
      # exists or does not. The configuration handed to it is empty because every
      # path a fixture declares here ignores it.
      materializedFor =
        fleet: args:
        resolve.materializeFor (
          fleet
          // {
            root = "";
            hostname = "somewhere";
            tags = [ ];
          }
          // args
        ) { };

      violationsOf = resolve.violations;

      # ── inertness ──
      # The repository's own fleet, and the same fleet with four more subject
      # records declared that nothing references. Every derived artifact has to be
      # identical: declaring a machine, a service, a group or a silo changes nothing
      # until an audience names one.
      bare = fleetOf {
        inherit (fixture.fleet)
          users
          catalogue
          machines
          services
          ;
      };

      declaredButUnused = bare // {
        machines = typed (lib.types.attrsOf types.machine) (
          fixture.fleet.machines
          // {
            deck = machine "deck" "ana";
            rack = machine "rack" "bo";
          }
        );
        services = typed (lib.types.attrsOf types.service) (
          fixture.fleet.services
          // {
            nginx = {
              machines = [ "deck" ];
              owner = "ana";
              user = "nginx";
              group = "nginx";
            };
          }
        );
        groups = typed (lib.types.attrsOf types.group) {
          oncall.members = [
            "ana"
            "bo"
          ];
          infra.members = [
            "deck"
            "rack"
          ];
        };
        silos = typed (lib.types.attrsOf types.silo) {
          corp.groups = [
            "oncall"
            "infra"
          ];
        };
      };

      derivedFrom = fleet: {
        policyText = policy.render fleet;
        audiences = resolve.audiencesOf fleet;
        placements = resolve.placementsOf fleet;
        publicPaths = resolve.publicPathsOf fleet;
      };

      # ── machines as subjects ──
      machineGrant = fleetOf {
        users.ana = holder "ana" { sharedWith.deck.token = { }; };
        machines.deck = machine "deck" "ana";
      };

      keylessMachine = fleetOf {
        users.ana = holder "ana" { sharedWith.deck.token = { }; };
        machines.deck = {
          owner = "ana";
        };
      };

      machineOwnedByNobody = fleetOf {
        users.ana = holder "ana" { };
        machines.deck = machine "deck" "zed";
      };

      nameDeclaredTwice = fleetOf {
        users = {
          ana = holder "ana" { };
          deck = keyholder "deck";
        };
        machines.deck = machine "deck" "ana";
      };

      unsafeMachineName = fleetOf {
        users.ana = holder "ana" { };
        machines."Deck" = machine "deck" "ana";
      };

      # ── services as subjects ──
      # A service resolving to one machine's key. `nginx` declares ownership, which
      # is what the system scope carries and the user scope refuses.
      serviceOn =
        hosts:
        fleetOf {
          users.ana = holder "ana" { sharedWith.nginx.token = { }; };
          machines = {
            deck = machine "deck" "ana";
            rack = machine "rack" "ana";
          };
          services.nginx = {
            machines = hosts;
            owner = "ana";
            user = "nginx";
            group = "nginx";
          };
        };

      serviceGrant = serviceOn [ "deck" ];
      grownService = serviceOn [
        "deck"
        "rack"
      ];
      shrunkService = serviceOn [ "rack" ];
      emptyService = serviceOn [ ];

      # A service with no ownership fields, which is what resolves at either scope.
      ownerlessService = fleetOf {
        users.ana = holder "ana" { sharedWith.nginx.token = { }; };
        machines.deck = machine "deck" "ana";
        services.nginx = {
          machines = [ "deck" ];
          owner = "ana";
        };
      };

      # A group whose member is a service, so the expansion crosses both kinds on
      # the way to a machine's key.
      serviceInGroup = fleetOf {
        users.ana = holder "ana" { sharedWith.oncall.token = { }; };
        machines.deck = machine "deck" "ana";
        services.nginx = {
          machines = [ "deck" ];
          owner = "ana";
        };
        groups.oncall.members = [ "nginx" ];
      };

      # Two services on one machine, granted one name. Each resolves under its own
      # key, so neither replaces the other and the provisioner's own default path —
      # a function of the name — is what holds them apart.
      twoServices =
        entry:
        fleetOf {
          users.ana = {
            recipient = keyOf "ana";
            private.token = entry;
            sharedWith = {
              alpha.token = { };
              beta.token = { };
            };
          };
          machines.deck = machine "deck" "ana";
          services = {
            alpha.machines = [ "deck" ];
            beta.machines = [ "deck" ];
          };
        };

      twoServicesOneMachine = twoServices { };

      # The same pair over an entry that declares its own path. Two keys, one
      # literal path, refused by the collision refusal every other pair of entries
      # meets rather than by a rule about services.
      twoServicesOnePath = twoServices { path = _cfg: "/var/lib/fixture/token"; };

      serviceOverUndeclaredMachine = fleetOf {
        users.ana = holder "ana" { };
        machines.deck = machine "deck" "ana";
        services.nginx.machines = [
          "rack"
          "deck"
        ];
      };

      serviceOwnedByNobody = fleetOf {
        users.ana = holder "ana" { };
        machines.deck = machine "deck" "ana";
        services.nginx = {
          machines = [ "deck" ];
          owner = "zed";
        };
      };

      serviceNameDeclaredTwice = fleetOf {
        users.ana = holder "ana" { };
        machines.nginx = machine "nginx" "ana";
        services.nginx.machines = [ "nginx" ];
      };

      unsafeServiceName = fleetOf {
        users.ana = holder "ana" { };
        machines.deck = machine "deck" "ana";
        services."Nginx".machines = [ "deck" ];
      };

      # A service whose machine records no key. The service resolves and the data
      # key cannot be wrapped for it, reported with the service named as the
      # declaration that put the machine in the audience.
      serviceOnKeylessMachine = fleetOf {
        users.ana = holder "ana" { sharedWith.nginx.token = { }; };
        machines.deck.owner = "ana";
        services.nginx.machines = [ "deck" ];
      };

      # ── groups ──
      # bo and cy each hold a key and nothing else, so what the group's audience
      # gains is exactly its members' recipients.
      groupFleet =
        members:
        fleetOf {
          users = {
            ana = holder "ana" { sharedWith.oncall.token = { }; };
            bo = keyholder "bo";
            cy = keyholder "cy";
            dee = keyholder "dee";
          };
          groups.oncall.members = members;
        };

      groupGrant = groupFleet [
        "bo"
        "cy"
      ];

      grownGroup = groupFleet [
        "bo"
        "cy"
        "dee"
      ];

      shrunkGroup = groupFleet [ "bo" ];

      # A group whose members are a group and a machine, so the expansion is
      # transitive and reaches both kinds of leaf.
      nestedGroup = fleetOf {
        users = {
          ana = holder "ana" { sharedWith.outer.token = { }; };
          bo = keyholder "bo";
        };
        machines.deck = machine "deck" "ana";
        groups = {
          outer.members = [
            "inner"
            "deck"
          ];
          inner.members = [ "bo" ];
        };
      };

      # Sharing with a group one is a member of. The owner is already in the
      # audience, so their own membership must not read as receiving their own
      # secret from outside.
      ownGroup = fleetOf {
        users = {
          ana = holder "ana" { sharedWith.oncall.token = { }; };
          bo = keyholder "bo";
        };
        groups.oncall.members = [
          "ana"
          "bo"
        ];
      };

      groupCycle = fleetOf {
        users.ana = holder "ana" { sharedWith.outer.token = { }; };
        groups = {
          outer.members = [ "inner" ];
          inner.members = [ "outer" ];
        };
      };

      emptyGroup = fleetOf {
        users.ana = holder "ana" { sharedWith.oncall.token = { }; };
        groups.oncall.members = [ ];
      };

      unknownMember = fleetOf {
        users.ana = holder "ana" { };
        groups.oncall.members = [ "zed" ];
      };

      # A member who holds no key. The group resolves, and the data key cannot be
      # wrapped for them, which is the same defect a direct grant to a keyless
      # person has and is reported with the group named.
      keylessMember = fleetOf {
        users = {
          ana = holder "ana" { sharedWith.oncall.token = { }; };
          bo = { };
        };
        groups.oncall.members = [ "bo" ];
      };

      # A group member who already holds the name. Their own value and the
      # group's are two statements about one name in one resolved set.
      memberHoldsTheName = fleetOf {
        users = {
          ana = holder "ana" { sharedWith.oncall.token = { }; };
          bo = {
            recipient = keyOf "bo";
            private.token = { };
          };
        };
        groups.oncall.members = [ "bo" ];
      };

      # ── silos ──
      # ana is staff and shares with a contractor, which is the corporate case:
      # one file both sides could open.
      siloFleet =
        grants:
        fleetOf {
          users = {
            ana = holder "ana" grants;
            bo = keyholder "bo";
            cy = keyholder "cy";
          };
          groups = {
            staff.members = [ "ana" ];
            contractors.members = [ "bo" ];
            partners.members = [ "cy" ];
          };
          silos.corp.groups = [
            "staff"
            "contractors"
          ];
        };

      crossSilo = siloFleet { sharedWith.contractors.token = { }; };

      # Two entries, each spanning the silo. Every violating grant is reported in
      # one evaluation rather than the first one found: an operator repairing a
      # corporate boundary one refusal per rebuild is an operator who does not know
      # how many are left.
      crossSiloTwice = fleetOf {
        users = {
          ana = {
            recipient = keyOf "ana";
            private = {
              token = { };
              other = { };
            };
            sharedWith.contractors = {
              token = { };
              other = { };
            };
          };
          bo = keyholder "bo";
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

      withinSilo = siloFleet { sharedWith.partners.token = { }; };

      groupInTwoSilos = fleetOf {
        users.ana = holder "ana" { };
        groups = {
          staff.members = [ "ana" ];
          contractors.members = [ ];
        };
        silos = {
          corp.groups = [
            "staff"
            "contractors"
          ];
          vendor.groups = [ "contractors" ];
        };
      };

      siloOverANonGroup = fleetOf {
        users.ana = holder "ana" { };
        machines.deck = machine "deck" "ana";
        silos.corp.groups = [
          "deck"
          "absent"
        ];
      };

      # One person owning machines in two silos, which D3 says is the operator
      # administering both sides and is not itself refused. The second entry is
      # what says the refusal still fires on a file: `token` reaches one silo,
      # `other` reaches the other, and `both` reaches both.
      machinesInTwoSilos =
        grants:
        fleetOf {
          users = {
            ana = {
              recipient = keyOf "ana";
              private = {
                token = { };
                other = { };
                both = { };
              };
            }
            // grants;
            bo = keyholder "bo";
            cy = keyholder "cy";
          };
          machines = {
            deck = machine "deck" "ana";
            rack = machine "rack" "ana";
          };
          groups = {
            red.members = [
              "deck"
              "bo"
            ];
            blue.members = [
              "rack"
              "cy"
            ];
          };
          silos.corp.groups = [
            "red"
            "blue"
          ];
        };

      ownsBothSides = machinesInTwoSilos {
        sharedWith = {
          red.token = { };
          blue.other = { };
        };
      };

      spansBothSides = machinesInTwoSilos {
        sharedWith = {
          red.both = { };
          blue.both = { };
        };
      };

      # ── ownership as a resolution record ──
      ownerOfFleet =
        owner:
        fleetOf {
          users = {
            ana = holder "ana" { sharedWith."ownerOf.deck".token = { }; };
            bo = keyholder "bo";
            cy = keyholder "cy";
          };
          machines.deck = machine "deck" owner;
        };

      ownedByBo = ownerOfFleet "bo";
      ownedByCy = ownerOfFleet "cy";

      ownerOfUnknownMachine = fleetOf {
        users.ana = holder "ana" { sharedWith."ownerOf.rack".token = { }; };
        machines.deck = machine "deck" "ana";
      };

      ownerOfUnownedMachine = fleetOf {
        users.ana = holder "ana" { sharedWith."ownerOf.deck".token = { }; };
        machines.deck = {
          recipient = keyOf "deck";
        };
      };

      # An `ownerOf` grant resolving to its own author widens nothing.
      ownerOfSelf = fleetOf {
        users.ana = holder "ana" { sharedWith."ownerOf.deck".token = { }; };
        machines.deck = machine "deck" "ana";
      };

      # ── the marked forms ──
      # Two audiences a marker collapsed onto one directory would join into one
      # rule. Every name is well-formed, so nothing else in the registry would
      # have refused them.
      markedPairs = [
        {
          label = "a group and a person of that name";
          a = [
            "@oncall"
            "ana"
          ];
          b = [
            "ana"
            "oncall"
          ];
        }
        {
          label = "an owner reference and a group of that name";
          a = [
            "@~deck"
            "ana"
          ];
          b = [
            "@deck"
            "ana"
          ];
        }
        {
          label = "a service and a person of that name";
          a = [
            "%nginx"
            "ana"
          ];
          b = [
            "ana"
            "nginx"
          ];
        }
        {
          label = "a service and a group of that name";
          a = [
            "%nginx"
            "ana"
          ];
          b = [
            "@nginx"
            "ana"
          ];
        }
      ];

      roundTrips = fleet: refs: map (ref: resolve.refOfElement (resolve.elementOf fleet ref) == ref) refs;

      # ── the two relationship families, over a fleet the subject model refuses ──
      # Both read the registry to decide whether custody resolved at all, and both
      # say nothing while it has not: one fault producing two unrelated sentences
      # is worse than the second sentence is worth. A subject violation is a
      # custody violation, so a mapping that would otherwise be reported has to go
      # quiet over one — and has to be reported over a fleet that resolves, or the
      # silence is a mapping nobody was judging.
      #
      # Each mapping names a person nobody declares, which is the one rule both
      # families state in the same words, so what differs between the two answers
      # is the fleet and nothing else.
      mirrorNamingNobody = {
        database = "/nonexistent/master.kdbx";
        group = "safix";
        mappings.token = typed keepassxcLib.mapping {
          mode = "safix-to-keepassxc";
          safix = {
            user = "zed";
            name = "token";
          };
          kdbx.path = "zed/token";
        };
      };

      bridgeNamingNobody = {
        clanFlake = "/nonexistent";
        mappings.token = typed bridgeLib.mapping {
          direction = "clan-to-safix";
          clan = {
            machine = "meridian";
            generator = "g";
            file = "f";
          };
          safix = {
            user = "zed";
            name = "token";
          };
        };
      };
    in
    {
      checks.safix-subjects = mkStructuralCheck {
        name = "safix-subjects";
        actual = {
          # An emptied fixture would let every claim below pass by having nothing
          # to judge.
          fixtureRosters =
            lib.mapAttrs
              (_n: f: {
                people = sorted (builtins.attrNames f.users);
                machines = sorted (builtins.attrNames f.machines);
                services = sorted (builtins.attrNames f.services);
                groups = sorted (builtins.attrNames f.groups);
                silos = sorted (builtins.attrNames f.silos);
              })
              {
                inherit
                  machineGrant
                  serviceGrant
                  groupGrant
                  crossSilo
                  ownedByBo
                  ownsBothSides
                  ;
              };

          # ── declaration alone is inert ──
          # Two machines, two groups and a silo declared over the repository's own
          # fleet, referenced by nothing. Every derived artifact — the policy text
          # a consumer commits, the audiences, the placements, the public paths —
          # has to be what it was without them.
          inertness = {
            declaresSubjects = {
              machines = sorted (builtins.attrNames declaredButUnused.machines);
              services = sorted (builtins.attrNames declaredButUnused.services);
              groups = sorted (builtins.attrNames declaredButUnused.groups);
              silos = sorted (builtins.attrNames declaredButUnused.silos);
            };
            identical = derivedFrom declaredButUnused == derivedFrom bare;
            noViolations = violationsOf declaredButUnused;
          };

          # ── a person shares with a machine ──
          machineGrant = {
            audience = (audienceOfToken machineGrant).audience;
            file = fileOfToken machineGrant;
            recipients = (audienceOfToken machineGrant).recipients;

            # The machine's own scope resolves the entry, at the file the
            # audience picked: one file, read from both sides.
            resolvedByTheMachine = filesFor machineGrant { machine = "deck"; };
            resolvedByTheOwner = filesFor machineGrant { user = "ana"; };

            # The rule the policy generates for it names the machine's key under
            # its own anchor, which is what a machine appearing in an audience
            # earns.
            anchors = map (a: a.anchor) (policy.plan machineGrant).anchors;
            ruleAnchors = map (r: {
              inherit (r) audience anchors;
            }) (policy.plan machineGrant).rules;
          };

          # A profile naming a machine nobody declared. Held like the person case:
          # the refusal fires, and the message is read off the named function
          # because `builtins.tryEval` never reports what a throw said.
          undeclaredMachineFires = fires (filesFor machineGrant { machine = "rack"; });
          undeclaredMachineMessage = resolve.unknownMachineMessage machineGrant.machines "rack";

          keylessMachineMessages = violationsOf keylessMachine;
          keylessMachineFires = fires (filesFor keylessMachine { user = "ana"; });

          machineOwnedByNobodyMessages = violationsOf machineOwnedByNobody;
          machineOwnedByNobodyFires = fires (filesFor machineOwnedByNobody { user = "ana"; });

          nameDeclaredTwiceMessages = violationsOf nameDeclaredTwice;
          nameDeclaredTwiceFires = fires (filesFor nameDeclaredTwice { user = "ana"; });

          unsafeMachineNameMessages = violationsOf unsafeMachineName;
          unsafeMachineNameFires = fires (filesFor unsafeMachineName { user = "ana"; });

          # ── a service's audience is its machines ──
          serviceGrant = {
            audience = (audienceOfToken serviceGrant).audience;
            file = fileOfToken serviceGrant;
            recipients = (audienceOfToken serviceGrant).recipients;

            # The machine resolves it, under the service's own key, and the key
            # inside the encrypted file is still the entry's own name.
            resolvedByTheMachine = filesFor serviceGrant { machine = "deck"; };
            resolvedByTheOwner = filesFor serviceGrant { user = "ana"; };
            sopsKeys = lib.mapAttrs (_n: s: s.sopsKey) (
              resolve.selectFor (
                serviceGrant
                // {
                  root = "";
                  hostname = "somewhere";
                  tags = [ ];
                  machine = "deck";
                }
              )
            );

            # The service's declared account and group reach the provisioner at
            # system scope, which is the narrowing a service grant does enforce.
            systemPlacement = materializedFor serviceGrant {
              machine = "deck";
              scope = "system";
            };
          };

          # A machine joining the service and a machine leaving it both leave the
          # file where it was and change the recipient list, exactly as a group's
          # membership does. That is what naming the directory for the service buys.
          serviceGrowthIsARewrap = {
            sameFile = fileOfToken grownService == fileOfToken serviceGrant;
            recipients = (audienceOfToken grownService).recipients;
            resolvedByTheNewMachine = filesFor grownService { machine = "rack"; };
          };

          serviceShrinkIsARewrap = {
            sameFile = fileOfToken shrunkService == fileOfToken serviceGrant;
            recipients = (audienceOfToken shrunkService).recipients;
            departedMachineResolvesNothing = filesFor shrunkService { machine = "deck"; };
          };

          # A group may hold a service, and the expansion reaches the machine's key
          # through both declarations.
          serviceInGroup = {
            audience = (audienceOfToken serviceInGroup).audience;
            recipients = (audienceOfToken serviceInGroup).recipients;
            resolvedByTheMachine = filesFor serviceInGroup { machine = "deck"; };
          };

          # Two services on one machine, granted one name. Two keys, so two entries
          # and two of the provisioner's own default paths; one silently winning is
          # what the composed key exists to prevent.
          twoServicesOneMachine = {
            violations = violationsOf twoServicesOneMachine;
            resolved = filesFor twoServicesOneMachine { machine = "deck"; };
            keys = sorted (
              builtins.attrNames (
                materializedFor twoServicesOneMachine {
                  machine = "deck";
                  scope = "system";
                }
              )
            );
          };

          # The same pair over an entry that declares its own path is two
          # resolutions onto one literal path, refused as any other collision is.
          twoServicesOnePathRefused = fires (
            materializedFor twoServicesOnePath {
              machine = "deck";
              scope = "system";
            }
          );

          # ── the ownership asymmetry, extended ──
          # A service declaring an account is refused where no ownership axis
          # exists, naming the service, the machine and the field; one declaring
          # none resolves there with the scope's ordinary placement.
          serviceOwnershipAtUserScope = {
            refused = fires (
              materializedFor serviceGrant {
                machine = "deck";
                scope = "user";
              }
            );
            ownerlessResolves = materializedFor ownerlessService {
              machine = "deck";
              scope = "user";
            };
            ownerlessAtSystemScope = materializedFor ownerlessService {
              machine = "deck";
              scope = "system";
            };
          };

          emptyServiceMessages = violationsOf emptyService;
          emptyServiceFires = fires (filesFor emptyService { user = "ana"; });

          serviceOverUndeclaredMachineMessages = violationsOf serviceOverUndeclaredMachine;
          serviceOverUndeclaredMachineFires = fires (filesFor serviceOverUndeclaredMachine { user = "ana"; });

          serviceOwnedByNobodyMessages = violationsOf serviceOwnedByNobody;

          serviceNameDeclaredTwiceMessages = violationsOf serviceNameDeclaredTwice;

          unsafeServiceNameMessages = violationsOf unsafeServiceName;

          serviceOnKeylessMachineMessages = violationsOf serviceOnKeylessMachine;

          # ── a group's audience is its members ──
          groupGrant = {
            audience = (audienceOfToken groupGrant).audience;
            file = fileOfToken groupGrant;
            recipients = (audienceOfToken groupGrant).recipients;
            resolvedByAMember = filesFor groupGrant { user = "bo"; };
            resolvedByANonMember = filesFor groupGrant { user = "dee"; };
          };

          # Membership growth and membership shrink both leave the file where it
          # was and change the recipient list. That is what makes each a re-wrap
          # rather than a migration, and it is the whole reason the directory is
          # named for the group.
          growthIsARewrap = {
            sameFile = fileOfToken grownGroup == fileOfToken groupGrant;
            recipients = (audienceOfToken grownGroup).recipients;
            resolvedByTheNewMember = filesFor grownGroup { user = "dee"; };
          };

          shrinkIsARewrap = {
            sameFile = fileOfToken shrunkGroup == fileOfToken groupGrant;
            recipients = (audienceOfToken shrunkGroup).recipients;
            removedMemberResolvesNothing = filesFor shrunkGroup { user = "cy"; };
          };

          nestedGroup = {
            audience = (audienceOfToken nestedGroup).audience;
            recipients = (audienceOfToken nestedGroup).recipients;
            resolvedByTheNestedPerson = filesFor nestedGroup { user = "bo"; };
            resolvedByTheNestedMachine = filesFor nestedGroup { machine = "deck"; };
          };

          ownGroupIsNotACollision = {
            violations = violationsOf ownGroup;
            audience = (audienceOfToken ownGroup).audience;
            ownerResolvesItOnce = filesFor ownGroup { user = "ana"; };
          };

          groupCycleMessages = violationsOf groupCycle;
          groupCycleFires = fires (filesFor groupCycle { user = "ana"; });

          emptyGroupMessages = violationsOf emptyGroup;
          emptyGroupFires = fires (filesFor emptyGroup { user = "ana"; });

          unknownMemberMessages = violationsOf unknownMember;
          unknownMemberFires = fires (filesFor unknownMember { user = "ana"; });

          keylessMemberMessages = violationsOf keylessMember;
          keylessMemberFires = fires (filesFor keylessMember { user = "bo"; });

          memberHoldsTheNameMessages = violationsOf memberHoldsTheName;
          memberHoldsTheNameFires = fires (filesFor memberHoldsTheName { user = "bo"; });

          # ── silos ──
          crossSiloMessages = violationsOf crossSilo;
          crossSiloFires = fires (filesFor crossSilo { user = "ana"; });
          crossSiloListsEveryGrant = violationsOf crossSiloTwice;

          # No rule and no file is generated for a refused audience, which is the
          # claim a message alone does not make: the refusal is where audiences
          # are computed, so there is nothing for a rule to be written from.
          crossSiloGeneratesNoRule = fires (policy.plan crossSilo);

          withinSiloResolves = {
            violations = violationsOf withinSilo;
            file = fileOfToken withinSilo;
          };

          groupInTwoSilosMessages = violationsOf groupInTwoSilos;
          groupInTwoSilosFires = fires (filesFor groupInTwoSilos { user = "ana"; });

          siloOverANonGroupMessages = violationsOf siloOverANonGroup;

          # D3: a person owning machines in two silos is the operator
          # administering both sides and is not refused. A file readable from both
          # is.
          machinesInTwoSilos = {
            violations = violationsOf ownsBothSides;
            eachSideResolves = filesFor ownsBothSides { user = "ana"; };
            spanningFileRefused = fires (filesFor spansBothSides { user = "ana"; });
            spanningMessages = violationsOf spansBothSides;
          };

          # ── ownership is a record a grant resolves through ──
          ownerOf = {
            audience = (audienceOfToken ownedByBo).audience;
            file = fileOfToken ownedByBo;
            recipients = (audienceOfToken ownedByBo).recipients;
            resolvedBy = filesFor ownedByBo { user = "bo"; };

            # The machine itself is not in the audience. The grant named the
            # owner, and a record that also handed the host the value would be a
            # power the record does not confer.
            machineResolvesNothing = filesFor ownedByBo { machine = "deck"; };
          };

          # A change of owner leaves the file where it was and re-wraps it toward
          # the new owner. The old owner's loss of future access is a narrowing
          # `safix check` reports; what is asserted here is that there is one file
          # to re-wrap rather than two to migrate between.
          ownerChangeIsARewrap = {
            sameFile = fileOfToken ownedByCy == fileOfToken ownedByBo;
            recipients = (audienceOfToken ownedByCy).recipients;
            newOwnerResolvesIt = filesFor ownedByCy { user = "cy"; };
            oldOwnerResolvesNothing = filesFor ownedByCy { user = "bo"; };
          };

          ownerOfUnknownMachineMessages = violationsOf ownerOfUnknownMachine;
          ownerOfUnknownMachineFires = fires (filesFor ownerOfUnknownMachine { user = "ana"; });

          ownerOfUnownedMachineMessages = violationsOf ownerOfUnownedMachine;
          ownerOfUnownedMachineFires = fires (filesFor ownerOfUnownedMachine { user = "ana"; });

          ownerOfSelfMessages = violationsOf ownerOfSelf;

          # ── the marked element forms ──
          # A marker that a name could carry would join two distinct audiences
          # into one directory, so one rule over both audiences' secrets — and
          # `audiencesOf` cannot report it, because `listToAttrs` keeps the first
          # binding and drops the second without a word.
          markedElementsAreDistinct = map (p: {
            inherit (p) label;
            distinct = resolve.audienceFileOf p.a != resolve.audienceFileOf p.b;
            fileA = resolve.audienceFileOf p.a;
            fileB = resolve.audienceFileOf p.b;
          }) markedPairs;

          markersOutsideNameAlphabet = lib.mapAttrs (
            _kind: marker: builtins.match "[a-z0-9_-]*" marker == null
          ) resolve.audienceMarkers;

          # Rendering a reference and reading it back is the identity, which is
          # what lets a file's audience be turned into the recipients it is
          # wrapped for.
          referencesRoundTrip = roundTrips serviceInGroup [
            "ana"
            "deck"
            "nginx"
            "oncall"
            "ownerOf.deck"
          ];

          # Neither relationship family reports over a fleet the subject model
          # refuses, and both report the same mapping over one it accepts.
          relationsWaitForCustody = {
            keepassxcQuietWhileRefused = safixChecks.keepassxcMessages crossSilo mirrorNamingNobody;
            keepassxcSpeaksWhenResolved = safixChecks.keepassxcMessages withinSilo mirrorNamingNobody != [ ];
            bridgeQuietWhileRefused = safixChecks.bridgeMessages crossSilo bridgeNamingNobody;
            bridgeSpeaksWhenResolved = safixChecks.bridgeMessages withinSilo bridgeNamingNobody != [ ];
          };

          # The generated rules over a subject-bearing fleet are still exactly one
          # directory each, still terminate on the extension, and still reach
          # nothing no declaration places anything in. A marker inside a directory
          # name is a character in a `path_regex`, so this is where a marker that
          # was a regex metacharacter would be caught.
          generatedRulesOverSubjects =
            lib.mapAttrs
              (_n: fleet: {
                ruleShape = safixChecks.ruleShapeMessages fleet;
                catchAll = safixChecks.catchAllMessages fleet;
                separator = safixChecks.separatorMessages fleet;
              })
              {
                inherit
                  machineGrant
                  serviceGrant
                  serviceInGroup
                  groupGrant
                  nestedGroup
                  ownedByBo
                  ;
              };
        };

        expected = {
          fixtureRosters = {
            machineGrant = {
              people = [ "ana" ];
              machines = [ "deck" ];
              services = [ ];
              groups = [ ];
              silos = [ ];
            };
            serviceGrant = {
              people = [ "ana" ];
              machines = [
                "deck"
                "rack"
              ];
              services = [ "nginx" ];
              groups = [ ];
              silos = [ ];
            };
            groupGrant = {
              people = [
                "ana"
                "bo"
                "cy"
                "dee"
              ];
              machines = [ ];
              services = [ ];
              groups = [ "oncall" ];
              silos = [ ];
            };
            crossSilo = {
              people = [
                "ana"
                "bo"
                "cy"
              ];
              machines = [ ];
              services = [ ];
              groups = [
                "contractors"
                "partners"
                "staff"
              ];
              silos = [ "corp" ];
            };
            ownedByBo = {
              people = [
                "ana"
                "bo"
                "cy"
              ];
              machines = [ "deck" ];
              services = [ ];
              groups = [ ];
              silos = [ ];
            };
            ownsBothSides = {
              people = [
                "ana"
                "bo"
                "cy"
              ];
              machines = [
                "deck"
                "rack"
              ];
              services = [ ];
              groups = [
                "blue"
                "red"
              ];
              silos = [ "corp" ];
            };
          };

          inertness = {
            declaresSubjects = {
              machines = [
                "deck"
                "fixture-host"
                "rack"
              ];
              services = [
                "fixture-web"
                "nginx"
              ];
              groups = [
                "infra"
                "oncall"
              ];
              silos = [ "corp" ];
            };
            identical = true;
            noViolations = [ ];
          };

          machineGrant = {
            audience = [
              "ana"
              "deck"
            ];
            file = "secrets/safix/shared/ana,deck/secrets.yaml";
            recipients = [
              (keyOf "ana")
              (keyOf "deck")
            ];
            resolvedByTheMachine.token = "/secrets/safix/shared/ana,deck/secrets.yaml";
            resolvedByTheOwner.token = "/secrets/safix/shared/ana,deck/secrets.yaml";
            anchors = [
              "ana-safix"
              "deck-safix"
            ];
            ruleAnchors = [
              {
                audience = [
                  "ana"
                  "deck"
                ];
                anchors = [
                  "ana-safix"
                  "deck-safix"
                ];
              }
            ];
          };

          undeclaredMachineFires = true;
          undeclaredMachineMessage = ''
            safix: 'rack' is not a declared machine of flake.safix.machines.

            Declared machines:
              - deck

            A profile selects a machine with safix.machine, which has no default: a
            machine is a subject an audience names, and safix has no host registry to
            derive one from. Name one of the above, or declare this one in
            flake.safix.machines with the age form of the host identity it already
            decrypts with.
          '';

          keylessMachineMessages = [
            "flake.safix.users.ana.sharedWith.deck shares 'token', but flake.safix.machines.deck.recipient is null, so no copy can be encrypted to them"
          ];
          keylessMachineFires = true;

          machineOwnedByNobodyMessages = [
            "flake.safix.machines.deck.owner names 'zed', which is not a declared user of flake.safix.users"
          ];
          machineOwnedByNobodyFires = true;

          nameDeclaredTwiceMessages = [
            "'deck' is declared as more than one kind of subject, by flake.safix.users and flake.safix.machines; people, machines, services and groups share one name space"
          ];
          nameDeclaredTwiceFires = true;

          unsafeMachineNameMessages = [
            "flake.safix.machines names 'Deck', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];
          unsafeMachineNameFires = true;

          serviceGrant = {
            audience = [
              "%nginx"
              "ana"
            ];
            file = "secrets/safix/shared/%nginx,ana/secrets.yaml";
            recipients = [
              (keyOf "deck")
              (keyOf "ana")
            ];
            resolvedByTheMachine."nginx/token" = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
            resolvedByTheOwner.token = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
            sopsKeys."nginx/token" = "token";
            systemPlacement."nginx/token" = {
              mode = "0400";
              sopsFile = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
              key = "token";
              owner = "nginx";
              group = "nginx";
            };
          };

          serviceGrowthIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "deck")
              (keyOf "rack")
              (keyOf "ana")
            ];
            resolvedByTheNewMachine."nginx/token" = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
          };

          serviceShrinkIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "rack")
              (keyOf "ana")
            ];
            departedMachineResolvesNothing = { };
          };

          serviceInGroup = {
            audience = [
              "@oncall"
              "ana"
            ];
            recipients = [
              (keyOf "deck")
              (keyOf "ana")
            ];
            resolvedByTheMachine."nginx/token" = "/secrets/safix/shared/@oncall,ana/secrets.yaml";
          };

          twoServicesOneMachine = {
            violations = [ ];
            resolved = {
              "alpha/token" = "/secrets/safix/shared/%alpha,%beta,ana/secrets.yaml";
              "beta/token" = "/secrets/safix/shared/%alpha,%beta,ana/secrets.yaml";
            };
            keys = [
              "alpha/token"
              "beta/token"
            ];
          };

          twoServicesOnePathRefused = true;

          serviceOwnershipAtUserScope = {
            refused = true;
            ownerlessResolves."nginx/token" = {
              mode = "0400";
              sopsFile = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
              key = "token";
            };
            ownerlessAtSystemScope."nginx/token" = {
              mode = "0400";
              sopsFile = "/secrets/safix/shared/%nginx,ana/secrets.yaml";
              key = "token";
            };
          };

          emptyServiceMessages = [
            "flake.safix.users.ana.sharedWith.nginx shares 'token' with flake.safix.services.nginx, whose machines is empty, so the file would be encrypted to nobody"
          ];
          emptyServiceFires = true;

          serviceOverUndeclaredMachineMessages = [
            "flake.safix.services.nginx.machines names 'rack', which is not a declared machine of flake.safix.machines"
          ];
          serviceOverUndeclaredMachineFires = true;

          serviceOwnedByNobodyMessages = [
            "flake.safix.services.nginx.owner names 'zed', which is not a declared user of flake.safix.users"
          ];

          serviceNameDeclaredTwiceMessages = [
            "'nginx' is declared as more than one kind of subject, by flake.safix.machines and flake.safix.services; people, machines, services and groups share one name space"
          ];

          unsafeServiceNameMessages = [
            "flake.safix.services names 'Nginx', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];

          serviceOnKeylessMachineMessages = [
            "flake.safix.users.ana.sharedWith.nginx shares 'token' with flake.safix.machines.deck, reached through flake.safix.services.nginx, but flake.safix.machines.deck.recipient is null, so no copy can be encrypted to them"
          ];

          groupGrant = {
            audience = [
              "@oncall"
              "ana"
            ];
            file = "secrets/safix/shared/@oncall,ana/secrets.yaml";
            recipients = [
              (keyOf "bo")
              (keyOf "cy")
              (keyOf "ana")
            ];
            resolvedByAMember.token = "/secrets/safix/shared/@oncall,ana/secrets.yaml";
            resolvedByANonMember = { };
          };

          growthIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "bo")
              (keyOf "cy")
              (keyOf "dee")
              (keyOf "ana")
            ];
            resolvedByTheNewMember.token = "/secrets/safix/shared/@oncall,ana/secrets.yaml";
          };

          shrinkIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "bo")
              (keyOf "ana")
            ];
            removedMemberResolvesNothing = { };
          };

          nestedGroup = {
            audience = [
              "@outer"
              "ana"
            ];
            recipients = [
              (keyOf "bo")
              (keyOf "deck")
              (keyOf "ana")
            ];
            resolvedByTheNestedPerson.token = "/secrets/safix/shared/@outer,ana/secrets.yaml";
            resolvedByTheNestedMachine.token = "/secrets/safix/shared/@outer,ana/secrets.yaml";
          };

          ownGroupIsNotACollision = {
            violations = [ ];
            audience = [
              "@oncall"
              "ana"
            ];
            ownerResolvesItOnce.token = "/secrets/safix/shared/@oncall,ana/secrets.yaml";
          };

          groupCycleMessages = [
            "flake.safix.groups declares a cycle: 'inner' -> 'outer' -> 'inner'. A membership that cannot be expanded is not a membership."
          ];
          groupCycleFires = true;

          emptyGroupMessages = [
            "flake.safix.users.ana.sharedWith.oncall shares 'token' with flake.safix.groups.oncall, which reaches no subject beyond flake.safix.users.ana, so the grant widens nothing"
          ];
          emptyGroupFires = true;

          unknownMemberMessages = [
            "flake.safix.groups.oncall.members names 'zed', which is not a declared subject of flake.safix.users, flake.safix.machines, flake.safix.services or flake.safix.groups"
          ];
          unknownMemberFires = true;

          keylessMemberMessages = [
            "flake.safix.users.ana.sharedWith.oncall shares 'token' with flake.safix.users.bo, reached through flake.safix.groups.oncall, but flake.safix.users.bo.recipient is null, so no copy can be encrypted to them"
          ];
          keylessMemberFires = true;

          memberHoldsTheNameMessages = [
            "flake.safix.users.bo declares 'token' in flake.safix.users.bo.private, and flake.safix.users.ana.sharedWith.oncall shares a secret of that name with flake.safix.users.bo, reached through flake.safix.groups.oncall"
          ];
          memberHoldsTheNameFires = true;

          crossSiloMessages = [
            "flake.safix.users.ana's 'token' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.contractors reaches bo and flake.safix.groups.staff reaches ana. secrets/safix/shared/@contractors,ana/secrets.yaml is one file with one data key, so it would be readable from both."
          ];
          crossSiloFires = true;
          crossSiloGeneratesNoRule = true;
          crossSiloListsEveryGrant = [
            "flake.safix.users.ana's 'other' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.contractors reaches bo and flake.safix.groups.staff reaches ana. secrets/safix/shared/@contractors,ana/secrets.yaml is one file with one data key, so it would be readable from both."
            "flake.safix.users.ana's 'token' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.contractors reaches bo and flake.safix.groups.staff reaches ana. secrets/safix/shared/@contractors,ana/secrets.yaml is one file with one data key, so it would be readable from both."
          ];

          withinSiloResolves = {
            violations = [ ];
            file = "secrets/safix/shared/@partners,ana/secrets.yaml";
          };

          groupInTwoSilosMessages = [
            "flake.safix.groups.contractors is named by more than one silo set, flake.safix.silos.corp and flake.safix.silos.vendor. A group in two sets closes each set's exclusions over the other's, which is one set written as two."
          ];
          groupInTwoSilosFires = true;

          siloOverANonGroupMessages = [
            "flake.safix.silos.corp.groups names 'deck', which is not a declared group of flake.safix.groups"
            "flake.safix.silos.corp.groups names 'absent', which is not a declared group of flake.safix.groups"
          ];

          machinesInTwoSilos = {
            violations = [ ];
            eachSideResolves = {
              both = "/secrets/safix/users/ana/secrets.yaml";
              other = "/secrets/safix/shared/@blue,ana/secrets.yaml";
              token = "/secrets/safix/shared/@red,ana/secrets.yaml";
            };
            spanningFileRefused = true;
            spanningMessages = [
              "flake.safix.users.ana's 'both' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.blue reaches cy, rack and flake.safix.groups.red reaches bo, deck. secrets/safix/shared/@blue,@red,ana/secrets.yaml is one file with one data key, so it would be readable from both."
            ];
          };

          ownerOf = {
            audience = [
              "@~deck"
              "ana"
            ];
            file = "secrets/safix/shared/@~deck,ana/secrets.yaml";
            recipients = [
              (keyOf "bo")
              (keyOf "ana")
            ];
            resolvedBy.token = "/secrets/safix/shared/@~deck,ana/secrets.yaml";
            machineResolvesNothing = { };
          };

          ownerChangeIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "cy")
              (keyOf "ana")
            ];
            newOwnerResolvesIt.token = "/secrets/safix/shared/@~deck,ana/secrets.yaml";
            oldOwnerResolvesNothing = { };
          };

          ownerOfUnknownMachineMessages = [
            "flake.safix.users.ana.sharedWith names the owner of 'rack', which is not a declared machine of flake.safix.machines"
          ];
          ownerOfUnknownMachineFires = true;

          ownerOfUnownedMachineMessages = [
            "flake.safix.users.ana.sharedWith names the owner of flake.safix.machines.deck, which records none, so the grant resolves to nobody"
          ];
          ownerOfUnownedMachineFires = true;

          ownerOfSelfMessages = [
            "flake.safix.users.ana.sharedWith.\"ownerOf.deck\" shares 'token' with the owner flake.safix.machines.deck records, which reaches no subject beyond flake.safix.users.ana, so the grant widens nothing"
          ];

          markedElementsAreDistinct = [
            {
              label = "a group and a person of that name";
              distinct = true;
              fileA = "secrets/safix/shared/@oncall,ana/secrets.yaml";
              fileB = "secrets/safix/shared/ana,oncall/secrets.yaml";
            }
            {
              label = "an owner reference and a group of that name";
              distinct = true;
              fileA = "secrets/safix/shared/@~deck,ana/secrets.yaml";
              fileB = "secrets/safix/shared/@deck,ana/secrets.yaml";
            }
            {
              label = "a service and a person of that name";
              distinct = true;
              fileA = "secrets/safix/shared/%nginx,ana/secrets.yaml";
              fileB = "secrets/safix/shared/ana,nginx/secrets.yaml";
            }
            {
              label = "a service and a group of that name";
              distinct = true;
              fileA = "secrets/safix/shared/%nginx,ana/secrets.yaml";
              fileB = "secrets/safix/shared/@nginx,ana/secrets.yaml";
            }
          ];

          markersOutsideNameAlphabet = {
            group = true;
            owner = true;
            service = true;
          };

          referencesRoundTrip = [
            true
            true
            true
            true
            true
          ];

          relationsWaitForCustody = {
            keepassxcQuietWhileRefused = [ ];
            keepassxcSpeaksWhenResolved = true;
            bridgeQuietWhileRefused = [ ];
            bridgeSpeaksWhenResolved = true;
          };

          generatedRulesOverSubjects =
            lib.genAttrs
              [
                "machineGrant"
                "serviceGrant"
                "serviceInGroup"
                "groupGrant"
                "nestedGroup"
                "ownedByBo"
              ]
              (_: {
                ruleShape = [ ];
                catchAll = [ ];
                separator = [ ];
              });
        };
      };
    };
}
