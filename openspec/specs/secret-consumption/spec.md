# secret-consumption Specification

## Purpose

How a resolved secret arrives in a running profile: the namespace a NixOS or home-manager configuration names to receive its share of the declarations, what that namespace may and may not say, which wiring mistakes are refused at evaluation, the identity contract and the activation ordering that makes refusing it atomic, and the behaviour of a profile that resolves nothing.

## Requirements

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

#### Scenario: A user-scope profile that resolves secrets and names no identity

- **WHEN** a home-manager profile's resolved set is non-empty and no identity source is configured
- **THEN** evaluation fails with this package's own message, naming both identity options and stating why neither can be defaulted at that scope
- **AND** it fails before the provisioner's own key-source assertion is reached, which names the provisioner's options and none of this package's

#### Scenario: The refusal is attributable

- **WHEN** the refusal above is held by a check
- **THEN** it is read off a profile evaluated without the profile framework's assertion collection
- **AND** the reason is recorded: a profile evaluated with it refuses either way, and reports that something refused rather than which module did

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

The system-scope module SHALL contribute the installer described by `secret-installation` and SHALL NOT install an activation guard beside it, and the documentation SHALL record that no atomic refusal point at system activation has been demonstrated.
The installer's own pre-decryption identity check SHALL NOT be described as such a guard: it refuses before this package writes anything, and it makes no claim about what the rest of the activation has already done.

#### Scenario: What the system module installs

- **WHEN** the system module's activation contribution is enumerated
- **THEN** it contains this package's installer and no identity check that claims atomicity
- **AND** the documentation states why rather than leaving the asymmetry with the user scope unexplained

#### Scenario: The installer's own check is not the guard the user scope has

- **WHEN** the installer's pre-decryption identity refusal is documented
- **THEN** it states that it refuses before this package writes, links, or restarts anything of its own
- **AND** it does not state that the system generation is intact, because system activation reached this step by running earlier ones

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

### Requirement: Service-scoped placement carries the service's ownership

An entry granted to a service SHALL resolve at each of the service's machines, landing with the unix user and group the service declares, so that the narrowing the declaration states is enforced by the host's own access control.
Two services on one machine SHALL NOT resolve one entry onto one path, refused exactly as any other path collision is.

#### Scenario: The landed file belongs to the service

- **WHEN** a service-granted entry resolves at a machine's system scope
- **THEN** the resolved entry carries the service's declared user and group
- **AND** the path is the service's own rather than shared with another service's entries

#### Scenario: Ownership without an axis is refused, not dropped

- **WHEN** a service declaring a user or group is granted an entry that would resolve on a machine served by a user-scope profile
- **THEN** evaluation refuses, naming the service, the machine, and the missing axis
- **AND** a service declaring no ownership resolves there with the scope's ordinary placement

### Requirement: The namespace names the installer's ordering and its identity derivation

The consumption namespace SHALL carry the activation steps and the units this package's installer runs after, and a switch governing whether the system-scope identity is derived from the host's ssh host keys.
These SHALL be selection and decryption only: none of them declares a secret, a recipient, a grant, or an audience.

#### Scenario: The ordering is named here and nowhere else

- **WHEN** a consumer orders this package's installer after another component's
- **THEN** they name that component's activation step or unit through options of this namespace
- **AND** no option of this namespace names a particular component

#### Scenario: Naming nothing stays supported

- **WHEN** neither ordering option is set
- **THEN** the installer is registered with no dependency on any other component
- **AND** the profile evaluates, because a host with no foreign secret store needs no ordering

#### Scenario: The derivation switch is decryption, not custody

- **WHEN** the derivation switch is turned off
- **THEN** the identity is exactly what the consumer named
- **AND** nothing about which entries resolve, or who may read them, changes
