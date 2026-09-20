---
title: "Migrating from sops-nix and agenix"
---

## What you will have

This page walks you through converting encrypted files you already have into new ciphertext, verified byte for byte, without deleting or rewriting the originals. At the end you have the new files, a generated module declaring how each one deploys, and a receipt recording what was verified.

`safix migrate` needs no declarations of its own. It reads one plan file and nothing else.

## Steps

1. Write the plan. Paths in it are relative to the plan file unless absolute:

   ```json
   {
     "version": 1,
     "sourceIdentities": { "ageKeyFile": "/home/alice/keys/source.txt" },
     "targetIdentities": { "ageKeyFile": "/home/alice/keys/target.txt" },
     "entries": [
       {
         "name": "service-token",
         "source": { "path": "old/token.age", "format": "age", "key": "" },
         "destination": { "path": "new/token.binary", "format": "binary", "key": "" },
         "recipients": ["age1TARGET..."],
         "deployment": {
           "path": "/run/service/token",
           "mode": "0400",
           "owner": "service",
           "group": "service",
           "restartUnits": ["service.service"]
         }
       }
     ],
     "deploymentTarget": "safix",
     "deploymentOutput": "new/deployment.nix",
     "receipt": "new/receipt.json"
   }
   ```

   Every field is listed in [The migration plan](../reference/migration-plan.md).

2. Create the destination directories. The run does not make them:

   ```console
   $ mkdir -p new
   ```

3. Run it:

   ```console
   $ safix migrate plan.json
   ```

4. Read the receipt at the path the plan named, then import the generated module beside the consumer it targets:

   ```nix
   {
     imports = [
       inputs.safix.nixosModules.default
       ./new/deployment.nix
     ];

     safix.identity.keyFile = "/var/lib/safix/identity.txt";
   }
   ```

   The generated module declares deployment and nothing else. Private identities stay yours to configure — see [`safix.identity.keyFile`](../reference/profile-options.md#safixidentitykeyfile).

## What the run verifies before it publishes

Each candidate is decrypted again, independently, with the plan's `targetIdentities`, and the bytes are compared with what came out of the source.

Those identities are explicit on purpose. Ambient keys cannot satisfy target verification, because a key that happened to be lying around would let a wrong target credential pass.

Verification failures publish nothing. Sources are retained on every path, so the originals remain the recovery path and remain authoritative.

Encrypted sources are copied into the Nix store where the generated module names them. No plaintext and no private identity is copied there.

## What the generated declarations look like

`deploymentTarget` chooses which consumer the generated module is written for: safix's own modules, sops-nix, or agenix.

For each entry the module carries the deployment metadata the plan gave: the path, the mode, and the owner and group or the numeric uid and gid. It carries the units to restart or reload, and whether the value is needed before user creation.

Optional `templates` carry a name, public `content` using `<safix:secret-name>` references, and their own deployment metadata.

Importing the generated module alongside the consumer's own module is the whole of the wiring. No source is deleted and no cutover happens on its own.

## The receipt

The receipt records the mappings, the paths of the identities used, the deployment metadata and the successful byte verification.

It holds no plaintext and no digest of plaintext, which is what lets it be committed and reviewed.

## Recovering an interrupted migration

A run that is killed part-way leaves a journal at the receipt's path plus `.journal`. It is written before the first output lands and removed once the receipt is published, so a completed migration leaves none.

The journal records the plan it belongs to and each output already published: its path, which of the three kinds of output it is, the device and inode the file was created with, and a digest of its bytes. It holds no plaintext and no digest of plaintext. Every field is listed in [The journal](../reference/migration-plan.md#the-journal).

Rerun the same plan to resume:

```console
$ safix migrate plan.json
```

The rerun does not refuse the outputs the journal proves are its own. Each is re-verified — its identity, its bytes, and for ciphertext a fresh decryption through the plan's `targetIdentities` — and kept. The rest are published, and the journal is removed last.

Two states refuse. A journal written for a different plan is refused naming both plans, so resume or abandon that migration before running this one. A recorded output whose identity or bytes changed since is refused by name, and is neither removed nor overwritten.

To discard the interrupted run instead:

```console
$ safix migrate --abandon plan.json
```

That removes exactly the outputs the journal records and still match their records, the staging directories the journal names, and then the journal. A file nobody recorded is left where it is. It refuses when there is no journal beside the receipt, when the journal names another plan, and when a recorded output no longer matches — a completed migration's outputs are not this command's to remove.

Sources are retained on every path, abandonment included, so the originals stay the recovery path throughout.

## What can go wrong

Everything below refuses rather than dropping the semantic it cannot express.

- `safix::migration_refused` is the code the whole verb reports under. The message names which of the following it hit.
- An output path that already exists. Nothing is overwritten.
- Two entries naming one input, which is an alias rather than two migrations.
- An unknown field anywhere in the plan, because a field silently ignored is a semantic silently dropped.
- A format the destination cannot represent. Dotenv, INI and text-only surfaces refuse bytes they would have to alter.
- Native sops-nix as the target for a keyed binary or intentionally empty value. Sops-nix cannot decode safix's keyed byte envelope, so choose a whole SOPS binary destination for those.
- Agenix as the target for a template, a service hook or early-user metadata. Agenix requires whole raw-age files and expresses none of the three.
- A cloud-provider recipient that isolated verification cannot be explicitly configured against.

Every code is listed in [Refusals](../reference/refusals.md).
