# Spec Delta

## MODIFIED Requirements

### Requirement: The secret store is never swappable

The system-scope store SHALL be ramfs by default, and tmpfs mounted with `noswap` where the consumer asks for tmpfs.
Where the kernel rejects `noswap` as an invalid argument, the installer SHALL mount ramfs instead, say so, and SHALL NOT mount a tmpfs without `noswap`.
Any other mount failure SHALL be refused, naming the filesystem and the path.
An existing store SHALL be reused only when the mount point is itself a mount whose type and options satisfy this requirement, as read from the mount table, and otherwise a store SHALL be mounted.

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

- **WHEN** tmpfs is requested and the mount point is itself a ramfs mount, or a tmpfs mount with `noswap` among its options
- **THEN** no filesystem is mounted, and the existing store is used

#### Scenario: A ramfs request accepts only ramfs

- **WHEN** tmpfs is not requested and the mount point is itself a ramfs mount
- **THEN** no filesystem is mounted, and the existing store is used
- **AND** when the mount point is instead a tmpfs mount, with or without `noswap`, ramfs is mounted over it

#### Scenario: A directory on a containing tmpfs is not taken for a store

- **WHEN** the mount point is a directory that is not itself a mount, on a filesystem such as a tmpfs `/run`
- **THEN** the installer mounts the requested store at the mount point, whether or not tmpfs is requested

#### Scenario: A swappable tmpfs store is mounted over

- **WHEN** tmpfs is requested and the mount point is itself a tmpfs mount without `noswap` among its options
- **THEN** the installer mounts a tmpfs with `noswap`, or ramfs where `noswap` is refused, over it before writing any secret

#### Scenario: An unreadable mount table is refused

- **WHEN** the installer cannot read the mount table on a system-scope install
- **THEN** it refuses with the file-unreadable error naming the mount table
- **AND** nothing is mounted or decrypted

#### Scenario: A booted host's store is its own mount

- **WHEN** a host boots and installs with tmpfs requested on a kernel that accepts `noswap`
- **THEN** the mount table lists the store mount point as a mount of type tmpfs with `noswap` among its options
- **AND** on a host with the default, the mount table lists it as a mount of type ramfs

#### Scenario: A live change of filesystem type is documented

- **WHEN** a consumer reads the description of the tmpfs option
- **THEN** it states that turning the option on for a running host keeps the existing ramfs until reboot
- **AND** that turning it off mounts ramfs over the existing tmpfs, whose earlier generations stay in memory, unreachable, until reboot
- **AND** that a running tmpfs store without `noswap` gets a `noswap` store mounted over it, whose earlier generations stay in memory, unreachable, until reboot
