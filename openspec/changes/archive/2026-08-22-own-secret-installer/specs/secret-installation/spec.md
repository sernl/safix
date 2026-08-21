## Purpose

The installer safix runs at system scope: that it invokes the secret provisioner's binary against a manifest of its own rather than borrowing the provisioner's activation, that the store it writes into is its own and disjoint from any other on the host, how it is ordered against a store it did not create, how the identity it decrypts with is derived and what happens when none is derivable, and what it leaves untouched on a host that already installs secrets by some other means.

## ADDED Requirements

### Requirement: safix installs its resolved set itself at system scope

At system scope the package SHALL build its own installer manifest and invoke the secret provisioner's installer binary against it, and SHALL NOT deliver its resolved set through the provisioner's own secrets option.
The provisioner's installer SHALL therefore be inert with respect to anything safix resolved.

#### Scenario: One installer, and it is this package's

- **WHEN** a system configuration establishes a non-empty resolved set
- **THEN** the configuration carries an installer invocation of this package's own, naming a manifest this package built
- **AND** the provisioner's secrets option is empty, so the provisioner's installer, its activation entry, its unit and its key-source assertion are all inert

#### Scenario: The resolved set is typed by the provisioner and refused by this package

- **WHEN** an entry of the resolved set is read back before the manifest is built
- **THEN** it has passed through the provisioner's own secret type, read off the provisioner's option declaration rather than restated, so its mode, ownership, sops-file and file-hash defaults are the provisioner's
- **AND** the refusals the provisioner applies in its own manifest builder rather than in that type — a sops file that does not exist, and one outside the nix store — are carried by this package's builder instead, because the type does not carry them and this package does not call that builder

#### Scenario: What a consumer reads to see what arrived

- **WHEN** a consumer inspects what this package established on a system configuration
- **THEN** the resolved set and the typed installed set are both readable in this package's namespace
- **AND** the migration records that the provisioner's secrets option is no longer where it appears

### Requirement: The secret store is this package's own and is named nowhere else

The manifest SHALL name a secrets mount point and a symlink path belonging to this package, distinct from the provisioner's defaults and from any store another component of the host owns.
No entry SHALL be written, symlinked, or removed outside that store except at a path a declaration states.

#### Scenario: The roots are this package's

- **WHEN** the built manifest is read
- **THEN** its mount point and its symlink path are this package's own
- **AND** neither is the path the provisioner hardcodes

#### Scenario: An entry that declares no path parks in this package's store

- **WHEN** a resolved entry declares no path of its own
- **THEN** the manifest gives it a path inside this package's symlink path
- **AND** the store root and the entry default are held by one check, because a root moved without the entry default turns a collision into a silent write into the other store

#### Scenario: A declared path is still honoured

- **WHEN** a resolved entry declares a path
- **THEN** that path is what the manifest carries
- **AND** the refusal of two entries resolving onto one path is unchanged

### Requirement: The manifest is validated by the binary that will read it

The manifest SHALL be checked at build time by the installer binary, in whichever of its checking modes the provisioner's own sops-file validation setting selects, so that this package validates neither less nor more than the provisioner does over the same entries.
The set of fields the manifest carries SHALL be held against the set the provisioner's own manifest builder emits, so that a field the provisioner adds is a failing check rather than a failing activation.

#### Scenario: A malformed manifest does not reach a machine

- **WHEN** the manifest derivation is built
- **THEN** the installer binary checks it as part of that build, in the mode the provisioner's validation setting selects
- **AND** the mode is not fixed to the weaker of the two, which validates the schema alone and never reads the ciphertext
- **AND** a manifest it rejects fails the build

#### Scenario: The provisioner grows a field

- **WHEN** the provisioner's manifest builder emits a field this package's does not
- **THEN** a check comparing the two field sets fails
- **AND** it fails on the commit that moves the provisioner's pin, not on a host

### Requirement: The installer runs after the stores it is told to wait for

The installer SHALL be registered under a name of this package's own, so that an ordering against another component's installer is expressible.
It SHALL accept, and honour, a consumer-named list of activation steps and a consumer-named list of units to run after, in whichever of the two activation mechanisms the host uses.

#### Scenario: The entry has a name of its own

- **WHEN** the system configuration's activation steps are enumerated
- **THEN** this package's installer is a step of its own, not a contribution to a step another component also defines
- **AND** the reason is recorded: two definitions of one step name merge into a single node, and a single node has no edge to state

#### Scenario: The ordering the consumer named is honoured

- **WHEN** a consumer names an activation step or a unit to run after
- **THEN** this package's installer declares that dependency in the mechanism the host is using
- **AND** naming nothing is a supported configuration that leaves the installer unordered

#### Scenario: The mechanism follows the host, not a switch of the provisioner's

- **WHEN** the host manages users through the systemd mechanisms that move secret installation into a unit
- **THEN** this package installs a unit and expresses its ordering as unit ordering
- **WHEN** the host does not
- **THEN** this package installs an activation step and expresses its ordering as a step dependency
- **AND** the selection is made from the host's own options rather than from the provisioner's installer switch, which now governs an installer this package does not use

### Requirement: The system-scope identity is derived by this package, excluding only its own store

Where the consumer names no identity, the system-scope module SHALL derive one from the host's ssh host keys, excluding the keys that lie inside this package's own secret store and no others.

#### Scenario: A host key another store deployed is usable

- **WHEN** the host's ssh host keys lie inside a secret store this package does not own
- **THEN** they are derived as this package's identity
- **AND** the reason is recorded: the exclusion exists to avoid decrypting with a key this installer itself deploys, which is a statement about this package's store and not about the path any store happens to use

#### Scenario: A host key this package deployed is not usable

- **WHEN** a host key lies inside this package's own secret store
- **THEN** it is excluded from the derived identity

#### Scenario: Derivation is a named switch

- **WHEN** a consumer turns the derivation off
- **THEN** only the identity the consumer named is used
- **AND** an identity the consumer named is never replaced by a derived one

### Requirement: No usable identity is refused in this package's own words

Where the resolved set is non-empty and no identity is configured or derivable, evaluation SHALL fail with this package's own message, naming this package's identity options.
Before decrypting, the installer SHALL check each configured identity path for presence and readability and refuse, naming the paths, the ordering options, and that a store which has not yet run is the usual cause.

#### Scenario: Nothing to decrypt with, refused at evaluation

- **WHEN** a system configuration resolves entries and no identity is configured or derivable
- **THEN** evaluation fails naming this package's identity options
- **AND** the reason it cannot be left to the provisioner is recorded: the provisioner's key-source assertion sits inside a condition on its own secrets option, which this package now leaves empty, so nothing would refuse at all

#### Scenario: The identity is not there yet, refused before decryption

- **WHEN** the installer runs and a configured identity path is absent or unreadable
- **THEN** it stops before decrypting, naming each path and how it failed, and names the ordering options as the remedy
- **AND** it states that a foreign store which has not run yet is the usual cause

#### Scenario: The limit of the activation check is stated

- **WHEN** the installer's identity refusal is reported
- **THEN** it states that presence and readability were checked and decryption was not

### Requirement: Another component's secret store is left exactly as it was

The installer SHALL NOT remove, replace, mount over, or write into a store it did not create.

#### Scenario: The store this package did not create survives

- **WHEN** the installer runs on a host where another component owns a secret store
- **THEN** that store's directory, its mount, and its contents are unchanged afterwards
- **AND** this is held against the installer binary itself rather than only against an evaluation, because the removal it exists to avoid is a runtime branch

#### Scenario: The destructive branch is real and is what is being avoided

- **WHEN** the installer binary is pointed at a symlink path that exists and is not a symlink
- **THEN** it removes what it finds there
- **AND** a check demonstrates this, so the claim that this package's store is disjoint is held against a measured hazard rather than an assumed one

#### Scenario: What is not claimed

- **WHEN** the coexistence this change establishes is documented
- **THEN** it states that it covers this package's own installer
- **AND** that a consumer writing the provisioner's secrets option directly, on such a host, still collides
