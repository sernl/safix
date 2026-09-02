## Context

Today `import` and `export` are two free functions in `crates/safix-core/src/bridge.rs`, dispatched from two separate `main.rs` verbs that differ in nothing but which `Direction` they pass to a shared `run` function (`crates/safix-core/src/bridge.rs:178-216` as read on this branch).
`audit` is a third, independent verb (`crates/safix-core/src/audit.rs`) that walks the same declared bridge mappings, comparing rather than writing, and refuses the whole run under the identical clan-unavailable condition `import`/`export` do — stated in `usage.rs`'s `AUDIT` text but never asserted as a scenario `bridge-transfer`'s own requirement names.
`sync` is a fourth, unrelated verb (`crates/safix-core/src/sync.rs`) converging a disjoint declared set, `flake.safix.keepassxc.mappings`, with no clan involvement at all.
All four verbs already share the same shape: read both sides of a declared relationship, compare, and either report (`audit`) or write the side that needs it (`import`, `export`, `sync`).
See `proposal.md` for why the verb names themselves, not just their count, are the defect being corrected.

## Goals / Non-Goals

**Goals:**
- Collapse `import`/`export` into `sync`'s `clan` target, and generalize `audit` to the same `clan`/`keepassxc` target pair `sync` gains, with no `all` keyword and no verb-name alias for either retired word.
- Make direction narrowing, where it applies, spelled the same way `bridge-surface` already requires mapping declarations to spell it: as endpoints, never as a tool-relative verb.
- Keep the mapping-id namespace and the target-keyword namespace disjoint by construction, so parsing `sync`'s and `audit`'s first positional argument never has to guess.

**Non-Goals:**
- Building the `two-way` direction value, the `bridge` verb's convergence logic, or the companion-entry memory `sync-clan-vars-two-way` adds. This change only makes room for `--direction two-way` to be added later by leaving `--direction`'s accepted-value set owned by a single place (see D2) rather than hard-coded at each call site.
- Building the `import`-as-external-ingestion feature the retired word is reserved for. This change records the reservation in the CLI's own absent-verbs help text and nowhere else; there is no scaffold, no stub subcommand, and no partially-wired parser branch for it.
- Anything about `safix upload`, which is `upload-safix-store-to-machine`'s change, dispatched separately.
- Re-deriving `keepassxc.nix`'s existing four-mode convergence logic. `sync keepassxc` is `sync`'s existing behavior under a target keyword; nothing about `Mode`, `pullCapable`, or the companion-entry reservation changes.

## Decisions

**D1 — Bare `sync`/`audit` is the one spelling for "every target, every mapping"; there is no `all` keyword.**
A keyword and an empty argument list both meaning "everything" is two spellings of one run, free to drift the day someone adds a target-specific default one place and not the other, and a operator reading `safix sync all` for the first time cannot tell it apart from a mapping literally named `all` without already knowing the answer.
Omitting the target argument entirely is unambiguous in a way a keyword can never be: there is no argument to parse, so there is nothing for a future target to redefine the meaning of.
Alternative rejected: an explicit `all` target keyword mirroring `clan`/`keepassxc`, offered for symmetry ("every target keyword form takes the same shape"). Rejected because symmetry between the two narrowing keywords and a third keyword that means "no narrowing" is a false symmetry — `all` cannot narrow, so an operator typing `sync all mapping-name` would be asking to narrow the unnarrowable, and the only sound answer is a refusal that then has to explain that `all` was never a target to begin with. The bare form has no such trap.

**D2 — `--direction` takes the endpoint-named vocabulary `bridge-surface` already owns, and is refused on any target but `clan`.**
`bridge-surface`'s "Direction is written as its endpoints, not relative to a tool" requirement already states why `pull`/`push` are refused as a mapping's declared direction: the words mean opposite things depending on which tool is speaking, and a declaration or a flag is read by someone with no tool in hand to be relative to.
`--direction` is a run-time filter over that same declared vocabulary, not a new vocabulary, so it is refused the same two values `clan-to-safix`/`safix-to-clan` a declaration accepts today (`sync-clan-vars-two-way`, once archived, adds `two-way` to both places at once — see D4).
`keepassxc` mappings declare a `mode`, not a `direction`, and `mode` already carries two non-endpoint values (`two-way`, `backup`) `bridge-surface`'s direction vocabulary was deliberately kept out of (`modules/flake/safix/keepassxc.nix:26-32`'s own comment records that decision).
Passing `--direction` to `sync keepassxc` or `audit keepassxc` is refused rather than silently ignored, naming `--direction`'s single target and that `keepassxc` mappings narrow by declaring fewer of them, not by a run-time flag.
Alternative rejected: a single `--filter`-style flag whose accepted values depend on the target, spanning both `direction` and `mode`. Rejected because it would make one flag's valid-value set conditional on another argument's value, which is exactly the kind of context-dependent parsing `bridge-surface`'s own endpoint-naming requirement exists to keep out of a declaration; keeping it out of the command line that mirrors that declaration is the same decision applied twice.

**D3 — `audit` gains `--direction` for the same reason and the same restriction as `sync`.**
Both verbs read the identical declared `clan` mapping set and the identical `Direction` field; the filter that narrows which mappings a write acts on narrows which mappings a comparison reports on for the same reason, and giving `sync` the flag while leaving `audit` without it would be an arbitrary asymmetry between two verbs this change already unifies under one target grammar.

**D4 — `import`/`export` retire with different endings: `export` permanently, `import` reserved.**
Clan's own `export` verb (`clan vars export <dir>`, `pkgs/clan-cli/clan_cli/vars/cli.py`) writes a machine's entire vars folder to plaintext on disk; `import` reads one back.
`openspec/specs/safix-cli/spec.md`'s "Absent verbs are recorded rather than left mysterious" requirement already refuses safix ever producing that plaintext tree, on the ground that a plaintext export tree outlives the migration that justified it — so a verb spelled `export` that does something else entirely is not a naming coincidence safix can afford to keep, once `sync clan`'s single run makes the old two-verbs-one-per-direction structure that motivated `import`/`export` as separate names unnecessary.
`import`, unlike `export`, names an operation safix has never built and might: pulling a single value from an external plaintext source — a file, a legacy secret manager's export, a person's typed-in credential — one entry at a time, analogous to clan's own `import-sops` (`pkgs/clan-cli/clan_cli/secrets/sops.py`), which ingests external age/sops key material without ever writing a plaintext tree of its own.
Retiring both words identically would erase that distinction; recording one as reserved and the other as permanently retired, in the same help text `safix-cli`'s absent-verbs requirement already governs, keeps the distinction on the record rather than leaving a future author to rediscover clan's own import/export split from scratch before deciding whether `import` is safe to reuse.
Alternative rejected: retire both permanently, on the ground that neither exists today and a future feature can pick whatever name fits it when it is built. Rejected because the operator has already named the future feature and its shape (V4), and recording that now costs one help-text scenario against the cost of a future author re-deriving the same clan-side research this change already did.

**D5 — Reserved mapping ids (`clan`, `keepassxc`, `all`) are refused in `bridge.nix` and `keepassxc.nix` independently, mirroring each file's existing `violationsOf` pattern rather than sharing one check.**
Both files already compute a `declared` list via `mappingsOf` (`id: m: m // { inherit id; }`, identical in both files) and fold refusal lists that read `m.id` directly (`bridge.nix:140-152`'s `unresolvableSafixSide`, `keepassxc.nix:208-211`'s `reservedName`, are both this exact shape).
A reserved-id check is one more `lib.concatMap` over `declared` in each file, in the same list-of-strings shape every other refusal in both files already returns, so a severity drill perturbs a fixture's mapping id to `clan` or `all` and expects the same `refuseScript` bytes the existing drills already run.
Alternative rejected: a single shared `reservedMappingIds` list-checking function in `resolve.nix`, called from both `bridge.nix` and `keepassxc.nix`. Rejected for the same reason `mappingsOf` itself is duplicated rather than shared today: the two files' `violationsOf` already read `registry`, `users`, and each other's namespace differently enough (`bridge.nix` groups by clan endpoint, `keepassxc.nix` by kdbx entry path) that a shared refusal function would be the only shared piece of an otherwise independent pair, coupling two files together for one three-line check.

**D6 — The rendered report narrates each written mapping's direction; the structured outcome enum does not grow a fifth state.**
`bridge-transfer`'s existing four states (`unchanged`, `updated`, `absent at source`, `refused`) answer "did this mapping change", which is a boundary-independent question `audit`'s exit-code logic and `sync`'s tally already key off; adding a direction-specific `updated-toward-clan`/`updated-toward-safix` pair to that enum would be a second axis smuggled into one, and every existing consumer of the enum (tests, exit codes, `is_clean`) would have to learn to treat two new variants as equivalent to the one they already handle.
The direction only has to be visible in the text a person reads, so it lives in `crates/safix/src/render.rs` alone: an `updated` outcome on a `clan-to-safix` mapping renders `pulled <mapping> ← clan`, on a `safix-to-clan` mapping renders `pushed <mapping> → clan`, and (once `sync-clan-vars-two-way` lands) a `two-way` mapping's convergence renders `converged <mapping>` — a third verb-in-report-prose that was already this pending change's own vocabulary for its `bridge` verb (`bridge-sync/spec.md`'s "updated toward safix, updated toward clan"), now spelled to match `pulled`/`pushed` rather than duplicating "toward clan"/"toward safix" a second time.
`unchanged`, `absent at source` and `refused` render exactly as they do today, because none of those three outcomes is a write an operator needs a direction arrow to understand.
Alternative rejected: keep the report generic ("updated") and let the operator infer direction from the mapping's own declaration. Rejected because inferring direction from a declaration the operator is not looking at, mid-report, is exactly the drifting-operational-knowledge failure mode `keepassxc.nix`'s own top-of-file comment already names as the reason the mode is declared per mapping rather than remembered per run; a report that requires cross-referencing a nix file to read is a worse report than the two before it.

**D7 — The target-and-direction dispatch grammar is stated once, in `safix-cli`, and each target-owning capability documents only its own target's specifics.**
`bridge-transfer` and `keepassxc-sync` each already state their own target's convergence rules; a third, shared statement of "bare means every target, a target keyword narrows, `--direction` exists on one target and not the other" belongs to neither capability specifically, and duplicating it in both risks the two copies drifting the way `mappingsOf` already does not (D5) because that duplication is deliberately narrow.
`safix-cli`'s purpose is already "the command that is the whole lifecycle of a secret," which is the CLI-dispatch-level frame this grammar belongs to.

## Risks / Trade-offs

[Risk] An operator scripting today's `safix import` or `safix export` breaks with no deprecation window, per the **BREAKING** marker in `proposal.md`.
→ Mitigation: the unknown-subcommand refusal `expected_verbs()` renders (`crates/safix/src/main.rs:196-210`) is derived from `VERBS`, so a script running `safix import` after this change gets the standard "not a subcommand, try one of: …" refusal immediately, naming `sync` and `audit` among the list, rather than a silent behavior change or a confusing error about a missing mapping. `CHANGELOG.md` records the retirement, matching how `main.rs`'s own top-of-file doc comment already records the shell-runtime-to-rust retirement precedent.

[Risk] Variadic mapping names (`[<mapping>...]`, replacing today's single optional `<mapping>`) change `bridge.rs`'s and `sync.rs`'s selection functions from "one name or every mapping" to "a list of names, possibly empty, or every mapping," which touches every call site that currently passes `Option<&str>`.
→ Mitigation: an empty list and "no argument given" are the same case (every mapping of the resolved target), so the existing `None` branch of each selection function becomes the empty-list branch; the non-empty-list branch replaces a single lookup with a filter over the declared set, refusing on the first name that resolves to nothing or to a mismatched `--direction`, which is the same refusal-shape `bridge-transfer`'s existing "named to the wrong verb" scenario already establishes for one name at a time.

## Migration Plan

`import`/`export` are removed in the same commit that adds `sync clan`; there is no intermediate release carrying both, per the clean-cutover default and the **BREAKING** marker.
`README.md`'s worked examples and verb-count sentences are updated in the same change, so no published documentation names a verb this change deletes.
`CHANGELOG.md` gains an entry under "Known differences" or its own heading, naming the retired verbs, the reservation on `import`, and the one-run-two-directions behavior change so a reader of the log does not have to reconstruct it from the diff.
