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
# Writing an organization's custody keys into each consenting person's
# `recoveryRecipients` rather than expanding them at resolution time leaves
# `escrowedConsent` green and fails `escrowedConsent.recoveryRecipientsUntouched`
# and `rotationHappensInOnePlace.noDeclarationChanged` — the pair that is design
# decision D3, and the whole property this phase adds.
# Giving an organization an audience element of its own on the escrow path fails
# `escrowedConsent.file` and `escrowedConsent.audience`: consent widens who can
# open a person's files and never who the files are for, so it moves nothing.
# Dropping the organization marker from `elementOf` fails
# `organizationGrant.file` and both organization rows of
# `markedElementsAreDistinct`, and the second is the severe half for the reason
# the group one is.
# Resolving `ownerOf.<machine>` only through `users` fails
# `organizationOwnership.recipients`, which is what D4 says the name space is
# checked against; naming that audience's file for the resolved owner instead
# leaves the recipients right and fails
# `organizationOwnership.sameFileAfterOwnerChange`.
# Reporting only the first empty-custody reach fails `emptyCustodyMessages`,
# whose three sentences are the three ways an organization is reached.
# Admitting an organization as a group member fails
# `organizationInAGroupMessages`, and leaving organizations out of the one subject
# name space fails `organizationNameDeclaredTwiceMessages` and
# `unsafeOrganizationNameMessages`.
# Letting a delegation reach an audience, a placement or the policy — a manager's
# key added to the files of the people they manage would be the obvious way to do
# it — fails `inertness.identical` while leaving every delegation row green, which
# is the pair that says managing is not reading.
# Covering a group by its own members' `managedBy` rather than by the silo set it
# is named in fails `delegation.groups` on `contractors` and `vendors`, the two
# groups that hold none of their organization's people; dropping the coverage
# rule entirely fails all four covered groups and leaves `standby` green.
# Reporting only the first bad manager fails `managersNobodyDeclaresMessages`,
# whose two sentences are one repair an operator makes in one pass.
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

      # A fleet, as the seven records the resolver reads. `machines`, `services`,
      # `groups` and `organizations` go through their own submodules for the same
      # reason `users` does.
      fleetOf =
        {
          users ? { },
          catalogue ? { },
          machines ? { },
          services ? { },
          groups ? { },
          organizations ? { },
          silos ? { },
        }:
        {
          users = typed (lib.types.attrsOf types.profile) users;
          catalogue = typed (lib.types.attrsOf types.entry) catalogue;
          machines = typed (lib.types.attrsOf types.machine) machines;
          services = typed (lib.types.attrsOf types.service) services;
          groups = typed (lib.types.attrsOf types.group) groups;
          organizations = typed (lib.types.attrsOf types.organization) organizations;
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

      fileOfToken = fleet: (resolve.placementsOf fleet).alice.token.file;

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
          organizations
          ;
      };

      declaredButUnused = bare // {
        # The delegation the rest of this record is judged against: acme names
        # alice a manager and bob consents to acme's management. Nothing else about
        # the fleet changes, so `identical` below is the whole byte-inertness
        # claim for a delegation — a record that decides who may act and places no
        # key anywhere.
        users = typed (lib.types.attrsOf types.profile) (
          fixture.fleet.users
          // {
            bob = fixture.fleet.users.bob // {
              managedBy = "acme";
            };
          }
        );
        machines = typed (lib.types.attrsOf types.machine) (
          fixture.fleet.machines
          // {
            deck = machine "deck" "alice";
            rack = machine "rack" "bob";
          }
        );
        services = typed (lib.types.attrsOf types.service) (
          fixture.fleet.services
          // {
            nginx = {
              machines = [ "deck" ];
              owner = "alice";
              user = "nginx";
              group = "nginx";
            };
          }
        );
        groups = typed (lib.types.attrsOf types.group) (
          fixture.fleet.groups
          // {
            oncall.members = [
              "alice"
              "bob"
            ];
            infra.members = [
              "deck"
              "rack"
            ];
          }
        );
        organizations = typed (lib.types.attrsOf types.organization) (
          fixture.fleet.organizations
          // {
            acme = fixture.fleet.organizations.acme // {
              managers = [ "alice" ];
            };
            globex.custody.globex-escrow.key = keyOf "globex-escrow";
          }
        );
        silos = typed (lib.types.attrsOf types.silo) (
          fixture.fleet.silos
          // {
            corp.groups = [
              "oncall"
              "infra"
            ];
          }
        );
      };

      derivedFrom = fleet: {
        policyText = policy.render fleet;
        audiences = resolve.audiencesOf fleet;
        placements = resolve.placementsOf fleet;
        publicPaths = resolve.publicPathsOf fleet;
      };

      # ── machines as subjects ──
      machineGrant = fleetOf {
        users.alice = holder "alice" { sharedWith.deck.token = { }; };
        machines.deck = machine "deck" "alice";
      };

      keylessMachine = fleetOf {
        users.alice = holder "alice" { sharedWith.deck.token = { }; };
        machines.deck = {
          owner = "alice";
        };
      };

      machineOwnedByNobody = fleetOf {
        users.alice = holder "alice" { };
        machines.deck = machine "deck" "zed";
      };

      nameDeclaredTwice = fleetOf {
        users = {
          alice = holder "alice" { };
          deck = keyholder "deck";
        };
        machines.deck = machine "deck" "alice";
      };

      unsafeMachineName = fleetOf {
        users.alice = holder "alice" { };
        machines."Deck" = machine "deck" "alice";
      };

      # ── services as subjects ──
      # A service resolving to one machine's key. `nginx` declares ownership, which
      # is what the system scope carries and the user scope refuses.
      serviceOn =
        hosts:
        fleetOf {
          users.alice = holder "alice" { sharedWith.nginx.token = { }; };
          machines = {
            deck = machine "deck" "alice";
            rack = machine "rack" "alice";
          };
          services.nginx = {
            machines = hosts;
            owner = "alice";
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
        users.alice = holder "alice" { sharedWith.nginx.token = { }; };
        machines.deck = machine "deck" "alice";
        services.nginx = {
          machines = [ "deck" ];
          owner = "alice";
        };
      };

      # A group whose member is a service, so the expansion crosses both kinds on
      # the way to a machine's key.
      serviceInGroup = fleetOf {
        users.alice = holder "alice" { sharedWith.oncall.token = { }; };
        machines.deck = machine "deck" "alice";
        services.nginx = {
          machines = [ "deck" ];
          owner = "alice";
        };
        groups.oncall.members = [ "nginx" ];
      };

      # Two services on one machine, granted one name. Each resolves under its own
      # key, so neither replaces the other and the provisioner's own default path —
      # a function of the name — is what holds them apart.
      twoServices =
        entry:
        fleetOf {
          users.alice = {
            recipient = keyOf "alice";
            private.token = entry;
            sharedWith = {
              alpha.token = { };
              beta.token = { };
            };
          };
          machines.deck = machine "deck" "alice";
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
        users.alice = holder "alice" { };
        machines.deck = machine "deck" "alice";
        services.nginx.machines = [
          "rack"
          "deck"
        ];
      };

      serviceOwnedByNobody = fleetOf {
        users.alice = holder "alice" { };
        machines.deck = machine "deck" "alice";
        services.nginx = {
          machines = [ "deck" ];
          owner = "zed";
        };
      };

      serviceNameDeclaredTwice = fleetOf {
        users.alice = holder "alice" { };
        machines.nginx = machine "nginx" "alice";
        services.nginx.machines = [ "nginx" ];
      };

      unsafeServiceName = fleetOf {
        users.alice = holder "alice" { };
        machines.deck = machine "deck" "alice";
        services."Nginx".machines = [ "deck" ];
      };

      # A service whose machine records no key. The service resolves and the data
      # key cannot be wrapped for it, reported with the service named as the
      # declaration that put the machine in the audience.
      serviceOnKeylessMachine = fleetOf {
        users.alice = holder "alice" { sharedWith.nginx.token = { }; };
        machines.deck.owner = "alice";
        services.nginx.machines = [ "deck" ];
      };

      # ── groups ──
      # bob and carol each hold a key and nothing else, so what the group's
      # audience gains is exactly its members' recipients.
      groupFleet =
        members:
        fleetOf {
          users = {
            alice = holder "alice" { sharedWith.oncall.token = { }; };
            bob = keyholder "bob";
            carol = keyholder "carol";
            dave = keyholder "dave";
          };
          groups.oncall.members = members;
        };

      groupGrant = groupFleet [
        "bob"
        "carol"
      ];

      grownGroup = groupFleet [
        "bob"
        "carol"
        "dave"
      ];

      shrunkGroup = groupFleet [ "bob" ];

      # A group whose members are a group and a machine, so the expansion is
      # transitive and reaches both kinds of leaf.
      nestedGroup = fleetOf {
        users = {
          alice = holder "alice" { sharedWith.outer.token = { }; };
          bob = keyholder "bob";
        };
        machines.deck = machine "deck" "alice";
        groups = {
          outer.members = [
            "inner"
            "deck"
          ];
          inner.members = [ "bob" ];
        };
      };

      # Sharing with a group one is a member of. The owner is already in the
      # audience, so their own membership must not read as receiving their own
      # secret from outside.
      ownGroup = fleetOf {
        users = {
          alice = holder "alice" { sharedWith.oncall.token = { }; };
          bob = keyholder "bob";
        };
        groups.oncall.members = [
          "alice"
          "bob"
        ];
      };

      groupCycle = fleetOf {
        users.alice = holder "alice" { sharedWith.outer.token = { }; };
        groups = {
          outer.members = [ "inner" ];
          inner.members = [ "outer" ];
        };
      };

      emptyGroup = fleetOf {
        users.alice = holder "alice" { sharedWith.oncall.token = { }; };
        groups.oncall.members = [ ];
      };

      unknownMember = fleetOf {
        users.alice = holder "alice" { };
        groups.oncall.members = [ "zed" ];
      };

      # A member who holds no key. The group resolves, and the data key cannot be
      # wrapped for them, which is the same defect a direct grant to a keyless
      # person has and is reported with the group named.
      keylessMember = fleetOf {
        users = {
          alice = holder "alice" { sharedWith.oncall.token = { }; };
          bob = { };
        };
        groups.oncall.members = [ "bob" ];
      };

      # A group member who already holds the name. Their own value and the
      # group's are two statements about one name in one resolved set.
      memberHoldsTheName = fleetOf {
        users = {
          alice = holder "alice" { sharedWith.oncall.token = { }; };
          bob = {
            recipient = keyOf "bob";
            private.token = { };
          };
        };
        groups.oncall.members = [ "bob" ];
      };

      # ── silos ──
      # alice is staff and shares with a contractor, which is the corporate
      # case: one file both sides could open.
      siloFleet =
        grants:
        fleetOf {
          users = {
            alice = holder "alice" grants;
            bob = keyholder "bob";
            carol = keyholder "carol";
          };
          groups = {
            staff.members = [ "alice" ];
            contractors.members = [ "bob" ];
            partners.members = [ "carol" ];
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
          alice = {
            recipient = keyOf "alice";
            private = {
              token = { };
              other = { };
            };
            sharedWith.contractors = {
              token = { };
              other = { };
            };
          };
          bob = keyholder "bob";
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

      withinSilo = siloFleet { sharedWith.partners.token = { }; };

      groupInTwoSilos = fleetOf {
        users.alice = holder "alice" { };
        groups = {
          staff.members = [ "alice" ];
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
        users.alice = holder "alice" { };
        machines.deck = machine "deck" "alice";
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
            alice = {
              recipient = keyOf "alice";
              private = {
                token = { };
                other = { };
                both = { };
              };
            }
            // grants;
            bob = keyholder "bob";
            carol = keyholder "carol";
          };
          machines = {
            deck = machine "deck" "alice";
            rack = machine "rack" "alice";
          };
          groups = {
            red.members = [
              "deck"
              "bob"
            ];
            blue.members = [
              "rack"
              "carol"
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
            alice = holder "alice" { sharedWith."ownerOf.deck".token = { }; };
            bob = keyholder "bob";
            carol = keyholder "carol";
          };
          machines.deck = machine "deck" owner;
        };

      ownedByBo = ownerOfFleet "bob";
      ownedByCy = ownerOfFleet "carol";

      ownerOfUnknownMachine = fleetOf {
        users.alice = holder "alice" { sharedWith."ownerOf.rack".token = { }; };
        machines.deck = machine "deck" "alice";
      };

      ownerOfUnownedMachine = fleetOf {
        users.alice = holder "alice" { sharedWith."ownerOf.deck".token = { }; };
        machines.deck = {
          recipient = keyOf "deck";
        };
      };

      # An `ownerOf` grant resolving to its own author widens nothing.
      ownerOfSelf = fleetOf {
        users.alice = holder "alice" { sharedWith."ownerOf.deck".token = { }; };
        machines.deck = machine "deck" "alice";
      };

      # ── organizations as principals ──
      # acme holds one custody key under one anchor. The rotated fleet is the same
      # declaration with a different key under that anchor, which is what a
      # rotation is, and the withdrawn fleet is alice's consent removed with acme
      # untouched.
      acmeCustody = {
        acme-escrow.key = keyOf "acme-escrow";
      };
      rotatedCustody = {
        acme-escrow.key = keyOf "acme-rotated";
      };

      escrowFleet =
        {
          custody,
          consenting ? true,
        }:
        fleetOf {
          users.alice = holder "alice" (lib.optionalAttrs consenting { escrowedTo = [ "acme" ]; });
          organizations.acme.custody = custody;
        };

      escrowed = escrowFleet { custody = acmeCustody; };
      rotatedEscrow = escrowFleet { custody = rotatedCustody; };
      withdrawnEscrow = escrowFleet {
        custody = acmeCustody;
        consenting = false;
      };

      # A grant naming the organization itself, where the escrow fleet above names
      # it from alice's record. alice does not consent here, so her own file's
      # recipients are hers alone and the two mechanisms stay separable.
      organizationGrant = fleetOf {
        users.alice = holder "alice" { sharedWith.acme.token = { }; };
        organizations.acme.custody = acmeCustody;
      };

      # A machine acme owns, granted through its ownership record. The owner change
      # is to a person, so one fleet shows the resolution branching on what the name
      # denotes.
      ownedMachineFleet =
        owner:
        fleetOf {
          users = {
            alice = holder "alice" { sharedWith."ownerOf.rack".token = { }; };
            bob = keyholder "bob";
          };
          machines.rack = machine "rack" owner;
          organizations.acme.custody = acmeCustody;
        };

      ownedByAcme = ownedMachineFleet "acme";
      ownedByBobInstead = ownedMachineFleet "bob";

      # Every way of reaching an organization with no custody, in one fleet: alice's
      # escrow consent, a grant naming it, and a grant resolving through a machine it
      # owns. One evaluation reports all three, because an operator repairing a
      # custody declaration one refusal per rebuild does not know how many are left.
      emptyCustody = fleetOf {
        users.alice = {
          recipient = keyOf "alice";
          private = {
            token = { };
            other = { };
          };
          escrowedTo = [ "acme" ];
          sharedWith = {
            acme.token = { };
            "ownerOf.rack".other = { };
          };
        };
        machines.rack = machine "rack" "acme";
        organizations.acme = { };
      };

      escrowToNobody = fleetOf {
        users.alice = holder "alice" { escrowedTo = [ "acme" ]; };
      };

      organizationInAGroup = fleetOf {
        users.alice = holder "alice" { sharedWith.oncall.token = { }; };
        groups.oncall.members = [ "acme" ];
        organizations.acme.custody = acmeCustody;
      };

      organizationNameDeclaredTwice = fleetOf {
        users.alice = holder "alice" { };
        groups.acme.members = [ "alice" ];
        organizations.acme.custody = acmeCustody;
      };

      unsafeOrganizationName = fleetOf {
        users.alice = holder "alice" { };
        organizations."Acme".custody = acmeCustody;
      };

      unsafeCustodyAnchor = fleetOf {
        users.alice = holder "alice" { };
        organizations.acme.custody."Escrow".key = keyOf "acme-escrow";
      };

      # One anchor over two keys, across the two records that define anchors. The
      # generated policy would define it twice and every rule referencing it would
      # resolve to whichever definition YAML kept.
      anchorSharedWithAPerson = fleetOf {
        users.alice = {
          recipient = keyOf "alice";
          recoveryRecipients.acme-escrow.key = keyOf "alice-escrow";
          private.token = { };
        };
        organizations.acme.custody = acmeCustody;
      };

      # ── delegation ──
      # Two organizations, five groups, two silo sets. acme manages bob, who is in
      # `oncall`, and the `corp` set holds `oncall` and `contractors` apart — so
      # both are acme's to manage, `contractors` included, which holds none of
      # acme's people. globex manages carol in `partners`, so `partners` and
      # `vendors` are globex's on the same terms. `standby` is in no silo set and
      # is nobody's, which is what leaves a group anyone who can commit may edit.
      #
      # dave is in two groups held apart by two different sets, which is legal and
      # is what makes coverage a property of the group rather than of the person.
      delegated = fleetOf {
        users = {
          alice = holder "alice" { };
          bob = keyholder "bob" // {
            managedBy = "acme";
          };
          carol = keyholder "carol" // {
            managedBy = "globex";
          };
          dave = keyholder "dave";
        };
        groups = {
          oncall.members = [ "bob" ];
          contractors.members = [ "dave" ];
          partners.members = [ "carol" ];
          vendors.members = [ "dave" ];
          standby.members = [ "bob" ];
        };
        organizations = {
          acme = {
            custody = acmeCustody;
            managers = [ "alice" ];
          };
          # No custody at all, and it manages people regardless: managing is not
          # reading, so an organization that holds no key still names managers.
          globex.managers = [
            "alice"
            "dave"
          ];
        };
        silos = {
          corp.groups = [
            "oncall"
            "contractors"
          ];
          trade.groups = [
            "partners"
            "vendors"
          ];
        };
      };

      # Two managers nobody declared, so the refusal is shown to list every
      # violation rather than the first one it meets.
      managersNobodyDeclares = fleetOf {
        users.alice = holder "alice" { };
        organizations.acme = {
          custody = acmeCustody;
          managers = [
            "mallory"
            "zed"
          ];
        };
      };

      managedByNobodyDeclares = fleetOf {
        users = {
          alice = holder "alice" { managedBy = "globex"; };
          bob = keyholder "bob" // {
            managedBy = "globex";
          };
        };
        organizations.acme = {
          custody = acmeCustody;
          managers = [ "alice" ];
        };
      };

      # One fleet declaring every kind of subject, for the round trip alone.
      everyKind = fleetOf {
        users.alice = holder "alice" { };
        machines.deck = machine "deck" "alice";
        services.nginx.machines = [ "deck" ];
        groups.oncall.members = [ "alice" ];
        organizations.acme.custody = acmeCustody;
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
            "alice"
          ];
          b = [
            "alice"
            "oncall"
          ];
        }
        {
          label = "an owner reference and a group of that name";
          a = [
            "@~deck"
            "alice"
          ];
          b = [
            "@deck"
            "alice"
          ];
        }
        {
          label = "a service and a person of that name";
          a = [
            "%nginx"
            "alice"
          ];
          b = [
            "alice"
            "nginx"
          ];
        }
        {
          label = "a service and a group of that name";
          a = [
            "%nginx"
            "alice"
          ];
          b = [
            "@nginx"
            "alice"
          ];
        }
        {
          label = "an organization and a person of that name";
          a = [
            "=acme"
            "alice"
          ];
          b = [
            "acme"
            "alice"
          ];
        }
        {
          label = "an organization and a group of that name";
          a = [
            "=acme"
            "alice"
          ];
          b = [
            "@acme"
            "alice"
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
                organizations = sorted (builtins.attrNames f.organizations);
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
                  escrowed
                  organizationGrant
                  ownedByAcme
                  delegated
                  ;
              };

          # ── declaration alone is inert ──
          # Two machines, a service, two groups, an organization, a silo and a
          # delegation declared over the repository's own fleet, referenced by
          # nothing. Every derived artifact — the policy text a consumer commits,
          # the audiences, the placements, the public paths — has to be what it was
          # without them.
          #
          # The delegation is the one record here that is referenced and still
          # inert: acme names alice a manager and bob consents to acme's
          # management, and every artifact below is byte-identical because a
          # delegation decides who may act and places no key in any audience. The
          # projection is read beside the claim so that an emptied delegation
          # cannot make the claim pass by having nothing to be inert about.
          inertness = {
            declaresSubjects = {
              machines = sorted (builtins.attrNames declaredButUnused.machines);
              services = sorted (builtins.attrNames declaredButUnused.services);
              groups = sorted (builtins.attrNames declaredButUnused.groups);
              organizations = sorted (builtins.attrNames declaredButUnused.organizations);
              silos = sorted (builtins.attrNames declaredButUnused.silos);
            };
            declaresDelegation = {
              inherit (resolve.delegationOf declaredButUnused) managers managedBy;
            };
            identical = derivedFrom declaredButUnused == derivedFrom bare;
            noViolations = violationsOf declaredButUnused;
          };

          # ── delegation is recorded on both sides ──
          # The projection the scaffolding verbs read, over a fleet whose silo sets
          # are what decide which groups each organization manages. `contractors`
          # and `vendors` hold none of their organization's people and are covered
          # regardless, because a silo set is administered as one boundary;
          # `standby` is in no set and is covered by nobody.
          delegation = resolve.delegationOf delegated;
          delegationResolves = violationsOf delegated;

          managersNobodyDeclaresMessages = violationsOf managersNobodyDeclares;
          managersNobodyDeclaresFires = fires (filesFor managersNobodyDeclares { user = "alice"; });

          managedByNobodyDeclaresMessages = violationsOf managedByNobodyDeclares;
          managedByNobodyDeclaresFires = fires (filesFor managedByNobodyDeclares { user = "alice"; });

          # ── a person shares with a machine ──
          machineGrant = {
            audience = (audienceOfToken machineGrant).audience;
            file = fileOfToken machineGrant;
            recipients = (audienceOfToken machineGrant).recipients;

            # The machine's own scope resolves the entry, at the file the
            # audience picked: one file, read from both sides.
            resolvedByTheMachine = filesFor machineGrant { machine = "deck"; };
            resolvedByTheOwner = filesFor machineGrant { user = "alice"; };

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
          keylessMachineFires = fires (filesFor keylessMachine { user = "alice"; });

          machineOwnedByNobodyMessages = violationsOf machineOwnedByNobody;
          machineOwnedByNobodyFires = fires (filesFor machineOwnedByNobody { user = "alice"; });

          nameDeclaredTwiceMessages = violationsOf nameDeclaredTwice;
          nameDeclaredTwiceFires = fires (filesFor nameDeclaredTwice { user = "alice"; });

          unsafeMachineNameMessages = violationsOf unsafeMachineName;
          unsafeMachineNameFires = fires (filesFor unsafeMachineName { user = "alice"; });

          # ── a service's audience is its machines ──
          serviceGrant = {
            audience = (audienceOfToken serviceGrant).audience;
            file = fileOfToken serviceGrant;
            recipients = (audienceOfToken serviceGrant).recipients;

            # The machine resolves it, under the service's own key, and the key
            # inside the encrypted file is still the entry's own name.
            resolvedByTheMachine = filesFor serviceGrant { machine = "deck"; };
            resolvedByTheOwner = filesFor serviceGrant { user = "alice"; };
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
          emptyServiceFires = fires (filesFor emptyService { user = "alice"; });

          serviceOverUndeclaredMachineMessages = violationsOf serviceOverUndeclaredMachine;
          serviceOverUndeclaredMachineFires = fires (
            filesFor serviceOverUndeclaredMachine { user = "alice"; }
          );

          serviceOwnedByNobodyMessages = violationsOf serviceOwnedByNobody;

          serviceNameDeclaredTwiceMessages = violationsOf serviceNameDeclaredTwice;

          unsafeServiceNameMessages = violationsOf unsafeServiceName;

          serviceOnKeylessMachineMessages = violationsOf serviceOnKeylessMachine;

          # ── a group's audience is its members ──
          groupGrant = {
            audience = (audienceOfToken groupGrant).audience;
            file = fileOfToken groupGrant;
            recipients = (audienceOfToken groupGrant).recipients;
            resolvedByAMember = filesFor groupGrant { user = "bob"; };
            resolvedByANonMember = filesFor groupGrant { user = "dave"; };
          };

          # Membership growth and membership shrink both leave the file where it
          # was and change the recipient list. That is what makes each a re-wrap
          # rather than a migration, and it is the whole reason the directory is
          # named for the group.
          growthIsARewrap = {
            sameFile = fileOfToken grownGroup == fileOfToken groupGrant;
            recipients = (audienceOfToken grownGroup).recipients;
            resolvedByTheNewMember = filesFor grownGroup { user = "dave"; };
          };

          shrinkIsARewrap = {
            sameFile = fileOfToken shrunkGroup == fileOfToken groupGrant;
            recipients = (audienceOfToken shrunkGroup).recipients;
            removedMemberResolvesNothing = filesFor shrunkGroup { user = "carol"; };
          };

          nestedGroup = {
            audience = (audienceOfToken nestedGroup).audience;
            recipients = (audienceOfToken nestedGroup).recipients;
            resolvedByTheNestedPerson = filesFor nestedGroup { user = "bob"; };
            resolvedByTheNestedMachine = filesFor nestedGroup { machine = "deck"; };
          };

          ownGroupIsNotACollision = {
            violations = violationsOf ownGroup;
            audience = (audienceOfToken ownGroup).audience;
            ownerResolvesItOnce = filesFor ownGroup { user = "alice"; };
          };

          groupCycleMessages = violationsOf groupCycle;
          groupCycleFires = fires (filesFor groupCycle { user = "alice"; });

          emptyGroupMessages = violationsOf emptyGroup;
          emptyGroupFires = fires (filesFor emptyGroup { user = "alice"; });

          unknownMemberMessages = violationsOf unknownMember;
          unknownMemberFires = fires (filesFor unknownMember { user = "alice"; });

          keylessMemberMessages = violationsOf keylessMember;
          keylessMemberFires = fires (filesFor keylessMember { user = "bob"; });

          memberHoldsTheNameMessages = violationsOf memberHoldsTheName;
          memberHoldsTheNameFires = fires (filesFor memberHoldsTheName { user = "bob"; });

          # ── silos ──
          crossSiloMessages = violationsOf crossSilo;
          crossSiloFires = fires (filesFor crossSilo { user = "alice"; });
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
          groupInTwoSilosFires = fires (filesFor groupInTwoSilos { user = "alice"; });

          siloOverANonGroupMessages = violationsOf siloOverANonGroup;

          # D3: a person owning machines in two silos is the operator
          # administering both sides and is not refused. A file readable from both
          # is.
          machinesInTwoSilos = {
            violations = violationsOf ownsBothSides;
            eachSideResolves = filesFor ownsBothSides { user = "alice"; };
            spanningFileRefused = fires (filesFor spansBothSides { user = "alice"; });
            spanningMessages = violationsOf spansBothSides;
          };

          # ── ownership is a record a grant resolves through ──
          ownerOf = {
            audience = (audienceOfToken ownedByBo).audience;
            file = fileOfToken ownedByBo;
            recipients = (audienceOfToken ownedByBo).recipients;
            resolvedBy = filesFor ownedByBo { user = "bob"; };

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
            newOwnerResolvesIt = filesFor ownedByCy { user = "carol"; };
            oldOwnerResolvesNothing = filesFor ownedByCy { user = "bob"; };
          };

          ownerOfUnknownMachineMessages = violationsOf ownerOfUnknownMachine;
          ownerOfUnknownMachineFires = fires (filesFor ownerOfUnknownMachine { user = "alice"; });

          ownerOfUnownedMachineMessages = violationsOf ownerOfUnownedMachine;
          ownerOfUnownedMachineFires = fires (filesFor ownerOfUnownedMachine { user = "alice"; });

          ownerOfSelfMessages = violationsOf ownerOfSelf;

          # ── escrow is the person's own declaration ──
          # alice's files gain acme's custody keys and stay in alice's own
          # directory: consent widens who can open her files and never who they are
          # for, so there is no element and no migration.
          #
          # The two structural fields are design decision D1 and D3 read off the
          # records rather than described. The consent is a list on alice; what the
          # organization declares is custody and managers, neither of which can
          # name her as escrowed to it; and her `recoveryRecipients` stays empty
          # while acme's key is on her file, which is the expansion happening beside
          # that record rather than through it.
          escrowedConsent = {
            file = fileOfToken escrowed;
            audience = (audienceOfToken escrowed).audience;
            recipients = (audienceOfToken escrowed).recipients;
            personDeclares = escrowed.users.alice.escrowedTo;
            organizationDeclares = sorted (builtins.attrNames escrowed.organizations.acme);
            recoveryRecipientsUntouched = escrowed.users.alice.recoveryRecipients;
            anchors = map (a: a.anchor) (policy.plan escrowed).anchors;
            ruleAnchors = map (r: {
              inherit (r) audience anchors;
            }) (policy.plan escrowed).rules;
          };

          # A rotation in the organization's declaration re-wraps the same file
          # toward the new key, and every person's record is byte-identical across
          # it. That pair is the whole property this phase exists to add.
          rotationHappensInOnePlace = {
            sameFile = fileOfToken rotatedEscrow == fileOfToken escrowed;
            recipients = (audienceOfToken rotatedEscrow).recipients;
            noDeclarationChanged = rotatedEscrow.users == escrowed.users;
          };

          # Withdrawal narrows the same file back to alice's own custody. What it
          # takes back is nothing: the report of that is `safix check`'s, over the
          # key left on the ciphertext.
          withdrawalNarrowsTheSameFile = {
            sameFile = fileOfToken withdrawnEscrow == fileOfToken escrowed;
            recipients = (audienceOfToken withdrawnEscrow).recipients;
          };

          # ── an organization is an audience element ──
          organizationGrant = {
            audience = (audienceOfToken organizationGrant).audience;
            file = fileOfToken organizationGrant;
            recipients = (audienceOfToken organizationGrant).recipients;

            # The organization resolves nothing. It holds keys rather than entries,
            # so alice's own resolution is the only one, and a grant to acme moves
            # her secret into the file acme can open.
            resolvedByTheOwner = filesFor organizationGrant { user = "alice"; };
          };

          # ── ownership resolves through an organization ──
          organizationOwnership = {
            audience = (audienceOfToken ownedByAcme).audience;
            file = fileOfToken ownedByAcme;
            recipients = (audienceOfToken ownedByAcme).recipients;
            machineResolvesNothing = filesFor ownedByAcme { machine = "rack"; };

            # A change of owner from the organization to a person leaves the file
            # where it was and re-wraps it, which is what makes the record the thing
            # the grant resolves through rather than a name it copied.
            sameFileAfterOwnerChange = fileOfToken ownedByBobInstead == fileOfToken ownedByAcme;
            recipientsAfterOwnerChange = (audienceOfToken ownedByBobInstead).recipients;
          };

          # ── what an organization with no custody cannot do ──
          emptyCustodyMessages = violationsOf emptyCustody;
          emptyCustodyFires = fires (filesFor emptyCustody { user = "alice"; });

          escrowToNobodyMessages = violationsOf escrowToNobody;
          escrowToNobodyFires = fires (filesFor escrowToNobody { user = "alice"; });

          organizationInAGroupMessages = violationsOf organizationInAGroup;
          organizationInAGroupFires = fires (filesFor organizationInAGroup { user = "alice"; });

          organizationNameDeclaredTwiceMessages = violationsOf organizationNameDeclaredTwice;

          unsafeOrganizationNameMessages = violationsOf unsafeOrganizationName;

          unsafeCustodyAnchorMessages = violationsOf unsafeCustodyAnchor;

          anchorSharedWithAPersonMessages = violationsOf anchorSharedWithAPerson;

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
          referencesRoundTrip = roundTrips everyKind [
            "acme"
            "alice"
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
                  escrowed
                  organizationGrant
                  ownedByAcme
                  ;
              };
        };

        expected = {
          fixtureRosters = {
            machineGrant = {
              people = [ "alice" ];
              machines = [ "deck" ];
              services = [ ];
              groups = [ ];
              organizations = [ ];
              silos = [ ];
            };
            serviceGrant = {
              people = [ "alice" ];
              machines = [
                "deck"
                "rack"
              ];
              services = [ "nginx" ];
              groups = [ ];
              organizations = [ ];
              silos = [ ];
            };
            groupGrant = {
              people = [
                "alice"
                "bob"
                "carol"
                "dave"
              ];
              machines = [ ];
              services = [ ];
              groups = [ "oncall" ];
              organizations = [ ];
              silos = [ ];
            };
            crossSilo = {
              people = [
                "alice"
                "bob"
                "carol"
              ];
              machines = [ ];
              services = [ ];
              groups = [
                "contractors"
                "partners"
                "staff"
              ];
              organizations = [ ];
              silos = [ "corp" ];
            };
            ownedByBo = {
              people = [
                "alice"
                "bob"
                "carol"
              ];
              machines = [ "deck" ];
              services = [ ];
              groups = [ ];
              organizations = [ ];
              silos = [ ];
            };
            ownsBothSides = {
              people = [
                "alice"
                "bob"
                "carol"
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
              organizations = [ ];
              silos = [ "corp" ];
            };
            escrowed = {
              people = [ "alice" ];
              machines = [ ];
              services = [ ];
              groups = [ ];
              organizations = [ "acme" ];
              silos = [ ];
            };
            organizationGrant = {
              people = [ "alice" ];
              machines = [ ];
              services = [ ];
              groups = [ ];
              organizations = [ "acme" ];
              silos = [ ];
            };
            ownedByAcme = {
              people = [
                "alice"
                "bob"
              ];
              machines = [ "rack" ];
              services = [ ];
              groups = [ ];
              organizations = [ "acme" ];
              silos = [ ];
            };
            delegated = {
              people = [
                "alice"
                "bob"
                "carol"
                "dave"
              ];
              machines = [ ];
              services = [ ];
              groups = [
                "contractors"
                "oncall"
                "partners"
                "standby"
                "vendors"
              ];
              organizations = [
                "acme"
                "globex"
              ];
              silos = [
                "corp"
                "trade"
              ];
            };
          };

          inertness = {
            declaresSubjects = {
              machines = [
                "acme-host"
                "deck"
                "fixture-host"
                "rack"
              ];
              services = [
                "fixture-web"
                "nginx"
              ];
              groups = [
                "fixture-oncall"
                "infra"
                "oncall"
              ];
              organizations = [
                "acme"
                "globex"
              ];
              silos = [
                "corp"
                "fixture-corp"
              ];
            };
            declaresDelegation = {
              managers = {
                acme = [ "alice" ];
                globex = [ ];
              };
              managedBy.bob = "acme";
            };
            identical = true;
            noViolations = [ ];
          };

          delegation = {
            managers = {
              acme = [ "alice" ];
              globex = [
                "alice"
                "dave"
              ];
            };
            managedBy = {
              bob = "acme";
              carol = "globex";
            };
            groups = {
              contractors = {
                members = [ "dave" ];
                organizations = [ "acme" ];
              };
              oncall = {
                members = [ "bob" ];
                organizations = [ "acme" ];
              };
              partners = {
                members = [ "carol" ];
                organizations = [ "globex" ];
              };
              standby = {
                members = [ "bob" ];
                organizations = [ ];
              };
              vendors = {
                members = [ "dave" ];
                organizations = [ "globex" ];
              };
            };
            subjects = [
              "acme"
              "alice"
              "bob"
              "carol"
              "contractors"
              "dave"
              "globex"
              "oncall"
              "partners"
              "standby"
              "vendors"
            ];
          };
          delegationResolves = [ ];

          managersNobodyDeclaresMessages = [
            "flake.safix.organizations.acme.managers names 'mallory', which is not a declared user of flake.safix.users"
            "flake.safix.organizations.acme.managers names 'zed', which is not a declared user of flake.safix.users"
          ];
          managersNobodyDeclaresFires = true;

          managedByNobodyDeclaresMessages = [
            "flake.safix.users.alice.managedBy names 'globex', which is not a declared organization of flake.safix.organizations"
            "flake.safix.users.bob.managedBy names 'globex', which is not a declared organization of flake.safix.organizations"
          ];
          managedByNobodyDeclaresFires = true;

          machineGrant = {
            audience = [
              "alice"
              "deck"
            ];
            file = "secrets/safix/shared/alice,deck/secrets.yaml";
            recipients = [
              (keyOf "alice")
              (keyOf "deck")
            ];
            resolvedByTheMachine.token = "/secrets/safix/shared/alice,deck/secrets.yaml";
            resolvedByTheOwner.token = "/secrets/safix/shared/alice,deck/secrets.yaml";
            anchors = [
              "alice-safix"
              "deck-safix"
            ];
            ruleAnchors = [
              {
                audience = [
                  "alice"
                  "deck"
                ];
                anchors = [
                  "alice-safix"
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
            "flake.safix.users.alice.sharedWith.deck shares 'token', but flake.safix.machines.deck.recipient is null, so no copy can be encrypted to them"
          ];
          keylessMachineFires = true;

          machineOwnedByNobodyMessages = [
            "flake.safix.machines.deck.owner names 'zed', which is not a declared user of flake.safix.users or an organization of flake.safix.organizations"
          ];
          machineOwnedByNobodyFires = true;

          nameDeclaredTwiceMessages = [
            "'deck' is declared as more than one kind of subject, by flake.safix.users and flake.safix.machines; people, machines, services, groups and organizations share one name space"
          ];
          nameDeclaredTwiceFires = true;

          unsafeMachineNameMessages = [
            "flake.safix.machines names 'Deck', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];
          unsafeMachineNameFires = true;

          serviceGrant = {
            audience = [
              "%nginx"
              "alice"
            ];
            file = "secrets/safix/shared/%nginx,alice/secrets.yaml";
            recipients = [
              (keyOf "deck")
              (keyOf "alice")
            ];
            resolvedByTheMachine."nginx/token" = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
            resolvedByTheOwner.token = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
            sopsKeys."nginx/token" = "token";
            systemPlacement."nginx/token" = {
              mode = "0400";
              sopsFile = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
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
              (keyOf "alice")
            ];
            resolvedByTheNewMachine."nginx/token" = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
          };

          serviceShrinkIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "rack")
              (keyOf "alice")
            ];
            departedMachineResolvesNothing = { };
          };

          serviceInGroup = {
            audience = [
              "@oncall"
              "alice"
            ];
            recipients = [
              (keyOf "deck")
              (keyOf "alice")
            ];
            resolvedByTheMachine."nginx/token" = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
          };

          twoServicesOneMachine = {
            violations = [ ];
            resolved = {
              "alpha/token" = "/secrets/safix/shared/%alpha,%beta,alice/secrets.yaml";
              "beta/token" = "/secrets/safix/shared/%alpha,%beta,alice/secrets.yaml";
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
              sopsFile = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
              key = "token";
            };
            ownerlessAtSystemScope."nginx/token" = {
              mode = "0400";
              sopsFile = "/secrets/safix/shared/%nginx,alice/secrets.yaml";
              key = "token";
            };
          };

          emptyServiceMessages = [
            "flake.safix.users.alice.sharedWith.nginx shares 'token' with flake.safix.services.nginx, whose machines is empty, so the file would be encrypted to nobody"
          ];
          emptyServiceFires = true;

          serviceOverUndeclaredMachineMessages = [
            "flake.safix.services.nginx.machines names 'rack', which is not a declared machine of flake.safix.machines"
          ];
          serviceOverUndeclaredMachineFires = true;

          serviceOwnedByNobodyMessages = [
            "flake.safix.services.nginx.owner names 'zed', which is not a declared user of flake.safix.users or an organization of flake.safix.organizations"
          ];

          serviceNameDeclaredTwiceMessages = [
            "'nginx' is declared as more than one kind of subject, by flake.safix.machines and flake.safix.services; people, machines, services, groups and organizations share one name space"
          ];

          unsafeServiceNameMessages = [
            "flake.safix.services names 'Nginx', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];

          serviceOnKeylessMachineMessages = [
            "flake.safix.users.alice.sharedWith.nginx shares 'token' with flake.safix.machines.deck, reached through flake.safix.services.nginx, but flake.safix.machines.deck.recipient is null, so no copy can be encrypted to them"
          ];

          groupGrant = {
            audience = [
              "@oncall"
              "alice"
            ];
            file = "secrets/safix/shared/@oncall,alice/secrets.yaml";
            recipients = [
              (keyOf "bob")
              (keyOf "carol")
              (keyOf "alice")
            ];
            resolvedByAMember.token = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
            resolvedByANonMember = { };
          };

          growthIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "bob")
              (keyOf "carol")
              (keyOf "dave")
              (keyOf "alice")
            ];
            resolvedByTheNewMember.token = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
          };

          shrinkIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "bob")
              (keyOf "alice")
            ];
            removedMemberResolvesNothing = { };
          };

          nestedGroup = {
            audience = [
              "@outer"
              "alice"
            ];
            recipients = [
              (keyOf "bob")
              (keyOf "deck")
              (keyOf "alice")
            ];
            resolvedByTheNestedPerson.token = "/secrets/safix/shared/@outer,alice/secrets.yaml";
            resolvedByTheNestedMachine.token = "/secrets/safix/shared/@outer,alice/secrets.yaml";
          };

          ownGroupIsNotACollision = {
            violations = [ ];
            audience = [
              "@oncall"
              "alice"
            ];
            ownerResolvesItOnce.token = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
          };

          groupCycleMessages = [
            "flake.safix.groups declares a cycle: 'inner' -> 'outer' -> 'inner'. A membership that cannot be expanded is not a membership."
          ];
          groupCycleFires = true;

          emptyGroupMessages = [
            "flake.safix.users.alice.sharedWith.oncall shares 'token' with flake.safix.groups.oncall, which reaches no subject beyond flake.safix.users.alice, so the grant widens nothing"
          ];
          emptyGroupFires = true;

          unknownMemberMessages = [
            "flake.safix.groups.oncall.members names 'zed', which is not a declared subject of flake.safix.users, flake.safix.machines, flake.safix.services, flake.safix.groups or flake.safix.organizations"
          ];
          unknownMemberFires = true;

          keylessMemberMessages = [
            "flake.safix.users.alice.sharedWith.oncall shares 'token' with flake.safix.users.bob, reached through flake.safix.groups.oncall, but flake.safix.users.bob.recipient is null, so no copy can be encrypted to them"
          ];
          keylessMemberFires = true;

          memberHoldsTheNameMessages = [
            "flake.safix.users.bob declares 'token' in flake.safix.users.bob.private, and flake.safix.users.alice.sharedWith.oncall shares a secret of that name with flake.safix.users.bob, reached through flake.safix.groups.oncall"
          ];
          memberHoldsTheNameFires = true;

          crossSiloMessages = [
            "flake.safix.users.alice's 'token' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.contractors reaches bob and flake.safix.groups.staff reaches alice. secrets/safix/shared/@contractors,alice/secrets.yaml is one file with one data key, so it would be readable from both."
          ];
          crossSiloFires = true;
          crossSiloGeneratesNoRule = true;
          crossSiloListsEveryGrant = [
            "flake.safix.users.alice's 'other' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.contractors reaches bob and flake.safix.groups.staff reaches alice. secrets/safix/shared/@contractors,alice/secrets.yaml is one file with one data key, so it would be readable from both."
            "flake.safix.users.alice's 'token' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.contractors reaches bob and flake.safix.groups.staff reaches alice. secrets/safix/shared/@contractors,alice/secrets.yaml is one file with one data key, so it would be readable from both."
          ];

          withinSiloResolves = {
            violations = [ ];
            file = "secrets/safix/shared/@partners,alice/secrets.yaml";
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
              both = "/secrets/safix/users/alice/secrets.yaml";
              other = "/secrets/safix/shared/@blue,alice/secrets.yaml";
              token = "/secrets/safix/shared/@red,alice/secrets.yaml";
            };
            spanningFileRefused = true;
            spanningMessages = [
              "flake.safix.users.alice's 'both' resolves an audience spanning silo set flake.safix.silos.corp: flake.safix.groups.blue reaches carol, rack and flake.safix.groups.red reaches bob, deck. secrets/safix/shared/@blue,@red,alice/secrets.yaml is one file with one data key, so it would be readable from both."
            ];
          };

          ownerOf = {
            audience = [
              "@~deck"
              "alice"
            ];
            file = "secrets/safix/shared/@~deck,alice/secrets.yaml";
            recipients = [
              (keyOf "bob")
              (keyOf "alice")
            ];
            resolvedBy.token = "/secrets/safix/shared/@~deck,alice/secrets.yaml";
            machineResolvesNothing = { };
          };

          ownerChangeIsARewrap = {
            sameFile = true;
            recipients = [
              (keyOf "carol")
              (keyOf "alice")
            ];
            newOwnerResolvesIt.token = "/secrets/safix/shared/@~deck,alice/secrets.yaml";
            oldOwnerResolvesNothing = { };
          };

          ownerOfUnknownMachineMessages = [
            "flake.safix.users.alice.sharedWith names the owner of 'rack', which is not a declared machine of flake.safix.machines"
          ];
          ownerOfUnknownMachineFires = true;

          ownerOfUnownedMachineMessages = [
            "flake.safix.users.alice.sharedWith names the owner of flake.safix.machines.deck, which records none, so the grant resolves to nobody"
          ];
          ownerOfUnownedMachineFires = true;

          ownerOfSelfMessages = [
            "flake.safix.users.alice.sharedWith.\"ownerOf.deck\" shares 'token' with the owner flake.safix.machines.deck records, which reaches no subject beyond flake.safix.users.alice, so the grant widens nothing"
          ];

          escrowedConsent = {
            file = "secrets/safix/users/alice/secrets.yaml";
            audience = [ "alice" ];
            recipients = [
              (keyOf "alice")
              (keyOf "acme-escrow")
            ];
            personDeclares = [ "acme" ];
            organizationDeclares = [
              "custody"
              "managers"
            ];
            recoveryRecipientsUntouched = { };
            anchors = [
              "alice-safix"
              "acme-escrow"
            ];
            ruleAnchors = [
              {
                audience = [ "alice" ];
                anchors = [
                  "alice-safix"
                  "acme-escrow"
                ];
              }
            ];
          };

          rotationHappensInOnePlace = {
            sameFile = true;
            recipients = [
              (keyOf "alice")
              (keyOf "acme-rotated")
            ];
            noDeclarationChanged = true;
          };

          withdrawalNarrowsTheSameFile = {
            sameFile = true;
            recipients = [ (keyOf "alice") ];
          };

          organizationGrant = {
            audience = [
              "=acme"
              "alice"
            ];
            file = "secrets/safix/shared/=acme,alice/secrets.yaml";
            recipients = [
              (keyOf "acme-escrow")
              (keyOf "alice")
            ];
            resolvedByTheOwner.token = "/secrets/safix/shared/=acme,alice/secrets.yaml";
          };

          organizationOwnership = {
            audience = [
              "@~rack"
              "alice"
            ];
            file = "secrets/safix/shared/@~rack,alice/secrets.yaml";
            recipients = [
              (keyOf "acme-escrow")
              (keyOf "alice")
            ];
            machineResolvesNothing = { };
            sameFileAfterOwnerChange = true;
            recipientsAfterOwnerChange = [
              (keyOf "bob")
              (keyOf "alice")
            ];
          };

          emptyCustodyMessages = [
            "flake.safix.users.alice.escrowedTo names flake.safix.organizations.acme, whose custody is empty, so the escrow would add no recipient to any file they hold"
            "flake.safix.users.alice.sharedWith.acme shares 'token', but flake.safix.organizations.acme.custody is empty, so the file would be encrypted to nobody"
            "flake.safix.users.alice.sharedWith.\"ownerOf.rack\" shares 'other' with flake.safix.organizations.acme, reached through the owner flake.safix.machines.rack records, but flake.safix.organizations.acme.custody is empty, so the file would be encrypted to nobody"
          ];
          emptyCustodyFires = true;

          escrowToNobodyMessages = [
            "flake.safix.users.alice.escrowedTo names 'acme', which is not a declared organization of flake.safix.organizations"
          ];
          escrowToNobodyFires = true;

          organizationInAGroupMessages = [
            "flake.safix.groups.oncall.members names flake.safix.organizations.acme, which is a principal rather than a member; an audience wanting its custody names the organization"
          ];
          organizationInAGroupFires = true;

          organizationNameDeclaredTwiceMessages = [
            "'acme' is declared as more than one kind of subject, by flake.safix.groups and flake.safix.organizations; people, machines, services, groups and organizations share one name space"
          ];

          unsafeOrganizationNameMessages = [
            "flake.safix.organizations names 'Acme', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];

          unsafeCustodyAnchorMessages = [
            "flake.safix.organizations.acme.custody names 'Escrow', which is not [a-z0-9][a-z0-9_-]* and so cannot be a recipient policy anchor"
          ];

          anchorSharedWithAPersonMessages = [
            "the declarations give the recipient policy anchor 'acme-escrow' more than one key, declared by flake.safix.users.alice and flake.safix.organizations.acme"
          ];

          markedElementsAreDistinct = [
            {
              label = "a group and a person of that name";
              distinct = true;
              fileA = "secrets/safix/shared/@oncall,alice/secrets.yaml";
              fileB = "secrets/safix/shared/alice,oncall/secrets.yaml";
            }
            {
              label = "an owner reference and a group of that name";
              distinct = true;
              fileA = "secrets/safix/shared/@~deck,alice/secrets.yaml";
              fileB = "secrets/safix/shared/@deck,alice/secrets.yaml";
            }
            {
              label = "a service and a person of that name";
              distinct = true;
              fileA = "secrets/safix/shared/%nginx,alice/secrets.yaml";
              fileB = "secrets/safix/shared/alice,nginx/secrets.yaml";
            }
            {
              label = "a service and a group of that name";
              distinct = true;
              fileA = "secrets/safix/shared/%nginx,alice/secrets.yaml";
              fileB = "secrets/safix/shared/@nginx,alice/secrets.yaml";
            }
            {
              label = "an organization and a person of that name";
              distinct = true;
              fileA = "secrets/safix/shared/=acme,alice/secrets.yaml";
              fileB = "secrets/safix/shared/acme,alice/secrets.yaml";
            }
            {
              label = "an organization and a group of that name";
              distinct = true;
              fileA = "secrets/safix/shared/=acme,alice/secrets.yaml";
              fileB = "secrets/safix/shared/@acme,alice/secrets.yaml";
            }
          ];

          markersOutsideNameAlphabet = {
            group = true;
            owner = true;
            service = true;
            organization = true;
          };

          referencesRoundTrip = [
            true
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
                "escrowed"
                "organizationGrant"
                "ownedByAcme"
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
