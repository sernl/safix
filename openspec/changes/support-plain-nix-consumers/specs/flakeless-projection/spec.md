## Purpose

The function a consumer without flake-parts calls to get the same resolver projection a flake-parts import produces, and how that projection reaches a consumption module that has no flake to read it from.

## ADDED Requirements

### Requirement: mkVault projects the catalogue without flake-parts

`lib.mkVault` SHALL be a function of the form `{ modules, root } -> projection`, published as a top-level flake output independent of `flakeModules.default`.
It SHALL evaluate `modules` together with safix's own resolver module through `lib.evalModules`, with `root` supplied as `_module.args.self`, and SHALL return exactly the value a flake-parts consumer's `flake.safix.lib` holds for the same declarations.

#### Scenario: The same declarations, two mechanisms, one value

- **WHEN** one fleet is declared once as `flake.safix.*` in a flake-parts module and again as the same fields passed through `mkVault`'s `modules`
- **THEN** `mkVault`'s return value and the flake-parts module's `flake.safix.lib` are the same value, field for field
- **AND** the comparison holds a real fleet, not a literal, so a divergence in either mechanism's resolution is what turns it red

#### Scenario: Declarations scattered across the modules list merge

- **WHEN** `modules` names several files, each declaring part of `flake.safix.catalogue` or `flake.safix.users`
- **THEN** `mkVault`'s projection sees the same merged record a single file declaring all of them would produce
- **AND** two of those files declaring the same name with different fields is a module-system evaluation error naming the option, exactly as it is under flake-parts

#### Scenario: root becomes the projection's path anchor

- **WHEN** `mkVault` is called with a given `root`
- **THEN** every `sopsFile` the projection resolves is a path rooted at `root`, the same way a flake-parts consumer's `self` roots its own
- **AND** `root` need not be a flake input; any path value that supports `+ "/…"` concatenation is sufficient, because the resolver algebra reads `self` only as a path to concatenate

#### Scenario: A module outside the namespace is refused the same way

- **WHEN** one of `modules` declares an option under `config.*` outside `flake.safix`
- **THEN** evaluation is refused by the same mechanism `modules/flake/checks/namespace.nix` holds for the flake-parts path
- **AND** no separate refusal exists for the `mkVault` path, because both paths evaluate the identical module tree

### Requirement: The projection is assignable directly into a consumption module

A value `mkVault` returns SHALL be usable wherever a consumption module's `safix.lib` option is documented as settable directly.

#### Scenario: A profile with no flake sets safix.lib from mkVault

- **WHEN** a NixOS or home-manager configuration with no `safix.flake` set instead sets `safix.lib = (import <safix path>).lib.mkVault { modules = [ ./secrets.nix ]; root = ./.; };`
- **THEN** the profile resolves secrets exactly as a flake-parts-backed profile with an equivalent `flake.safix.catalogue` would
- **AND** none of `config.flake`, `inputs.safix`, or flake-parts is present anywhere in that configuration's evaluation

#### Scenario: onboardingHook and enrollHook are not part of the returned value

- **WHEN** `mkVault`'s return value is inspected
- **THEN** it carries no `onboardingHook` or `enrollHook` field, because those are sibling options of `flake.safix`, not of `flake.safix.lib`, and a flake-parts consumer's `flake.safix.lib` never carried them either
- **AND** a consumer who wants either hook available to a flakeless CLI entry file declares it directly in that file, beside the `lib` field, rather than through `mkVault`
