# consumer-integration Specification

## Purpose

What a consumer must supply and what the package refuses to assume: that no option path outside safix's own namespace is ever read, that a consumer's existing user vocabulary is bridged by an adapter the consumer owns, that host attachment during onboarding is a consumer-provided hook rather than a built-in idiom, and that one declaration serves both system and user scope.

## Requirements

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

### Requirement: A consumer bridges its own vocabulary with an adapter it owns

Bridging a consumer's existing user vocabulary to safix's SHALL be a projection written in the consumer's repository.
The package SHALL ship no adapter for any particular consumer.

#### Scenario: What an adapter is

- **WHEN** a consumer with its own user registry adopts the package
- **THEN** the bridge is a mapping from their records into safix's user records
- **AND** it lives in their repository

#### Scenario: The package ships none

- **WHEN** the package's modules are enumerated
- **THEN** none of them names a particular consumer's option path
- **AND** the documentation carries a worked example rather than an importable adapter

#### Scenario: An adapter is sufficient on its own

- **WHEN** a consumer supplies only an adapter
- **THEN** every capability of the package is available to them
- **AND** no further integration point is required

### Requirement: Host attachment during onboarding is a consumer-supplied hook

Onboarding SHALL scaffold only the declarations safix owns and regenerate the recipient policy.
Any further action — attaching an account on a host, allocating an identifier, editing a host's module imports — SHALL be performed by a consumer-supplied hook, and its absence SHALL be a supported configuration.

#### Scenario: What onboarding does by itself

- **WHEN** onboarding runs with no hook configured
- **THEN** it writes the person's safix declarations and regenerates the recipient policy
- **AND** it succeeds, having simply done less

#### Scenario: The hook's contract

- **WHEN** a hook is configured
- **THEN** it is invoked after the safix-owned scaffolding is written, with the new person's name and recipient
- **AND** the package makes no assumption about what it does

#### Scenario: The idiom that is not built in

- **WHEN** the package's onboarding is documented
- **THEN** it records that host account attachment, identifier allocation, and refusing hosts that lack a particular module are consumer concerns
- **AND** the reason is stated: those idioms are properties of one consumer's module tree

### Requirement: The governed file set is computed, with a consumer-named extension

The set of encrypted files the package governs SHALL be computed from the audiences the declarations imply.
A consumer SHALL be able to name further files it wants governed, and the set the re-wrapping command acts on SHALL be the union of the two.

#### Scenario: The required half is derived, not discovered

- **WHEN** the governed set is computed
- **THEN** it follows from the audiences the declarations imply
- **AND** no directory of the consumer's tree is read to find it, because a tree layout is not the package's to assume

#### Scenario: A file no declaration implies

- **WHEN** a consumer holds an encrypted file that rides an existing rule but that no declaration names
- **THEN** it names that file through the extension option
- **AND** the file then appears in the set the re-wrapping command acts on

#### Scenario: The union is what the command re-wraps

- **WHEN** the re-wrapping command acts on the governed set
- **THEN** that set is the union of the derived half and the consumer-named half
- **AND** narrowing it to the derived half alone is a defect, since a file left out of it is a file a change of audience reaches for every other file and not for that one

#### Scenario: Naming a file does not create a rule for it

- **WHEN** a consumer names a path no rule matches
- **THEN** no rule is emitted for it
- **AND** encryption against that path still fails closed, because a rule comes from a user record with a recipient and from nothing else

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

### Requirement: Ordering against a foreign secret store is a consumer-named dependency

Where another component of the host installs secrets, the step or unit this package's installer must follow SHALL be named by the consumer.
The package SHALL NOT read any option belonging to that component to discover it, and naming nothing SHALL be a supported configuration.

#### Scenario: The package does not look for it

- **WHEN** the package decides what its installer runs after
- **THEN** it reads only its own namespace and the host's own activation and unit options
- **AND** no option path belonging to a particular secret-management framework is consulted

#### Scenario: The reason it is not built in

- **WHEN** the ordering is documented
- **THEN** it records that discovering one framework's installer would make that framework's option tree part of this package's interface, and would answer for one foreign store out of the many a host may run
- **AND** the same reason is the one already given for a consumer's user registry

#### Scenario: What is given up by not discovering it

- **WHEN** a consumer names no ordering on a host that has a foreign store
- **THEN** the package cannot guarantee the ordering and does not claim to
- **AND** the installer's own pre-decryption refusal names these options as the remedy, so the failure is attributable rather than silent

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
