## Purpose

How a resolved secret arrives in a running profile: the namespace a NixOS or home-manager configuration names to receive its share of the declarations, what that namespace may and may not say, which wiring mistakes are refused at evaluation, the identity contract and the activation ordering that makes refusing it atomic, and the behaviour of a profile that resolves nothing.

## ADDED Requirements

### Requirement: Arrival is declared in the module system the profile is written in

The package SHALL provide a module for each consumption scope, declaring a namespace of its own inside that module system, so a profile establishes its secrets by naming which resolved set it serves.

#### Scenario: A profile names its set and nothing else

- **WHEN** a home-manager profile imports the home module and names the person and host it serves
- **THEN** every secret that person resolves on that host is established in that profile
- **AND** no further wiring is written by the consumer

#### Scenario: The same for a system configuration

- **WHEN** a system configuration imports the system module and names the person and host it serves
- **THEN** that person's system-scope entries are established on that host
- **AND** the declarations they come from are the same ones the home module reads

#### Scenario: What the consumer replaces

- **WHEN** a consumer already wires the resolver into a provisioner by hand
- **THEN** the module reproduces that wiring, including its identity configuration and its activation guard
- **AND** the hand-written form and the module form establish the same set

### Requirement: The consumption namespace declares no custody

The consumption namespace SHALL be able to select which resolved set arrives and SHALL NOT be able to declare a secret, a recipient, a grant, or an audience.

#### Scenario: Selection only

- **WHEN** the options of the consumption namespace are enumerated
- **THEN** each one names which resolved set arrives, or how the machine decrypts it
- **AND** none of them adds to or alters the catalogue or the user records

#### Scenario: Why custody cannot live here

- **WHEN** the placement of custody declarations is documented
- **THEN** the reason is recorded: an audience is a function of every user's declarations at once, and the recipient policy is one repository-global file
- **AND** a single profile is therefore structurally unable to compute either

### Requirement: A profile is bound to the consumer's own declarations by one named option

The module SHALL obtain the resolved declarations through an option the consumer sets, and SHALL NOT require any particular evaluation-seam argument.

#### Scenario: The binding is stated

- **WHEN** a consumer wires a profile to their declarations
- **THEN** they set one option naming their own flake, or the resolver projection directly
- **AND** the module reads no module argument that the consumer must have arranged to pass

#### Scenario: A flake that carries no declarations

- **WHEN** the option is set to a flake that has not imported the package's flake module
- **THEN** evaluation fails naming the option and the likely cause
- **AND** the message belongs to this package

### Requirement: Each consumption module ships in a form that imports the provisioner and a form that does not

The package SHALL export, for each scope, one module that imports the secret provisioner's module and one that declares the same namespace and imports nothing.

#### Scenario: A tree without the provisioner

- **WHEN** a consumer whose tree does not already import the provisioner imports the default form
- **THEN** the provisioner's options are available and the profile evaluates
- **AND** the consumer writes one import rather than two

#### Scenario: A tree that already pins its own provisioner

- **WHEN** a consumer already imports the provisioner at a revision of their own
- **THEN** they import the form that imports nothing
- **AND** the namespace and the behaviour are identical to the default form

#### Scenario: The fact the two forms exist for

- **WHEN** two distinct copies of one option-declaring module are imported into a single evaluation
- **THEN** evaluation fails naming the option and both files
- **AND** this is held by a check rather than stated only in prose, because no option can repair it after the fact — imports cannot depend on configuration

### Requirement: A profile that resolves nothing is inert

When the resolved set is empty, the module SHALL define nothing: no secrets, no identity configuration, no activation entry, and no unit.

#### Scenario: Nothing resolved

- **WHEN** a profile imports the module and the person it serves resolves no secret on that host
- **THEN** the profile's activation entries contain no entry from this package
- **AND** no secret-provisioning unit exists on that profile

#### Scenario: The enable default follows the resolution

- **WHEN** the module's enable flag is not set explicitly
- **THEN** it is on exactly when the resolved set is non-empty
- **AND** the whole of the module's configuration is conditional on it

### Requirement: Wiring mistakes are refused as this package's evaluation errors

A missing or malformed binding SHALL fail evaluation with a message naming the option of this package that is wrong.
A custody violation SHALL be reported by this package, in full, rather than as the provisioner's first failure.

#### Scenario: Bound but unaddressed

- **WHEN** a profile is bound to a consumer's declarations but names no person, or no host where none can be derived
- **THEN** evaluation fails naming the option that is unset
- **AND** the module defines nothing in the meantime, so the refusal is what speaks

#### Scenario: Declarations that do not resolve

- **WHEN** the declarations a profile is bound to carry custody violations
- **THEN** evaluation fails listing every violation
- **AND** the failure names this package rather than arising from inside the provisioner's own evaluation

### Requirement: The identity contract carries the provisioner's fatality semantics

The module SHALL default the key-file identity to unset, and SHALL document at the option that the provisioner treats a set-but-unreadable key file as fatal while a missing ssh key path is skipped with a warning.

#### Scenario: The default that cannot abort activation

- **WHEN** a profile configures no key file
- **THEN** none is set on the provisioner
- **AND** the reason is recorded at the option: a set-but-missing key file aborts activation, a missing ssh key path does not

#### Scenario: The identity reaches the provisioner

- **WHEN** a profile names ssh key paths or a key file
- **THEN** the provisioner's corresponding options carry exactly those values
- **AND** they are defined at a priority that overrides a default set elsewhere in the consumer's tree

### Requirement: A user-scope profile refuses the switch before anything is linked when no identity is usable

Where the resolved set is non-empty and an identity is configured, the module SHALL install an activation check that sorts ahead of the profile's link step, verifies the configured identity for presence and readability, and aborts.

#### Scenario: The ordering the guarantee rests on

- **WHEN** the profile's activation entries are topologically sorted
- **THEN** this package's identity check precedes the link-checking step
- **AND** the ordering is asserted against a real evaluation of the profile rather than described

#### Scenario: A machine without the key

- **WHEN** activation runs where the configured identity is absent or unreadable
- **THEN** activation stops at that check, naming each identity path and how it failed
- **AND** nothing has been linked, installed, restarted, or written

#### Scenario: The required and the sufficient are distinguished

- **WHEN** the identity is a key file that will not be generated
- **THEN** its absence alone fails the check
- **WHEN** the identity is one or more ssh key paths and no other source can decrypt
- **THEN** the check fails only if none of them is readable

#### Scenario: Absent when there is nothing to check

- **WHEN** a profile configures no identity at all
- **THEN** no such activation check is installed

#### Scenario: The limit of the check is stated

- **WHEN** the check's failure is reported
- **THEN** it states that presence and readability were checked and decryption was not
- **AND** it does not imply that a readable identity can open the files

### Requirement: The system scope claims no activation guard

The system-scope module SHALL NOT install an activation guard, and the documentation SHALL record that no atomic refusal point at system activation has been demonstrated.

#### Scenario: What the system module installs

- **WHEN** the system module's activation contribution is enumerated
- **THEN** it contains no identity check of this package's
- **AND** the documentation states why rather than leaving the asymmetry unexplained

### Requirement: Scope is a property of the module, not of a declaration

Each consumption module SHALL materialize at its own scope, and no option of the consumption namespace SHALL name a scope.

#### Scenario: One declaration, two arrivals

- **WHEN** one fixture's declarations are consumed by both modules
- **THEN** the user-scope profile and the system configuration establish the same names
- **AND** nothing in the declarations or in either profile states which scope it is for

#### Scenario: The ownership asymmetry is inherited

- **WHEN** a declaration setting an ownership field arrives at the system scope
- **THEN** the field reaches the provisioner
- **WHEN** the same declaration arrives at the user scope
- **THEN** evaluation fails naming the entry and the field
