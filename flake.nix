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

    # A check dependency only, and no output references it. Proving that the
    # identity preflight sorts ahead of `checkLinkTargets` means topologically
    # sorting a real profile's activation DAG, which needs a real home-manager
    # evaluation rather than the module evaluated on its own.
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    # Read only by `safix-bridge-real-clan`, and the reason that check needs an
    # input at all: clan-cli is not packaged in nixpkgs, so there is no
    # attribute to reach for. It supplies two things — the command the check
    # drives, and `packages.clan-core-flake`, a copy of clan-core whose lock
    # names store paths instead of URLs, which is what lets the throwaway clan
    # lock offline in a sandbox with no network.
    #
    # Pinned to a revision rather than to a branch. The subject of this check is
    # a specific clan's behaviour at a specific version, and an input that moved
    # on every `nix flake update` would redden it for reasons that have nothing
    # to do with safix. Moving the pin is a deliberate act with its own commit.
    #
    # nixpkgs is deliberately not made to follow this flake's: clan-cli pins its
    # own nix version and its own runtime dependency set, and the throwaway clan
    # is evaluated by clan against clan's nixpkgs.
    clan-core.url = "https://git.clan.lol/api/v1/repos/clan/clan-core/archive/56e35624d94e4f1ac55d36575ebab97cbd9b9cdd.tar.gz";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    let
      # One file per scope, published under both its names. The two names are
      # kept so that every `imports` line written against the previous surface
      # keeps resolving, which is what makes this a collapse rather than a
      # rename.
      #
      # Each published name is the file plus one wrapper whose whole content is
      # a default: `safix.installer.package`. The consumption modules import
      # nothing and hold no flake input, by contract, so they cannot reach
      # safix's own build themselves — their own default is `pkgs.safix`,
      # which is what a genuinely flakeless tree has to supply. Here
      # `packages` is in scope, so the flake supplies it, at `mkDefault`, and
      # a consumer of any published name needs no line of their own.
      # `validationPackage` is the build-platform build for the same reason
      # `installer.nix` splits the two: the manifest's check phase runs inside
      # the build where the activation runs on the host.
      #
      # The bare files stay importable with no flake at all: the wrapper's
      # `imports` names one file inside this package's own consumption
      # directory and nothing else.
      installerPackageDefaults =
        { pkgs, lib, ... }:
        {
          safix.installer.package =
            lib.mkDefault
              inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.safix;
          safix.installer.validationPackage =
            lib.mkDefault
              inputs.self.packages.${pkgs.stdenv.buildPlatform.system}.safix;
        };

      homeModule = {
        imports = [
          ./modules/consume/home.nix
          installerPackageDefaults
        ];
      };

      nixosModule = {
        imports = [
          ./modules/consume/nixos.nix
          installerPackageDefaults
        ];
      };

      homeModules = {
        safix = homeModule;
        default = homeModule;
      };
    in
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
        ./modules/flake/checks/bridge-sync.nix
        ./modules/flake/checks/cli.nix
        ./modules/flake/checks/consumption.nix
        ./modules/flake/checks/custody.nix
        ./modules/flake/checks/entrypoints.nix
        ./modules/flake/checks/envelope.nix
        ./modules/flake/checks/examples.nix
        ./modules/flake/checks/exported.nix
        ./modules/flake/checks/gate-guard.nix
        ./modules/flake/checks/generators.nix
        ./modules/flake/checks/installer-vm.nix
        ./modules/flake/checks/installer.nix
        ./modules/flake/checks/keepassxc.nix
        ./modules/flake/checks/materialization.nix
        ./modules/flake/checks/namespace.nix
        ./modules/flake/checks/policy.nix
        ./modules/flake/checks/portability.nix
        ./modules/flake/checks/real-clan.nix
        ./modules/flake/checks/single-runtime.nix
        ./modules/flake/checks/storage.nix
        ./modules/flake/checks/subjects.nix
        ./modules/flake/checks/vault-projection.nix
        ./modules/flake/checks/vault.nix
        ./modules/flake/devshell.nix
        ./modules/flake/lib.nix
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
        # arrive. Each scope publishes one module under both names — `.safix`
        # and `.default` are one value — and every published form imports
        # nothing outside its own file, so any of them is importable as a plain
        # file path with no flake in the tree.
        #
        # Both names are retained so that every `imports` line written against
        # the previous surface keeps resolving. The split existed for one
        # reason: `imports` cannot depend on an option, so a tree without the
        # secret provisioner needed a form that imported it and a tree pinning
        # its own needed a form that did not. With no provisioner to import the
        # distinction has no content, and what is left is strictly the stronger
        # of the two properties it used to offer separately.
        #
        # Importing two distinct copies of one declaring module is an
        # evaluation error rather than a merge — `safix-module-collision` holds
        # that fact — and no configuration can repair it after the fact.
        inherit homeModules;
        homeManagerModules = homeModules;

        nixosModules = {
          safix = nixosModule;
          default = nixosModule;
        };
      };
    };
}
