---
title: "Migration plan schema"
---

## What this page is for

This page lets you write a migration plan that `safix migrate` accepts, and read
the receipt it publishes. The walk-through that produces one is
[Migrating from sops-nix and agenix](../guides/migrating-from-sops-nix-and-agenix.md);
the verb itself is in [Command reference](cli.md#migrate).

A plan is one JSON object. Every object in it forbids unknown fields, so a
misspelled key is a refusal naming the line and column rather than a field
quietly dropped. Every relative path is resolved against the plan file's own
directory.

## The plan object

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `version` | number | yes | The schema version; the only accepted value is `1` |
| `sourceIdentities` | object | no | The identities that decrypt the sources; empty by default |
| `targetIdentities` | object | no | The identities that verify the destinations; empty by default |
| `entries` | array | yes | One object per secret to convert; at least one is required |
| `templates` | array | no | One object per public template to carry over; empty by default |
| `receipt` | string | yes | Where the receipt is written |
| `deploymentOutput` | string | yes | Where the generated declaration module is written |
| `deploymentTarget` | string | yes | `"safix"`, `"sops-nix"` or `"agenix"` |

`deploymentOutput` must name a file whose parent is an existing directory. An
output that already exists is refused rather than replaced.

## Identities

`sourceIdentities` and `targetIdentities` take the same three fields. Identities
are explicit: ambient keys in the environment cannot satisfy target
verification, which is what makes the verification independent of whatever
decrypted the source.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `ageKeyFile` | string | no | A file of native age identities |
| `ageSshKeyPaths` | array of strings | no | Private SSH keys native age accepts directly |
| `gnupgHome` | string | no | A GnuPG keyring directory; it must be a directory |

```json
{
  "sourceIdentities": { "ageKeyFile": "keys/old.txt" },
  "targetIdentities": { "ageSshKeyPaths": ["/home/alice/.ssh/id_ed25519"] }
}
```

## An entry

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | The secret's name, used for the declaration and the output |
| `source` | object | yes | The document to read from |
| `destination` | object | yes | The document to write |
| `recipients` | array of strings | yes | The public recipients of the destination |
| `deployment` | object | yes | The metadata the generated declaration carries |

A name must be nonempty, must carry no control character, and must be a plain
relative path component: no absolute path, no `..`, no leading separator. Entry
names and template names share one space, and no two of them — nor their
deployment paths — may be equal or nested.

### `source` and `destination`

Both take the same three fields, and all three are required.

| Field | Type | Meaning |
| --- | --- | --- |
| `path` | string | The ciphertext file |
| `format` | string | `"yaml"`, `"json"`, `"dotenv"`, `"ini"`, `"binary"` or `"age"` |
| `key` | string | The slash-separated selection; an empty string selects the whole document |

`binary` and `age` require an empty key, on either side. A source file must
exist; sources are always retained.

### `recipients`

The list must be nonempty and carry no duplicate. Each entry is one of three
shapes.

- A native age recipient: `age1` followed by lowercase letters and digits.
- An SSH public key. Against a destination whose format is not `age`, one
  carrying a comma is refused.
- `pgp:` followed by an uppercase hexadecimal fingerprint of 40 or 64
  characters.

A `pgp:` recipient is refused against a raw age destination.

### `deployment`

Every field is optional. What is absent is left to the deployment target's own
default.

| Field | Type | Meaning |
| --- | --- | --- |
| `path` | string | The installed path of the decrypted value |
| `mode` | string | Four octal digits beginning with `0` |
| `owner` | string | The owning account; nonempty |
| `group` | string | The owning group; nonempty |
| `uid` | number | The numeric owner |
| `gid` | number | The numeric group |
| `restartUnits` | array of strings | Units restarted when the value changes |
| `reloadUnits` | array of strings | Units reloaded when the value changes |
| `neededForUsers` | boolean | Install before user creation |

No deployment string may contain a control character. A unit name must be
nonempty, must not begin with `-`, and must be unique within its list. `owner`
together with a nonzero `uid` is refused, and so is `group` together with a
nonzero `gid`.

`neededForUsers: true` requires root ownership: `owner` and `group`, where
given, must be `root`, and `uid` and `gid` must be zero.

## Templates

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | The template's name, in the same space as entry names |
| `content` | string | yes | The public template text |
| `deployment` | object | yes | The same metadata an entry's deployment carries |

A template references a secret's runtime value as `<safix:secret-name>`. Each
reference must name an entry of this plan that is not an early-user output. An
unterminated placeholder is refused, as is a control character in the content.

A template may not itself be an early-user output.

```json
{
  "templates": [
    {
      "name": "grafana-env",
      "content": "GF_SECURITY_ADMIN_PASSWORD=<safix:grafana-admin-password>\n",
      "deployment": { "mode": "0400", "restartUnits": ["grafana.service"] }
    }
  ]
}
```

## What each deployment target refuses

Unsupported target semantics are refused rather than dropped, so a plan never
publishes a declaration that carries less than it says.

`agenix` requires whole-document raw age destinations: the format must be `age`
and the key must be empty. It supports no `restartUnits`, no `reloadUnits` and no
`neededForUsers`, and it cannot carry templates at all — select `safix` or
`sops-nix` to preserve them.

`sops-nix` supports no raw age destination. A keyed destination additionally
cannot carry a value that is not valid text, or an intentionally empty one:
safix's keyed byte envelope is not something native sops-nix decodes, so those
values need a whole binary destination.

`safix` accepts every shape above. Its one extra rule is that an early-user
secret's mode, where given, must grant owner-only permissions.

## How a plan is executed

Each entry is prepared as a private candidate: the source is decrypted under
`sourceIdentities`, the candidate is encrypted to `recipients`, and the candidate
is then decrypted again under `targetIdentities` alone and compared with the
source byte for byte. A mismatch refuses the run with sources retained and
nothing published.

Only after every candidate has verified are the ciphertexts, the declaration
module and the receipt published together. A recoverable error rolls back new
outputs, and an interruption between two steps rolls the run back the same way
and exits with the signal's own status. What a killed process leaves is
described by the journal below, and the retained sources remain the recovery
path on every path.

## The receipt

The receipt is a JSON object written at `receipt`.

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | number | `1` |
| `verification` | string | `"independent-target-decryption-byte-equal"` |
| `sourceRetained` | boolean | `true` |
| `deploymentTarget` | string | The target the plan named |
| `deploymentOutput` | string | The published declaration module's path |
| `sourceIdentities` | object | The identities the sources were read under |
| `targetIdentities` | object | The identities the destinations were verified under |
| `entries` | array | One object per entry |
| `templates` | array | The templates, as the plan declared them |

Each entry of `entries` carries `name`, `source`, `destination`, `recipients`,
`deployment` and its own `verification`, with the resolved absolute paths rather
than the ones the plan wrote.

## The journal

The journal is a JSON object written at the receipt's path plus `.journal`,
before the first output lands, and rewritten after each output is published. It
is removed once the receipt is published, so a completed migration leaves none
and the file's presence is the whole meaning of "interrupted".

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | number | `1`; a journal stating another version is refused |
| `planDigest` | string | A digest of the plan's bytes and its canonical directory, so the same JSON at another path is a different plan |
| `plan` | string | The plan's canonical path |
| `staging` | array of strings | The private staging directories the run created |
| `outputs` | array | One object per output already published |

Each object of `outputs` carries `path`, `kind` — `ciphertext`, `declarations`
or `receipt` — the `device` and `inode` the file was created with, and the
`sha256` of its bytes. The identity says the file is the one the run created,
and the digest says its bytes are the ones the run verified.

Unknown fields are forbidden here as they are in the plan. No plaintext and no
digest of plaintext is recorded: `sha256` covers published ciphertext and
public output.
