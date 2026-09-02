# Two-way clan vars sync, with a placement model and a memory to detect conflicts

Revisions are safix's own working tree on `propose-flakeless-and-first-class-integrations`; clan-core is pinned at `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, read at `/home/sernl/ghq/git.clan.lol/clan/clan-core`.

## Why

`bridge-surface` refuses a two-way relationship between a clan var and a safix entry however it is spelled (`modules/flake/safix/bridge.nix:188-202`), and the refusal names the reason: a two-way synchronisation has no conflict resolution.
That was true when it was written.
`keepassxc-sync` has since built exactly the mechanism a two-way clan sync is missing — a remembered last-agreed state that turns "which side wins" from a guess into a comparison — and D2 directs building the clan-facing counterpart rather than leaving the refusal standing as a permanent limitation.

Two structural gaps sit underneath the refusal and have to close for a two-way sync to be honest rather than merely permitted.
First, clan's placement is a three-way sum — `Shared`, `PerMachine`, `PerExport` (`clan_lib/vars/_types.py:23`, `:45`, `:65`) — and safix's `ClanSide` requires a machine unconditionally (`crates/safix-core/src/model.rs:657-659`), so a shared or export-scoped var cannot be declared without naming a machine that does not own it, which also means two mappings of the same shared var can name different machines and silently evade the duplicate-target and two-way-conflict detection that groups mappings by that string.
Second, `Generator.share` is derived and documented as existing "for comparison against another system's generator" (`openspec/specs/secret-generators/spec.md:195-197`), and no such comparison exists in `bridge.rs`, `audit.rs`, or `clan.rs` — an undischarged spec intent this change is the first place with the machinery (a declared clan placement) to discharge.

## What Changes

- `flake.safix.bridge.mappings.<id>.direction` gains a third value, `two-way`, declared once rather than spelled as two mappings naming the same pair of endpoints with opposite one-way directions.
  The old refusal narrows rather than disappears: declaring the pair twice is still refused, now because a two-way relationship is declared once, not because none exists.
- `ClanSide` gains `placement` (`shared | per-machine | per-export`, default `per-machine`, so every mapping declared before this change is unaffected).
  `machine` becomes `nullOr str`, required exactly when `placement = per-machine` and refused otherwise; a new `export` field is required exactly when `placement = per-export`, naming the exports key clan itself uses (`clan_lib/vars/generate.py:384`).
  Endpoint identity for duplicate-target and two-way-conflict detection is computed from the placement rather than always from a machine, so two mappings of one shared var collide correctly regardless of which machine either declaration happened to name.
- **BREAKING** for any consumer reading `${m.clan.machine}` unconditionally in their own tooling: the field is now `null` for `shared` and `per-export` mappings.
  No existing declaration in this repository or its checks does this outside `bridge.nix` and `bridge.rs` themselves, both of which this change updates.
- A new evaluation-time refusal: a generator's derived `share` and a `safix-to-clan` mapping's declared `placement` must agree — `share = true` requires `placement = shared`, `share = false` requires `placement = per-machine` or `placement = per-export`, matching clan's own derivation exactly (`share` is `isinstance(placement, Shared)`, `clan_lib/vars/generator.py:418-420`).
  This is the comparison `secret-generators` already documents and discharges the defect without touching that spec.
- `two-way` becomes a third accepted value of `sync clan`'s `--direction` option, alongside `clan-to-safix` and `safix-to-clan`: `sync clan --direction two-way` (or a bare `sync clan` run, which already acts on every mapping in its own declared direction) converges `two-way` mappings by reading both sides, comparing against a remembered last agreement, writing toward whichever side changed, and reporting a conflict — writing nothing — when both moved or when no agreement has ever been recorded and the sides disagree.
  No separate verb is added: `rename-transfer-verbs`, sequenced to apply before this change, already collapses the old `import`/`export` split into `sync clan`'s single run and generic `--direction` filter, so a `two-way`-declared mapping named under `--direction clan-to-safix` or `--direction safix-to-clan` is refused by that same generic filter-mismatch refusal, naming `two-way` as its actual declared direction, with no `two-way`-specific "wrong verb" wording to add.
- The agreement itself lives in a sops-encrypted companion safix entry, minted automatically per `two-way` mapping, sharing the mapped entry's file and audience, and reserved so a consumer cannot declare a colliding name — reproducing the property `keepassxc-sync`'s own companion entry relies on, and specifically not in clan's own store (prohibited, `openspec/specs/bridge-transfer/spec.md`'s existing requirement, held by a whole-tree digest test at `crates/safix/tests/real_clan.rs:543`) and not in the plaintext, committed `state/safix/definitions/` tree (`crates/safix-core/src/definition.rs:12-27`), where a digest of a secret value would be an offline-confirmable oracle for anyone holding the tree.
- A two-way push into clan is bound by the same two properties a `safix-to-clan` write already has, with no relaxation: the comparison against clan's current value happens before any write (`clan vars set` writes and commits unconditionally), and a generator clan reports as stale refuses the write outright, with no override.
- The runtime discovers a CLI-addressing machine for a `shared` or `per-export` mapping by asking clan's own `vars` command which machines it has, never by a second declared field a consumer would have to keep in step with their own clan flake by hand.

## Capabilities

### New Capabilities

- `bridge-sync`: the two-way mode itself — the companion entry's reservation and minting, the four-outcome convergence decision (unchanged, one side moved, both moved, no agreement yet), the write-after-value ordering of the agreement, the shared discipline a push into clan inherits from `sync`'s safix-to-clan direction, and the report shape.

### Modified Capabilities

- `bridge-surface`: the mapping's own declared shape changes (a third direction value, a placement-conditional clan side, placement-aware endpoint identity), the blanket two-way refusal narrows to the double-declaration spelling alone, and a new refusal compares a generator's share against its mapping's placement.
- `bridge-transfer`: `sync clan`'s direction vocabulary and its `--direction` option grow a third value, `two-way`, converged by `bridge-sync`'s own rule; no new verb and no new "named to the wrong verb" case, because `rename-transfer-verbs`'s generic direction-filter-mismatch refusal already covers a `two-way` mapping the same way it covers the two one-way directions.

## Impact

Affected nix: `modules/flake/safix/bridge.nix` (`directions`, `clanSide`, `endpointsOf`, `targetOf`, `violationsOf`'s refusal list and messages); `modules/flake/safix/resolve.nix` (minting and reserving the companion entry per `two-way` mapping, sharing the mapped entry's file and audience).

Affected rust: `crates/safix-core/src/model.rs` (`ClanSide` gains `placement`/`export`, `machine` becomes optional; a new `ClanPlacement` enum, named apart from the existing per-entry `Placement` struct at `:147`); `crates/safix-core/src/clan.rs` (a `machines` method reaching `clan machines list`, reused to find an addressing machine); `crates/safix-core/src/bridge.rs` (the `two-way` convergence function mirroring `crate::sync::two_way`, reached from the same `sync` entry point `rename-transfer-verbs` gives the clan target, the companion write); `crates/safix-core/src/error.rs` (new refusal variants for the placement/machine/export mismatch, the share/placement disagreement, and the conflict outcome); `crates/safix/src/main.rs` and `usage.rs` (`two-way` added to `--direction`'s accepted values and `sync`'s help text — no new verb to register).

Affected checks: `modules/flake/checks/bridge.nix` (three-directions fixtures, the narrowed two-way refusal's new message and its accepted single-declaration counterpart, the share/placement refusal, the placement-aware endpoint-identity drills); a new `modules/flake/checks/bridge-sync.nix` or an extension of `real-clan.nix`'s fixtures for the companion reservation; `crates/safix/tests/real_clan.rs` (the four outcome classes, the conflict remedy, the whole-tree digest check extended to prove the companion write still reaches only clan's command and never clan's files).

Sequencing: this change is written assuming `rename-transfer-verbs` has already landed, and for a structural reason rather than a scheduling preference.
That change collapses `import`/`export` into `sync clan`'s single run and its `--direction` filter; this change's own third verb folds into that filter as a third value, `two-way`, rather than registering a fourth CLI form.
`bridge-transfer`'s and `bridge-surface`'s deltas here are written against `rename-transfer-verbs`'s post-archive shape of those two capabilities, not against today's `import`/`export`-named spec, so landing that change first means this change's diff to those two files is written against their post-rename shape once, rather than against a moving target.

Every claim gets a severity drill in `tasks.md`, following `modules/flake/checks/bridge.nix`'s own established pattern of a perturbed fixture per refusal.
