---
title: "Vault"
---

## What you will have

This page shows you how to move every ciphertext document, public value and definition record into a second repository whose file names disclose nothing. At the end the declaring flake still computes both the readable names and the opaque ones, and the vault host sees only the opaque ones.

## Steps

1. Take the second repository as a non-flake input and declare it:

   ```nix
   {
     flake.safix.vault = {
       root = inputs.vault;
       namingKey = "<64 or more lowercase hexadecimal characters>";
     };
   }
   ```

   See [`flake.safix.vault.root`](../reference/declarations.md#flakesafixvaultroot) and [`flake.safix.vault.namingKey`](../reference/declarations.md#flakesafixvaultnamingkey).

2. Point the command at your own working tree of that repository, which is what it writes and commits into:

   ```console
   $ export SAFIX_VAULT_ROOT=/home/alice/src/vault
   ```

   The variable is unset and unread while no vault is declared, and required whenever one is. See [Environment](../reference/environment.md).

3. Move what is still at the declaration root:

   ```console
   $ safix fix
   ```

   Each readable-layout document, public output and definition record is decrypted under your own identity, re-encrypted into its opaque destination, and then removed from the declaration root. A destination that is already present is left alone, so an interrupted run resumes where it stopped.

4. Commit in the order the run's closing note names, and update the pin afterwards:

   ```console
   $ nix flake lock --update-input vault
   ```

## Two roots, and which one holds what

The declaration root keeps the declarations, `.sops.yaml` and everything nix evaluates from your own tree.

The vault root holds every ciphertext document, every public value and every definition record, in place of the declaring flake's own storage roots. The three roots themselves are described in [Storage layout](../concepts/storage-layout.md).

`.sops.yaml` never moves. A bare run of the encryption tool against a vault-rooted document therefore finds no creation rule above it, and a vault document is not browsable by hand.

`set`, `edit` and `get` are the tools there. Each renders the disposable rules the write needs, uses them, and removes them again.

## What the opaque names hide, and what they do not

A document's file name, the key inside it, a public value's file name and a definition record's path are each a hash of the naming key, a use-specific tag and the readable name.

So a vault host learns none of the audience, key or secret names the declaring tree carries, while the declaring flake computes both forms and the mapping between them.

Opacity is a property of names rather than of shape. A vault host still sees how many documents there are, how many keys each holds, each ciphertext's length, every document's recipient keys, and one commit per write.

## Commit order and the lock bump

A command touching both roots commits the vault first and the declaration root second, with a trailer naming the first commit.

`fix` itself commits nothing. It prints the order that follows, so the diff is yours to read first.

A vault commit is not visible to any consuming build until the declaring flake's lock entry for the vault is updated. The commit's closing note says so, and names the exact `nix flake lock --update-input` line when exactly one lock input settles on the vault root.

## The one file safix writes at the vault root

safix writes no `.gitignore` at the declaration root, and exactly one at a vault root, covering only the scratch rules file.

`safix check` reports a declared vault whose own `.gitignore` does not cover that file, and `safix fix` writes the entry when it is missing. The scratch registry already sweeps those files on every exit path; the ignore rule is the second, independent guarantee, for a scratch file that still exists at the moment `git add` runs.

## Relocating, rolling back, rotating the key

Adopting a vault and abandoning one are the same move in opposite directions, and both are `safix fix`.

```console
$ safix fix                  # readable layout at the declaration root → the vault
$ safix fix --vault-rollback # the vault → readable layout at the declaration root
```

Run `--vault-rollback` while `flake.safix.vault` is still declared. The naming key needed to find a vault-rooted entry's readable name is reachable only through the standing declaration. Remove the declaration and unset `SAFIX_VAULT_ROOT` yourself once it finishes.

Rolling back skips the re-wrap. It moves documents, outputs and records and nothing else.

Rotating the naming key is the identical migration run again: change the key, then `safix fix`.

## What can go wrong

- `safix::vault_declared_without_root` — a vault is declared and `SAFIX_VAULT_ROOT` names no working tree.
- `safix::vault_root_without_declaration` — the variable is set and no vault is declared.
- `safix::vault_not_a_repository` — the path it names is not a repository.
- `safix::vault_root_not_top_level` — the path is inside a repository rather than at its top level.
- `safix::vault_relocation_unreadable` — a file pending relocation could not be read or decrypted under your own identity.
- `safix::vault_commit_half_landed` — one of the two commits landed and the other did not. The message names which.

Every code is listed in [Refusals](../reference/refusals.md).
