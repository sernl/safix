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
# Emptying any fixture fleet fails `fixtureRosters`.
{
  perSystem =
    { pkgs, lib, ... }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      safixChecks = import ../safix/checks.nix { inherit lib; };
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

      # A fleet, as the five records the resolver reads. `machines` and `groups`
      # go through their own submodules for the same reason `users` does.
      fleetOf =
        {
          users ? { },
          catalogue ? { },
          machines ? { },
          groups ? { },
          silos ? { },
        }:
        {
          users = typed (lib.types.attrsOf types.profile) users;
          catalogue = typed (lib.types.attrsOf types.entry) catalogue;
          machines = typed (lib.types.attrsOf types.machine) machines;
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

      violationsOf = resolve.violations;

      # ── inertness ──
      # The repository's own fleet, and the same fleet with three subject records
      # declared that nothing references. Every derived artifact has to be
      # identical: declaring a machine, a group or a silo changes nothing until an
      # audience names one.
      bare = fleetOf { inherit (fixture.fleet) users catalogue; };

      declaredButUnused = bare // {
        machines = typed (lib.types.attrsOf types.machine) {
          deck = machine "deck" "ana";
          rack = machine "rack" "bo";
        };
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
      ];

      roundTrips = fleet: refs: map (ref: resolve.refOfElement (resolve.elementOf fleet ref) == ref) refs;
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
                groups = sorted (builtins.attrNames f.groups);
                silos = sorted (builtins.attrNames f.silos);
              })
              {
                inherit
                  machineGrant
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
          referencesRoundTrip = roundTrips nestedGroup [
            "bo"
            "deck"
            "outer"
            "ownerOf.deck"
          ];

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
                "rack"
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
            "'deck' is declared as more than one kind of subject, by flake.safix.users and flake.safix.machines; people, machines and groups share one name space"
          ];
          nameDeclaredTwiceFires = true;

          unsafeMachineNameMessages = [
            "flake.safix.machines names 'Deck', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];
          unsafeMachineNameFires = true;

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
            "flake.safix.groups.oncall.members names 'zed', which is not a declared subject of flake.safix.users, flake.safix.machines or flake.safix.groups"
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
          ];

          markersOutsideNameAlphabet = {
            group = true;
            owner = true;
          };

          referencesRoundTrip = [
            true
            true
            true
            true
          ];

          generatedRulesOverSubjects =
            lib.genAttrs
              [
                "machineGrant"
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
