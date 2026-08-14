## Purpose

What a consumer must supply and what the package refuses to assume: that no option path outside safix's own namespace is ever read, that a consumer's existing user vocabulary is bridged by an adapter the consumer owns, that host attachment during onboarding is a consumer-provided hook rather than a built-in idiom, and that one declaration serves both system and user scope.

## ADDED Requirements

### Requirement: safix reads no option outside its own namespace

The package SHALL read its declarations exclusively from its own namespace.
It SHALL NOT read, require, or default from any option belonging to a consumer's user registry, host registry, or module-selection scheme.

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

### Requirement: One declaration serves both system and user scope

A resolved entry SHALL be materializable into either the system-scope or the user-scope form the secret provisioner accepts, from the same declaration.

#### Scenario: The same entry on either side

- **WHEN** the same entry is materialized for a system configuration and for a user profile
- **THEN** its mode, its path, and the key it reads are the same declaration in both
- **AND** nothing about the declaration states which scope it is for

#### Scenario: A scope-specific field is refused where it is meaningless

- **WHEN** an ownership field is set on an entry materialized into a scope whose provisioner has no ownership axis
- **THEN** evaluation fails naming the entry and the field
- **AND** the field is not silently dropped

#### Scenario: A path is a function of the consuming configuration

- **WHEN** an entry's path needs a value the consuming configuration computes
- **THEN** the path is declared as a function of that configuration
- **AND** the same declaration resolves correctly under either scope
