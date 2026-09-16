# examples/profiles

Three files, and no fleet of their own.

Both profiles bind `safix.lib` to `../plain-nix/fleet.nix` — the one fleet in
this repository, the same file `../dendritic/modules/` re-declares one
statement per file. A feature added there is a feature these profiles resolve,
with nothing to keep in agreement by hand.

## `nixos.nix`

A system-scope profile serving the machine `deck`. It names the subject it
serves, the host it resolves on, and the projection it is bound to, and it sets
every `safix.installer.*` option to something other than its default so that
what the profile asked for is readable in the manifest it builds.

This is the scope with an ownership axis: the entries the service `web` carries
onto `deck` arrive owned by `web:web`.

## `home.nix`

A user-scope profile serving `alice` on `deck`, over the same declarations.
It names the tags the host carries, the identity alice decrypts with, and the
user-scope installer surface.

It declares no ownership field on any entry, and none may be declared: only
system scope has an ownership axis, so the user-scope materialization refuses
an entry carrying `owner` or `group` rather than dropping it, naming the entry
and the field.

## `relocated.nix`

The two fleet-wide declarations — `flake.safix.storage` and
`flake.safix.vault` — as a module merged beside the shared fleet rather than
into it. Both relocate or rename everything: three roots move every path, and a
naming key makes every resolved name a keyed hash. Declaring either in the
shared fleet would stop it showing the readable layout the documentation is
written against.

`vault/` is the vault's root, committed empty: nothing reads a document out of
it at evaluation.

## What evaluates these

`safix-examples-profiles` (`modules/flake/checks/examples-profiles.nix`). It
evaluates `nixos.nix` through a real `nixosSystem` and `home.nix` through
home-manager's own library, and compares the resolved result against literals.

It is separate from `safix-examples`, which compares the two declaration-side
consumers to each other. What a profile resolves is a materialization under a
scope, and the fields that check compares are the pre-scope projection — so an
entry's `mode`, an entry's `path`, a `perHost.<h>.add` override and a `perTag`
`force` are observable here and nowhere there.
