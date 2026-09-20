---
title: "safix documentation"
---

## What is here

This page is the map of the documentation. Pages are grouped by what you came
for: a lesson to work through, a task to get done, a model to understand, or an
exact value to look up.

Start with [Your first secret](tutorials/first-secret.md) if safix is new to
you, and with [The model](concepts/model.md) if you want the vocabulary before
the commands.

## Tutorials

- [Your first secret](tutorials/first-secret.md) — a plain NixOS flake to a
  service reading a decrypted value, then the same value in home-manager.

## Guides

- [Unattended hosts](guides/unattended-hosts.md) — machine identities against
  human ones, derived host keys, and the GnuPG caveats.
- [Inspectable recipients](guides/inspectable-recipients.md) — choosing SOPS
  over raw age where the recipient roster has to be auditable, and what
  `fix --yes` authorizes.
- [After removing access](guides/after-removing-access.md) — removing a member,
  checking what narrowed, and rotating the value.
- [Rotating secrets](guides/rotating-secrets.md) — rotation policies, the
  rotate verb and the workstation timer.
- [Generators](guides/generators.md) — scripts, prompts, dependencies,
  multi-output values, validation and the sandbox.
- [Templates and services](guides/templates-and-services.md) — templates,
  `restartUnits` and `reloadUnits`, and early-user secrets.
- [Migrating from sops-nix and agenix](guides/migrating-from-sops-nix-and-agenix.md)
  — a plan walked end to end, with recovery.
- [Hardware keys](guides/hardware-keys.md) — enrolling a token, uploading it,
  and the proof it carries.
- [Identity backup](guides/identity-backup.md) — minting, backing up and
  restoring an identity, and the independence rule.
- [Vaults](guides/vault.md) — opaque names, the two roots and the commit order.
- [Syncing to password managers](guides/syncing-password-managers.md) — the one
  synchronisation chapter, with a same-shaped section per target.
- [flake-parts](guides/flake-parts.md) — the flake module, the dendritic
  layout, and the worked example.

## Concepts

- [The model](concepts/model.md) — the three questions, how an audience becomes
  a file, and the namespace rule.
- [Custody](concepts/custody.md) — the subjects that can hold a key, recipients
  against identities, delegation, and the revocation rule.
- [Storage layout](concepts/storage-layout.md) — the three storage roots, the
  ignore and backup rules, and renaming a root.

## Reference

- [CLI](reference/cli.md) — one row per subcommand: what it does, what it
  reads, what it writes, and whether it needs a terminal.
- [Declarations](reference/declarations.md) — every option of the
  `flake.safix.*` namespace.
- [Profile options](reference/profile-options.md) — every `safix.*` option at
  both scopes.
- [Migration plan](reference/migration-plan.md) — the plan schema a migration
  reads.
- [Environment](reference/environment.md) — the `SAFIX_*` variables and the
  upstream ones safix honours.
- [Refusals](reference/refusals.md) — the stable refusal codes and what each
  one asks for.
