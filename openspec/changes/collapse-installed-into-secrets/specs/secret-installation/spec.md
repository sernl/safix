## MODIFIED Requirements

### Requirement: safix installs its resolved set itself at system scope

At system scope the package SHALL build its own installer manifest and invoke the secret provisioner's installer binary against it, and SHALL NOT deliver its resolved set through the provisioner's own secrets option.
The provisioner's installer SHALL therefore be inert with respect to anything safix resolved.
Exactly one option in this package's namespace SHALL carry what arrived, and it SHALL be the same option the manifest is built from, so that a consumer's reading and the installer's input cannot disagree.

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
- **THEN** one option in this package's namespace carries the resolved set, typed by the provisioner's own entry type and carrying the path each entry will arrive at
- **AND** it is the same option the installer manifest is built from, so what a consumer reads and what is installed cannot disagree
- **AND** it is read-only, because it is a projection of the declarations for this subject on this host at this scope and not a place to write
- **AND** the migration records that the provisioner's secrets option is no longer where it appears, and that a second option which previously carried the typed set is gone rather than aliased, so reading it fails naming it

#### Scenario: The single surface is carried at both scopes

- **WHEN** the same option is read on a user-scope profile
- **THEN** it carries that profile's resolved set in the shape the provisioner's option tree takes, as it does at system scope
- **AND** the typing each scope applies is the scope's own, so the user scope is unchanged by the system scope carrying the provisioner's entry type

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
- **AND** that check's expected paths are computed from the declarations' own resolution, reached independently of the option the manifest is built from, so that holding one option against itself cannot satisfy it

#### Scenario: A declared path is still honoured

- **WHEN** a resolved entry declares a path
- **THEN** that path is what the manifest carries
- **AND** the refusal of two entries resolving onto one path is unchanged
