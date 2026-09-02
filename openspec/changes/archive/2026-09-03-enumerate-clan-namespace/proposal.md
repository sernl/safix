# safix's audit gains a view of clan's own namespace

Revisions: clan-core is `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, this flake's `inputs.clan-core`, the same pin the shared program contract and `own-secret-installer` both cite.
Every clan-core line anchor below was read at that revision.

This change depends on `sync-clan-vars-two-way` and is written to land after it; see Impact for why, and see Sequencing below for what that dependency does and does not touch.

Amended 2026-09-03 while finishing `sync-clan-vars-two-way`: per-export is dropped from that change after confirming, at the pinned clan-core revision, that `clan vars get`/`set` can never resolve a `PerExport`-placed var through any machine (`get_machine_generators`, `clan_lib/vars/generator.py:229-351`, builds every generator's placement as `Shared()` or `PerMachine(machine)` alone). `ClanPlacement` has two variants, `shared` and `per-machine`, and `clanSide` gains no `export` field. Every per-export mention this change's own artifacts carried is removed in the same pass, since D2's and D3's per-export paragraphs were reasoning about a placement that no longer exists; see `design.md`'s amendment note for the full citation.

## Why

`crates/safix-core/src/clan.rs` invokes exactly three clan verbs — `vars get`, `vars set`, `vars check` — plus `secrets users add`/`add-key` and `--help`.
`clan vars list` appears nowhere as a command safix runs; the only place the string appears at all is inside a refusal, at `crates/safix-core/src/error/prose.rs:244-246`, telling the operator to go run it themselves.
So safix's bridge has no way to ask clan what clan actually holds, and only a way to ask clan about one declared mapping at a time.

The consequence is exactly the one keepassxc's mirror already solved for its own store and clan's mirror has not: when a bridge mapping is removed from the declarations, the clan var it named does not stop existing, and nothing safix runs ever looks at it again.
It sits in clan's repository, live, disconnected from any current declaration, and the only way to notice is to remember to check by hand — which is what the refusal message above is already asking an operator to do reactively, one triple at a time, only after something else has already gone wrong.
`sync.rs:346-363`'s `lingering` computation is the shape that answers this for keepassxc: entries under the declared group that no mapping currently accounts for, including the companion of a mapping that is gone, reported as information on every run, deleted by nobody but a person.
`audit` — the verb whose entire purpose is comparing declarations against both stores and changing nothing (`crates/safix-core/src/audit.rs:1-19`) — has no equivalent, because it only ever iterates the mappings that are still declared and never asks clan what else is there.

## What Changes

- `Clan` gains a fourth contract, `vars list <machine>`, alongside the three `sync-clan-vars-two-way` leaves it with (`get`, `set`, `machines`).
  It is invoked once per machine that at least one currently declared mapping names, never per mapping and never against the whole clan.
- `audit`'s report gains a `lingering` field, in the same shape and under the same name as `sync::Report::lingering`: clan vars, named as `<machine> <generator>/<file>`, that no currently declared mapping's clan side accounts for.
  It is reported as information on every `audit` run where at least one mapping is declared, alongside the existing per-mapping agreement findings, and it does not change `audit`'s exit status — exactly as `lingering` does not change `sync`'s tally or `is_clean`.
- No mode this change adds deletes anything, on either side of the boundary.
  Reporting is the entire deliverable; a person still removes a var, on clan's side, with clan's own command.
- The scope of what gets enumerated is exactly the machines the bridge currently names or resolves through its mappings — declared directly for a per-machine mapping, discovered via clan for a shared one — not clan's whole machine inventory (`clan machines list` is deliberately not called — see design.md's D2 for why "one consumer bridges one clan" does not mean one consumer may see every machine of that clan).
  A machine whose last mapping is removed becomes invisible to this report; that is a stated, deliberate limitation and not a defect — see design.md's Risks.
- `crates/safix/tests/support/clan-stub.rs` answers `vars list` for the hermetic suite, and `modules/flake/checks/real-clan.nix` gains a var no mapping names, so the parsing this change adds is held against the real clan CLI's real output shape, not only against a stub written to agree with it.

Not in scope: deleting a lingering var on either side, on any operator's say-so or any mode's; enumerating a clan machine no mapping currently names; anything about `clan secrets users`, which `safix-bridge-real-clan` also does not exercise today and which this change does not touch; and the two-way agreement-memory mechanism `sync-clan-vars-two-way` is building, which lives entirely inside safix's own store and has no clan-side namespace footprint for this change's enumeration to interact with.

## Capabilities

### Modified Capabilities

- `bridge-transfer`: the delegation requirement gains a third kind of clan interaction (enumeration, alongside read and write), and a new requirement states that the audit verb reports clan vars no currently declared mapping accounts for, how that report is scoped, and that it never deletes anything.

## Impact

Affected code:

- `crates/safix-core/src/clan.rs` — new `Clan::list` method and its doc comment (`sync-clan-vars-two-way`'s own rewrite, "the three contracts" naming read, write and machine discovery, becomes four); a new error path for a machine that cannot be listed.
- `crates/safix-core/src/error/mod.rs` and `error/prose.rs` — one new `Error` variant and its message.
- `crates/safix-core/src/audit.rs` — the clan section's `lingering` field, alongside `rename-transfer-verbs`' own keepassxc section and its own `lingering` field on the same per-target report, the machine-grouping and enumeration that fills the clan one, and the module's own doc comment.
- `crates/safix/src/render.rs` and `crates/safix/src/usage.rs` — the lingering section of `audit`'s printed report, and its help text.
- `README.md`'s "The bridge to clan" section, in the paragraph beginning "`safix audit` is the report over the same declarations" — it currently says `audit` compares declared mappings and nothing else; it needs the same correction.
- `crates/safix/tests/support/clan-stub.rs`, `crates/safix/tests/audit.rs` — hermetic coverage of the new report field.
- `crates/safix/tests/real_clan.rs`, `modules/flake/checks/real-clan.nix` — real-clan coverage that a var no mapping names is actually reported, against the real `clan vars list` output shape.
- `CHANGELOG.md` — an `## [Unreleased]` entry naming the new lingering report, following the file's existing style.

Sequencing: this change is written assuming `sync-clan-vars-two-way` has already landed, and for a structural reason rather than a scheduling preference.
That change gives `ClanSide` a `placement` (`shared | per-machine`, defaulting to `per-machine`) and makes `machine` `nullOr str` — null for a shared mapping, whose addressing machine is instead discovered at run time by trying each machine `Clan::machines` returns against the mapping's generator (two-way's D3) — rather than the mode-like third relationship kind this change originally assumed before two-way's design was written.
This change's `claimed` computation is placement-sensitive rather than a single-field match: a per-machine mapping's clan triple is claimed on its declared machine, and a shared mapping's is claimed on any machine that lists it (design.md's D2/D3 record why, and why comparison stays machine-insensitive there).
Both changes touch `crates/safix-core/src/audit.rs` and `modules/flake/checks/real-clan.nix`; landing the two-way change first means this change's diff to those two files is written against their post-two-way shape once, rather than against two moving targets.
