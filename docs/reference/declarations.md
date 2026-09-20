---
title: "Declaration options"
---

## What this page is for

This page lets you look up every option under `options.flake.safix` — the
declarations safix reads — with its type, its default and what it means.

Each option appears once, grouped by the record it belongs to. A placeholder
attribute name is written `<name>`, and `<subject>` stands for a person, a
machine, a service, a group, an organization, or the owner of a machine.

Profile options live in [Profile options](profile-options.md). The vocabulary
these records are built from is in [The model](../concepts/model.md).

## The records

#### `flake.safix.catalogue`

*Type:* attribute set of entries. *Default:* `{ }`.

The secret catalogue: one definition per secret more than one person may hold,
selected by name in a person's `carries`.

#### `flake.safix.users`

*Type:* attribute set of people. *Default:* `{ }`.

Who holds what — safix's own custody record, deliberately not a consumer's user
registry and never reading one.

#### `flake.safix.machines`

*Type:* attribute set of machines. *Default:* `{ }`.

The machines an audience may name, each with the recipient its system scope
already decrypts with, its owner and its tags.

#### `flake.safix.services`

*Type:* attribute set of services. *Default:* `{ }`.

The services an audience may name, each with the machines it runs on, its owner,
and the unix user and group its landed entries belong to.

#### `flake.safix.groups`

*Type:* attribute set of groups. *Default:* `{ }`.

The groups an audience may name, each a set of subjects whose membership lives in
one place.

#### `flake.safix.organizations`

*Type:* attribute set of organizations. *Default:* `{ }`.

The organizations an audience may name, each holding its own recovery custody and
naming the people who scaffold for it.

#### `flake.safix.silos`

*Type:* attribute set of silo sets. *Default:* `{ }`.

Named sets of groups that no one file's audience may span.

#### `flake.safix.rotation`

*Type:* attribute set of rotation policies. *Default:* `{ }`.

Named rotation policies: how long a class of values may live. An entry opts in
by naming one in its own `rotation` field, and an entry naming a policy declared
nowhere here is refused at evaluation.

#### `flake.safix.rotation.<name>.every`

*Type:* an interval as a positive whole number of hours, days or weeks — `<n>h`,
`<n>d` or `<n>w`. *No default.*

How long a value this policy governs may live after its last write. One
declaration decides the deadline of every entry naming the policy, so shortening
an interval moves all of them at the next evaluation and no entry is edited.

#### `flake.safix.extraGovernedFiles`

*Type:* list of strings. *Default:* `[ ]`.

Encrypted files a consumer wants governed that no declaration implies, as
repository-relative paths that a generated rule already covers.

#### `flake.safix.onboardingHook`

*Type:* null or shell lines. *Default:* `null`.

A shell fragment `safix adduser` runs after the person's declaration and the
regenerated policy are committed, receiving their name, their recipient and every
`--host` given.

#### `flake.safix.enrollHook`

*Type:* null or shell lines. *Default:* `null`.

A shell fragment `safix enroll` runs after the card's identity, its recipient and
the regenerated policy are committed, receiving the person's name, the card's
serial and its age recipient.

#### `flake.safix.lib`

*Type:* attribute set. *Read-only, no default.*

The resolution helpers, maps and generated policy this module derives from the
declared records; every value in it is a projection of what a consumer declared.

## Storage roots

Evaluation refuses a root that is empty, absolute, ends in `/` or carries a `..`
component, and refuses any two of the three that are equal or nested. See
[Storage layout](../concepts/storage-layout.md).

#### `flake.safix.storage`

*Type:* a submodule. *Default:* `{ }`.

Where safix keeps the three trees it places files in, as three independent
repository-relative roots whose defaults are today's spelling, so that leaving
this unset changes nothing.

#### `flake.safix.storage.encrypted`

*Type:* string. *Default:* `"secrets/safix"`.

The repository-relative directory holding every ciphertext document safix
places, one file per distinct audience in a directory named for that audience.

#### `flake.safix.storage.plaintextOutputs`

*Type:* string. *Default:* `"public/safix"`.

The repository-relative directory holding generator outputs declared
`secret = false`, written in the clear so a nix module can read one at
evaluation.

#### `flake.safix.storage.generatorRecords`

*Type:* string. *Default:* `"state/safix/definitions"`.

The repository-relative directory holding every per-value plaintext record: a
digest of the generator definition that minted the value, and the unix seconds it
was created and last updated.

## The vault

#### `flake.safix.vault`

*Type:* null or a submodule. *Default:* `null`.

A separate repository every ciphertext document, generated public value and
generator definition record moves to, in place of this flake's own source; the
recipient policy never moves.

#### `flake.safix.vault.root`

*Type:* path. *No default.*

The repository every audience file's ciphertext, public value and definition
record resolves rooted at, typically a `flake = false` input's own path.

#### `flake.safix.vault.namingKey`

*Type:* string. *Default:* `""`.

At least 64 lowercase hexadecimal characters that every vault-rooted name is a
keyed hash of; evaluation refuses a declared vault whose key is unset, shorter,
or carries a character outside `[0-9a-f]`.

## An entry

The entry submodule is the value of `flake.safix.catalogue.<name>` and of
`flake.safix.users.<name>.private.<name>`, with the same fields and the same
defaults in both places. The paths below name the catalogue form.

#### `flake.safix.catalogue.<name>.format`

*Type:* one of `"yaml"`, `"json"`, `"dotenv"`, `"ini"`, `"binary"`, `"age"`.
*Default:* `"yaml"`.

The ciphertext format; raw age is handled by safix rather than SOPS, and binary
and age always store a whole document.

#### `flake.safix.catalogue.<name>.sopsKey`

*Type:* null or string. *Default:* `null`.

The key to read inside the encrypted file; `null` uses the entry's own name for
structured formats, an empty string selects the whole document, and binary and
age derive an empty key automatically.

#### `flake.safix.catalogue.<name>.mode`

*Type:* string. *Default:* `"0400"`.

The on-disk mode of the decrypted value, matching the secret provisioner's own
default.

#### `flake.safix.catalogue.<name>.path`

*Type:* null or a function to a string. *Default:* `null`.

Where the decrypted value is written, as a function of the configuration
materializing it; `null` takes the provisioner's own name-derived default.

#### `flake.safix.catalogue.<name>.owner`

*Type:* null or string. *Default:* `null`.

The owning account of the decrypted file, or `null` to leave it to the
provisioner; the user-scope materialization refuses an entry that sets it.

#### `flake.safix.catalogue.<name>.group`

*Type:* null or string. *Default:* `null`.

The owning group of the decrypted file, refused at user scope on the same ground
as `owner`.

#### `flake.safix.catalogue.<name>.neededForUsers`

*Type:* boolean. *Default:* `false`.

Whether to install this value before user creation at system scope.

#### `flake.safix.catalogue.<name>.restartUnits`

*Type:* list of strings. *Default:* `[ ]`.

The units to restart when the installed value changes.

#### `flake.safix.catalogue.<name>.reloadUnits`

*Type:* list of strings. *Default:* `[ ]`.

The units to reload when the installed value changes.

#### `flake.safix.catalogue.<name>.shared`

*Type:* boolean. *Default:* `false`.

Whether the carriers of this entry hold one value between them — one ciphertext
whose audience is every carrier — or each hold their own.

#### `flake.safix.catalogue.<name>.generator`

*Type:* null or a generator. *Default:* `null`.

How `safix generate` mints this secret's value, or `null` when the value comes
from somewhere this repository cannot compute.

#### `flake.safix.catalogue.<name>.rotation`

*Type:* null or string. *Default:* `null`.

The [`flake.safix.rotation`](#flakesafixrotation) policy deciding how long this
value may live, or `null` for a value with no deadline. The deadline is the
entry's last recorded write plus the policy's interval, and an entry with a
policy and no timestamp record is due at once.

#### `flake.safix.catalogue.<name>.sopsFile`

*Type:* null or path. *Default:* `null`.

Refused if set, and declared so that the refusal has a name to attach to: safix
derives every entry's file from its audience, and a file chosen by hand would
carry recipients no declaration names.

## A generator

The generator submodule is the value of an entry's `generator`. See
[Generators](../guides/generators.md).

#### `flake.safix.catalogue.<name>.generator.script`

*Type:* shell lines. *No default.*

The fragment that produces this generator's output values, running under
`bash -euo pipefail` with `runtimeInputs` on PATH, its working directory at a
private staging root, and one file written per declared output.

#### `flake.safix.catalogue.<name>.generator.runtimeInputs`

*Type:* list of strings. *Default:* `[ "coreutils" ]`.

The nixpkgs attribute names put on PATH while the script runs, as strings because
the generator reaches the command as JSON.

#### `flake.safix.catalogue.<name>.generator.network`

*Type:* boolean. *Default:* `false`.

Whether this generator's fragments reach the network; `true` re-shares the
network and nothing else, leaving the filesystem confinement in force.

#### `flake.safix.catalogue.<name>.generator.prompts`

*Type:* attribute set of prompts. *Default:* `{ }`.

The values the operator supplies when this generator runs, each readable from the
script at `$prompts/<name>` and holding exactly what was typed.

#### `flake.safix.catalogue.<name>.generator.prompts.<name>.type`

*Type:* one of `"hidden"`, `"line"`, `"multiline"`. *Default:* `"hidden"`.

How the operator's input is read: without echoing it, with it echoed, or every
line until end of input.

#### `flake.safix.catalogue.<name>.generator.prompts.<name>.description`

*Type:* string. *Default:* the prompt's own attribute name.

What the operator is being asked for, shown at the prompt.

#### `flake.safix.catalogue.<name>.generator.dependencies`

*Type:* list of strings. *Default:* `[ ]`.

Other secrets of the same person whose plaintext this script reads, each readable
at `$in/<generator>/<name>`, and each enrolling this generator in the rotation of
what it reads.

#### `flake.safix.catalogue.<name>.generator.files`

*Type:* attribute set of generator outputs. *Default:* `{ }`.

Further outputs of the same person this one generator also writes, beside the
entry it is declared on.

#### `flake.safix.catalogue.<name>.generator.files.<name>.secret`

*Type:* boolean. *Default:* `true`.

Whether this output is encrypted into the entry's audience file, or written in
the clear under the plaintext-output root with no creation rule and readable at
evaluation.

#### `flake.safix.catalogue.<name>.generator.validation`

*Type:* null or shell lines. *Default:* `null`.

A fragment that judges a candidate value on standard input before anything is
written, with `$out_name` naming which output is being judged.

#### `flake.safix.catalogue.<name>.generator.description`

*Type:* null or string. *Default:* `null`.

What this generator mints, shown by `safix list` and `safix check`.

#### `flake.safix.catalogue.<name>.generator.share`

*Type:* null or boolean. *Default:* `null`. *Read-only and hidden.*

Derived rather than authored: true exactly when every entry this generator writes
is `shared`, and setting it is refused by name.

## A person

The person submodule is the value of `flake.safix.users.<name>`. See
[Custody](../concepts/custody.md).

#### `flake.safix.users.<name>.recipient`

*Type:* null or string. *Default:* `null`.

The age public key this person's secrets are encrypted to — a recipient, never an
identity — and the field that gives them an anchor and a creation rule in the
generated policy.

#### `flake.safix.users.<name>.recipientNote`

*Type:* null or lines. *Default:* `null`.

Prose emitted above this person's key in the generated policy's keys block.

#### `flake.safix.users.<name>.recoveryRecipients`

*Type:* attribute set of recovery recipients. *Default:* `{ }`.

Further recipients belonging to this same person's custody, keyed by the anchor
the generated policy defines them as, and added to every file their audience
includes.

#### `flake.safix.users.<name>.recoveryRecipients.<anchor>.key`

*Type:* string. *No default.*

The age public key, as it appears in the generated recipient policy.

#### `flake.safix.users.<name>.recoveryRecipients.<anchor>.note`

*Type:* null or lines. *Default:* `null`.

Prose emitted above this key in the generated policy's keys block.

#### `flake.safix.users.<name>.escrowedTo`

*Type:* list of strings. *Default:* `[ ]`.

The organizations this person consents to the escrow of; every file their
audience covers gains those organizations' custody keys at the next re-wrap.

#### `flake.safix.users.<name>.managedBy`

*Type:* null or string. *Default:* `null`.

The organization whose managers scaffold for this person, which hands out no read
of anything.

#### `flake.safix.users.<name>.carries`

*Type:* attribute set of overrides. *Default:* `{ }`.

The catalogue entries this person carries on every host, each optionally adjusted
by an override.

#### `flake.safix.users.<name>.private`

*Type:* attribute set of entries. *Default:* `{ }`.

Secrets belonging to this person alone, declared here rather than in the
catalogue; declaring one is itself selecting it.

#### `flake.safix.users.<name>.sharedWith`

*Type:* attribute set of attribute sets of grants. *Default:* `{ }`.

Outbound sharing declared by the owner: `<subject>.<name>` makes this person's
`<name>` resolve into that subject's secret set as well.

#### `flake.safix.users.<name>.sharedWith.<subject>.<name>`

*Type:* a submodule with no fields. *No default.*

The grant itself, which carries no fields: it is the owner's statement that a
name they hold is to reach one other subject.

#### `flake.safix.users.<name>.perHost`

*Type:* attribute set of scopes, keyed by hostname. *Default:* `{ }`.

Per-hostname adjustments to the union of `carries`, `private` and the secrets
shared to this person.

#### `flake.safix.users.<name>.perTag`

*Type:* attribute set of scopes, keyed by tag. *Default:* `{ }`.

Per-tag adjustments to the union of `carries`, `private` and the secrets shared
to this person.

## A scope, and an override

A scope is the value of one `perHost.<host>` or `perTag.<tag>` key. An override
is a partial entry: each field is nullable with a null default, and only the
non-null ones apply. The paths below name the `perHost` form; `perTag` takes the
same submodule, as does `carries.<name>`.

#### `flake.safix.users.<name>.perHost.<host>.add`

*Type:* attribute set of overrides. *Default:* `{ }`.

The secrets carried in this scope.

#### `flake.safix.users.<name>.perHost.<host>.omit`

*Type:* attribute set of overrides. *Default:* `{ }`.

The secrets dropped in this scope; only the keys are used.

#### `flake.safix.users.<name>.perHost.<host>.force`

*Type:* attribute set of overrides. *Default:* `{ }`.

The secrets re-added after `omit`, beating it within the same resolution.

#### `flake.safix.users.<name>.carries.<name>.mode`

*Type:* null or string. *Default:* `null`.

Overrides the on-disk mode in this scope; `null` leaves the entry's mode
standing.

#### `flake.safix.users.<name>.carries.<name>.path`

*Type:* null or a function to a string. *Default:* `null`.

Overrides the on-disk path in this scope, as a function of the configuration
materializing it; `null` leaves the entry's path standing.

#### `flake.safix.users.<name>.carries.<name>.rotation`

*Type:* null or string. *Default:* `null`.

Overrides the rotation policy in this scope; `null` leaves the entry's policy
standing.

## A machine

#### `flake.safix.machines.<name>.recipient`

*Type:* null or string. *Default:* `null`.

The age public key this machine's system scope already decrypts with — the
`ssh-to-age` form of the host's ed25519 key — so declaring a machine mints no
second identity.

#### `flake.safix.machines.<name>.recipientNote`

*Type:* null or lines. *Default:* `null`.

Prose emitted above this machine's key in the generated policy's keys block,
where the key earns an anchor at all.

#### `flake.safix.machines.<name>.owner`

*Type:* null or string. *Default:* `null`.

The person or organization this machine belongs to — a record that confers no
powers, and what gives an `ownerOf.<machine>` grant somewhere to resolve through.

#### `flake.safix.machines.<name>.tags`

*Type:* list of strings. *Default:* `[ ]`.

The tags this machine carries, against which a person's `perTag` adds, omits and
forces entries.

## A service

#### `flake.safix.services.<name>.machines`

*Type:* list of strings. *Default:* `[ ]`.

The machines this service runs on, whose recipients an audience naming the
service is encrypted to.

#### `flake.safix.services.<name>.owner`

*Type:* null or string. *Default:* `null`.

The person or organization this service belongs to, on the same terms as a
machine's owner: it confers no powers.

#### `flake.safix.services.<name>.user`

*Type:* null or string. *Default:* `null`.

The unix account this service's landed entries belong to, or `null` to leave them
to the provisioner; refused at user scope, which has no ownership axis.

#### `flake.safix.services.<name>.group`

*Type:* null or string. *Default:* `null`.

The unix group this service's landed entries belong to, refused at user scope on
the same ground as `user`.

## A group, a silo set, and an organization

#### `flake.safix.groups.<name>.members`

*Type:* list of strings. *Default:* `[ ]`.

The subjects this group consists of: people, machines, services, or other groups;
an organization is refused here.

#### `flake.safix.silos.<name>.groups`

*Type:* list of strings. *Default:* `[ ]`.

The groups this set holds mutually exclusive, so that evaluation refuses any file
whose audience would reach subjects of two of them.

#### `flake.safix.organizations.<name>.custody`

*Type:* attribute set of recovery recipients. *Default:* `{ }`.

The escrow identities this organization holds, keyed by the anchor the generated
policy defines each as; rotating one here re-wraps every consenting person's
files at the next `safix fix`.

#### `flake.safix.organizations.<name>.managers`

*Type:* list of strings. *Default:* `[ ]`.

The people who scaffold for this organization, which places no key in any
audience and adds no recipient to any file.

## The safix side of a mapping

Every sync target's mapping carries the same two-field `safix` side, so it is
documented once here. The paths below name the clan target's form; the other
targets take the identical submodule under their own `mappings`.

#### `flake.safix.bridge.mappings.<name>.safix`

*Type:* a submodule. *No default.*

The safix half: a person, and a name that person holds.

#### `flake.safix.bridge.mappings.<name>.safix.user`

*Type:* string. *No default.*

The person that holds the value.

#### `flake.safix.bridge.mappings.<name>.safix.name`

*Type:* string. *No default.*

The secret that person holds, as they hold it.

## What a mapped entry carries beside its value

The field submodule is shared by every target that has one: it is the value of
`kdbx.fields`, `pass.fields`, `bitwarden.fields` and `onepassword.fields`. Each
scalar takes a literal string or `{ entry = "<name>"; }`, which sources the field
from another entry of the mapping's own person, decrypted when the mapping is
converged and never at evaluation. A target that cannot carry a field refuses the
declaration at evaluation rather than writing less than it says.

#### `fields.username`

*Type:* null, a string, or an entry reference. *Default:* `null`.

The username to set on the far side's entry; `null` leaves the field alone.

#### `fields.url`

*Type:* null, a string, or an entry reference. *Default:* `null`.

The address the entry's credential is used at; `null` leaves the field alone.

#### `fields.notes`

*Type:* null, a string, or an entry reference. *Default:* `null`.

The entry's note, as free text; `null` leaves the field alone.

#### `fields.tags`

*Type:* list of strings or entry references. *Default:* `[ ]`.

The tags to set on the far side's entry; an empty list leaves them alone, and a
target that cannot carry tags refuses a non-empty list.

#### `fields.<field>.entry`

*Type:* string. *No default.*

Another entry of this mapping's own person, whose value becomes this field.

## The clan bridge

#### `flake.safix.bridge.clanFlake`

*Type:* null or path. *Default:* `null`.

The clan this consumer bridges to, as a flake reference, declared once for the
consumer; a second declaration is refused rather than resolved by taking the
first.

#### `flake.safix.bridge.mappings`

*Type:* attribute set of clan mappings. *Default:* `{ }`.

Every standing relationship between a clan var and a safix entry, keyed by the
mapping's own identifier.

#### `flake.safix.bridge.mappings.<name>.direction`

*Type:* one of `"clan-to-safix"`, `"safix-to-clan"`, `"two-way"`. *No default.*

Which way the value moves, written as its endpoints rather than as a verb either
tool speaks.

#### `flake.safix.bridge.mappings.<name>.clan`

*Type:* a submodule. *No default.*

The clan half, none of which evaluation verifies: a clan side that does not
resolve is refused when a transfer reaches the mapping.

#### `flake.safix.bridge.mappings.<name>.clan.placement`

*Type:* one of `"shared"`, `"per-machine"`. *Default:* `"per-machine"`.

Which of clan's own placements the var is declared under.

#### `flake.safix.bridge.mappings.<name>.clan.machine`

*Type:* null or string. *Default:* `null`.

The clan machine the var belongs to: required under `per-machine`, and refused
under `shared`, where the answering machine is discovered at run time.

#### `flake.safix.bridge.mappings.<name>.clan.generator`

*Type:* string. *No default.*

The clan generator that declares the var.

#### `flake.safix.bridge.mappings.<name>.clan.file`

*Type:* string. *No default.*

The file that generator declares, named as clan names it.

## KeePassXC

#### `flake.safix.keepassxc.database`

*Type:* null or string. *Default:* `null`.

The password database `safix sync` converges against, as an absolute path on the
machine the verb runs on and a string rather than a nix path, which would be
copied into the world-readable store.

#### `flake.safix.keepassxc.keyFile`

*Type:* null or string. *Default:* `null`.

A key file the database's own composite key requires to open, as an absolute path
and for the same reason a string rather than a nix path.

#### `flake.safix.keepassxc.yubikey`

*Type:* null or a submodule. *Default:* `null`.

A YubiKey challenge-response slot the database's own composite key requires, or
`null` when the database opens on its password alone; nothing here programs,
reprograms or deletes a slot.

#### `flake.safix.keepassxc.yubikey.slot`

*Type:* string. *No default.*

The challenge-response slot the composite key reads, as `keepassxc-cli`'s `-y`
flag takes it.

#### `flake.safix.keepassxc.yubikey.serial`

*Type:* null or string. *Default:* `null`.

The card's serial number, disambiguating which connected YubiKey answers the
challenge, or `null` to accept whichever one does.

#### `flake.safix.keepassxc.group`

*Type:* string. *Default:* `"safix"`.

The group every mapped entry's path is relative to; `safix sync` creates it and
the groups under it where they are absent, and removes none of them.

#### `flake.safix.keepassxc.mappings`

*Type:* attribute set of database mappings. *Default:* `{ }`.

Every standing relationship between a safix entry and an entry in the database,
keyed by the mapping's own identifier.

#### `flake.safix.keepassxc.mappings.<name>.mode`

*Type:* one of `"safix-to-keepassxc"`, `"keepassxc-to-safix"`, `"two-way"`,
`"backup"`. *No default.*

Which way this mapping converges, declared here rather than passed at the
invocation.

#### `flake.safix.keepassxc.mappings.<name>.kdbx`

*Type:* a submodule. *No default.*

The database half, which evaluation does not verify: the group and the entry are
content of an encrypted file.

#### `flake.safix.keepassxc.mappings.<name>.kdbx.path`

*Type:* string. *No default.*

Where the entry sits under the declared group; the last segment becomes the
entry's title and the leading segments are groups.

#### `flake.safix.keepassxc.mappings.<name>.kdbx.fields`

*Type:* the field submodule. *Default:* `{ }`.

What the entry carries beside its value; `tags` and an entry-sourced field are
both refused for this target, whose only channel for a field is an argument
vector.

## pass

#### `flake.safix.pass.store`

*Type:* string. *Default:* `"~/.password-store"`.

The `pass` store `safix sync` converges against; a leading `~` is expanded by the
runtime rather than by nix, because evaluation has no home to expand against.

#### `flake.safix.pass.mappings`

*Type:* attribute set of store mappings. *Default:* `{ }`.

Every standing relationship between a safix entry and an entry in the `pass`
store, keyed by the mapping's own identifier.

#### `flake.safix.pass.mappings.<name>.mode`

*Type:* one of `"safix-to-pass"`, `"pass-to-safix"`, `"two-way"`, `"backup"`.
*No default.*

Which way this mapping converges.

#### `flake.safix.pass.mappings.<name>.pass`

*Type:* a submodule. *No default.*

The store half, which evaluation does not verify: the entry is one gpg-encrypted
file.

#### `flake.safix.pass.mappings.<name>.pass.path`

*Type:* string. *No default.*

The entry path inside the declared store, as `pass` itself spells one: no leading
slash and no `.gpg` suffix.

#### `flake.safix.pass.mappings.<name>.pass.fields`

*Type:* the field submodule. *Default:* `{ }`.

What the record carries beside its value; this target carries every field,
because the whole body crosses on standard input.

## Bitwarden

#### `flake.safix.bitwarden.server`

*Type:* null or string. *Default:* `null`.

The vault server every mapping here is converged against, or `null` for whichever
server the operator's own client is configured against; a declared URL that is
not the one reached is refused before any side is read.

#### `flake.safix.bitwarden.mappings`

*Type:* attribute set of vault mappings. *Default:* `{ }`.

Every standing relationship between a safix entry and an item in the vault, keyed
by the mapping's own identifier.

#### `flake.safix.bitwarden.mappings.<name>.mode`

*Type:* one of `"safix-to-bitwarden"`, `"bitwarden-to-safix"`, `"two-way"`,
`"backup"`. *No default.*

Which way this mapping converges.

#### `flake.safix.bitwarden.mappings.<name>.bitwarden`

*Type:* a submodule. *No default.*

The vault half, none of which evaluation verifies: the folder and the item are
content of a vault behind a network service.

#### `flake.safix.bitwarden.mappings.<name>.bitwarden.folder`

*Type:* null or string. *Default:* `null`.

The vault folder the item sits in, or `null` for the vault's root, which is where
an item in no folder lives.

#### `flake.safix.bitwarden.mappings.<name>.bitwarden.item`

*Type:* string. *No default.*

The item's name as the person holding it sees it, never the vault's own
identifier.

#### `flake.safix.bitwarden.mappings.<name>.bitwarden.fields`

*Type:* the field submodule. *Default:* `{ }`.

What the item carries beside its value; `tags` is refused for this target, which
has no tag concept.

## 1Password

#### `flake.safix.onepassword.account`

*Type:* null or string. *Default:* `null`.

The account shorthand, sign-in address or user id every invocation names, or
`null` to let the program resolve its own; safix signs nothing in and holds no
session.

#### `flake.safix.onepassword.mappings`

*Type:* attribute set of vault mappings. *Default:* `{ }`.

Every standing relationship between a safix entry and an item in a 1Password
vault, keyed by the mapping's own identifier.

#### `flake.safix.onepassword.mappings.<name>.mode`

*Type:* one of `"safix-to-1password"`, `"1password-to-safix"`, `"two-way"`,
`"backup"`. *No default.*

Which way this mapping converges.

#### `flake.safix.onepassword.mappings.<name>.onepassword`

*Type:* a submodule. *No default.*

The 1Password half, which evaluation verifies neither side of, because both are
content of a remote service.

#### `flake.safix.onepassword.mappings.<name>.onepassword.vault`

*Type:* string. *No default.*

The vault the mapped item lives in, by name; there is no default, because a
service account cannot reach a built-in vault at all.

#### `flake.safix.onepassword.mappings.<name>.onepassword.item`

*Type:* string. *No default.*

The item's title inside that vault.

#### `flake.safix.onepassword.mappings.<name>.onepassword.fields`

*Type:* the field submodule. *Default:* `{ }`.

What the item carries beside its value; every field is carried, and a declared
url becomes the item's own autofill website.
