# The field surface every target's far side declares, and the table saying
# which of them each target can carry.
#
# A field is metadata carried beside the value on the far side of a mapping: a
# username, a URL a password manager offers to fill, a note saying where the
# credential came from, a tag. It exists on the far side only and never on
# `safixSide`, on any target, ever — a safix entry is a file, a key inside it
# and an audience, with no slot for a URL and nowhere to hold one; inventing
# one would create a second authoring surface for the same information beside
# the declaration that already carries it, and then a pull would have to decide
# which of the two wins.
#
# A literal is not a secret. The declaration it is written in is evaluated into
# a world-readable nix store, so a bare string may travel any channel a target
# offers, including an argument vector. An `{ entry = "<name>"; }` reference is
# a secret: it names another entry of the mapping's own person, is decrypted
# when the mapping is converged rather than at evaluation, and may travel only
# a pipe. That single distinction is what the capability table below exists to
# enforce.
#
# Each target refuses what it cannot carry in its own `violationsOf`, reading
# the table this file exports. No message builder is exported with it: a
# refusal's wording names the option path it refuses, so it belongs beside that
# path rather than one indirection away from it.
{ lib }:
let
  fieldValue = lib.types.either lib.types.str (
    lib.types.submodule {
      options.entry = lib.mkOption {
        type = lib.types.str;
        example = "grafana-url";
        description = ''
          Another entry of this mapping's own person, whose value becomes this
          field.

          Decrypted when the mapping is converged, never at evaluation: a value
          resolved at evaluation would land in the world-readable nix store,
          which is the one thing this project is built not to do. Because the
          resolved value is a secret, a target whose only channel for this
          field is an argument vector refuses the declaration rather than
          writing it.
        '';
      };
    }
  );

  scalar =
    what: exampleValue:
    lib.mkOption {
      type = lib.types.nullOr fieldValue;
      default = null;
      example = exampleValue;
      description = ''
        ${what}

        Null leaves the field alone: safix declares nothing about a field the
        mapping does not name, and an entry's own is the operator's until a
        declaration claims it. A target that cannot carry this field refuses
        the declaration at evaluation, naming the target and the field, rather
        than writing less than the declaration says.
      '';
    };

  fields = lib.types.submodule {
    options = {
      username = scalar "The username to set on the far side's entry." "alice@example.com";

      url = scalar "The address the entry's credential is used at." "https://grafana.example.com";

      notes = scalar "The entry's note, as free text." "minted by safix for the fleet dashboard";

      tags = lib.mkOption {
        type = lib.types.listOf fieldValue;
        default = [ ];
        example = [ "work" ];
        description = ''
          The tags to set on the far side's entry.

          An empty list leaves them alone, for the reason a null scalar does. A
          target that cannot carry tags refuses a non-empty list at evaluation,
          naming the target and the field — keepassxc is such a target, because
          its own command has no tags flag.
        '';
      };
    };
  };

  # The fixed order every report, every refusal and every diff names fields in,
  # so that two runs over one mapping read the same and a reordering is a
  # deliberate edit rather than an accident of an attribute set's sort.
  fieldNames = [
    "username"
    "url"
    "notes"
    "tags"
  ];

  # Which channel each target carries each field on, and therefore what each
  # target refuses: `"unsupported"` refuses the field outright, and `"argv"`
  # refuses an `{ entry = …; }` source for it because a secret value may not
  # travel an argument vector.
  #
  # Where each row was verified:
  #
  # keepassxc — `keepassxc-cli add --help` and `edit --help`, read on
  # 2026-09-16 against 2.7: both take `-u/--username`, `--url` and `--notes` in
  # the argument vector, and neither has a tags flag or any custom-attribute
  # write, so tags is not a field this transport can carry at all.
  #
  # pass, bitwarden, onepassword — the three rows are `add-pass-bridge`'s,
  # `add-bitwarden-bridge`'s and `add-onepassword-bridge`'s own evidence, and
  # they are declared here rather than in those changes so that the table is
  # one artifact rather than four. `bw` has no tags on a login item, which is
  # why its row differs from the other two.
  #
  # clan — a clan var is a file's bytes and clan's own command offers no field
  # beside it, so this target declares none rather than leaving the absence to
  # be read as an omission.
  channels = {
    keepassxc = {
      username = "argv";
      url = "argv";
      notes = "argv";
      tags = "unsupported";
    };

    pass = {
      username = "stdin";
      url = "stdin";
      notes = "stdin";
      tags = "stdin";
    };

    bitwarden = {
      username = "stdin";
      url = "stdin";
      notes = "stdin";
      tags = "unsupported";
    };

    onepassword = {
      username = "stdin";
      url = "stdin";
      notes = "stdin";
      tags = "stdin";
    };

    clan = {
      username = "unsupported";
      url = "unsupported";
      notes = "unsupported";
      tags = "unsupported";
    };
  };
in
{
  inherit
    fieldValue
    fields
    fieldNames
    channels
    ;
}
