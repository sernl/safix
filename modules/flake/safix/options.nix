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
  };
}
