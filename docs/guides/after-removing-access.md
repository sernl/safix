---
title: "After removing access"
---

## What you will have

This page shows you the sequence that follows taking somebody out of an audience. At the end the declarations, the recipient policy and the ciphertext agree again, the value they could read has been replaced, and the machines that use it have picked up the new one.

Re-wrapping is not the point of the sequence; minting a new value is. See the revocation rule in [Custody](../concepts/custody.md).

## Steps

1. Remove the access. For a group, the command edits the `members` list in that group's own module file and commits it with a regenerated `.sops.yaml`:

   ```console
   $ safix group remove oncall bob
   ```

   For a grant, delete the declaration:

   ```nix
   {
     # was: flake.safix.users.alice.sharedWith.bob.deploy-key = { };
   }
   ```

   See [`flake.safix.groups.<name>.members`](../reference/declarations.md#flakesafixgroupsnamemembers) and [`flake.safix.users.<name>.sharedWith`](../reference/declarations.md#flakesafixusersnamesharedwith).

2. Read the shrink:

   ```console
   $ safix check
   ```

   The report names each file whose recipients the declarations no longer carry, lists whose keys those are, and names what mints a new value for each name the file holds.

3. Align the ciphertext with the policy:

   ```console
   $ safix fix
   ```

   This regenerates `.sops.yaml`, then re-wraps each governed file's data key to the audience the policy declares. It commits nothing, so the diff is yours to read first.

4. Mint a new value for every name that person could read. For a hand-typed value:

   ```console
   $ safix set alice deploy-key
   ```

   For a generated one, which also re-runs everything downstream of it:

   ```console
   $ safix rotate alice deploy-key
   ```

   [Rotating secrets](rotating-secrets.md) covers the scheduled form of this step, and what `rotate` refuses for a value no generator mints.

5. Let the machines take the new value. Each profile's next rebuild installs it, and the units you declared on the entry are acted on because the installed value changed:

   ```nix
   {
     flake.safix.catalogue.deploy-key.restartUnits = [ "deploy.service" ];
   }
   ```

   See [Templates and services](templates-and-services.md) for when a unit is restarted and when it is not.

## Which names need a new value

Every name in every file whose audience that subject was in. One document has one data key, so the whole file was readable, not only the entry you had in mind.

`safix check` answers this for you: each finding of a narrowed audience carries the list of names the file holds and how each is minted. Work that list rather than your memory of what they used.

A value nobody outside the remaining audience has ever held is not on the list, because the file never named them.

## What can go wrong

- `safix::unknown_group` and `safix::unknown_subject` — the group or the member you named is not declared.
- `safix::no_group_declaration` — no module file declares that group's `members`, so there is nothing to edit.
- `safix::unparsable` — the group's module file is not in a shape the editor can rewrite. Edit it by hand and rerun `safix fix`.
- `safix::actor_undeclared` — the commit identity the repository resolves is not a person the declarations name. `git config user.name` is the remedy.
- `safix::recipient_drift` — raised by `check` for exactly the shrink from step 1; it clears after step 3.
- `safix::cascade_declined` — you were shown the set of downstream generators the rotation would re-run and declined. Nothing ran.
- `safix::no_value_yet` — the name has no value to replace, which is a state of the declarations rather than of the removal.

Every code is listed in [Refusals](../reference/refusals.md).
