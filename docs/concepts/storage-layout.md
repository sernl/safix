---
title: "Storage layout"
---

## What this page is for

This page shows you which files safix writes into your repository, what decides
each path, and how to move a whole tree without re-encrypting anything.

## The three roots

safix places files in exactly three trees, each a repository-relative path you
name.

```nix
{
  flake.safix.storage = {
    encrypted = ".safix/encrypted";
    plaintextOutputs = ".safix/plaintext-outputs";
    generatorRecords = ".safix/generator-records";
  };
}
```

The defaults are `secrets/safix`, `public/safix` and
`state/safix/definitions` — see
[the declaration reference](../reference/declarations.md).

Evaluation refuses a root that is empty, absolute, ends in `/` or carries a `..`
component. It also refuses any two of the three that are equal or nested, naming
both options and both values.

Comparison is on component boundaries. So `secrets/fleet` and
`secrets/fleet-public` are disjoint, while `secrets/fleet` and
`secrets/fleet/pub` are one inside the other.

## Audience directories under the encrypted root

The audience picks the directory, so the path states who can open the files in
it — see [The model](model.md).

An audience of one unmarked subject keeps that subject's own directory:

```text
secrets/safix/users/alice/secrets.yaml
```

Any wider audience is named for its elements in sorted order, joined with `,`:

```text
secrets/safix/shared/alice,web/secrets.yaml
secrets/safix/shared/%grafana,alice/secrets.yaml
secrets/safix/shared/@oncall,alice/secrets.yaml
secrets/safix/shared/=acme,alice/secrets.yaml
```

`%` marks a service, `@` a group, `@~` the owner of a machine and `=` an
organization. People and machines carry no marker.

Every generated rule is start-anchored under the encrypted root,
extension-terminated, and covers one directory level. There is no catch-all
rule and the generator emits none, so an unmatched path fails closed with the
encryption tool's own "no matching creation rules found" instead of picking up a
default recipient set.

`.sops.yaml` is written by `safix fix` and never by hand, because the encryption
tool reads the committed file off disk and that version decides what a new file
is encrypted to.

## One file per format

An ordinary keyed YAML entry shares its audience's `secrets.yaml`.

Every other case gets its own file in the same audience directory, named for the
entry and its format:

```text
secrets/safix/users/alice/deploy-key.age
secrets/safix/users/alice/service-env.dotenv
secrets/safix/shared/alice,web/kubeconfig.binary
```

A whole-document entry is one where the key inside the document is empty. Raw
age and SOPS binary derive that empty key automatically, and a non-empty
`sopsKey` on either is refused.

## The other two roots

The plaintext-output root holds the public values a generator emits, one file per
value: `public/safix/users/<u>/<name>/value`, and
`public/safix/shared/<audience>/<name>/value` for a wider audience. A nix module
of your own reads those at evaluation — see
[Generators](../guides/generators.md).

The generator-record root holds one plaintext record per generated value:
`state/safix/definitions/<owner>/<name>`, or
`state/safix/definitions/shared/<audience>/<name>` for a shared entry, with the
stamps beside it. A record carries a digest of the definition that minted the
value and no value of its own, which is what lets `safix check` report a changed
generator without decrypting anything.

## Ignore, backup and files safix did not place

A file safix did not place is not in the set `safix fix` re-wraps, though it
still rides its audience's rule, because a rule covers one directory level
rather than one literal filename. A change of audience therefore reaches every
file safix placed and leaves that one behind, encrypted to whoever it was
encrypted to when it was written.

`flake.safix.extraGovernedFiles` is a list of such paths. Naming one there puts
it into the set `fix` re-wraps and the checks judge. Every path must be
repository-relative and covered by an existing audience rule.

The case it exists for is a value with no declaration of its own, decrypted on
demand by whatever invokes it, such as a credentials command in a module of your
own reading one key with `sops -d --extract`. Use that shape for a credential
only a person invokes, and a declared entry for anything a service reads from a
path.

safix writes no `.gitignore` at the declaration root, and exactly one at a vault
root covering only the scratch rules file. Everything else is yours.

The encrypted tree is ciphertext without qualification and belongs in a backup.
The plaintext-output and generator-record trees hold values a module reads and
digests with no value in them, and belong in the repository.

## Renaming a root

Change the option, `git mv` the tree, run `safix fix`, then run `safix check`.

No re-encryption is involved. A SOPS document does not embed its own path, the
in-document key names do not change, and `fix` rewrites `.sops.yaml` and nothing
else.

A half-finished rename is visible rather than silent: between the option change
and the `git mv`, `safix check` reports every governed file the regenerated
policy no longer names.

## Moving the ciphertext out of this repository

A vault puts every ciphertext document, every public value and every definition
record in a second repository, under opaque names — see
[Vaults](../guides/vault.md).
