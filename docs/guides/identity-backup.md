---
title: "Identity backup"
---

## What you will have

This page shows you how to mint an identity, back it up so that the backup is provably openable, and restore it onto an empty destination. At the end you hold a recovery package that was decrypted and checked before it was written, using a key that is not the key it protects.

## Steps

1. Mint the identity on your own machine, as yourself:

   ```console
   $ safix keygen --kind age
   ```

   The public half and what to do with it are printed. The private half never is.

   The age form appends to the managed key file and never truncates it. The encryption tool tries every identity in that file, so a second identity beside a first is a working state, and overwriting is how somebody loses the key to everything they hold.

2. Use the GnuPG profile instead where you want a certificate with an expiry:

   ```console
   $ safix keygen --kind pgp --uid "Alice <alice@example.com>" --expires 2y
   ```

   It creates an expiring Ed25519 certification primary with a cv25519 encryption subkey. Upstream pinentry handles the passphrase, including confirmation and any export prompt needed to verify the complete private key. An existing compatible key is neither weakened nor replaced.

3. Print the public recipient of the identity already minted here, without minting anything:

   ```console
   $ safix keygen --show
   ```

4. Create and retain a separate recovery identity, then back up against it:

   ```console
   $ safix identity backup age /secure/offline/age-backup.json \
       --recipient age1RECOVERY... --age-key-file /secure/recovery/identity.txt
   ```

   Use `backup pgp` for the managed GnuPG keyring. `--gnupg-home` selects an explicit recovery keyring instead of an age recovery file. A GnuPG recipient is written `pgp:FULL_UPPERCASE_FINGERPRINT`.

5. Restore onto an empty destination, which is also how a recovery drill is run:

   ```console
   $ safix identity restore /secure/offline/age-backup.json \
       --age-key-file /secure/recovery/identity.txt
   ```

   Do not remove the working identity to test a backup. Select an empty managed destination and restore into that.

## The independence rule

A backup is only as good as the separate thing that opens it. So the recovery identity has to be independent of the identity being backed up, and a self-only backup is refused: a key cannot be its own recovery path.

Backup does not stop at encrypting. It decrypts the package again with the recovery identities you named, and verifies every private primary and subkey, before the package is published.

Protect the recovery identity yourself. Nothing here escrows it for you.

The same rule decides a person's independence from an operator. Leaving [`flake.safix.users.<name>.recoveryRecipients`](../reference/declarations.md#flakesafixusersnamerecoveryrecipients) empty keeps their custody independent at a cost no later edit undoes: with only their activation key, losing it makes their files unopenable by everyone, operator included. An offline master key or a hardware token the person holds is the mitigation that keeps the independence — see [Hardware keys](hardware-keys.md).

## Where the private material lives

The managed age file is the one the encryption tool reads by default, and `SAFIX_AGE_KEY_FILE` overrides it. The managed keyring is `SAFIX_GNUPGHOME`, or a path under the user's data directory. Both are listed in [Environment](../reference/environment.md).

Private files use mode `0600` and private directories `0700`. Private custody paths have to lie outside repositories and outside the Nix store, with no symlinked ancestor.

Ordinary reads honour the native `SOPS_AGE_KEY_FILE` and `GNUPGHOME` before the managed defaults, along with the encryption tool's own identity settings. `SAFIX_AGE_SSH_KEY_PATHS` supplies further SSH identities.

Migration, installation and recovery verification use their explicitly configured identities instead. An ambient key cannot rescue an incorrect target credential.

Output from these verbs contains public recipients and custody instructions, never private key bytes.

## What can go wrong

- `safix::keygen_for_someone_else` — minting an identity for another person needs `--for-someone-else`, because doing it means holding their private key.
- `safix::keygen_no_identity_yet` — `--show` found nothing minted on this machine. Plain `keygen` is the remedy.
- `safix::keygen_no_public_key` — the minted identity yielded no public half to print.
- `safix::keygen_failed` — the underlying key generation failed.
- `safix::key_management` — the refusal `identity backup` and `identity restore` report under. Its message names which case was hit.

Backup refuses a self-only recovery set, a recovery keygrip shared with the key being backed up, a hardware stub, an incomplete primary-key export, a private-key file with no corresponding public certificate, and a destination that already exists.

Restore refuses an existing age file, a colliding GnuPG fingerprint or private keygrip, two different certificates sharing one private key, and a private component absent from the public keyring.

Every code is listed in [Refusals](../reference/refusals.md).
