---
title: "flake-parts"
---

## What you will have

This page shows you how to reach safix's declarations through flake-parts, including a tree that scatters one declaration per file. At the end your flake exposes `flake.safix.lib`, your profiles read it, and no module in your tree enumerates another.

The flake-parts route is fully supported. It produces the same projection a plain flake produces, so nothing about your declarations, your files or your generated policy differs between the two.

## Steps

1. Import the flake module:

   ```nix
   {
     inputs.safix.url = "github:you/safix";

     outputs =
       inputs@{ flake-parts, safix, ... }:
       flake-parts.lib.mkFlake { inherit inputs; } {
         systems = [ "x86_64-linux" ];
         imports = [ safix.flakeModules.default ];

         flake.safix.users.alice = {
           recipient = "age1...";
           private.alice-token = { };
         };
       };
   }
   ```

   `flake.safix.lib` now holds the audiences, the placements, the generated policy text and the check builders, and `packages.safix` is the command.

2. Declare anywhere. `flake.safix.*` is an ordinary flake-parts option namespace, so any module in your `imports` may set it:

   ```nix
   {
     flake.safix.machines.web = {
       recipient = "age1...";
       owner = "alice";
     };
   }
   ```

   Declarations merge, so that block lives in its own file beside a hundred others. safix finds them through the module system and reads no path, no filename and no directory structure to do it.

3. Consume the projection from a profile:

   ```nix
   {
     imports = [ inputs.safix.nixosModules.default ];

     safix.flake = inputs.self;
     safix.machine = "web";
   }
   ```

   [`safix.flake`](../reference/profile-options.md#safixflake) is the one thing a module cannot derive, because requiring a particular name in a profile's arguments would make your evaluation seam part of safix's interface. Where your flake reaches the profile some other way, set [`safix.lib`](../reference/profile-options.md#safixlib) directly.

4. Add checks over your own declarations:

   ```nix
   { config, ... }:
   {
     perSystem =
       { pkgs, ... }:
       {
         checks = config.flake.safix.lib.mkChecks pkgs { committedPolicy = ./.sops.yaml; };
       };
   }
   ```

   With no arguments `mkChecks` returns checks over the declarations alone. `committedPolicy` adds the check that fails while the committed and the generated policy differ, naming `safix fix`. `materializations` adds the path-collision check, and forces what you hand it so the refusal reaches hosts nobody has built this week.

## A dendritic layout

A tree that scatters one declaration per file does not enumerate them. Read the directory instead:

```nix
{
  outputs =
    inputs@{ flake-parts, nixpkgs, safix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        safix.flakeModules.default
      ]
      ++ nixpkgs.lib.filesystem.listFilesRecursive ./modules;
    };
}
```

A hand-written list beside a discovered one drifts on the next added file, so the discovered one is the whole list.

Each file under `./modules` is an ordinary flake-parts module setting one thing:

```nix
{
  flake.safix.groups.oncall.members = [
    "alice"
    "bob"
  ];
}
```

`examples/dendritic/` is a worked copy of exactly this: one fleet, one declaration per file, read by directory, with `flake.nix` naming no path under `./modules`.

## The same projection without flake-parts

`flake.lib.mkVault` evaluates a list of modules together with safix's own resolver and returns exactly what a flake-parts consumer's `flake.safix.lib` holds.

```nix
{
  safix = {
    lib = (import <safix>).lib.mkVault {
      modules = [ ./secrets.nix ];
      root = ./.;
    };
    onboardingHook = null;
    enrollHook = null;
  };
}
```

Declarations passed through `modules` scatter and merge exactly as a flake-parts `imports` list would. The command reads such a file through `--entry`, or its environment form `SAFIX_ENTRY` — see [Environment](../reference/environment.md). `examples/plain-nix/` is a working copy.

Choose the route your tree already uses. Neither is a lesser one, and the generated policy and the placements are byte-identical between them.

## Projecting from a registry you already have

See the namespace rule in [The model](../concepts/model.md). What it costs you, where a registry of people already exists, is one projection written once in your own tree:

```nix
{ config, lib, ... }:
{
  flake.safix.users = lib.mapAttrs (_name: person: {
    recipient = person.meta.ageRecipient;
    recoveryRecipients = lib.mapAttrs (_anchor: key: { inherit key; }) person.meta.ageRecoveryKeys;
  }) config.flake.users;
}
```

[`flake.safix.users`](../reference/declarations.md#flakesafixusers) is safix's own record and carries only custody, so a person can exist in your registry and hold nothing here. `flake.safix.machines` takes a projection on the same terms from a host inventory, and `flake.safix.services` from whatever record already says which units run where.

## What can go wrong

- `safix::nix_eval_failed` — the declarations did not evaluate. The nix error is passed through.
- `safix::nix_schema_mismatch` — the projection evaluated to a shape this build does not read, which is what a mismatched safix input looks like.
- `safix::no_declaration_file` — a command that edits declarations found no module file to edit.
- `safix::generate_needs_nixpkgs` — running `generate` under `--entry` with neither `--nixpkgs` nor `SAFIX_NIXPKGS` set. The generator sandbox resolves its tools through a flake.
- A module passed to `mkVault` that declares an option outside safix's namespace is refused at evaluation.
- A profile bound to declarations that names neither a person nor a host refuses, and one that names either while bound to nothing refuses naming `safix.flake`.

Every code is listed in [Refusals](../reference/refusals.md).
