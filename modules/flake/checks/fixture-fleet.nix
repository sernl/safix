# The fleet this repository declares into its own `flake.safix.*`, so that the
# checks safix exports are instantiated here the way a consumer instantiates
# them rather than called with arguments assembled beside them.
#
# Three people who do not exist. ana and bo each carry a catalogue entry into
# their own custody and share a second one between them; ana escrows to a
# recovery identity, holds a generated secret, and declares a path; bo declares
# ownership fields, which is what makes his profile the one the user-scope
# refusal fires on; cy records a recipient and holds nothing, which earns an
# anchor and no rule.
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
  anaKey = "age1fixtureaaa00000000000000000000000000000000000000000000000";
  boKey = "age1fixturebbb00000000000000000000000000000000000000000000000";
  cyKey = "age1fixtureccc00000000000000000000000000000000000000000000000";
  escrowKey = "age1fixturevault0000000000000000000000000000000000000000000000";
in
{
  inherit
    anaKey
    boKey
    cyKey
    escrowKey
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
      ana = {
        recipient = anaKey;
        recipientNote = "ana — fixture identity, decrypts nothing";
        recoveryRecipients.ana-escrow = {
          key = escrowKey;
          note = "ana's escrow — a second identity she holds";
        };

        carries = {
          ops-tooling = { };
          team-vault = { };
        };

        private = {
          # The entry the materialization check reads: it declares a path, so
          # the two scopes have a path to be identical about rather than two
          # provisioner defaults that differ by construction.
          ana-alone = {
            mode = "0440";
            sopsKey = "ana_alone";
            path = cfg: "${cfg.home}/.config/safix-fixture/ana-alone";
          };

          # Held by ana and granted onward, which is what gives ana and bo a
          # shared audience. `team-vault` lands in that same directory: the
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
              script = "printf '%s' fixture";
              runtimeInputs = [ "coreutils" ];
            };
          };
        };

        sharedWith.bo.ops-handover = { };
      };

      bo = {
        recipient = boKey;
        recipientNote = "bo — fixture identity, decrypts nothing";

        carries = {
          ops-tooling = { };
          team-vault = { };
        };

        private = {
          # Ownership fields, which only the system scope has an axis for. This
          # is what the user-scope refusal fires on, and it is declared on bo
          # rather than on ana so that ana's profile stays materializable at
          # both scopes and the two claims do not have to share a fixture.
          bo-service = {
            mode = "0400";
            owner = "bo";
            group = "staff";
            path = _cfg: "/var/lib/safix-fixture/bo-service";
          };
        };
      };

      # A recipient and nothing held. Every audience excludes them, so they
      # appear in the keys block and in no rule.
      cy = {
        recipient = cyKey;
        recipientNote = "cy — fixture identity, decrypts nothing";
      };
    };
  };
}
