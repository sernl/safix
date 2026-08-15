# The bridge: declared mappings, and clan kept as the authority on its own store

## Why

The operator's requirement is bidirectional and explicit: safix must be able to "take secrets from clan vars for nixos and or use/transfer them for a home-manager module and/or nixos host", and equally to "generate secrets also and have them populated in clan vars for clan vars to use too" — "top to bottom, bottom to top".
"import and export is a requirement".

`clan-generator-contract` makes the two systems agree on what a generator is.
This change makes values move between them.

There is a working prototype of one direction already, and it is instructive rather than reusable.
`modules/flake/agents/agents.sh` in the dotfiles repository moves service tokens from clan vars into the workstation's safix file, and it does it with its own `sops set --value-stdin` invocation, its own temp-file registry, its own shredder and its own trunk-branch guard — a second implementation of things safix does, living in a shell script, with the mapping between the two sides hardcoded as a `MIRROR_SOPS_KEY` table.
That is the shape this change replaces: the mapping becomes a declaration, and the transfer becomes a verb.

## What Changes

- A bridge surface in nix. Each mapping names a clan side (`machine`, `generator`, `file`), a safix side (`user`, `name`), and a direction. Mappings are declarations, so they are diffable, reviewable, and checkable at evaluation — which a repeated CLI invocation with arguments is not.
- Direction is written absolutely — `clan-to-safix` or `safix-to-clan` — rather than as `import` or `export`. The verbs stay `safix import` and `safix export`, named relative to safix as every other verb is; the *declaration* does not get to be relative, because the word `export` moves data in opposite directions across the clan boundary depending on which tool says it, and a declaration is read by people who do not have that context loaded.
- `safix import` moves declared `clan-to-safix` mappings into safix, writing through the existing `set` path so the recipient-drift refusal, the scratch-and-rename write and the per-secret commit all apply unchanged.
- `safix export` moves declared `safix-to-clan` mappings into clan by invoking the clan CLI as a subprocess with the value on a pipe. Nothing here reimplements clan's store, its encryption, or its layout, exactly as nothing in safix reimplements the sops file format.
- Both directions read the clan side through the clan CLI. This is a deviation from the brief and is argued below rather than assumed.
- Both verbs converge. A mapping whose two sides already agree is not written and not committed; a run reports per mapping whether it was unchanged, updated, absent at source, or refused; and a second run immediately after a successful one writes nothing.
- `safix check` gains bridge rows, reporting a mapping whose two sides have diverged since the last transfer.
- If the clan CLI is not on PATH, both verbs refuse loudly, naming that clan is the authority on its own store.
- The dotfiles mirror becomes a consumer of this rather than a second implementation. That is named here and built in a dotfiles follow-up, not in this change.

Not in scope: any change to clan-core; any two-way mapping; any scheduled or automatic synchronisation.

## The deviation, stated up front

The brief specifies that `safix import` "decrypts clan material with the operator's admin identity" while `safix export` goes through the clan CLI.
That asymmetry is recommended against, on evidence rather than on taste, and the operator should decide.

This fleet's clan does not use the sops backend.
`modules/clan/vars.nix:80` in the dotfiles repository sets `secretStore = "age"`, and the accompanying analysis in `docs/notes/architecture/clan-vars-sops-agenix-bridge.md` records the repository opting out of clan's sops default deliberately.
So "decrypt clan material" does not mean "read a sops file safix already understands".
It means implementing clan's age backend — its directory layout, its recipient sidecars, its stanza type — inside safix.

That is the same reimplementation the requirement forbids in the other direction, and there is no principled reason it is permitted here.
It also fails in three ways beyond principle: it breaks the moment a consumer's clan uses `password-store` or a different backend, it silently reads a layout clan is free to change between releases, and it duplicates a decryption path safix would then have to hold correct for a store it does not own.

Reading through `clan vars get` avoids all of it, is backend-agnostic, and has a property worth having on its own after `clan-generator-contract` spent the pipes-only invariant: the value crosses the boundary on a pipe in both directions.

The one thing lost is that `safix import` then requires the clan CLI as well as `safix export`, so a consumer with no clan cannot import either.
That is acceptable and is arguably correct — a consumer with no clan has nothing to import from.

## Capabilities

### New Capabilities

- `bridge-surface`: the declared mapping, what each side names, how direction is written, and every refusal evaluation can reach — which is not all of them, because half of each mapping lives in another flake.
- `bridge-transfer`: the two verbs, the subprocess delegation that keeps clan the authority, convergence and reporting, the commit discipline, and the divergence rows `check` gains.

### Modified Capabilities

None.
The two verbs are specified inside `bridge-transfer` rather than as a modification to `safix-cli`, because `clan-generator-contract` already carries a `safix-cli` delta and two changes editing one requirement is a conflict at sync time rather than a merge.

## Impact

Affected code:

- New: `modules/flake/safix/bridge.nix` — the mapping option surface and its resolvers.
- Modified: `modules/flake/safix/types.nix`, `resolve.nix` — the mapping type and the evaluation-time refusals.
- Modified: `modules/flake/safix/checks.nix` — the bridge message family, instantiated over a consumer's mappings the same way custody and generator-tool messages already are.
- New: `crates/safix-core/src/bridge.rs` — mapping consumption, the clan subprocess driver, convergence.
- Modified: `crates/safix/src/main.rs`, `usage.rs` — the two verbs.
- Modified: `crates/safix-core/src/check.rs` — bridge divergence rows.
- Modified: `crates/safix-core/src/error/` — the new refusals and their codes, with paired reporter snapshots.

Affected checks: `safix-bridge-refusals` is new; the integration suite gains bridge tests driving a stub clan CLI whose behaviour is asserted, and one check driving the real clan CLI if it is available in the check closure.

Affected consumers: dotfiles' `modules/flake/agents/agents.sh` loses its mirror half, in a follow-up change named `retire-agents-mirror` and not built here.
