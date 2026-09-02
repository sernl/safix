# The fleet this example resolves, as one file. Passed to `lib.mkVault`'s
# `modules` list in ./entry.nix, and read the identical way by
# `modules/flake/checks/examples.nix`, which compares it field for field
# against ../dendritic's scattered declarations of the same fleet.
#
# alice carries a per-carrier catalogue entry and a shared one, holds a private
# secret and a generated one, shares one entry each with a person, a machine, a
# service and a group, consents to acme's recovery custody, and adjusts her own
# placement per host and per tag. bob carries the same two catalogue entries as
# a second, independent carrier. deck is the machine, web the service that runs
# on it, oncall the group, acme the organization, corp the silo that names
# oncall.
{
  flake.safix = {
    catalogue = {
      # Carried separately by alice and bob: each holds their own copy.
      shelf-item = { };

      # Carried by both: one value, one ciphertext, shared between them.
      team-wifi.shared = true;
    };

    users = {
      alice = {
        recipient = "age1exampleaaa00000000000000000000000000000000000000000000000";
        recipientNote = "alice — example identity, decrypts nothing";

        carries = {
          shelf-item = { };
          team-wifi = { };
        };

        private = {
          laptop-token = { };

          generated-token.generator = {
            script = ''openssl rand -hex 32 > "$out/generated-token"'';
            runtimeInputs = [ "openssl" ];
          };

          # Each of the four below is granted onward through sharedWith; a
          # grant hands on an entry the granter already holds, it does not
          # create one.
          handoff-note = { };
          fleet-token = { };
          web-token = { };
          pager-token = { };
        };

        sharedWith = {
          bob.handoff-note = { }; # a person
          deck.fleet-token = { }; # a machine
          web.web-token = { }; # a service
          oncall.pager-token = { }; # a group
        };

        # acme's recovery custody can open everything alice holds.
        escrowedTo = [ "acme" ];

        # Placement adjustments: still alice's everywhere, simply not landed
        # here.
        perHost.deck.omit.laptop-token = { };
        perTag.portable.omit.shelf-item = { };
      };

      bob = {
        recipient = "age1examplebbb00000000000000000000000000000000000000000000000";
        recipientNote = "bob — example identity, decrypts nothing";

        carries = {
          shelf-item = { };
          team-wifi = { };
        };
      };
    };

    machines.deck = {
      recipient = "age1exampledeck0000000000000000000000000000000000000000000000";
      recipientNote = "deck — the age form of a host identity that does not exist";
      owner = "alice";
      tags = [ "portable" ];
    };

    services.web = {
      machines = [ "deck" ];
      owner = "alice";
      user = "web";
      group = "web";
    };

    groups.oncall.members = [
      "alice"
      "bob"
    ];

    organizations.acme.custody.acme-escrow = {
      key = "age1exampleacme0000000000000000000000000000000000000000000000";
      note = "acme's escrow — held offline by the operator";
    };

    silos.corp.groups = [ "oncall" ];
  };
}
