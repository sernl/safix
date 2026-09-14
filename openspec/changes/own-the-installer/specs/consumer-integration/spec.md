## Purpose

Two requirements in this capability were true of the resolver and false of the installer.
"safix reads no option outside its own namespace" held for audiences, placements and the recipient policy, and did not hold for the fourteen options the system-scope installer read out of the secret provisioner's namespace, nor for the three the user-scope module wrote into it.
"Module entrypoints follow the secrets provisioner's own naming and import without a flake" described a two-form split whose only content was whether the provisioner was imported; with no provisioner, every form is the import-nothing form.
The scope-parity requirement is modified because the ownership axis it refuses a field against is now read off a type this package declares rather than off another framework's option declaration.

## REMOVED Requirements

### Requirement: Module entrypoints follow the secrets provisioner's own naming and import without a flake

**Reason**: Two of this requirement's three scenarios are statements about the secret provisioner.
"The alias matches the provisioner's own" justifies the doubled `homeModules`/`homeManagerModules` naming by that provisioner's own flake publishing both names, and "The `.safix` forms stay dependency-free" asserts an asymmetry — that the `.default` forms import the provisioner's module and the `.safix` forms do not — which is exactly the asymmetry this change removes.
With no provisioner imported anywhere, the alias is retained for compatibility rather than for conformance, and every published form is dependency-free, so the asymmetry has no second side.
The requirement is replaced by the ADDED requirement "Module entrypoints keep both published names and import nothing" below, which states the stronger, symmetric property.

**Migration**: No consumer action.
Every published name continues to resolve, and a consumer who chose a form for the property the retired asymmetry offered now gets it from every form.

## MODIFIED Requirements

### Requirement: safix reads no option outside its own namespace

The package SHALL read its declarations exclusively from its own namespace.
It SHALL NOT read, require, or default from any option belonging to a consumer's user registry, host registry, or module-selection scheme, or to any other secret-management framework.
It SHALL NOT define any option outside its own namespace.

#### Scenario: The whole input

- **WHEN** the resolver computes audiences, placements, and the recipient policy
- **THEN** its input is the catalogue and the user records in safix's own namespace
- **AND** no other option path is consulted

#### Scenario: A configurable path into a consumer's tree is refused as a design

- **WHEN** integration with a consumer's existing registry is considered
- **THEN** the package offers no option naming where that registry lives
- **AND** the reason is recorded: it would make every consumer's option tree part of this package's interface

#### Scenario: Declaring by hand and declaring by projection are indistinguishable

- **WHEN** user records are set from a mapping over a consumer's own registry
- **THEN** the resolver treats them exactly as it treats hand-written records
- **AND** no field defaults from anything outside the namespace

#### Scenario: The installer reads and writes only this package's namespace

- **WHEN** either scope's installer manifest, its invocation, its unit and its activation entry are constructed
- **THEN** every value they read comes from this package's own namespace or from the host's own activation, unit and ssh options
- **AND** no option belonging to another secret-management framework is read or defined, so a consumer running such a framework for their own secrets is unaffected by this package and this package is unaffected by whichever revision of it they pin

#### Scenario: The exception that used to exist is recorded

- **WHEN** the history of this requirement is documented
- **THEN** it records that the system-scope resolved set was previously typed by another framework's own option declaration in the same evaluation, which made that framework's module a required import
- **AND** it records that this was the reason the entry type had to become this package's own before the dependency could be dropped

### Requirement: One declaration serves both system and user scope

A resolved entry SHALL be materializable into either the system-scope or the user-scope form this package's own installer accepts, from the same declaration.

#### Scenario: The same entry on either side

- **WHEN** the same entry is materialized for a system configuration and for a user profile
- **THEN** its mode, its path, and the key it reads are the same declaration in both
- **AND** nothing about the declaration states which scope it is for

#### Scenario: A scope-specific field is refused where it is meaningless

- **WHEN** an ownership field is set on an entry materialized into a scope that has no ownership axis
- **THEN** evaluation fails naming the entry and the field
- **AND** the axis is read off the entry type this package declares, so the refusal does not depend on any other framework's option declaration being present

#### Scenario: A path is a function of the consuming configuration

- **WHEN** an entry's path needs a value the consuming configuration computes
- **THEN** the path is declared as a function of that configuration
- **AND** the same declaration resolves correctly under either scope

## ADDED Requirements

### Requirement: Module entrypoints keep both published names and import nothing

The package's consumption modules SHALL be published under `nixosModules.{safix,default}` and `homeModules.{safix,default}`, with `homeManagerModules` as an alias of `homeModules`, and the two names within each scope SHALL name the same value.
Every published module SHALL import nothing outside its own file, so that any of them is importable as a plain file path with no flake, no flake-parts, and no `inputs.safix` present anywhere in the importing tree.

#### Scenario: The alias and the doubled names are retained

- **WHEN** `homeManagerModules` and `homeModules` are compared, and each scope's two names are compared to each other
- **THEN** the alias holds and each scope's two names name one value
- **AND** the names are retained so that every `imports` line written against the previous surface keeps resolving, which is what makes the collapse a collapse rather than a rename

#### Scenario: A consumption module imports with no flake in the tree

- **WHEN** a NixOS or home-manager configuration imports `modules/consume/nixos.nix` or `modules/consume/home.nix` by a plain file path, with no flake input naming safix anywhere in that configuration's evaluation
- **THEN** the module evaluates, because it imports nothing beyond itself and reads only its own `safix.*` namespace and the host's own options
- **AND** resolving a secret still requires `safix.lib` to reach the module by some route — set directly, or from `flake.safix.lib`, or from `lib.mkVault` — which is unchanged by this requirement

#### Scenario: No published form names a flake input

- **WHEN** every published module value's import closure is inspected
- **THEN** none of them names any flake input
- **AND** the check that holds this is symmetric over all four published names, where it previously asserted that two of them did name an input and two did not, and its anti-vacuity probe is rebuilt so that a deliberately input-naming fixture still fires it
