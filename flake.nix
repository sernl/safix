{
  description = "safix — custody-first secrets management for nix, on sops";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";

    # The cargo builder for the rust runtime. It takes no nixpkgs of its own;
    # the toolchain comes from this flake's pinned nixpkgs, which is what makes
    # the workspace's stated minimum version the one anything is ever compiled
    # with.
    crane.url = "github:ipetkov/crane";

    # Read only by `safix-rs-audit`. It is an input rather than a fetch at build
    # time because the sandbox has no network, and pinning it here is what gives
    # a newly published advisory a date: it reddens that one check on the commit
    # that updates this lock, and not before.
    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;

    # sops-nix is what a consumer's NixOS or home-manager profile reads, so the
    # checks that prove a resolved entry is consumable have to build against it.
    # It is also imported by `homeModules.default` and `nixosModules.default`, so
    # for those two outputs it is a runtime dependency as well; the `.safix`
    # variants of both import nothing and leave the choice of revision to the
    # consumer.
    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";

    # A check dependency only, and no output references it. Proving that the
    # identity preflight sorts ahead of `checkLinkTargets` means topologically
    # sorting a real profile's activation DAG, which needs a real home-manager
    # evaluation rather than the module evaluated on its own.
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ flake-parts, sops-nix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        # x86_64-darwin is absent because the pinned nixpkgs (26.11) dropped
        # the platform; listing it makes `nix flake show` fail on evaluation.
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      imports = [
        ./modules/flake/checks/bridge.nix
        ./modules/flake/checks/cli.nix
        ./modules/flake/checks/consumption.nix
        ./modules/flake/checks/custody.nix
        ./modules/flake/checks/exported.nix
        ./modules/flake/checks/generators.nix
        ./modules/flake/checks/materialization.nix
        ./modules/flake/checks/namespace.nix
        ./modules/flake/checks/policy.nix
        ./modules/flake/checks/single-runtime.nix
        ./modules/flake/devshell.nix
        ./modules/flake/rust.nix
        ./modules/flake/safix
        ./modules/flake/treefmt.nix
      ];

      flake = {
        # The module a consumer imports at flake level, where custody is
        # declared. This flake imports it too, so the checks exercise the same
        # module a consumer gets rather than a second copy that agrees with it by
        # inspection.
        flakeModules.default = ./modules/flake/safix;

        # The modules a consumer imports into a profile, where resolved secrets
        # arrive. Each ships twice, and the choice is made at import time because
        # `imports` cannot depend on an option: the `.default` forms import
        # sops-nix for a tree that has not got one, and the `.safix` forms import
        # nothing, for a tree that pins its own revision. Importing two distinct
        # copies of one declaring module is an evaluation error rather than a
        # merge — `safix-module-collision` holds that fact — and no configuration
        # can repair it after the fact.
        homeModules = {
          safix = ./modules/consume/home.nix;
          default = {
            imports = [
              sops-nix.homeManagerModules.sops
              ./modules/consume/home.nix
            ];
          };
        };

        nixosModules = {
          safix = ./modules/consume/nixos.nix;
          default = {
            imports = [
              sops-nix.nixosModules.sops
              ./modules/consume/nixos.nix
            ];
          };
        };
      };
    };
}
