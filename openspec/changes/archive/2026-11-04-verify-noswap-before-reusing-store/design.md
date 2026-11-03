# Design

## Context

See proposal.md for the defect.
`mount_store` (`crates/safix-core/src/install.rs:1250-1285`) makes the mount point directory, then calls `store_already_mounted(use_tmpfs, filesystem_magic(mount_point))` and mounts only when that is false.
`filesystem_magic` is `statfs(2)`, whose `f_type` is the type of the filesystem holding the path, so an unmounted directory under a tmpfs `/run` reports `TMPFS_MAGIC`.
`harden-installer-ordering` D-e made a tmpfs request accept either magic so a ramfs fallback store is not mounted over, and recorded that a ramfs request accepts only ramfs because `statfs` does not report mount options.
That reasoning holds; the input it worked from does not.

Linux publishes each mount of the calling process's namespace in `/proc/self/mountinfo` (proc(5)), one per line:
`36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw,errors=continue`.
Field 5 is the mount point, with space, tab, newline and backslash written as `\ooo` octal escapes.
After the optional fields comes a lone `-`, then the filesystem type, the source, and the super-block options.
tmpfs writes `noswap` among its super-block options when it was mounted with it (kernel 6.4 and later, which is also when the option was introduced).

## Goals / Non-Goals

**Goals:**

- Reuse a store only when the mount point is itself a mount that the store requirement allows.
- Keep the reuse decision a pure function, unit-tested without a mount.
- Observe the mounted store's type and options on a booted VM, for both requests.

**Non-Goals:**

- Unmounting or migrating a covered store; the stacked-mount consequence is accepted (D-c).
- Any change to the mount flags, the mode, or the `noswap` fallback from `harden-installer-ordering` D-e.
- The user-scope store, which is not a mount (`mount_store` returns before this step in user mode).

## Decisions

### D-a. Read the mount table, not the containing filesystem

`mount_store` canonicalizes the mount point after making it, reads `/proc/self/mountinfo`, and takes the last line whose decoded mount point equals the canonical path.
The last line is the topmost mount: the kernel lists a namespace's mounts in the order they were made, so a mount stacked on an earlier one at the same point follows it.
Canonicalizing is needed because the table lists resolved paths, and the configured mount point may sit under a symlinked parent.
The mount point is compared as bytes against an iterator decoding the `\ooo` escapes, so no line is copied; a backslash not followed by three octal digits is kept literally.

Alternatives: `statx(2)` with `STATX_ATTR_MOUNT_ROOT` answers "is this a mount" but not the options, so the table would still be needed for `noswap`.
`statvfs(2)` reports generic flags (`ST_NOSUID` and the like) and not filesystem-specific options.
Comparing the mount point's device with its parent's misses a bind mount of the same filesystem and still says nothing about options.

### D-b. The reuse rule

`store_already_mounted(use_tmpfs, entry)` takes the parsed entry (or none) and is true when:

- there is an entry at the mount point, and
- its type is `ramfs`, or its type is `tmpfs`, tmpfs was requested, and its super-block options contain `noswap` as a whole comma-separated option.

Every other case mounts: no entry (a directory on the containing filesystem), any other type, a tmpfs without `noswap`, and a tmpfs with `noswap` under a ramfs request.
The last case is decided to mount ramfs over it, consistent with the rule `harden-installer-ordering` D-e kept: a ramfs request accepts only ramfs.
Accepting it would be safe on swap grounds, but would make the observed store type depend on history rather than on the option, and the type change is already documented to take effect over a stacked mount (D-c).

### D-c. Stacked mounts are accepted

Mounting over an existing store hides its generations until reboot: they stay in memory, unreachable through the mount point.
The secrets symlink lives outside the mount point, so numbering continues from the covered generation, and pruning sees only the new mount's directories.
This already happened when turning `useTmpfs` off; it now also happens once on a host whose tmpfs store lacks `noswap`, and when a ramfs request meets a `noswap` tmpfs.
A fresh directory that was never mounted covers nothing but the secrets an earlier safix wrote onto the containing filesystem, which stay there until reboot clears `/run`.
Unmounting the old store first was rejected: services may hold open files in it, a lazy unmount would still leave the pages in memory, and the installer is not the place to decide when a running service loses its secrets.
A reboot clears the covered mounts; the `useTmpfs` description says so.

### D-d. An unreadable mount table is refused

On linux a failure to read `/proc/self/mountinfo` is `Error::FileUnreadable` naming it, before anything is mounted.
Treating it as "not mounted" would stack a store at every activation on a host without `/proc`; such a host has no working NixOS activation in any case.
Off linux there is no mount table to read and no memory-backed mount to make; the entry is none and `mount_memory_backed`'s existing refusal stands.

### D-e. Removal of the `statfs` helper

`filesystem_magic`, `TMPFS_MAGIC` and `RAMFS_MAGIC` in `install.rs` have no other caller and are removed.
`staging.rs` keeps its own copies, which test the filesystem holding a staging directory and are correct for that purpose.

### D-f. Checks

- Unit tests over `store_already_mounted` and the table lookup: not a mount point (an entry only for `/run`); ramfs; tmpfs with `noswap`; tmpfs without `noswap`; tmpfs with `noswap` under a ramfs request; a `noswap` substring that is not the whole option; a mount point with `\040` in its name; and the topmost of two stacked entries.
- VM (`modules/flake/checks/installer-vm.nix`): a node `tmpfsMachine` built from `machineFor` with `safix.installer.useTmpfs = true` and a fresh store asserts from `/proc/self/mountinfo` that the last entry at `/run/safix.d` is tmpfs with `noswap` among its super-block options.
  The three default nodes assert from the same table that `/run/safix.d` is its own ramfs mount.
  The pinned nixpkgs' default kernel is recorded in tasks.md against the 6.4 floor.

## Risks / Trade-offs

- [A host that ran a swappable tmpfs store gets a second mount over it at the next activation] → Accepted (D-c); the earlier generations are unreachable and freed at reboot.
- [A mount table line in an unforeseen shape] → A line that does not parse is skipped; if that is the store's line, a store is mounted over it, which is the safe direction.
- [A kernel older than 6.4 accepts no `noswap`] → Unchanged: the mount gets `EINVAL` and the ramfs fallback applies, and a ramfs store is reused.

## Migration Plan

No consumer action is required.
A `useTmpfs` host whose store was never mounted, or was mounted without `noswap`, gets a `noswap` store at the next activation and moves its secrets onto it.
