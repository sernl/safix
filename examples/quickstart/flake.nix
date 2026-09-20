{
  description = "safix quickstart: one person, one host, one service";

  # --8<-- [start:inputs]
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    safix.url = "path:../..";
  };
  # --8<-- [end:inputs]

  # --8<-- [start:outputs]
  outputs =
    inputs@{ nixpkgs, home-manager, ... }:
    {
      # What the command reads: every verb evaluates an attribute of
      # `safix.lib`, so this output is what binds the declarations to the tree.
      safix.lib = inputs.safix.lib.mkVault {
        modules = [ ./secrets.nix ];
        root = ./.;
      };

      nixosConfigurations.web = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs; };
        modules = [ ./hosts/web.nix ];
      };

      homeConfigurations.alice = home-manager.lib.homeManagerConfiguration {
        pkgs = nixpkgs.legacyPackages.x86_64-linux;
        extraSpecialArgs = { inherit inputs; };
        modules = [ ./home/alice.nix ];
      };
    };
  # --8<-- [end:outputs]
}
