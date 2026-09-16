{
  description = "safix, dendritic pattern: one declaration per file, merged by the module system";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

  inputs.safix.url = "path:../..";

  outputs =
    inputs@{
      flake-parts,
      nixpkgs,
      safix,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # This example resolves `flake.safix.lib` alone and builds nothing per
      # system, so it declares no `systems` and no `perSystem`: nothing here
      # needs a package set.
      #
      # The modules are read by directory rather than named one by one. A tree
      # that scatters one declaration per file does not enumerate them, and a
      # hand list beside a discovered one drifts on the next added file —
      # `safix-examples` asserts this file names no path under ./modules, so
      # the list cannot come back.
      imports = [
        safix.flakeModules.default
      ]
      ++ nixpkgs.lib.filesystem.listFilesRecursive ./modules;
    };
}
