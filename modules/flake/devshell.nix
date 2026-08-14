{ ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      devShells.default = pkgs.mkShellNoCC {
        packages = [
          pkgs.age
          pkgs.jq
          pkgs.sops
        ];
      };
    };
}
