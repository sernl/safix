## ADDED Requirements

### Requirement: Module entrypoints follow the secrets provisioner's own naming and import without a flake

The package's consumption modules SHALL be published under `nixosModules.{safix,default}` and `homeModules.{safix,default}`, and `homeManagerModules` SHALL exist as an alias of `homeModules`, matching the naming the secrets provisioner itself publishes.
Each of `nixosModules.safix` and `homeModules.safix` SHALL import nothing outside its own file, so that either is importable as a plain file path with no flake, no flake-parts, and no `inputs.safix` present anywhere in the importing tree.

#### Scenario: The alias matches the provisioner's own

- **WHEN** `homeManagerModules` and `homeModules` are compared
- **THEN** `homeManagerModules.safix` and `homeManagerModules.default` name the same values as `homeModules.safix` and `homeModules.default`
- **AND** the alias exists because the secrets provisioner's own flake publishes both names for the same module

#### Scenario: A consumption module imports with no flake in the tree

- **WHEN** a NixOS or home-manager configuration imports `modules/consume/nixos.nix` or `modules/consume/home.nix` by a plain file path, with no flake input naming safix anywhere in that configuration's evaluation
- **THEN** the module evaluates, because it imports nothing beyond itself and reads only its own `safix.*` namespace and the provisioner's `sops.*` namespace
- **AND** resolving a secret still requires `safix.lib` to reach the module by some route — set directly, or from `flake.safix.lib`, or from `lib.mkVault` — which is unchanged by this requirement

#### Scenario: The `.safix` forms stay dependency-free

- **WHEN** `nixosModules.safix` and `homeModules.safix` are compared to their `.default` counterparts
- **THEN** neither `.safix` form imports the secrets provisioner's own module, where each `.default` form does
- **AND** that asymmetry is what makes the `.safix` forms importable with no flake at all, not merely with no flake-parts: a consumer supplying their own provisioner revision supplies it themselves, and a consumer with neither a flake nor a pinned provisioner uses `.default` from inside a flake that has one
