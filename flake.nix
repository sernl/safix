{
  description = "safix — custody-first secrets management for nix, on sops";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";

    # A development and check dependency rather than a runtime one. safix
    # generates the recipient policy and places the ciphertext; sops-nix is what
    # a consumer's NixOS or home-manager profile then reads, so the checks that
    # prove a resolved entry is consumable have to build against it.
    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      imports = [
        ./modules/flake/devshell.nix
        ./modules/flake/safix
        ./modules/flake/treefmt.nix
      ];

      # The module a consumer imports. This flake imports it too, so the checks
      # exercise the same module a consumer gets rather than a second copy that
      # agrees with it by inspection.
      flake.flakeModules.default = ./modules/flake/safix;
    };
}
