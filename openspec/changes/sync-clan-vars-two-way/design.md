# Design: two-way clan vars sync

Citations are read at the revisions named in `proposal.md`.

Amended 2026-09-03 while proposing `rename-transfer-verbs`: vocabulary updated to the sync family; substance unchanged.
`import`/`export`/`audit` become target-scoped forms of `sync`/`audit`; this change's own third verb, described below as `bridge` when this design was first written, folds into `sync clan`'s `--direction` option as a third value, `two-way`, rather than registering as a fourth CLI verb.
Every decision below that named `bridge` as a verb, or `import`/`export` as the verbs a two-way mapping is refused under, is restated for the folded shape; the underlying mechanism — the companion entry, the four-outcome decision function, the write-after-value ordering, the inherited safix-to-clan write discipline — is exactly what it was.

## Context

Today `bridge.nix` computes four evaluation-time refusals over declared mappings — an unresolvable safix side, a second producer, two mappings on one target, and a pair of endpoints declared with opposite directions — and refuses the last of those unconditionally, naming the reason: "which is a two-way synchronisation and has no conflict resolution" (`modules/flake/safix/bridge.nix:200`).
`bridge.rs` moves a value in exactly one direction per mapping, comparing before it writes and refusing a stale generator before an export (`crates/safix-core/src/bridge.rs:269-402`).
`keepassxc-sync` already solved the general problem this refusal defers: `crate::sync::two_way` (`crates/safix-core/src/sync.rs:439-481`) converges toward whichever side changed, against a digest of the last agreement recorded in a companion entry inside the same encrypted store, and reports a conflict — writing nothing — when both sides moved or when no agreement has ever been recorded.

Two facts specific to clan make porting that mechanism non-trivial rather than a renaming exercise.
First, clan's placement is a three-way sum (`Shared`, `PerMachine`, `PerExport`, `clan_lib/vars/_types.py:23,45,65`), and safix's `ClanSide` today requires a machine unconditionally (`crates/safix-core/src/model.rs:657-659`) — so a shared or export-scoped var cannot be declared honestly, and, more concretely, `endpointsOf`'s and `targetOf`'s string keys (`modules/flake/safix/bridge.nix:122-124,169-174`) are machine-keyed, so two mappings of one shared var addressed through two different machines would silently evade the very refusals meant to catch a duplicate.
Second, `clan vars set` writes and commits unconditionally, and a re-encrypting backend produces fresh ciphertext for an unchanged value (`crates/safix-core/src/bridge.rs:10-17`), so any write toward clan — bootstrap, one-sided convergence, or a manually forced resolution — has to carry the same pre-write comparison and the same stale-generator refusal `one_export` already has, with no exception carved for the new path.

## Goals / Non-Goals

**Goals:**
Make a two-way relationship between a clan var and a safix entry declarable as one mapping, converge it safely without ever guessing a winner, express all three of clan's placements honestly, and discharge the `secret-generators` share-comparison defect using the placement this change adds.

**Non-Goals:**
A CLI override that picks a winner on conflict; the remedy is redeclaring the mapping's direction and running the verb that already exists for it, mirroring the remedy `keepassxc-sync` already documents for the same situation.
Any change to `keepassxc-sync` itself, or to the mechanism it uses; this change ports its shape, not its code.
Verifying a declared placement against clan's own generator beyond the share comparison; clan's placement is otherwise a run-time fact, exactly as the machine, generator and file already are.
A general-purpose "ask clan what it has" cache or command; the addressing-machine lookup this change adds is scoped to shared and per-export mappings only.

## Decisions

### D1. Direction gains a third value, declared once

`clan-to-safix` and `safix-to-clan` keep their meaning; `two-way` is added, naming neither a source nor a destination because the value may originate on either side.
The old refusal — declaring one pair of endpoints as two mappings with opposite one-way directions — narrows rather than disappears: it is still wrong, now because the relationship has a single correct spelling and this is not it, rather than because no conflict resolution exists.
`bridge-surface`'s "Evaluation refuses every mapping mistake that is local to the consumer" requirement carries both the narrowed refusal and a new scenario accepting the single-declaration form; its requirement header is kept unedited for archive continuity even though the scenario beneath it is renamed to state the new reason, the same discipline `bridge-transfer`'s "sync moves declared mappings, scoped by target and narrowed by direction" requirement keeps under D10 below (`rename-transfer-verbs`'s renamed and rewritten successor to what this design originally cited as "Two verbs move declared mappings, one per direction").

### D2. `ClanSide` gains a placement, and `machine` becomes conditional on it rather than always required

`placement` is `shared | per-machine | per-export`, defaulting to `per-machine` so every mapping declared before this change parses unchanged.
`machine` becomes `nullOr str`: required when placement is per-machine, refused otherwise.
An `export` field, `nullOr str`, is required exactly when placement is per-export, naming the exports key clan itself keys generators by (`clan_lib/vars/generate.py:384`, `get_flake_generators`'s `GeneratorId(gen_name, PerExport(scope))`).

Forbidding `machine` outside per-machine is not cosmetic.
`endpointsOf` and `targetOf` are string keys that `byPair`/`bothDirections` and `byTarget`/`twoMappingsOneTarget` group by (`bridge.nix:122-124,169-174,178-202`); today they always include the machine.
Two mappings of the same shared var, each naming a different (both otherwise-valid) machine, would produce two different keys and evade both the duplicate-target refusal and the two-way-declared-twice refusal — the exact class of silent loss D1's refusal exists to prevent.
This change redefines both keys per placement: per-machine keeps `<machine>:<generator>/<file>`; shared becomes `shared:<generator>/<file>`; per-export becomes `export:<export>:<generator>/<file>`.
A shared var is then one target and one pair of endpoints no matter which machine a declaration happens to name, which is what "not expressible" in the operator brief actually names: not merely refused, but incapable of being represented as one stable identity, which is a stronger defect than a missing type and is what motivates changing the field's cardinality rather than just relaxing its validation.

`crate::model::Placement` already exists (`model.rs:147-168`) and names safix's own per-entry file/key/public shape; the new clan-side enum is `ClanPlacement` to avoid two unrelated concepts sharing one name in the same crate.

### D3. The addressing machine for a shared or per-export mapping is discovered from clan, not declared as a second field

`clan vars get`/`set` take a machine positionally regardless of a var's real placement (`clan_cli/vars/get.py:39-41`, `set.py:16-18`), and `get_machine_vars` scopes generators to the machine asked (`clan_lib/vars/list.py:24-25`, `get_machine_generators`), so an arbitrary fleet machine is not guaranteed to see a given shared or per-export generator.

The alternative considered and rejected is a fourth declared field, `addressingMachine`, naming a machine the consumer asserts imports the generator.
It was rejected because it is a second copy of a fact only clan's own flake holds, it would drift silently the moment that machine is renamed or removed from the fleet, and `consumer-integration` already argues against exactly this shape of coupling for a different field — safix reading one of clan's options to decide something would make that option part of safix's interface.

The chosen mechanism is `Clan::machines`, a new method beside `Clan::probe`/`register_user`/`generator_stale` (`crates/safix-core/src/clan.rs:126-138,240-269,295-317`) invoking `clan machines list --flake <flake>` (confirmed to exist, `clan_cli/machines/list.py:12-24`), memoized per run keyed on `(generator, file)` so mappings sharing one clan var do not repeat the search.
For a shared or per-export mapping, the runtime tries each returned machine against `clan vars get`/`set` in turn, using the existing `NO_SUCH_VAR` substring match (`clan.rs:52-57,205-212,374-381`) to tell "this machine does not see this generator" apart from a genuine failure, and stops at the first that resolves.
No machine resolving it is `bridge-transfer`'s new refusal, naming the mapping, the placement, the generator and the file.

### D4. The generator-share comparison, discharged with the field this change adds

`Generator.share` in clan is a derived property, `isinstance(self.key.placement, Shared)` (`clan_lib/vars/generator.py:418-420`) — `true` for `Shared` alone, `false` for both `PerMachine` and `PerExport`.
safix's own `Generator.share` exists "for comparison against another system's generator" (`crates/safix-core/src/model.rs:118-124`, `openspec/specs/secret-generators/spec.md:195-197`), and no comparison existed because nothing until this change carried a declared clan placement to compare it against.
`bridge-surface` gains a new requirement: for a safix-to-clan mapping whose source is generator-produced, `share = true` must pair with `placement = shared` and `share = false` must pair with `placement = per-machine` or `per-export`.
This is scoped to safix-to-clan alone: a clan-to-safix or two-way mapping whose safix side is generator-produced is already refused by the broadened two-producers rule in D5, so by the time this comparison would run, no other direction can reach it with a generator on the safix side.
A hand-set source is exempt, for the reason every other generator-shaped rule already exempts one: there is no generator to derive a share from.

### D5. Two producers, broadened to two-way

`twoProducers` today fires only for `clan-to-safix` (`bridge.nix:162-167`), because a `safix-to-clan` mapping's generator producing the value it exports is the intended shape, not a conflict.
A `two-way` mapping whose safix side is generator-produced has the same hazard `clan-to-safix` does: a pull could overwrite what the generator produces.
The rule's condition becomes `direction == "clan-to-safix" || direction == "two-way"`, carried in the same `bridge-surface` delta as D4, and is what makes D4's exemption for two-way precise rather than incidental.

### D6. The companion entry: where the agreement lives, and why nowhere else

Three candidates, and two are refused on record rather than merely unconsidered.

Clan's own store is refused because it is prohibited: `bridge-transfer`'s existing requirement that the runtime never read, write, decrypt, encrypt or parse a file clan placed, held by a whole-tree digest test (`crates/safix/tests/real_clan.rs:543-559`, `digest()` at `:562-585`) that walks every path clan's own tree holds before and after a run and asserts byte-identity.
Writing the agreement there would be reachable only by parsing or writing a file clan placed, which is the one thing this runtime is built not to do.

`state/safix/definitions/` is refused on a sharper ground than "the wrong prefix": it is plaintext and committed by design, one line per file, specifically so that a check can read it without decrypting anything (`crates/safix-core/src/definition.rs:1-27`).
A digest of a secret value written there would be readable by anyone who can read the tree, and a digest is an offline-confirmable oracle against a guess — exactly the property `crate::sync`'s own module documentation already names as the reason its own memory is not there (`sync.rs:26-33`): "a committed digest of a secret value is an oracle: anyone holding the tree could confirm a guess offline."

The companion entry inside safix's own sops-encrypted store reproduces the property `keepassxc-sync` relies on: the memory is only as readable as the value it is about.
Concretely, for a two-way mapping the resolver mints a second placement sharing the mapped entry's `file` (`crate::model::Placement::file`, `model.rs:147-150`) — so it lands in the same document, encrypted to the same audience, at no extra custody grant — distinguished by a reserved key suffix rather than by a new file, mirroring `store::companion_of`'s path-suffix mechanism (`crates/safix-core/src/store.rs:62-85`) adapted to a namespace where safix's own resolver, not a flat kdbx path list, is the source of truth.
This is why the companion is written through `set::run_committing`, the same path every other safix write goes through (`crates/safix-core/src/set.rs:61-73`), rather than through a bespoke "write one more key" primitive: the companion is a genuinely resolved entry, with its own recipients, its own drift refusal, and its own commit, and inventing a second write path for it would be the second authoring surface `secret-generators`' own share requirement already warns against for a different fact.

### D7. The decision function mirrors `crate::sync::two_way`, adapted to `Reading` and `Secret`

`bridge_sync::decide` (new, alongside `bridge.rs`'s existing one-way clan-to-safix and safix-to-clan write functions, whatever `rename-transfer-verbs` ends up naming them once it collapses `import`/`export` into `sync`) reads `clan.read(...)` and `bridge::held_by_safix(...)`, reads the companion through the same read path a mapped entry uses, and reproduces `two_way`'s four-way match exactly (`sync.rs:447-479`): both absent is unchanged; exactly one absent is a bootstrap push or pull, remembered; both present and equal is unchanged; both present and unequal consults the companion — one side still agreeing is a converge toward the other, remembered; neither agreeing, or no companion yet, is a conflict, settled with nothing written.
`agrees`/`memory_of`/`FORMAT` (`sync.rs:483-511`) are the same shape reused with a distinct tag — `safix-bridge-sync-v1` rather than `safix-sync-v1` — so the two mechanisms' memories are never mistaken for one another if a consumer somehow points both at overlapping entries, and so a future change to one format tag does not silently reinterpret the other's records.

### D8. The four outcome classes, exactly

**Neither moved** (clan's current value equals safix's current value, whatever the companion holds): outcome unchanged.
Nothing is written: no clan write, no safix write, no companion write, no commit, no invocation of clan's write command.

**One moved** (the two sides differ, a companion exists, and exactly one side still matches it): outcome updated-toward-clan or updated-toward-safix, named by which side received the write.
If safix moved: clan is written first, through `clan.write` under the same comparison and stale-generator refusal an export has (D9); the companion is written second, as its own safix commit, only once the clan write is confirmed to have landed.
If clan moved: safix's mapped entry is written first, through `set::run_committing` (the same path a pull already uses), as its own commit; the companion is written second, as a second, separate safix commit.
Either way the value lands before the agreement, per C7 and per D6's oracle reasoning: an interruption between the two leaves a companion describing the older agreement, and the next divergence on that mapping is reported as a conflict rather than resolved by a guess, which `sync.rs`'s own documentation already states is the safe direction (`sync.rs:35-41`).

**Both moved** (the two sides differ from the companion, or from each other with no companion recorded yet): outcome conflict.
Nothing is written anywhere.
The report names the mapping and the remedy: narrow a `sync clan` run to it with `--direction clan-to-safix` or `--direction safix-to-clan` and run once — which never touches the companion, since only the two-way convergence path does — then revert the mapping's declared direction to two-way.
This mirrors `keepassxc-sync`'s own documented remedy exactly: switching a `two-way` mapping's declared mode and running `sync` once, which also never remembers (`sync.rs:394-397,409-412`, both one-way `Decision` arms pass `remember: false`), for the identical reason — forcing agreement by fiat should not be indistinguishable, in the record, from a genuine converging write.

**No memory yet** is not a fifth class; it is the condition under which "both moved" and "one side never held a value" diverge in outcome.
A two-way mapping whose sides already agree, with no companion ever written, stays unchanged indefinitely — nothing ever needed converging, so nothing ever needed remembering.
The first time the sides genuinely disagree with no companion recorded, the mapping is refused as a conflict rather than guessed at, exactly as `sync.rs`'s own documentation states for the keepassxc case (`sync.rs:36-38`): "a two-way mapping whose sides already agree before safix ever ran has no memory, so its first divergence is a conflict rather than a guess."
The companion's first-ever write therefore only ever happens through a bootstrap push or pull (exactly one side ever having held a value) or through a tiebreak once a companion already exists — never through the "both moved, no memory" branch, which is deliberate: nothing here ever manufactures an agreement it did not observe.

### D9. A two-way push inherits sync's safix-to-clan discipline verbatim, with no override

The comparison before a clan write and the stale-generator refusal are structural properties of `clan vars set` (unconditional write and commit, silent replacement of an unvalidated definition), not preferences of one particular direction's implementation.
A two-way push therefore calls the same comparison and the same `Clan::generator_stale` check a safix-to-clan write calls (`bridge.rs:352-376`), under the identical condition and the identical message, so a two-way mapping and a one-way safix-to-clan write of the same generator are refused by the same sentence.
No flag on `sync` bypasses it, mirroring `bridge-transfer`'s existing "No option defeats the refusal" scenario for the safix-to-clan direction.

### D10. `two-way` folds into `sync clan`'s `--direction` filter; no fourth verb

Amended while proposing `rename-transfer-verbs`: this design originally proposed a third verb, `bridge`, reusing the operator-facing category name the CLI's comment gave `import`/`export` (`crates/safix/src/main.rs:107-108`).
`rename-transfer-verbs`, sequenced to land first, already collapses `import`/`export` into one `sync` verb whose `clan` target converges every declared mapping in its own direction and accepts an optional `--direction` filter over that same declared vocabulary.
A third verb spelled `bridge` would sit beside a `sync` that already means "converge a declared relationship" for the `keepassxc` target, forcing an operator to remember which of two verbs converges which target's two-way relationship — the same drifting-operational-knowledge failure mode `rename-transfer-verbs`'s own design rejects `--filter`-shaped alternatives for.
`two-way` becomes `--direction`'s third accepted value instead: `sync clan --direction two-way` (or a bare `sync clan`, or bare `sync`, each of which already converges every mapping in its own declared direction) reaches the identical convergence function.
A two-way mapping named under `--direction clan-to-safix` or `--direction safix-to-clan` is refused by the same generic filter-mismatch refusal every direction mismatch already gets, naming `two-way` as the mapping's actual direction — no `bridge`-specific "wrong verb" wording survives, because there is no longer a second verb to be wrong about.
The convergence function itself keeps its own name and home, `bridge_sync::decide`/`converge` in `crates/safix-core/src/bridge.rs`, reused by the `sync` entry point exactly as the one-way convergence functions are; its own `Report`/`Outcome` types still mirror `crate::sync::Report`/`Outcome` (`sync.rs:121-197`) rather than `crate::bridge::Outcome` (`bridge.rs:62-92`), because a two-way run has a fifth possible finding — conflict — the one-way runs do not.

### D11. `Secret` wraps the digest too, not only the value

The agreement is a digest (`Secret::fingerprint()`, reused identically to `sync.rs:509-510`'s `memory_of`), but it is still constructed and carried as a `Secret` end to end rather than as a `String`.
A digest of a low-entropy value narrows a guess even without reversing it, so it inherits the same no-`Debug`/no-`Display`/no-`Serialize` compile-time probes (`crates/safix-core/src/secret.rs:60-81`) the value itself has, and the same zeroize-on-drop discipline.
This is not a new decision so much as the same one `keepassxc-sync` already made, restated because a reader who has not read `sync.rs` might assume a hash is exempt from the value-handling discipline; it is not.

## Risks / Trade-offs

A two-way mapping's memory degrades to "next divergence is a conflict" after any manually forced resolution, because the one-way override verbs never write the companion — this is D8's stated safe direction, not a defect, but it means an operator who resolves a conflict by hand should expect the very next legitimate one-sided edit to also be reported as a conflict, once, before the mapping settles back into ordinary convergence.

The addressing-machine search (D3) costs one or more `clan vars get`/`set` attempts per distinct shared or per-export generator per run, bounded by fleet size; this is paid only by mappings that use those two placements, and is memoized within a run rather than repeated per mapping.

Extending `twoProducers` to two-way (D5) is, on its own, a behavior change for any two-way mapping a consumer might already have hand-simulated as two opposed one-way declarations naming a generator-produced safix side — but D1's own refusal already forbids that spelling outright, so the set of mappings D5 newly refuses that D1 does not already refuse is empty.

## Migration Plan

Every mapping declared before this change has `direction` in `{clan-to-safix, safix-to-clan}` and no `placement` field, which defaults to `per-machine` and requires no edit.
`${m.clan.machine}` stays non-null for every such mapping; only a newly declared `shared` or `per-export` mapping ever sees it null, so no existing consumer's own tooling observes a behavior change on that field.
Adopting two-way for an existing pair of one-way mappings is a deliberate edit: remove both declarations, add one with `direction = "two-way"`; the bridge-surface refusal added in D1 is what catches the case of doing this only halfway.

## Open Questions

None: every question this design raised while being written had a concrete, cited answer, recorded above as the corresponding decision rather than deferred.
