# The fleet this repository declares into its own `flake.safix.*`, so that the
# checks safix exports are instantiated here the way a consumer instantiates
# them rather than called with arguments assembled beside them.
#
# Three people who do not exist, two machines, one service, one group, one silo
# set, and one organization. alice and bob each carry a catalogue entry into their
# own custody and share a second one between them; alice escrows to a recovery
# identity she holds and consents to acme's escrow beside it, holds a generated
# secret, declares a path, and grants one entry to the service, one to acme and
# one to the owner of the machine acme owns; bob declares ownership fields, which
# is what makes his profile the one the user-scope refusal fires on, and consents
# to acme's management; carol records a recipient and holds nothing, which earns an
# anchor and no rule.
#
# acme is the organization: one custody key, one consenting person, one owned
# machine, one manager. The direct grant is what puts the fifth audience element
# in front of every check a consumer runs — the rule shape, the catch-all probes,
# the separator and the committed policy — and the `ownerOf` grant is what resolves
# an ownership record through to an organization's custody.
#
# The delegation is here for the property it does not have. alice manages for acme
# and bob consents to that, and `modules/flake/checks/fixture-policy.yaml` — the
# committed policy this fleet's drift check regenerates and compares — is
# byte-identical to what it was before either record existed, because managing
# confers scaffolding and never a read. The group and the silo set are what give
# the delegation a scope over groups: `fixture-corp` reaches bob, so acme manages
# every group the set holds.
#
# The recipients are literals shaped like an age public key and are not keys.
# Nothing in this repository encrypts anything, nothing here has a private half
# anywhere, and no ciphertext exists for any of them to open. They are chosen to
# be distinguishable in a diff.
#
# This is a declaration rather than an argument because that is the claim: the
# exported builders read `flake.safix.users` and `flake.safix.catalogue` through
# the same binding a consumer's flake gives them, so a rename of an option or a
# change to a default reaches the checks here the way it would reach theirs.
let
  aliceKey = "age1fixtureaaa00000000000000000000000000000000000000000000000";
  bobKey = "age1fixturebbb00000000000000000000000000000000000000000000000";
  carolKey = "age1fixtureccc00000000000000000000000000000000000000000000000";
  escrowKey = "age1fixturevault0000000000000000000000000000000000000000000000";
  hostKey = "age1fixturehost00000000000000000000000000000000000000000000000";
  acmeKey = "age1fixtureacme00000000000000000000000000000000000000000000000";
  acmeHostKey = "age1fixtureacmehost000000000000000000000000000000000000000000";
in
{
  inherit
    aliceKey
    bobKey
    carolKey
    escrowKey
    hostKey
    acmeKey
    acmeHostKey
    ;

  fleet = {
    catalogue = {
      # Carried separately by two people, so each holds their own copy in their
      # own file and the two files have different audiences.
      ops-tooling = {
        mode = "0400";
        sopsKey = "ops_tooling";
      };

      # One value, many people: a single file whose audience is everyone whose
      # `carries` names it.
      team-vault = {
        shared = true;
        mode = "0400";
      };
    };

    users = {
      alice = {
        recipient = aliceKey;
        recipientNote = "alice — fixture identity, decrypts nothing";
        # The dotted anchor form below is, shape for shape, what `safix enroll`
        # writes into a declaration (crates/safix-core/src/enroll/declaration.rs
        # asserts the emitted text); its acceptance through the real option here
        # is what proves an enrolled record evaluates.
        recoveryRecipients."alice-escrow".key = escrowKey;
        recoveryRecipients."alice-escrow".note = "alice's escrow — a second identity she holds";

        # Consent to acme's escrow, beside the recovery identity above rather than
        # inside it: the keys are acme's and arrive at resolution time, which is
        # what keeps a rotation of them a change to one declaration.
        escrowedTo = [ "acme" ];

        carries = {
          ops-tooling = { };
          team-vault = { };
        };

        private = {
          # The entry the materialization check reads: it declares a path, so
          # the two scopes have a path to be identical about rather than two
          # provisioner defaults that differ by construction.
          alice-alone = {
            mode = "0440";
            sopsKey = "alice_alone";
            path = cfg: "${cfg.home.homeDirectory}/.config/safix-fixture/alice-alone";
          };

          # Held by alice and granted onward, which is what gives alice and bob
          # a shared audience. `team-vault` lands in that same directory: the
          # audience picks the file, so two names with one audience are one
          # file.
          ops-handover = {
            mode = "0400";
          };

          # Generated rather than typed in. `runtimeInputs` names a nixpkgs
          # attribute as a string, which is the declaration the runtime-tool
          # check resolves.
          api-token = {
            generator = {
              script = ''printf '%s' fixture > "$out/api-token"'';
              runtimeInputs = [ "coreutils" ];
            };
          };

          # clan's wireguard keypair, ported. One generator, two outputs: a
          # private half that is encrypted and a public half stored in the clear
          # and readable at evaluation, which is how clan's own service modules
          # reach a peer's public key without a deployment-time indirection.
          #
          # `wg pubkey` reads the private half on standard input, so the script
          # is exactly the two lines clan's is, addressing `$out` the same way.
          wg-private = {
            mode = "0400";
            generator = {
              runtimeInputs = [ "wireguard-tools" ];
              script = ''
                wg genkey > "$out/wg-private"
                wg pubkey < "$out/wg-private" > "$out/wg-public"
              '';
              files.wg-public.secret = false;
            };
          };

          # The public half is a registry entry in its own right, so it carries
          # its own mode; being public, nothing encrypts it and no rule covers
          # it.
          wg-public = {
            mode = "0444";
          };

          # Granted to a service rather than to a person, so the fixture fleet
          # exercises the fourth audience element and the composed key every
          # custody check reads.
          web-token = {
            mode = "0400";
          };

          # Granted to the organization itself, which is the fifth audience
          # element: a directory named `=acme,alice`, so the marker reaches a
          # generated `path_regex` in this repository's own policy.
          corp-token = {
            mode = "0400";
          };

          # Granted to whoever owns acme-host, which acme does, so the ownership
          # record resolves through to an organization's custody keys.
          corp-handover = {
            mode = "0400";
          };
        };

        sharedWith = {
          bob.ops-handover = { };
          fixture-web.web-token = { };
          acme.corp-token = { };
          "ownerOf.acme-host".corp-handover = { };
        };
      };

      bob = {
        recipient = bobKey;
        recipientNote = "bob — fixture identity, decrypts nothing";

        # The other half of acme's delegation, in bob's own record, where every
        # consent in this model is written. It places no key on any of his files
        # and adds no anchor to the policy: what it decides is which acting
        # identity `safix enroll` and `safix group` accept for him.
        managedBy = "acme";

        carries = {
          ops-tooling = { };
          team-vault = { };
        };

        private = {
          # Ownership fields, which only the system scope has an axis for. This
          # is what the user-scope refusal fires on, and it is declared on bob
          # rather than on alice so that alice's profile stays materializable at
          # both scopes and the two claims do not have to share a fixture.
          bob-service = {
            mode = "0400";
            owner = "bob";
            group = "staff";
            path = _cfg: "/var/lib/safix-fixture/bob-service";
          };
        };
      };

      # A recipient and nothing held. Every audience excludes them, so they
      # appear in the keys block and in no rule.
      carol = {
        recipient = carolKey;
        recipientNote = "carol — fixture identity, decrypts nothing";
      };
    };

    machines.fixture-host = {
      recipient = hostKey;
      recipientNote = "fixture-host — the age form of a host identity that does not exist";
      owner = "alice";
    };

    # Owned by the organization rather than by a person, which is what makes the
    # `ownerOf` grant above resolve to custody keys instead of to someone's
    # recipient. It records a recipient of its own and no grant names the machine,
    # so it earns no anchor: what the audience names is its owner.
    machines.acme-host = {
      recipient = acmeHostKey;
      recipientNote = "acme-host — the age form of a host identity that does not exist";
      owner = "acme";
    };

    # One organization, holding one escrow identity and naming one manager. alice
    # consents to its escrow, acme owns a machine, and one grant names it directly,
    # so all three ways of reaching an organization are exercised by the checks a
    # consumer runs.
    organizations.acme = {
      custody.acme-escrow = {
        key = acmeKey;
        note = "acme's escrow — a fixture identity that decrypts nothing";
      };
      managers = [ "alice" ];
    };

    # One group and one silo set, declared for the delegation's scope over groups
    # and for nothing else: no audience names the group, so both are inert. The
    # shape is the one `safix group` edits — a `members` list of subject names — held
    # here against the real option so the writer and the option cannot drift apart.
    groups.fixture-oncall.members = [
      "alice"
      "bob"
    ];

    silos.fixture-corp.groups = [ "fixture-oncall" ];

    # One granted service, so the exported checks are instantiated over a fleet
    # that has one. It declares no ownership, which keeps alice's own profile
    # materializable at either scope: the ownership asymmetry is bob's to carry.
    services.fixture-web = {
      machines = [ "fixture-host" ];
      owner = "alice";
    };
  };
}
