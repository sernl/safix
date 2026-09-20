---
title: "Inspectable recipients"
---

## What you will have

This page shows you how to choose a format whose recipient roster can be audited, and what to do with the files you already have in a format that cannot be. At the end you can tell which of your entries `safix check` can judge, and which it reports as unverifiable.

## Why the format decides

A SOPS document records its own recipients. Each recipient gets a stanza in the file, so a reader with no identity at all can list who the data key was wrapped to.

That is what lets `safix check` compare a file's recipients against the audience the declarations compute, and report the two halves of a disagreement: keys that can open the file and are not in its audience, and subjects in the audience that cannot open it.

A raw age file records nothing a reader can attribute. The wrapped keys are there, but the file discloses no roster, so the comparison has no left-hand side. `check` reports such a file as unverifiable rather than as agreeing.

## Steps

1. Leave entries in a SOPS format where the roster matters. The default is keyed YAML, and nothing needs declaring:

   ```nix
   {
     flake.safix.users.alice.private.grafana-token = { };
   }
   ```

2. Choose SOPS `binary` when the value is arbitrary bytes and the roster still has to be auditable:

   ```nix
   {
     flake.safix.users.alice.private.tls-key.format = "binary";
   }
   ```

   The whole document is the value, and the recipients stay inspectable. See the entry field `format` in [Declarations](../reference/declarations.md).

3. Choose raw `age` only where a consumer reads the file with `age` itself, and accept that its roster is opaque:

   ```nix
   {
     flake.safix.users.alice.private.appliance-blob.format = "age";
   }
   ```

4. Run `safix check` and read which files it could judge:

   ```console
   $ safix check
   ```

   A raw-age file appears as a roster it could not verify, naming the path. Nothing about it is an error yet.

5. Converge a raw-age file's roster with `--yes`, which is the explicit authorization that step needs:

   ```console
   $ safix fix --yes
   ```

   `fix` re-wraps the file to the declared audience and verifies that the resulting plaintext bytes are the ones that went in. It cannot verify what the old roster was, because the old file never said.

## What `check` can and cannot read

`check` reads age and GnuPG recipients out of a SOPS document. It decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not hold.

Two shapes are reported as findings rather than folded into an ordinary audience. A threshold key group has a rule no flat recipient list expresses, and a recipient from a provider `check` cannot interpret is a key it cannot attribute to a subject. Both are named in the report.

A raw-age content write is in the same position as `fix --yes`: it writes to the declared audience and cannot compare that to what the opaque file held before.

Enrolling a hardware key over a tree holding raw-age files needs `--trust-declared-recipients` for the same reason — see [Hardware keys](hardware-keys.md).

## What can go wrong

- `safix::recipients_unreadable` — a governed file's recipient stanzas could not be read at all, so `check` could not answer for it.
- `safix::stanza_unreadable` — one stanza inside a SOPS document is malformed.
- `safix::candidate_recipients_unreadable` — a file `fix` was about to re-wrap did not disclose what it currently holds.
- `safix::recipient_drift` — the file's recipients and its declared audience disagree. Where the drift is a narrowing, the report names who held the key and what mints a new value, because re-wrapping is not revocation — see the revocation rule in [Custody](../concepts/custody.md).
- `safix::no_creation_rule` — no rule in `.sops.yaml` covers the path, so a write would fail closed rather than pick up a default audience.

Every code is listed in [Refusals](../reference/refusals.md).
