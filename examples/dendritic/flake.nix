{
  description = "safix, dendritic pattern: one declaration per file, merged by the module system";

  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.safix.url = "path:../..";

  outputs =
    inputs@{ flake-parts, safix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # This example resolves `flake.safix.lib` alone and builds nothing per
      # system, so it declares no `systems` and no `perSystem`: nothing here
      # needs a package set.
      imports = [
        safix.flakeModules.default

        ./modules/catalogue/shelf-item.nix
        ./modules/catalogue/team-wifi.nix

        ./modules/machines/deck.nix
        ./modules/services/web.nix
        ./modules/groups/oncall.nix
        ./modules/organizations/acme.nix
        ./modules/silos/corp.nix

        ./modules/users/alice/profile.nix
        ./modules/users/alice/carries-shelf-item.nix
        ./modules/users/alice/carries-team-wifi.nix
        ./modules/users/alice/private-laptop-token.nix
        ./modules/users/alice/private-generated-token.nix
        ./modules/users/alice/shared-with-bob.nix
        ./modules/users/alice/shared-with-deck.nix
        ./modules/users/alice/shared-with-web.nix
        ./modules/users/alice/shared-with-oncall.nix
        ./modules/users/alice/escrowed-to-acme.nix
        ./modules/users/alice/per-host-deck.nix
        ./modules/users/alice/per-tag-portable.nix

        ./modules/users/bob/profile.nix
        ./modules/users/bob/carries-shelf-item.nix
        ./modules/users/bob/carries-team-wifi.nix
      ];
    };
}
