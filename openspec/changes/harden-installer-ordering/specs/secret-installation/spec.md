# Spec Delta

## ADDED Requirements

### Requirement: Installed secrets precede ordinary services

Where the host installs through units, both installer units SHALL be ordered before `sysinit.target`, so a service with default dependencies starts only after the installer unit has finished.
Being wanted by `sysinit.target` SHALL NOT be relied on for ordering, because a wanting edge pulls a unit in without ordering it.

#### Scenario: A service with default dependencies sees its secret at start

- **WHEN** a host that installs through units boots with a oneshot service that has default dependencies, names no ordering against the installer, and tests for an installed secret at start
- **THEN** that service is active, because the secret existed when it started

#### Scenario: The ordering is held structurally as well

- **WHEN** a system configuration that installs through units is evaluated
- **THEN** each installer unit declares `sysinit.target` among the units it runs before
- **AND** on a booted host `sysinit.target` lists the normal installer unit among the units it runs after, because the observed start of one service is timing-dependent and does not on its own prove the ordering

### Requirement: Installer units stop at shutdown without an ordering cycle

Both installer units SHALL conflict with `shutdown.target` and be ordered before it.
They SHALL be ordered after the mounts holding configured identity paths without requiring those mounts, so stopping a key mount at shutdown cannot pull an installer unit or the user-creation units it precedes into a dependency loop.

#### Scenario: Both units carry the shutdown edges

- **WHEN** a system configuration that installs through units and declares an early entry is evaluated
- **THEN** both installer units conflict with `shutdown.target` and are ordered before it
- **AND** both order after the identity mounts through a wanting edge, and neither carries a requiring mount edge

#### Scenario: A key on an initrd mount survives a reboot cleanly

- **WHEN** a host with userborn and an early entry keeps its identity on a mount needed for boot, boots, shuts down and boots again
- **THEN** the early secret is installed on both boots and the user created from it has its password
- **AND** the previous boot's service-manager journal records a reached system shutdown and no line reporting an ordering cycle

### Requirement: The secret store is never swappable

The system-scope store SHALL be ramfs by default, and tmpfs mounted with `noswap` where the consumer asks for tmpfs.
Where the kernel rejects `noswap` as an invalid argument, the installer SHALL mount ramfs instead, say so, and SHALL NOT mount a tmpfs without `noswap`.
Any other mount failure SHALL be refused, naming the filesystem and the path.

#### Scenario: A kernel without noswap gets ramfs

- **WHEN** tmpfs is requested and the `noswap` mount fails with an invalid-argument error
- **THEN** the installer mounts ramfs at the same mount point with the same flags and mode
- **AND** its commentary names the mount point and states that ramfs was mounted because `noswap` was refused
- **AND** no mount without `noswap` is attempted

#### Scenario: Other mount failures are refused

- **WHEN** the tmpfs mount fails with any error other than an invalid argument, or the ramfs fallback itself fails
- **THEN** the installer refuses with the mount-failed error, naming the filesystem it last attempted and the path
- **AND** nothing is decrypted

#### Scenario: A fallback store is not mounted over on the next activation

- **WHEN** tmpfs is requested and the mount point already holds a ramfs or a tmpfs
- **THEN** no filesystem is mounted, and the existing store is used

#### Scenario: A live change of filesystem type is documented

- **WHEN** a consumer reads the description of the tmpfs option
- **THEN** it states that turning the option on for a running host keeps the existing ramfs until reboot
- **AND** that turning it off mounts ramfs over the existing tmpfs, whose earlier generations stay in memory, unreachable, until reboot

## MODIFIED Requirements

### Requirement: The installer runs after the stores it is told to wait for

The installer SHALL be registered under a name of this package's own, so that an ordering against another component's installer is expressible.
It SHALL accept, and honour, a consumer-named list of activation steps and a consumer-named list of units to run after, in whichever of the two activation mechanisms the host uses.
A unit named in that list SHALL itself run before `sysinit.target`, and a named unit the configuration defines with default dependencies SHALL be refused at evaluation.

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

#### Scenario: A named unit that runs after sysinit is refused

- **WHEN** a consumer names a unit to run after, and the configuration defines that unit with default dependencies left on
- **THEN** evaluation fails with this package's own message naming the unit, stating that the installer runs before `sysinit.target`, and that the ordering would be a cycle
- **AND** a named unit the configuration defines with default dependencies off, or does not define at all, is accepted

#### Scenario: The constraint is stated where the option is described

- **WHEN** a consumer reads the description of the units-to-run-after option
- **THEN** it states that each named unit must run before `sysinit.target`, and that only units this configuration defines are checked

### Requirement: The system-scope identity is derived by this package, excluding only its own store

Where the consumer names no identity, the system-scope module SHALL derive one from the host's ssh host keys, excluding the keys that lie inside this package's own secret store and no others.
The host keys SHALL count as present when the host runs its ssh daemon or, where the host's option set has it, when it generates ssh host keys without running the daemon.

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

#### Scenario: Generated host keys without the daemon are usable

- **WHEN** a host does not run its ssh daemon but generates its ssh host keys
- **THEN** its eligible host keys are derived as this package's identity
- **AND** a host that does neither derives nothing
- **AND** a host whose option set has no host-key generation switch still evaluates, deriving from the daemon switch alone
