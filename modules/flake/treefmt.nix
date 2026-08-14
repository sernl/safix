# Nix formatting held to exactly what the source repository this package is
# extracted from runs, so a ported file arrives already formatted and the port
# commits carry no reformatting noise. Adding a formatter here before the port
# completes would rewrite every incoming file on its first pass and bury the
# extraction diff.
{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem = {
    treefmt.config = {
      projectRootFile = "flake.nix";
      flakeCheck = true;
      programs.nixfmt.enable = true;
    };
  };
}
