# Three consumers of one fleet

`examples/plain-nix/fleet.nix` is the only fleet in this repository.
`examples/dendritic/` declares the identical fleet again, scattered one declaration per file, and `examples/profiles/` consumes that same fleet from a NixOS profile and a home-manager profile.
Two checks read them: `safix-examples` compares the first two field for field and probes the features they declare, and `safix-examples-profiles` evaluates the third.

## `plain-nix`

`fleet.nix` declares the whole fleet in one file.
`hooks.nix` declares `onboardingHook` and `enrollHook` directly, beside `lib` rather than inside it, per design decision D4 in `openspec/changes/support-plain-nix-consumers/design.md`.
`entry.nix` is the `--entry` target itself.
It reaches `mkVault` by importing `../../lib` — a plain function of `{ lib }` rather than a flake output — and supplies its own `lib` from `<nixpkgs>`, which `nix eval --file` already resolves through `NIX_PATH`.
No `config.flake`, no `inputs.safix`, no flake-parts and no `builtins.getFlake` appear in it.

```console
$ safix --entry examples/plain-nix/entry.nix list
```

Export `SAFIX_ENTRY=examples/plain-nix/entry.nix` to drop the flag from every later invocation.
Every verb behaves identically under `--entry` as against a flake; `generate` additionally needs `--nixpkgs <flake-ref>` or `SAFIX_NIXPKGS`, because the generator sandbox resolves its tools through a flake regardless of how the declarations were reached.

Copy this one if your tree has no flake at all, or a flake that does not use flake-parts.

## `dendritic`

A complete flake-parts flake.
`flake.nix` imports `safix.flakeModules.default` and reads its own module directory with `lib.filesystem.listFilesRecursive ./modules`, so adding a declaration is one new file and no import line.
Each file under `modules/` declares exactly one thing and reads no path, no filename and no other file; the module system merges them into the fleet `plain-nix/fleet.nix` declares in one file.

Copy this one if your tree already uses flake-parts, or if you want your own declarations to scatter the way safix's own opinion says they may.

## `profiles`

`nixos.nix` serves the machine `deck` at system scope and `home.nix` serves `alice` at user scope; both bind `safix.lib` to `mkVault` over `../plain-nix/fleet.nix` rather than declaring a fleet of their own.
`relocated.nix` is a fleet-wide overlay merged only for the check's own evaluation: it moves the three storage roots and declares a vault, so the shared fleet stays the readable default the documentation is written against.
`profiles/README.md` says what each of the three files is for.

Copy these if you want a worked example of the `safix.*` consumption surface at either scope.

## What each check reads

`safix-examples` executes `plain-nix/entry.nix` for real, inside a sandbox holding only `examples/plain-nix`, the top-level `lib/` and `modules/flake/safix`.
It reads the dendritic side by evaluating every `.nix` file under `dendritic/modules/` through `lib.evalModules`, discovered by directory rather than listed.
`examples/dendritic/flake.nix` is therefore not evaluated by any check: it names `inputs.safix.url = "path:../.."`, so evaluating it inside the sandbox would resolve this repository's whole input closure a second time with no network.
It is read as text instead, for one assertion — that it contains no `./modules/` path, so the hand list cannot come back and cannot silently shrink.

`safix-examples-profiles` evaluates `profiles/nixos.nix` through a real `nixosSystem` and `profiles/home.nix` through home-manager's own library, and compares what each profile materializes against literals.
It is separate from `safix-examples` because `safix.secrets` is a materialization under a scope rather than a field of the pre-scope projection those two consumers are compared on.

Root-dependent absolute strings are elided to `<example-root>` before the two projections are diffed, because `bridge.clanFlake` and `vault.root` are `lib.types.path` and stringify against each example's own root.
Only those two prefixes are elided; a store path anywhere else is still compared, and a divergence in the tail of such a string still fails.

## What the examples cover, and what holds each claim

Every row names an assertion.
A feature with no assertion is a gap to close in the check rather than a row to write here.

| feature | declared in | held by |
|---|---|---|
| an organization as a grant audience | `examples/dendritic/modules/users/alice/shared-with-acme.nix` | `coverageProbes.organizationAudience`, `.organizationGrantFile` |
| an `ownerOf.<m>` grant | `examples/dendritic/modules/users/alice/shared-with-owner-of-rack.nix` | `coverageProbes.ownerOfAudience`, `.ownerOfGrantFile` |
| a person's own recovery identity | `examples/dendritic/modules/users/alice/recovery-master.nix` | `coverageProbes.recoveryRecipient` |
| a file no declaration implies | `examples/dendritic/modules/extra-governed.nix` | `coverageProbes.extraGovernedFile` |
| delegation, both consenting sides | `examples/dendritic/modules/organizations/acme-managers.nix`, `…/users/bob/managed-by-acme.nix` | `coverageProbes.delegationManagers`, `.delegationManagedBy` |
| a generator prompt | `examples/dendritic/modules/users/alice/private-prompted-token.nix` | `coverageProbes.generatorPrompt` |
| a chained generator | `examples/dendritic/modules/users/alice/private-derived-token.nix` | `coverageProbes.generatorDependencyEdge` |
| a validation fragment | `examples/dendritic/modules/users/alice/private-validated-token.nix` | `coverageProbes.generatorValidation` |
| a generator that reaches the network | `examples/dendritic/modules/users/alice/private-fetched-token.nix` | `coverageProbes.generatorNetwork` |
| a public output beside a secret one | `examples/dendritic/modules/users/alice/private-wg-key.nix` | `coverageProbes.generatorMultiOutput`, `.publicOutputPath`, `.secretOutputIsNotPublic` |
| a machine, a service and a nested group as subjects | `examples/dendritic/modules/machines/rack.nix`, `…/services/web.nix`, `…/groups/infra.nix` | `coverageProbes.subjectMachines`, `.subjectServices`, `.subjectGroups`, `.nestedGroupMembers` |
| a silo over two groups | `examples/dendritic/modules/silos/corp.nix`, `…/groups/contractors.nix` | `comparedFields.policyText`, `comparedFields.subjects` |
| a clan mapping | `examples/dendritic/modules/sync/clan.nix` | `coverageProbes.bridgeMappings` |
| a keepassxc mapping and its fields | `examples/dendritic/modules/sync/keepassxc.nix` | `coverageProbes.keepassxcMappings`, `.keepassxcFields` |
| a pass mapping, all four fields, one of them an `{ entry = …; }` source | `examples/dendritic/modules/sync/pass.nix` | `coverageProbes.passMappings`, `.passFields` |
| a bitwarden mapping, three literal fields and no tags | `examples/dendritic/modules/sync/bitwarden.nix` | `coverageProbes.bitwardenMappings`, `.bitwardenFields` |
| a 1password mapping, all four fields, one of them an `{ entry = …; }` source | `examples/dendritic/modules/sync/onepassword.nix` | `coverageProbes.onepasswordMappings`, `.onepasswordFields` |
| both hooks, on both sides | `examples/plain-nix/hooks.nix`, `examples/dendritic/modules/hooks.nix` | `hooksAreNotEmpty`, and the hook fields of the field-for-field diff |
| no hand-maintained module list | `examples/dendritic/flake.nix` | `noHandList.offendingLines` |
| an entry `mode`, invisible to the projection | `examples/dendritic/modules/catalogue/deploy-key.nix` | `safix-examples-profiles`' `homeSecrets` |
| a `perHost.<h>.add` adjusting placement | `examples/dendritic/modules/users/alice/per-host-deck-add.nix` | `safix-examples-profiles`' `homeSecrets` |
| an entry `path`, applied under its own scope | `examples/plain-nix/fleet.nix` | `safix-examples-profiles`' `entryPath` |
| the ownership axis, and its user-scope refusal | `examples/dendritic/modules/services/web.nix` | `safix-examples-profiles`' `ownershipAxis`, `ownershipRefused` |
| the identity surface at both scopes | `examples/profiles/nixos.nix`, `examples/profiles/home.nix` | `safix-examples-profiles`' `identity` |
| the installer surface at both scopes | `examples/profiles/nixos.nix`, `examples/profiles/home.nix` | `safix-examples-profiles`' `installerSurface` |
| relocated storage roots, and a vault | `examples/profiles/relocated.nix` | `safix-examples-profiles`' `relocatedStorage`, `relocatedVault` |

Every dendritic file above has a counterpart block in `examples/plain-nix/fleet.nix`: a feature added to one side and not the other fails `safix-examples`, which is the check working.
