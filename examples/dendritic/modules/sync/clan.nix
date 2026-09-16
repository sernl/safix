# The clan this consumer bridges to, and one mapping into it. `../..` is this
# example's own root, which is the same declaration `examples/plain-nix` makes
# against a different tree — and the reason the comparison elides a leading
# example root rather than dropping the field.
{
  flake.safix.bridge = {
    clanFlake = ../..;
    mappings.ntfy-token = {
      direction = "clan-to-safix";
      clan = {
        machine = "meridian";
        generator = "ntfy";
        file = "token";
      };
      safix = {
        user = "alice";
        name = "ntfy-token";
      };
    };
  };
}
