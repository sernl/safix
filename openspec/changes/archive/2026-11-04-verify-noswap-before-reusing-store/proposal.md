# Proposal

## Why

The installer decides whether the secret store is already mounted from `statfs` on the mount point, which names the filesystem that contains the path rather than a mount at the path, and says nothing about mount options.
With `safix.installer.useTmpfs` set, a fresh `/run/safix.d` on NixOS's tmpfs `/run` therefore looks mounted, so no store is mounted and secrets land on the swappable `/run`.
An older tmpfs store mounted without `noswap` is accepted the same way.
Both contradict the `secret-installation` requirement that the secret store is never swappable, which `harden-installer-ordering` added.

## What Changes

- The installer reads the process's mount table instead of `statfs`, and reuses the store only when the mount point is itself a mount and that mount is ramfs, or is tmpfs with `noswap` in its options while tmpfs is requested.
- Anything else is mounted over: ramfs when tmpfs is not requested, and tmpfs with `noswap` (with the existing ramfs fallback) when it is.
  This includes a fresh directory on a containing tmpfs, an older tmpfs store without `noswap`, and a tmpfs with `noswap` when ramfs is requested.
- A mount table that cannot be read is refused rather than guessed past.
- New checks: unit tests for the reuse decision and the mount-table reading, and VM assertions that a `useTmpfs` host's store is its own tmpfs mount with `noswap` and a default host's store is its own ramfs mount.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-installation`: the "secret store is never swappable" requirement's reuse scenario is narrowed to a store that is its own mount with the right type and options, with scenarios for a containing tmpfs, a swappable tmpfs store, and the live type change.

## Impact

- `crates/safix-core/src/install.rs`: `mount_store`, `store_already_mounted`, and the `statfs` helper and magic constants it alone used, with unit tests in the `tests` module.
- `modules/flake/checks/installer-vm.nix`: a `useTmpfs` node and a mount-table assertion on the default nodes.
- Docs: `docs/reference/profile-options.md` `useTmpfs`, and the `useTmpfs` option description in `modules/consume/installer.nix`.
- A host that already runs a swappable tmpfs store gets a `noswap` tmpfs mounted over it at the next activation; no consumer action is required.
- No manifest schema change, no option rename, no new dependency.
