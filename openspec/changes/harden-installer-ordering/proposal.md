# Proposal

## Why

The system installer's systemd units carry ordering that sops-nix has since corrected (`docs/notes/research/upstream-survey-2026-11.md`, sops-nix §3.1–3.3).
`safix-install-secrets` is wanted by `sysinit.target` but not ordered before it, so a service with default dependencies can start before its secrets exist.
Neither installer unit stops at shutdown, and both require, rather than want, the mounts that hold the identity, which is the shape of the shutdown ordering loop reported against sops-nix with userborn and an initrd key mount.
Two smaller gaps sit beside these.
The derived host identity ignores hosts that generate SSH host keys without running sshd (sops-nix §3.4).
With `safix.installer.useTmpfs` set, a kernel that refuses `noswap` gets a swappable tmpfs store, where agenix falls back to ramfs (agenix §3.3).

## What Changes

- `safix-install-secrets` is ordered before `sysinit.target` and before `shutdown.target`, and conflicts with `shutdown.target`.
- `safix-install-users-secrets` gets the same `sysinit.target` and `shutdown.target` edges.
- Both units order after the mounts holding configured identity paths through `WantsMountsFor=` instead of `RequiresMountsFor=`.
- `safix.installer.afterUnits` states that a named unit must itself run before `sysinit.target`, and evaluation refuses a named unit this configuration defines with default dependencies.
- `safix.identity.deriveHostKeys` also derives from `services.openssh.hostKeys` when `services.openssh.generateHostKeys` is on, where the host's nixpkgs has that option.
- **BREAKING** (narrow): with `useTmpfs`, a kernel that rejects `noswap` with `EINVAL` now gets a ramfs store instead of a swappable tmpfs; any other mount error is still refused.
  An existing ramfs at the mount point satisfies a tmpfs request, so a fallback store is not mounted over on the next activation.
- New checks: structural assertions for the new unit edges and the `afterUnits` refusal, a VM consumer that must see its secret at start, a VM reboot node with userborn and a `neededForBoot` key mount that must log no ordering cycle, an identity fixture for `generateHostKeys`, and unit tests for the mount fallback.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-installation`: adds requirements for ordering before ordinary services, stopping at shutdown without ordering cycles, and a store that is never swappable; extends the `afterUnits` ordering requirement with its cycle constraint and the host-key derivation requirement with `generateHostKeys`.

## Impact

- `modules/consume/installer.nix`: `unitBase` (`:58-78`), both unit definitions (`:189-228`), the `afterUnits` option description (`:100-104`), a new assertion in `config.assertions` (`:141-151`), and `derivedHostKeys` (`:112-138`).
- `crates/safix-core/src/install.rs`: `mount_store` (`:1250-1277`) and `mount_memory_backed` (`:1292-1326`), with unit tests in the `tests` module (`:1640-`).
- `modules/flake/checks/installer.nix`: `orderingFacts` (`:626-761`), `identityFixture` and `identityManifests` (`:770-842`), `safix-installer-identity` (`:1486-1518`).
- `modules/flake/checks/installer-vm.nix`: node set (`:160`) and test script (`:179-232`).
- Docs: `docs/tutorials/first-secret.md:134-138`, `docs/reference/profile-options.md` (`deriveHostKeys` `:281-283`, `useTmpfs` `:413-418`, `useSystemdActivation` `:420-425`, `afterUnits` `:434-439`), `docs/guides/templates-and-services.md:100-102`.
- Requires systemd 256 or later for `WantsMountsFor=`; the pinned nixpkgs ships systemd 261.1.
- No manifest schema change, no option rename, no new dependency.
