---
title: "Syncing to password managers"
---

## What you will have

This page shows you how to keep a declared entry and one entry in another store from drifting apart. At the end a mapping names both sides, `safix audit` tells you whether they agree, and `safix sync` converges the one that has not moved.

## A mapping

A mapping is a declared relationship. It names one safix entry, one address in the target's own store, and the mode the two converge under.

```nix
{
  flake.safix.keepassxc.mappings.grafana = {
    mode = "two-way";
    safix = {
      user = "alice";
      name = "grafana-password";
    };
    kdbx = {
      path = "alice/grafana";
      fields = {
        username = "alice@example.com";
        url = "https://grafana.example";
        notes = "where this credential came from";
      };
    };
  };
}
```

The mapping's identifier is its own name and belongs to neither endpoint. It is the word you pass to narrow a run, and an identifier colliding with a target's own keyword is refused at evaluation.

## The modes

Four, and a mode is written as its two endpoints in the order the value moves. The endpoints are this package's own name and the target's word.

| mode | what it does |
|---|---|
| `<target>-to-safix` | the safix entry converges to the target's value |
| `safix-to-<target>` | the target converges to the safix entry's value |
| `two-way` | converges toward whichever side changed since the last agreement |
| `backup` | writes safix's value where the target holds none, and never overwrites one that differs |

So `clan-to-safix` moves a value out of clan and into a safix entry, and the same pair with the endpoints swapped moves it the other way.

Every run reads both sides before it writes either, so a mapping whose sides agree is not written and not committed.

## How a conflict is judged

The last agreement is remembered beside the mapped entry, as one line holding a format tag and a fingerprint of the agreed value. The fingerprint never reaches safix's own plaintext trees.

Where the target can hold a hidden field on the item itself, that memory lives there. Where it cannot, it is a companion entry named for the mapped one plus a reserved suffix. Deleting a companion is safe and returns the mapping to bootstrap.

A conflict is a finding and never a guess. Where both sides have moved since the last agreement, nothing is written, and the report names the two one-way modes that each resolve it.

## What is never deleted

Nothing, on either side, in any mode. Remove a mapping and its last value on the target stays until a person removes it, and the report says that nothing declares it.

## Fields

The far side also carries fields, which is everything beside the value that a store shows a person.

Each field is either a literal string or `{ entry = "<name>"; }` naming another entry of the mapping's own person, decrypted at run time rather than interpolated at evaluation.

Fields are declarations, so they have one author. A pulling mode writes only the value into safix, because a safix entry is a placement with no slot for a URL. A pushing mode writes the declared fields beside the value in the same write. `backup` writes them only where it writes a value, and a two-way mapping's fields are push-only.

A field the target cannot carry is refused at evaluation, naming the target and the field. A field whose source is another entry is a secret value, so a target whose only channel for that field is an argument vector refuses it as well.

## The refusals every target shares

Four are declared the same way everywhere, and the per-target lists below name only what is theirs:

- a mapping whose safix side does not resolve;
- a pull into a generator-produced value, because the generator is the author;
- two mappings writing one address on the target;
- a declared address carrying the reserved companion suffix.

All refusal codes are listed in [Refusals](../reference/refusals.md).

### clan

#### What it addresses

A var inside another flake, by the machine that owns it, the generator that produces it, and the file within that generator.

#### How it is declared

[`flake.safix.bridge.clanFlake`](../reference/declarations.md#flakesafixbridgeclanflake) holds the other side, and each mapping under [`flake.safix.bridge.mappings`](../reference/declarations.md#flakesafixbridgemappings) names `clan.machine`, `clan.generator` and `clan.file`.

This target spells its mode `direction`, which predates the shared vocabulary and is the one place the word differs.

`clan.placement = "shared"` says the clan side is one var no machine owns, so `clan.machine` is refused and the runtime tries each machine clan lists until one resolves.

`safix sync clan` converges every mapping, naming mappings narrows the run, and `--direction` narrows it to mappings declared with that value.

#### What it can carry

Nothing beside the value, and the heading stays to say so. A clan var is a file's bytes, and clan's own command offers no field beside it.

#### How it unlocks

Nothing of its own. Every read and every write is clan's own command with the value on a pipe, so clan's credentials and backends apply unchanged.

safix reads, writes, encrypts, decrypts and parses none of clan's stored files, and a consumer without clan's command cannot reach clan's side at all.

#### What it refuses

Locally: one pair of endpoints declared both ways, and a mapping with no clan flake.

At transfer time: a clan side that does not resolve, refused in clan's own words, and a push out of an entry holding no value.

A push into a generator clan considers outdated is refused with no override, because clan's next routine generation would replace what was written without saying so. The refusal names both remedies.

### keepassxc

#### What it addresses

A path inside an encrypted database on this machine, under a declared group.

#### How it is declared

[`flake.safix.keepassxc.database`](../reference/declarations.md#flakesafixkeepassxcdatabase) is a string rather than a nix path, because a path is copied into the world-readable store on every evaluation and this file is large.

[`flake.safix.keepassxc.group`](../reference/declarations.md#flakesafixkeepassxcgroup) is the group entries live under, [`flake.safix.keepassxc.yubikey`](../reference/declarations.md#flakesafixkeepassxcyubikey) names a challenge-response slot, and [`flake.safix.keepassxc.keyFile`](../reference/declarations.md#flakesafixkeepassxckeyfile) names a key file.

Each mapping addresses its entry through `kdbx.path` and declares its far side under `kdbx.fields`.

#### What it can carry

`username`, `url` and `notes`, as literals only.

`tags` is refused, because this store's command has no flag for one. A field sourced from another entry is refused because the only channel here is an argument vector.

#### How it unlocks

With a composite key, asked for once per run on the terminal, and the run refuses before reading anything where there is none.

The password travels standard input, and so does every value. The session's secret service is not a second way in, because the collection it publishes is its own exposed group.

#### What it refuses

A value carrying a newline, because the store's command reads a password as one line and nothing here trims the byte for you.

No database is created, no database key is changed, and no hardware slot is written under any flag.

### pass

#### What it addresses

A path under a store root, which is one gpg-encrypted file per entry.

#### How it is declared

[`flake.safix.pass.store`](../reference/declarations.md#flakesafixpassstore) is the store root, a string with `~` expanded by the runtime. It reaches the store's command in the child's environment as a path and never as a value.

Each mapping names `pass.path` and its `pass.fields`.

#### What it can carry

All four fields, including a field sourced from another entry, because the whole record crosses on one pipe as a value followed by a trailing block of fields.

A multi-line value crosses whole, because this store's read is byte-exact and imposes no one-line rule.

#### How it unlocks

Nothing of its own, and the heading stays to say so. The store shells to gpg, so the unlock belongs to the ambient agent.

The preflight is that the store exists, and a locked or refusing agent is reported as exactly that rather than as a generic command failure.

#### What it refuses

Nothing beyond the shared four. Nothing here initialises a store or manages its recipients, because a store's own recipient file is its audience declaration.

### bitwarden

#### What it addresses

An item in a personal vault, by an optional folder and the item's own name rather than by the store's item id, because an opaque identifier is not a reviewable declaration.

#### How it is declared

[`flake.safix.bitwarden.server`](../reference/declarations.md#flakesafixbitwardenserver) names a self-hosted server, and `null` means whatever server the operator's own client is configured against.

Each mapping names `bitwarden.folder`, `bitwarden.item` and `bitwarden.fields`. The last agreement lives in a hidden custom field on the item.

#### What it can carry

`username`, `url` and `notes`, each as a literal or sourced from another entry, crossing as JSON on standard input in both directions.

`tags` is refused, because this store has no tag concept: folders and collections are the only grouping, and both are placements rather than labels.

#### How it unlocks

By prompting once for the master password, which travels the child's standard input. The session key it returns travels to every later child in that child's environment and nowhere else.

That is the one place a value-bearing environment variable is accepted. An argument vector is world-readable through `/proc`, where an environment variable is readable by the same account alone.

No master password and no mapped value is in an argument vector or an environment on any invocation, safix never logs a vault in, and an unauthenticated client is reported as locked.

#### What it refuses

Two items of one name in one folder, reported as ambiguous rather than guessed at.

A declared server differing from the one the unlocked session reached.

A failed pre-read synchronisation, since a stale local copy would be compared as though it were the vault.

An absent item under a mode that needs one.

### 1password

#### What it addresses

An item in a named vault.

#### How it is declared

[`flake.safix.onepassword.account`](../reference/declarations.md#flakesafixonepasswordaccount) is an account shorthand or sign-in address, and `null` means whatever account the command itself resolves.

Each mapping names `onepassword.vault`, `onepassword.item` and `onepassword.fields`. The last agreement lives in a concealed field on the item.

#### What it can carry

All four fields, including a field sourced from another entry, because the whole item crosses as JSON on standard input.

safix never spells a `field=value` argument word, since this store's own documentation records that such assignments are logged in shell history.

An edit is a round-trip rather than a template: a write starts from the item's own JSON, replaces only the declared fields, the value and the memory, and writes the whole object back.

#### How it unlocks

By inheriting a session the operator already established, or a service-account token, from safix's own environment.

safix runs no sign-in of its own and passes no session token in an argument vector. A signed-out run is refused before any mapping's safix side has been decrypted.

#### What it refuses

An absent vault, named as such rather than reported as a failed command.

An absent item under a mode that needs one.

No verb this target issues deletes anything.

## `audit` against `check`

`safix audit <target>` compares both sides of every declared mapping, or of the ones named after the target, and changes nothing on either side.

A mapping agrees when both sides hold the same bytes, and also when neither side holds a value yet, which is a relationship nobody has bootstrapped.

A divergence names the mapping, its two endpoints and the command that converges it, and never a value. Where the values agree and a declared field does not, the report names the diverged field and not its content, because a note may itself be sensitive.

A mapping that could not be judged is reported as such rather than quietly left out. A report that dropped those would be a report about who ran it.

Alongside the findings, `audit` names every entry under the declared address space that no mapping accounts for. That is information, and it never moves the exit status.

`audit` is a verb of its own rather than more rows in `safix check`. `check` decrypts nothing and needs no target, while comparing a mapping's sides decrypts safix's side and runs the target's own command.
