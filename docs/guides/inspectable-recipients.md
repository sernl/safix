---
title: "Inspectable recipients"
---

## What you will have

This page shows you how to choose a format whose recipient roster can be audited, and what to do with the files you already have in a format that cannot be. At the end you can tell which of your entries `safix check` can judge, and which it reports as unverifiable.

## Why the format decides

A SOPS document records its own recipients. Each recipient gets a stanza in the file, so a reader with no identity at all can list who the data key was wrapped to.

That is what lets `safix check` compare a file's recipients against the audience the declarations compute, and report the two halves of a disagreement: keys that can open the file and are not in its audience, and subjects in the audience that cannot open it.

A raw age file names each SSH recipient by a four-byte tag of its key and every other recipient by nothing.
An `ssh-ed25519` or `ssh-rsa` stanza carries the first four bytes of SHA-256 over the recipient's public key; an X25519 stanza carries only an ephemeral share, a plugin stanza's arguments are the plugin's, and an `scrypt` stanza is a passphrase.
So `check` reads the header of a raw age file, binary or ASCII-armored, without decrypting it, and compares its SSH tags with the tags of the SSH recipients its audience declares.
A disagreement is drift.
An exact match is reported as a note, because a four-byte tag can be collided with by anyone able to mint keys and write the file, so the match does not establish which keys the file is wrapped to.
A file holding, or declared to hold, any recipient whose stanza names nobody is reported as unverifiable rather than as agreeing.

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

3. Choose raw `age` only where a consumer reads the file with `age` itself, and accept that its roster can be compared by SSH tag at best:

   ```nix
   {
     flake.safix.users.alice.private.appliance-blob.format = "age";
   }
   ```

4. Run `safix check` and read which files it could judge:

   ```console
   $ safix check
   ```

   A raw-age file whose SSH stanzas disagree with its declared SSH recipients is reported as drift, listing the extra, missing and duplicated stanzas, and fails the run.
   A raw-age file whose stanzas are all SSH and match its declared SSH recipients one for one is a note, `Recipient roster matched by tag only`, which does not fail the run.
   A raw-age file with an X25519, plugin or `scrypt` recipient on either side is a roster `check` could not verify, naming the path, and fails the run.

5. Converge a raw-age file's roster with `--yes`, which is the explicit authorization that step needs:

   ```console
   $ safix fix --yes
   ```

   `fix` re-wraps the file to the declared audience, verifies that the resulting plaintext bytes are the ones that went in, and records the audience and the new bytes beside the definition records.
   It cannot show the old roster's non-SSH recipients, because the old file never named them.
   A later `fix`, with or without `--yes`, leaves a file alone when its record names the declared audience and the file's current bytes; a content write changes the bytes, so the next `fix --yes` re-wraps that file once, and a file named only through `extraGovernedFiles` has no record and is re-wrapped on every run.

## What `check` can and cannot read

`check` reads age and GnuPG recipients out of a SOPS document, and the SSH tags out of a raw age header. It decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not hold.

`check` also reads the audience record `fix` leaves beside a raw age file.
When the record names the file's current bytes and an audience other than the one declared now, the file is reported as wrapped to an audience that is no longer declared — the one case in which a raw age file's X25519 recipients are known to disagree with the declarations.
An absent record, one this version cannot read, or one naming other bytes says nothing.

Two shapes are reported as findings rather than folded into an ordinary audience. A threshold key group has a rule no flat recipient list expresses, and a recipient from a provider `check` cannot interpret is a key it cannot attribute to a subject. Both are named in the report.

A raw-age content write is in the same position as `fix --yes`: it writes to the declared audience and cannot compare that to the non-SSH recipients the file held before.

Enrolling a hardware key over a tree holding raw-age files needs `--trust-declared-recipients` for the same reason — see [Hardware keys](hardware-keys.md).

## What can go wrong

- `safix::recipients_unreadable` — a governed file's recipient stanzas could not be read at all, so `check` could not answer for it.
- `safix::age_header_unreadable` — the cause carried by `safix::recipients_unreadable` when a governed raw age file does not begin with the age v1 header, its ASCII armor does not decode, or its header has no closing MAC line.
- `safix::stanza_unreadable` — one stanza inside a SOPS document is malformed.
- `safix::candidate_recipients_unreadable` — a file `fix` was about to re-wrap did not disclose what it currently holds.
- `safix::recipient_drift` — the file's recipients and its declared audience disagree. Where the drift is a narrowing, the report names who held the key and what mints a new value, because re-wrapping is not revocation — see the revocation rule in [Custody](../concepts/custody.md).
- `safix::no_creation_rule` — no rule in `.sops.yaml` covers the path, so a write would fail closed rather than pick up a default audience.

Every code is listed in [Refusals](../reference/refusals.md).
