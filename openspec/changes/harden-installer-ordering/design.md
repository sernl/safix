# Design

## Context

See proposal.md for motivation; evidence is `docs/notes/research/upstream-survey-2026-11.md`, sops-nix §3.1–3.4 and agenix §3.3.

The system installer has two units built from one `unitBase` (`modules/consume/installer.nix:58-78`).
`unitBase` sets `DefaultDependencies = "no"` and `RequiresMountsFor = identityMounts` (`:74-77`), where `identityMounts` is the SSH identity paths, the key file and the GnuPG home (`:54-57`).
`safix-install-users-secrets` (`:189-211`) is wanted by `sysinit.target` and `sysinit-reactivation.target`, required by and ordered before `systemd-sysusers.service`, `userborn.service` and `sysinit-reactivation.target`, and ordered after `local-fs.target`.
`safix-install-secrets` (`:212-228`) is wanted by `sysinit.target`, required by and ordered before `sysinit-reactivation.target`, and ordered after `local-fs.target`, the user-creation units, the early unit and `cfg.installer.afterUnits`.
Neither unit has a `shutdown.target` edge, and with default dependencies off neither receives the implicit one.

In the pinned nixpkgs (systemd 261.1), `userborn.service` has `DefaultDependencies = false`, is ordered before `sysinit.target` and `shutdown.target`, conflicts with `shutdown.target`, and is aliased as `systemd-sysusers.service` (`nixos/modules/services/system/userborn.nix:144-173`).
`sshd-keygen.service` exists when `services.openssh.generateHostKeys` is on, and that option defaults to `services.openssh.enable` (`nixos/modules/services/networking/ssh/sshd.nix:405-408`, `:986-991`).

The mount step is `mount_store` (`crates/safix-core/src/install.rs:1250-1277`), which mounts when `statfs` at the mount point does not report the requested filesystem's magic, and `mount_memory_backed` (`:1292-1326`), which on `EINVAL` from `noswap,mode=0751` retries tmpfs with `mode=0751` alone.

## Goals / Non-Goals

**Goals:**

- Make the unit edges state what the units are for: done before ordinary services, stopped at shutdown, never bound so tightly to a key mount that shutdown loops.
- Keep every mount the installer makes non-swappable.

**Non-Goals:**

- No change to the legacy activation-script path's ordering; it already runs in stage 2 before systemd starts services.
- No change to `afterActivation` or to which units the early unit precedes.
- No `sysinit-reactivation.target` change; both units already order before it.
- No change to the default store filesystem, which stays ramfs.

## Decisions

### D-a. Shutdown edges on both units

`safix-install-secrets` gains `before = [ "sysinit.target" "shutdown.target" ]` beside its existing `sysinit-reactivation.target`, and `conflicts = [ "shutdown.target" ]`, matching sops-nix 166fc0d and a8be3b1.

`safix-install-users-secrets` gets the same three edges.
Reasons:

- It is the same kind of unit: a oneshot with `RemainAfterExit`, default dependencies off, wanted by `sysinit.target`.
  Without the conflict it stays active until the final kill, which is the state the main unit is being moved out of.
- The units it is required by and ordered before, `userborn.service` and `systemd-sysusers.service`, conflict with `shutdown.target` and are ordered before it themselves; the early unit is the one unit in that chain that does not.
- An explicit `before sysinit.target` makes the early unit's position independent of whether the host enables userborn or sysusers.
  `useSystemdActivation` can be set to true on a host with neither, and the early unit is then otherwise unordered against `sysinit.target`.
- Neither unit has an `ExecStop`, so the stop job deactivates the unit and removes nothing; secrets stay in place until the store is unmounted.

Upstream added these edges only to its main unit (survey sops-nix §5 Q1).
The early unit's coupling is tighter than upstream's (`requiredBy` rather than `wantedBy` the user-creation unit), which is a reason to give it the same shutdown behaviour as the units it is coupled to rather than leave it out.

### D-b. `WantsMountsFor` on both units

Both units use `WantsMountsFor = identityMounts`, and `RequiresMountsFor` is removed from `unitBase`.
`unitBase` therefore stays shared and is not split; the shutdown and `sysinit.target` edges, which are also shared, move into `unitBase`.

The early unit's change follows sops-nix e80b8b7 (#962).
For the main unit, sops-nix kept `RequiresMountsFor` (`modules/sops/default.nix:496`); safix does not, for these reasons:

- `WantsMountsFor=` adds the same `After=` on the mount units as `RequiresMountsFor=`; only the requirement edge differs, so the main unit still starts after its key mounts.
- After this change the main unit has the same shape as the early unit that looped in #962: default dependencies off, before `sysinit.target`, conflicting with `shutdown.target`, with a requiring edge on a mount stopped at shutdown.
  The reason for weakening the early unit's edge applies to it as well.
- When a key mount fails, a requiring edge fails the unit with a systemd dependency error and safix's own message never runs.
  With a wanting edge the unit starts, and the identity preflight refuses, naming each identity path and how it failed (`common.identityPreflight`, `modules/consume/installer.nix:42-47`).
  The outcome for the boot is the same: the installer unit fails and no secrets are installed.

`WantsMountsFor=` needs systemd 256; the pinned nixpkgs ships 261.1 and safix's flake follows nixos-unstable.

### D-c. `afterUnits` cycle: description plus a partial assertion

Once the main unit is before `sysinit.target`, any unit in `afterUnits` that is itself after `sysinit.target` forms an ordering cycle; a unit with default dependencies is.
systemd breaks such a cycle by dropping a job at boot, which is silent at evaluation.

Two parts:

- The `afterUnits` option description (`installer.nix:100-104`) and `docs/reference/profile-options.md:434-439` state that each named unit must run before `sysinit.target`, which for a service means default dependencies off, and that only units this configuration defines are checked.
- A new entry in `config.assertions` refuses each name in `afterUnits` that ends in `.service`, is defined in `config.systemd.services`, and has default dependencies on.
  Default dependencies count as off when `unitConfig.DefaultDependencies` is `false`, `"no"`, `"false"` or `"0"`; absent counts as on.
  The message names the unit, states that `safix-install-secrets` runs before `sysinit.target`, and names `DefaultDependencies=no` as the remedy.

Units not defined through `systemd.services` (packaged unit files, other unit types) cannot be inspected at evaluation and are not refused; the description says so.
The existing fixture name `age-decrypt-secrets.service` is not defined in its fixture and stays accepted.

Alternatives considered: documentation only, which leaves the defined-unit case to a silent boot-time cycle; refusing every name not provably early, which would refuse the undefined units that are the option's main use.

### D-d. `generateHostKeys`

`derivedHostKeys` (`installer.nix:116-135`) replaces `config.services.openssh.enable` with `config.services.openssh.enable || (options.services.openssh ? generateHostKeys && config.services.openssh.generateHostKeys)`.
The `enable` clause stays, because a host can run sshd with `generateHostKeys = false` and keys placed by another store, which is the clan shape the existing `foreignKeys` fixture covers.
The guard keeps evaluation working on a nixpkgs without the option; the pinned nixpkgs has it, so no check exercises the guard's false branch.

First-boot ordering is unchanged: `sshd-keygen.service` has default dependencies and runs after `sysinit.target`, so on a first boot the generated key does not exist when the installer runs.
That was already true with `services.openssh.enable`, and a key generated on the host's first boot cannot have been a recipient of the documents yet.
The identity preflight reports the absent path in that case.

### D-e. Ramfs fallback (project decision D1)

On `EINVAL` from the tmpfs `noswap` mount, `mount_memory_backed` mounts ramfs with the same flags and `mode=0751`, and never mounts tmpfs without `noswap`.
Any other errno from the tmpfs mount, and any error from the ramfs fallback, is refused as `Error::InstallMountFailed`, naming the filesystem last attempted.
The fallback writes one commentary line through the `Progress` sink already passed to `run` (`install.rs:360`), using `progress::note`, naming the mount point and stating that `noswap` was refused and ramfs was mounted; `mount_store` takes the sink as a new parameter.

Testability: the syscall is put behind a seam.
A private function takes the mount call as `impl FnMut(&str, &CStr) -> rustix::io::Result<()>` (filesystem, data) and returns the filesystem mounted; `mount_memory_backed` passes the real `rustix::mount::mount` closure.
Unit tests drive it with scripted errno sequences and record each attempt.

Re-activation: after a fallback, the next run with `use_tmpfs` would see `RAMFS_MAGIC` where it expects `TMPFS_MAGIC` and, under the current check at `install.rs:1272`, mount again on top.
To prevent that, a tmpfs request accepts an existing mount reporting either magic; a ramfs request still accepts only ramfs.
Extracted as a pure function of the request and the observed magic, so it is unit-tested without a mount.

Live type change: with that rule, turning `useTmpfs` on for a running host keeps the existing ramfs until reboot.
Turning it off mounts ramfs over the existing tmpfs, as today, and the earlier generations stay in memory under the covered mount until reboot.
A ramfs request does not accept an existing tmpfs, because a tmpfs mounted by an earlier safix's fallback may lack `noswap`, and `statfs` does not report mount options.
This is documented on `useTmpfs`, with a reboot as the remedy; agenix's tmpfs change has the same hazard in the other direction (survey agenix B8).

### D-f. Checks

- Structural: `orderingFacts.actual.unit` (`modules/flake/checks/installer.nix:671-680`) gains `beforeSysinit`, `beforeShutdown`, `conflictsShutdown`, `wantsIdentityMounts` and `requiresNoMounts` beside `wantedBySysinit`, read off `orderedUnitFixture`.
  A new fixture with one `neededForUsers` entry through `safix.importedSecrets` and a store `sopsFile` written with `pkgs.writeText` (the pattern at `:295`) supplies the same facts for `safix-install-users-secrets` under a new `earlyUnit` key.
  A fixture defining a default-dependency service and naming it in `afterUnits`, and one defining it with default dependencies off, supply `afterUnitsCycle` facts read from the failed `config.assertions` messages.
- VM (`modules/flake/checks/installer-vm.nix`): every node gains a `safix-consumer` oneshot, wanted by `multi-user.target`, with default dependencies and no ordering against safix, whose script tests for `/run/safix/shared-token`.
  On the unit-mode nodes the script also asserts `sysinit.target`'s `After` lists `safix-install-secrets.service`, and both units' `Before` and `Conflicts` hold `shutdown.target`.
- VM key-mount node: a fourth node `userbornKeyMount` mirrors sops-nix `checks/nixos-test.nix:635-693` (`user-passwords-userborn-key-mount`): systemd initrd, userborn, `users.mutableUsers = false`, the age identity on a `neededForBoot` bind mount from a store directory, `safix.identity.keyFile` forced to the mounted path, and the `account-password` early entry backing a user's `hashedPasswordFile`.
  After the first boot it asserts the password and that the early unit's `After` names the key mount; after `shutdown()` and `start()` it asserts the password again and reads `journalctl -b -1 --no-pager -o cat _PID=1` for `Reached target System Shutdown` and the absence of `ordering cycle`.
  It is excluded from the shared loop at `:181`, whose rotation steps it does not need.
- Identity: `identityManifests` gains `generatedOnly` (sshd disabled with `lib.mkForce false`, `generateHostKeys = true`, `foreignHostKeys`), and the check reads `derivedHostKeys` of a `neither` fixture.
- Rust: unit tests in `crates/safix-core/src/install.rs`'s `tests` module, run by `safix-rs-test` (`modules/flake/rust.nix:118-124`, `--lib --bins`).

## Risks / Trade-offs

- [The reboot node may not reproduce #962 with `RequiresMountsFor` restored, so it may guard the shape without demonstrating the failure] → The severity drill in tasks.md records what it observed; if the drill stays green the check comment and this design say so rather than claiming a reproduction.
- [An identity on a filesystem mounted after `sysinit.target`, such as a network filesystem, now forms an ordering cycle] → Stated in the `afterUnits` and `useSystemdActivation` docs as part of the before-`sysinit.target` constraint; the reboot node's ordering-cycle assertion covers the local case.
- [The `afterUnits` assertion inspects only units defined through `systemd.services`] → Stated in the option description.
- [A container with `useTmpfs` that previously ran on a swappable tmpfs keeps that mount until reboot, because a tmpfs request accepts an existing tmpfs] → Documented on `useTmpfs`; a reboot of the container remounts as ramfs.
- [The `EINVAL` errno in an unprivileged user namespace is inferred, not observed (survey agenix §5 Q2)] → The fallback is keyed to `EINVAL` as the existing code is; a different errno is refused rather than silently weakened.
- [The VM test grows by one node and a reboot] → Accepted; it is the only check that observes shutdown.

## Migration Plan

No consumer action is required.
Hosts installing through units pick up the new edges on their next switch; the shutdown edges take effect at the next shutdown.
Hosts with `useTmpfs` on a kernel that accepts `noswap` see no change.
A consumer whose `afterUnits` names a default-dependency service defined in their configuration gets an evaluation failure naming it; the remedy is to set that unit's `DefaultDependencies=no` or remove it from `afterUnits`.
Rollback is reverting the change; no state outlives a reboot.
