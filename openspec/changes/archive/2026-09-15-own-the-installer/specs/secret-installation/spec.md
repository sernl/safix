## Purpose

This capability is rewritten end to end.
Every requirement in it was phrased around a secret provisioner safix borrows from — "the provisioner's installer binary", "the provisioner's own secret type", "the provisioner's own manifest builder" — and there is no longer a second component to borrow from: safix declares the entry type, defines the manifest schema, and ships the program that reads it.
The capability also widens from system scope alone to both scopes, because installation at user scope was previously somebody else's, which is why nothing here described it.

## REMOVED Requirements

### Requirement: safix installs its resolved set itself at system scope

**Reason**: The requirement's second scenario asserts that the resolved set has "passed through the provisioner's own secret type, read off the provisioner's option declaration rather than restated", and its third records that "the provisioner's secrets option is no longer where it appears".
Both are statements about a component this package no longer imports, and the first is the coupling this change exists to remove: an entry type read out of another framework's option declaration in the same evaluation.
The requirement is replaced, under a name that says whose program installs, by the ADDED requirement "safix installs its resolved set with its own program at system scope" below.

**Migration**: No consumer action beyond the rename table.
The installer's shape, its name, its ordering options and its store are unchanged; what changes is the program it invokes and the namespace the values come from.

## MODIFIED Requirements

### Requirement: The secret store is this package's own and is named nowhere else

The manifest SHALL name a secrets mount point and a symlink path belonging to this package, distinct from any store another component of the host owns, at both scopes.
No entry SHALL be written, symlinked, or removed outside that store except at a path a declaration states.

#### Scenario: The roots are this package's

- **WHEN** a built manifest is read, at either scope
- **THEN** its mount point and its symlink path are this package's own
- **AND** neither is a path any other secret-installation component defaults to

#### Scenario: An entry that declares no path parks in this package's store

- **WHEN** a resolved entry declares no path of its own
- **THEN** the entry type's own default gives it a path inside this package's symlink path
- **AND** the store root and that default are held together by one check against an oracle independent of the default itself, because a root moved without the default turns a collision into a silent write into another store's directory

#### Scenario: A declared path is still honoured

- **WHEN** a resolved entry declares a path
- **THEN** that path is what the manifest carries
- **AND** the refusal of two entries resolving onto one path is unchanged

### Requirement: The manifest is validated by the binary that will read it

The manifest SHALL be checked at build time by this package's own program, in one of two modes: a schema mode that reads no ciphertext, and a document mode that additionally decrypts each named document and verifies each declared key resolves in it.
A named option SHALL select between them, and the manifest's structure SHALL be held by an accepted snapshot so that a field added, removed or renamed on either side of the nix-to-program boundary is a failing check.

#### Scenario: A malformed manifest does not reach a machine

- **WHEN** the manifest derivation is built
- **THEN** this package's own program checks it as part of that build
- **AND** the mode is the document mode unless the consumer turned validation off, so the weaker mode that never reads ciphertext is not the default
- **AND** a manifest it rejects fails the build

#### Scenario: The schema is held on both sides of the boundary

- **WHEN** the manifest's structure changes on the nix side without the program's own definition of it changing, or the reverse
- **THEN** a check comparing the built manifest against an accepted snapshot fails
- **AND** it fails on the commit that makes the change, not on a host

#### Scenario: A mutated manifest is refused by name

- **WHEN** the program is handed a manifest carrying an unknown schema version, an unparsable mode, a key absent from its document, or an unknown field
- **THEN** it refuses, naming the field and what is wrong with it
- **AND** each of those mutations is exercised by a check, so the acceptance of the real manifest is evidence rather than coincidence

#### Scenario: The checking binary is built for the platform that runs it

- **WHEN** the package is cross-built
- **THEN** the build-time check uses a build-platform build of the program and the activation uses a host-platform one
- **AND** the two are separate named options, so a consumer cross-building through a non-default path can redirect either

#### Scenario: The provisioner grows a field

- **WHEN** another secret-installation framework's own manifest builder grows a field this package's manifest does not carry
- **THEN** nothing in this package changes and no check fails, because this package's manifest is its own and is validated against its own schema
- **AND** the retirement of the field-set parity obligation is recorded, because that obligation — and the check that discharged it — was the reason this scenario previously asserted the opposite

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
- **AND** the selection is made from the host's own options alone, and no switch belonging to another installer is consulted

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
Sufficiency SHALL count only this package's own identity sources, and no configuration belonging to another framework SHALL suppress the refusal.
Before decrypting, the installer SHALL check each configured identity path for presence and readability and refuse, naming the paths, the ordering options, and that a store which has not yet run is the usual cause.

#### Scenario: Nothing to decrypt with, refused at evaluation

- **WHEN** a system configuration resolves entries and no identity is configured or derivable
- **THEN** evaluation fails naming this package's identity options
- **AND** the two options it names are a key file and a list of ssh keys, and nothing else counts as an identity

#### Scenario: Another framework's key configuration does not count

- **WHEN** a consumer's tree configures a key source belonging to another secret-management framework and names no identity of this package's
- **THEN** evaluation still fails, because that configuration is not an identity this package can decrypt with
- **AND** the change is recorded, because such a configuration previously suppressed this refusal

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
- **AND** this is held against this package's own installer binary rather than only against an evaluation, because the removal it exists to avoid is a runtime branch

#### Scenario: The destructive branch is real and is what is being avoided

- **WHEN** the installer binary is pointed at a symlink path that exists and is not a symlink
- **THEN** it removes what it finds there
- **AND** a check demonstrates this against the binary this package ships, so the claim that this package's store is disjoint is held against a measured hazard rather than an assumed one

#### Scenario: What is not claimed

- **WHEN** the coexistence this capability establishes is documented
- **THEN** it states that it covers this package's own installer
- **AND** that a consumer writing another framework's secrets option directly, on such a host, still collides with that framework's own store

## ADDED Requirements

### Requirement: safix installs its resolved set with its own program at system scope

At system scope the package SHALL build its own installer manifest and invoke its own installer, published as a subcommand of its own command-line program, against it.
It SHALL NOT read, define, or depend on any option belonging to another secret-management framework in order to do so.

#### Scenario: One installer, and it is this package's own program

- **WHEN** a system configuration establishes a non-empty resolved set
- **THEN** the configuration carries an activation invocation of this package's own program, naming a manifest this package built
- **AND** no option outside this package's own namespace and the host's own activation, unit and ssh options is read to construct either

#### Scenario: The resolved set is typed by this package

- **WHEN** an entry of the resolved set is read back before the manifest is built
- **THEN** it has passed through an entry type this package declares in its own files, carrying the entry's name, key, path, mode, owner, group, uid, gid, document, format and unit lists
- **AND** the refusals for a document that does not exist and a document outside the nix store are carried by this package's manifest builder, where they were before, because the type carries neither

#### Scenario: What a consumer reads to see what arrived

- **WHEN** a consumer inspects what this package established on a system configuration
- **THEN** one option of this package's namespace carries the resolved set, and it is the same option the manifest is built from
- **AND** the migration records which option of the other framework's namespace each setting a consumer may have tuned is renamed to

### Requirement: The manifest schema is this package's own, versioned, and defined once

The manifest SHALL be defined once, in the runtime library, and emitted by both scopes' modules against that one definition.
It SHALL carry a schema version, and a manifest naming a version the program does not know SHALL be refused naming both versions rather than decoded on a best-effort basis.
Unknown fields SHALL be refused rather than ignored.

#### Scenario: One definition, two emitters

- **WHEN** the system-scope and user-scope manifests are compared against the runtime library's own definition
- **THEN** both parse against it with no field left over and none missing
- **AND** the definition is the single place the schema is written down

#### Scenario: An unknown version is a diagnosis

- **WHEN** the program reads a manifest whose version it does not know
- **THEN** it refuses, naming the version it read and the version it supports
- **AND** the reason the field exists is recorded: a decoder that silently ignores what it does not understand turns a schema change into a wrong installation rather than a message

#### Scenario: A document edit still changes the derivation

- **WHEN** a ciphertext document a manifest names is edited
- **THEN** the manifest derivation's output changes, because the manifest carries a hash over the documents it names
- **AND** the installer ignores that field entirely, so its only purpose — making the derivation a function of the ciphertext — is stated rather than inferred

### Requirement: Installation is a subcommand of this package's own command

The installer SHALL be a verb of the package's command-line program, taking a manifest path, a check mode, a switch that skips user and group resolution, and a dry-run switch, and SHALL honour the host's own dry-activation signal as implying the last.

#### Scenario: The verb and its arguments

- **WHEN** the verb is invoked with a manifest path
- **THEN** it installs that manifest's entries
- **AND** its check mode selects between validating the schema alone, validating against each document, and not validating at all

#### Scenario: A dry activation performs everything but the swap

- **WHEN** the verb runs under the dry-run switch, or under the host's own dry-activation signal
- **THEN** every step up to the atomic replacement of the store's symlink is performed and that replacement is not
- **AND** the module's declaration that it supports dry activation is therefore true of the program rather than only of the script

#### Scenario: The verb is machine-facing and says so

- **WHEN** the command's help enumerates its verbs
- **THEN** this verb is listed with the others and described as the one an activation runs rather than one an operator types
- **AND** nothing about its presence changes what the operator-facing verbs do

### Requirement: The user scope installs rather than delegating

At user scope the package SHALL build a user-mode manifest and invoke its own installer against it, through a user service where the platform has one and through a home-activation entry on every platform.
The store roots SHALL be runtime-directory-relative, expanded by the installer against the platform's own runtime directory.

#### Scenario: The user scope carries an installer of its own

- **WHEN** a user profile establishes a non-empty resolved set
- **THEN** the profile carries this package's own manifest and an invocation of this package's own program against it
- **AND** no module belonging to another secret-management framework is imported or configured to make that happen

#### Scenario: The runtime directory is resolved per platform

- **WHEN** the installer expands the store roots at user scope
- **THEN** it uses the session's runtime directory on linux and the platform's own per-user temporary directory on darwin
- **AND** both are exercised, because the package supports a darwin platform and a user-scope install that only works on linux is a scope that only half exists

#### Scenario: What user mode does not do

- **WHEN** a user-mode installation runs
- **THEN** it mounts no filesystem, changes no file's ownership, and restarts or reloads no unit
- **AND** the reason is recorded: each of the three requires privileges the scope does not have, and each was already absent from the behaviour this installation replaces

#### Scenario: The user-scope path change is recorded

- **WHEN** a consumer migrating a user profile reads the migration note
- **THEN** it names the path a secret arrived at before and the path it arrives at now
- **AND** it states that the new location is a runtime directory, so a secret does not survive a reboot without a login

#### Scenario: A key file can be minted at activation

- **WHEN** a user profile turns on key generation and the configured key file is absent
- **THEN** the activation mints one before installing
- **AND** the switch defaults off, so adopting this capability changes no existing profile's behaviour

### Requirement: Templates, user-relocated secrets, and gnupg identities are unsupported and said to be

The capability SHALL state, rather than imply by omission, that it renders no templates, relocates no secret for early-boot user creation, and accepts no gnupg identity.

#### Scenario: The three absences are named

- **WHEN** the manifest schema and the identity options are documented
- **THEN** each of the three is named as unsupported
- **AND** none of them appears as an empty field, an ignored option, or a tolerated configuration

#### Scenario: What the consumer loses is the workaround, not a feature

- **WHEN** the migration note explains the three
- **THEN** it states that this package never supported any of them
- **AND** it states what does change: an entry this package resolved can no longer be reached by another framework's template or relocation, even on a host where that framework is also installed

### Requirement: A real activation is exercised on a booted host

The capability's central claim — that entries arrive, at their declared modes and owners, in generations that rotate and prune — SHALL be held by a test that boots a machine and reads the installed store, not only by evaluations and sandboxed invocations.

#### Scenario: A booted host holds its secrets

- **WHEN** a machine importing this package's system module and resolving entries boots
- **THEN** each entry's file exists inside the store at its declared mode and ownership
- **AND** the store's symlink names the first generation

#### Scenario: A second activation rotates and prunes

- **WHEN** the same machine activates again with a changed entry
- **THEN** a new generation directory is created, the symlink moves to it, and the count of retained generations obeys the configured limit
- **AND** a unit an entry names for restart is observed restarted
