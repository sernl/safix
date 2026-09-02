## MODIFIED Requirements

### Requirement: Plaintext staged during a run lives in a private directory on a memory-backed filesystem

Any plaintext the runtime materializes transiently during a run — during generation, during editing, or in the tarball a machine-provisioning transfer assembles before streaming it — SHALL be placed inside a directory created for that run with owner-only permissions, on a memory-backed filesystem, with owner-only permissions on every file in it.

#### Scenario: The directory is private

- **WHEN** the staging root is created
- **THEN** its mode permits the owner only
- **AND** every file created inside it permits the owner only

#### Scenario: The staging root is per-run

- **WHEN** two runs stage plaintext
- **THEN** each has its own root
- **AND** neither can observe the other's

#### Scenario: Nothing plaintext is placed outside it

- **WHEN** the paths at which plaintext exists during a run are enumerated
- **THEN** every one is inside that run's staging root

#### Scenario: A destination the operator named on the command line is not staging

- **WHEN** a command writes plaintext to a location the operator supplied as its own argument, such as machine-provisioning's `--directory DIR`
- **THEN** that location is the run's deliverable rather than its staging root, and this requirement does not apply to it
- **AND** the distinction is stated wherever such a destination is documented, so the memory-backed rule is never read as reaching a disk-resident output the operator asked for
