---
title: "Generators"
---

## What you will have

This page shows you how to declare a value that writes itself, chain one generator onto another, publish a public half in the clear, and rotate the result. At the end `safix generate` mints every declared value that is still empty, and the repository records which declaration minted each one.

## The script's contract

A generator's `script` writes files. It does not print a value.

```nix
{
  flake.safix.users.alice.private.grafana-token.generator = {
    script = ''openssl rand -hex 32 > "$out/grafana-token"'';
    runtimeInputs = [ "openssl" ];
    description = "a grafana service account token";
  };
}
```

The script runs under `bash -euo pipefail`, with the staging root as its working directory. Three directories are addressed from there:

| path | what it holds |
|---|---|
| `$out/<name>` | one file per declared output, written by this script |
| `$prompts/<name>` | one answered prompt each, present only where prompts are declared |
| `$in/<generator>/<name>` | a dependency's plaintext, keyed by the entry its generator is declared on |

Those are clan's own directories, so a script written for either system runs under the other. One difference is deliberate: only the dependencies this generator declares appear under `$in`.

Every declared output has to exist when the script exits. A missing one refuses the whole run and lists what `$out` did contain.

Bytes are stored exactly as written. `echo` leaves a trailing newline and `printf` does not, and nothing removes one.

## Steps

1. Name the tools. The generator field `runtimeInputs` holds nixpkgs attribute names as strings, because the whole generator reaches the command as JSON and a derivation cannot cross that boundary. Each string is resolved against the package set at build time, so a misspelling fails then rather than at a rotation.

2. Grant the network where the value comes from outside:

   ```nix
   {
     flake.safix.users.alice.private.acme-account.generator = {
       network = true;
       runtimeInputs = [ "curl" ];
       script = ''curl -fsS https://issuer.example/token > "$out/acme-account"'';
     };
   }
   ```

   The generator field `network` lives on the declaration rather than on the invocation, so which generators may reach out is a question your tree answers in a line a reviewer sees. It governs the script and the validation fragments alike. Both fields are described in [Declarations](../reference/declarations.md).

3. Chain one onto another, and ask where computing is not enough:

   ```nix
   {
     flake.safix.users.alice.private = {
       db-password.generator = {
         prompts.passphrase.description = "the account's login password";
         script = ''cp "$prompts/passphrase" "$out/db-password"'';
       };

       db-password-hash.generator = {
         dependencies = [ "db-password" ];
         script = ''mkpasswd -sm bcrypt <"$in/db-password/db-password" > "$out/db-password-hash"'';
         runtimeInputs = [ "mkpasswd" ];
       };
     };
   }
   ```

   `dependencies` names other entries of the same person whose plaintext this generator reads. Depending on another person's secret is refused at evaluation: your machine holds no identity that opens their file. A prompt asks instead of computing, and its answer arrives as a file under `$prompts`.

4. Write several outputs from one run, and publish the ones that are not secret:

   ```nix
   {
     flake.safix.users.alice.private.wg-private.generator = {
       runtimeInputs = [ "wireguard-tools" ];
       files.wg-public.secret = false;
       script = ''
         wg genkey > "$out/wg-private"
         wg pubkey < "$out/wg-private" > "$out/wg-public"
       '';
     };
   }
   ```

   Each name under `files` is a registry entry in its own right, with its own mode, path and key. Both halves land in one commit, because a keypair split across two commits is an incoherent state.

   An entry named there may carry no generator of its own and may not be named by a second generator. Two producers for one value is a race.

5. Read a public output while nix evaluates, from a flake-parts module of your own:

   ```nix
   { config, ... }:
   {
     flake.wireguardPeers = [
       { publicKey = config.flake.safix.lib.publicValue "alice" "wg-public"; }
     ];
   }
   ```

   `publicValue` is for a public key, a fingerprint or a derived identifier. `outputPath` answers for every output and is a path rather than a value. Asking for a value on a secret output fails with a sentence naming the entry.

6. Judge a candidate before anything is written:

   ```nix
   {
     flake.safix.users.alice.private.grafana-token.generator.validation = ''
       grep -qE '^[0-9a-f]{64}$' || exit 1
     '';
   }
   ```

   The candidate arrives on standard input and `$out_name` names the output under judgement. A non-zero exit refuses the whole run while the values are still only in memory. The generator field `validation` is described in [Declarations](../reference/declarations.md).

7. Mint:

   ```console
   $ safix generate                              # every declared value with none yet
   $ safix generate --regenerate grafana-token   # rotate one
   $ safix generate --regenerate --yes alice     # rotate hers without the prompt
   ```

   Naming either half of a multi-output generator runs the generator that mints both.

## Rotation cascades

`--regenerate` re-runs every generator that reads what the named one writes, transitively, in dependency order.

Otherwise a rotation would leave values derived from the value it replaced, and a hash of a retired password reads exactly like a hash of the current one.

The set is listed before anything runs and confirmed once, because each re-run commits as it goes. `--yes` answers that confirmation in advance. A generator nothing reads is not a cascade and asks nothing.

## The sandbox

A script and its validation fragments run confined. The staging root is the only writable path, the nix store is readable, and there is no network unless the declaration grants it.

A write outside `$out` fails, so a fragment cannot leave plaintext somewhere safix does not look and cannot shred. `runtimeInputs` is therefore the whole of what a fragment can run.

A validation fragment has no writable path at all: the staging root is already shredded by the time a candidate is judged.

The staging directory is created mode `0700` on a filesystem safix asks the kernel about rather than infers from its name, and it is overwritten and removed however the run ends.

There is no fallback to `/tmp`. On a host whose `/tmp` is disk-backed that would leave plaintext in free blocks under a code path that looked like success. Where no memory-backed filesystem is found the run refuses, and `--allow-disk-staging` accepts a disk-backed one.

`SAFIX_STAGING_DIR` names the mount to use instead of the conventional ones, replacing them rather than being tried first — see [Environment](../reference/environment.md).

There is no flag that disables the sandbox.

## The records a mint leaves

A generated value carries nothing saying which declaration produced it, so `generate` writes a digest of the declaration it ran, in the same commit as the value.

The record sits under [`flake.safix.storage.generatorRecords`](../reference/declarations.md#flakesafixstoragegeneratorrecords), at `<generatorRecords>/<user>/<name>` or `<generatorRecords>/shared/<audience>/<name>`, as one plaintext line holding a format tag and a digest.

It covers the script, its `runtimeInputs`, its network grant, its prompts, its dependencies, the outputs it writes with their secrecy, and the validation fragment. No value and no derivative of a value is in it, which is what lets it be committed in the clear.

`safix check` reads it back and reports a value whose declaration has changed since it was minted. It names regeneration and reverting the edit as the two remedies and recommends neither. A value with no record predates the record and is not a finding.

Beside each definition record sits one stamp record per value, named for the value with a `.stamps` suffix. It holds the seconds the value was first written and the seconds it last changed, and it is what `safix view` prints as `CREATED` and `UPDATED`.

## What can go wrong

- `safix::no_generator` — the name you asked to regenerate has no generator declared on it.
- `safix::generator_cycle` — the declared dependencies form a cycle, named with its participants. Self-reference is the same finding.
- `safix::dependency_has_no_value` — an upstream entry the script reads holds nothing yet.
- `safix::generate_needs_nixpkgs` — running under `--entry` with neither `--nixpkgs` nor `SAFIX_NIXPKGS` set. The sandbox resolves its tools through a flake.
- `safix::sandbox_unavailable` and `safix::sandbox_unsupported` — no confinement backend was found, named before the first fragment runs.
- `safix::staging_not_memory_backed` — no memory-backed filesystem was available. `--allow-disk-staging` is the explicit acceptance.
- `safix::staging_unusable` — the staging mount exists and cannot be used as required.
- `safix::prompt_unanswered` and `safix::no_value_for_prompt` — a declared prompt got no answer.
- `safix::generator_failed` — the script exited non-zero.
- `safix::generator_output_missing` — the script exited cleanly without writing a declared output.
- `safix::generator_produced_nothing` — the run wrote nothing at all.
- `safix::validation_rejected` — a validation fragment refused the candidate. Nothing was written.
- `safix::cascade_declined` — the cascade set was shown and declined.
- `safix::generator_definition_drifted` — a command that needs the minted definition found a different one recorded.
- `safix::stamp_record_unparsable` — a stamp record is not in a shape this build reads.

Every code is listed in [Refusals](../reference/refusals.md).
