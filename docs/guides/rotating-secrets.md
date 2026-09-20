---
title: "Rotating secrets"
---

## What you will have

This page shows you how to declare how long a value may live, see which values have outlived that, and mint new ones — by hand or on a timer. At the end a policy governs a class of entries, `safix list` and the picker count each one down, `safix check` reports the ones past their deadline, and a user timer on your workstation re-mints what a generator can mint.

A deadline is not custody. Assigning one encrypts nothing and takes nothing back; see the revocation rule in [Custody](../concepts/custody.md). What rotation adds is that an old value is reported as old, and that minting its replacement is one verb.

## Steps

1. Declare a policy. A policy is an interval, named once, so shortening it is one edit rather than one per entry:

   ```nix
   {
     flake.safix.rotation = {
       quarterly.every = "90d";
       weekly.every = "7d";
     };
   }
   ```

   The interval is a positive whole number of hours, days or weeks — `<n>h`, `<n>d` or `<n>w`. See [`flake.safix.rotation.<name>.every`](../reference/declarations.md#flakesafixrotationnameevery).

2. Opt an entry in, by naming the policy in the entry's own declaration:

   ```nix
   {
     flake.safix.users.alice.private.grafana-secret-key.rotation = "quarterly";
   }
   ```

   The same field exists on a per-host or per-tag override, where it replaces the entry's policy in that scope alone. See [`flake.safix.catalogue.<name>.rotation`](../reference/declarations.md#flakesafixcataloguenamerotation).

   The command does the same edit, to the entry's block in that person's declaration file, and commits it:

   ```console
   $ safix rotation set alice grafana-secret-key quarterly
   ```

   `safix rotation unset alice grafana-secret-key` removes the line again. Neither form writes a value, encrypts anything or re-wraps anything.

3. Read the deadlines:

   ```console
   $ safix list alice
   ```

   The last column is `ROTATES`. Its cell reads `12d 04:12:09` while a value has time left, days alone beyond a year, `due` once the deadline has passed, and `-` for an entry naming no policy.

4. Mint a new value for what is due:

   ```console
   $ safix rotate --due --yes
   ```

## What a deadline is

The deadline is the value's last recorded write plus the interval its policy declares. The write is read from the timestamp record safix keeps beside each value, in the clear, which is the same record `list` reads its date columns from.

An entry with a policy and no timestamp record is due at once. That is the only reading that cannot silently extend a value's life: a deadline that had not started yet would let a policy applied to an old value wait out a whole interval it never spent.

Shortening a policy's interval moves every entry naming it at the next evaluation, and no entry is edited.

## Seeing what is due

`safix list` prints `ROTATES` for every entry, and the picker holds the same column behind Tab, beside `FILE`:

```console
$ safix view alice
```

Inside the picker the countdown ticks: every cell on screen is recomputed on each frame from one clock, so a row crosses into `due` while you watch. A row past its deadline is drawn red, and that colour outranks the tints for the entry you chose last and the newest one. See [`view`](../reference/cli.md#view).

## What `check` reports

`safix check` reports a value past its deadline as drift, naming the entry, the policy and how long ago the deadline passed:

```console
$ safix check alice
```

The finding is arithmetic over a declared interval and a plaintext timestamp, so `check` opens no ciphertext to produce it and needs no identity.

Each finding carries one remedy rather than two, because the entry already decides which it is. A generator-backed value is given `safix rotate <user> <name>`. A value nothing mints is given `safix set <user> <name>`, because only a person can supply it.

## Rotating by hand

The named form rotates one entry now:

```console
$ safix rotate alice grafana-secret-key
```

The value travels exactly the path `safix generate` writes one on: the same sandbox, the same pipe, the same per-generator commit, and the same definition and timestamp records. What the verb adds is why the generator ran.

Every generator reading the rotated value re-runs, in the plan's own order. You are shown that set and asked before the first commit, because declining afterwards takes nothing back out of history. `--yes` answers the question in advance.

The bulk form takes everything past its deadline, for one person or for all of them:

```console
$ safix rotate --due alice
```

It merges the cascades of everything due into one ordered set, announced once. Nothing due is a quiet success: it says so and exits zero.

A value no generator declares is not minted here. The named form refuses and names `safix set`. `--due` lists every due typed entry with the `safix set` each one needs and exits zero, because refusing over one would leave the values it could mint unminted.

## The workstation timer

The home-manager profile installs a systemd user timer that runs `safix rotate --due --yes` for you:

```nix
{
  safix.rotation = {
    enable = true;
    repository = "/home/alice/fleet";
    onCalendar = "Mon *-*-* 03:00:00";
    environment.SOPS_AGE_KEY_FILE = "/home/alice/.config/sops/age/keys.txt";
  };
}
```

Each option is described in [Profile options](../reference/profile-options.md#scheduled-rotation). The timer is off by default and independent of `safix.enable`: this is the workstation where the declarations and your git identity are, and a machine that only consumes secrets holds neither.

`repository` is the mutable working tree the unit runs in, as an absolute path in a string rather than a nix path, so the repository is not copied into the nix store. `onCalendar` is taken as systemd takes it, and the timer is persistent, so a machine that was asleep at the hour catches up on its next boot.

The identity in `environment` must decrypt without a prompt and without a card. A unit has no terminal and no pinentry, so a rotation needing either fails the unit with the underlying refusal rather than hanging, and leaves the repository uncommitted. Put a key file's path here, never a value: this attribute set is rendered into the world-readable nix store.

The timer commits locally and pushes nothing. Reviewing the commits and pushing them are yours, and the machines take the rotated values on their next rebuild.

## What can go wrong

- `safix::unknown_rotation_policy` — `rotation set` was given a policy the declarations do not define. It lists the ones they do, and refuses before reading anything, because an entry naming an undeclared policy is refused at the next evaluation.
- `safix::unknown_name` and `safix::unknown_user` — the entry or the person named is not declared.
- `safix::no_entry_declaration` — the entry is declared somewhere other than that person's own declaration file, or computed rather than written. Edit it by hand.
- `safix::unparsable` — the declaration file is not in a shape the editor can rewrite. The file is put back as it was.
- `safix::uncommitted_changes` — the tree is dirty. Both verbs refuse one, which is why a timer firing over uncommitted work fails visibly and rotates nothing.
- `safix::no_generator` — `safix rotate <user> <name>` was given a value no generator declares. `safix set` is the remedy, and `--due` reports the same entry rather than refusing over it.
- `safix::cascade_declined` — you were shown the downstream generators the rotation would re-run, and declined. Nothing ran.
- `safix::actor_undeclared` — the commit identity the repository resolves is not a person the declarations name, or not one of the managers an organization declares for this person.

Every code is listed in [Refusals](../reference/refusals.md).
