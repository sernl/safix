# The flake-parts module a consumer imports. It declares the two records and
# nothing else yet; the resolution helpers land here as the port proceeds.
{ ... }:
{
  imports = [ ./options.nix ];
}
