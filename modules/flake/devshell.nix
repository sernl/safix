{ ... }:
{
  perSystem =
    { config, pkgs, ... }:
    {
      devShells.default = pkgs.mkShellNoCC {
        packages = [
          pkgs.age
          pkgs.jq
          pkgs.sops
          # The command this repository builds, driven against this repository.
          # It reads flake.safix.* out of whatever flake its working directory
          # belongs to, so a shell holding it is a shell that can operate the
          # declarations beside it.
          config.packages.safix
        ];
      };
    };
}
