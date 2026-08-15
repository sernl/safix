# The two records safix reads, and the whole of what a consumer declares.
#
# Both are plain `attrsOf` options with no default derived from anything outside
# this namespace. That is the property an adapter rests on: a module that sets
# `flake.safix.users` from a `mapAttrs` over a consumer's own registry and a
# module that sets it by hand are indistinguishable to the resolver, so bridging
# an existing user vocabulary is a projection the consumer writes rather than an
# integration point safix has to offer.
#
# Attrsets merge, so declarations may be scattered one per file anywhere in a
# consumer's tree and the resolver still sees one record. safix reads no path, no
# filename and no directory structure to find them; the only requirement is that
# the consumer's flake imports the modules.
{ lib, ... }:
let
  types = import ./types.nix { inherit lib; };
  bridge = import ./bridge.nix { inherit lib; };

  # One consumer bridges one clan. Two definitions of a scalar cannot both
  # survive into a list for `violationsOf` to count, so this refusal is a merge
  # that throws rather than a message in that family — it is the one bridge rule
  # whose evidence is gone by the time the resolver could look at it.
  oneClanFlake = lib.types.path // {
    merge =
      loc: defs:
      if builtins.length defs > 1 then
        throw "safix bridge: flake.safix.bridge.clanFlake is declared ${toString (builtins.length defs)} times, in ${
          lib.concatMapStringsSep " and " (d: toString d.file) defs
        }. One consumer bridges one clan."
      else
        lib.types.path.merge loc defs;
  };
in
{
  options.flake.safix = {
    catalogue = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.entry;
      description = ''
        The secret catalogue: one definition per secret more than one person may
        hold, selected by name in `flake.safix.users.<u>.carries`.

        A secret only one person will ever hold does not have to be published
        here to be resolvable — `flake.safix.users.<u>.private` takes the same
        entry submodule with the same defaults, and declaring one there is itself
        selecting it.
      '';
    };

    users = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.profile;
      description = ''
        Who holds what. This is safix's own user record and carries only custody:
        a recipient, further recipients of the same person's custody, the
        catalogue entries they carry, the secrets they declare alone, the secrets
        they grant outward, and their per-host and per-tag adjustments.

        It is deliberately not a consumer's user registry and never reads one. A
        consumer with its own users writes a projection from theirs into this one;
        the two are different objects that happen to share a name.
      '';
    };

    bridge = {
      clanFlake = lib.mkOption {
        default = null;
        type = lib.types.nullOr oneClanFlake;
        example = lib.literalExpression "./.";
        description = ''
          The clan this consumer bridges to, as the flake reference clan's own
          command takes for `--flake`.

          Declared once for the consumer rather than once per mapping. A
          consumer with two clans is not a case this supports, and declaring a
          second one is refused rather than resolved by taking the first.
        '';
      };

      mappings = lib.mkOption {
        default = { };
        type = lib.types.attrsOf bridge.mapping;
        example = lib.literalExpression ''
          {
            ntfy-token = {
              direction = "clan-to-safix";
              clan = {
                machine = "sundog";
                generator = "ntfy";
                file = "token";
              };
              safix = {
                user = "ana";
                name = "ntfy-token";
              };
            };
          }
        '';
        description = ''
          Every standing relationship between a clan var and a safix entry.

          The attribute name is the mapping's own identifier. It appears in
          reports, in commit messages and in refusals, and it is not derived
          from either endpoint — a name taken from one side reads wrongly in a
          sentence about the other.

          Evaluation refuses a mapping whose safix side does not resolve, whose
          import target a generator also produces, which writes a target another
          mapping also writes, or which pairs one set of endpoints in both
          directions. It refuses nothing about the clan side: that half lives in
          another flake, and a clan side that does not resolve is refused when a
          transfer reaches the mapping, naming the machine, the generator and
          the file.
        '';
      };
    };
  };
}
