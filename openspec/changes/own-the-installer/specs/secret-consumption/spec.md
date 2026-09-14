## Purpose

The requirement that each consumption module ships in two forms — one importing the secret provisioner, one importing nothing — exists only because `imports` cannot depend on an option and a consumer who already pinned that provisioner needed a form that did not import a second copy of it.
With no provisioner to import, the two forms are the same module, and the requirement's three scenarios are about a distinction with no content.
What replaces it is the stronger property the split used to offer as only one of its two halves: every published form imports nothing outside its own file.

The user scope's inertness requirement is also modified: "no unit" and "no activation entry" now describe things this package would otherwise define itself rather than things another module would define.

## REMOVED Requirements

### Requirement: Each consumption module ships in a form that imports the provisioner and a form that does not

**Reason**: The distinction the two forms encode is whether the consumer's tree already imports the secret provisioner's own module.
This package no longer imports that module in any form, so both forms are one value and the choice between them has nothing to select.
The requirement's third scenario — that two distinct copies of one option-declaring module are an evaluation error — is not lost: it moves to the ADDED requirement below, restated over this package's own declaring module, which is the module a consumer can now import twice.

**Migration**: No consumer action.
Both published names per scope continue to resolve, to the same value, so every existing `imports` line keeps working.
A consumer who chose the provisioner-importing form because their tree had no provisioner now gets one that needs none; a consumer who chose the other form gets the same module they already had.

## MODIFIED Requirements

### Requirement: A profile that resolves nothing is inert

When the resolved set is empty, the module SHALL define nothing: no resolved entries, no identity configuration, no manifest, no activation entry, and no unit.

#### Scenario: Nothing resolved

- **WHEN** a profile imports the module and the person it serves resolves no secret on that host
- **THEN** the profile's activation entries contain no entry from this package
- **AND** no unit of this package's exists on that profile, at either scope

#### Scenario: The enable default follows the resolution

- **WHEN** the module's enable flag is not set explicitly
- **THEN** it is on exactly when the resolved set is non-empty
- **AND** the whole of the module's configuration is conditional on it

#### Scenario: Inertness is now a statement about this package alone

- **WHEN** the reason inertness is checkable is documented
- **THEN** it states that every entry, unit and manifest the check looks for belongs to this package
- **AND** the change is recorded: inertness previously also meant leaving another framework's secrets option empty, and this package no longer writes that option at all

### Requirement: Wiring mistakes are refused as this package's evaluation errors

A missing or malformed binding SHALL fail evaluation with a message naming the option of this package that is wrong.
A custody violation SHALL be reported by this package, in full, rather than as some other component's first failure.

#### Scenario: Bound but unaddressed

- **WHEN** a profile is bound to a consumer's declarations but names no person, or no host where none can be derived
- **THEN** evaluation fails naming the option that is unset
- **AND** the module defines nothing in the meantime, so the refusal is what speaks

#### Scenario: Declarations that do not resolve

- **WHEN** the declarations a profile is bound to carry custody violations
- **THEN** evaluation fails listing every violation
- **AND** the failure names this package, which is now the only component that could raise it

#### Scenario: Configured but bound to nothing

- **WHEN** a profile names a person or a host and no binding to a consumer's declarations was set
- **THEN** evaluation fails naming the option that supplies the binding
- **AND** the two states are told apart by whether a definition exists, never by its value, since each scope defaults one of those options to something it can derive

#### Scenario: Imported and unconfigured stays a no-op

- **WHEN** a profile imports the module and sets nothing of the namespace at all
- **THEN** evaluation succeeds, the enable flag is off, and nothing is established
- **AND** no refusal is raised, so an inert import and a mis-wired one do not look alike

#### Scenario: A person nobody declared

- **WHEN** a profile, or a direct call to the resolver, selects a person the user records do not declare
- **THEN** evaluation fails naming that person and listing the declared users
- **AND** the refusal belongs to the resolver, so a profile, a direct call and the command reach one message rather than three

## ADDED Requirements

### Requirement: One module per scope, published under both names

The package SHALL publish one consumption module per scope, reachable under both the scope's plain name and its default name, with both names naming the same value.
Each SHALL import nothing outside its own file.

#### Scenario: The two names are one value

- **WHEN** a scope's two published module names are compared
- **THEN** they name the same value
- **AND** both names remain published, so an existing import of either keeps resolving

#### Scenario: Every form is dependency-free

- **WHEN** any published consumption module's import list is inspected
- **THEN** it names no flake input and no file outside this package's own consumption directory
- **AND** a check holds this over every published name rather than over a chosen one, because a form that quietly regains a dependency is exactly what the retired two-form split was compensating for

#### Scenario: Importing one declaring module twice is still an evaluation error

- **WHEN** two distinct copies of this package's own declaring module are imported into a single evaluation
- **THEN** evaluation fails naming the option and both files
- **AND** this is held by a check rather than stated only in prose, because no option can repair it after the fact — imports cannot depend on configuration
