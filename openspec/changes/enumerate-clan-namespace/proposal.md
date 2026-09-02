# safix's audit gains a view of clan's own namespace

Revisions: clan-core is `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, this flake's `inputs.clan-core`, the same pin the shared program contract and `own-secret-installer` both cite.
Every clan-core line anchor below was read at that revision.

This change depends on `sync-clan-vars-two-way` and is written to land after it; see Impact for why, and see Sequencing below for what that dependency does and does not touch.

## Why

`crates/safix-core/src/clan.rs` invokes exactly three clan verbs — `vars get`, `vars set`, `vars check` — plus `secrets users add`/`add-key` and `--help`.
`clan vars list` appears nowhere as a command safix runs; the only place the string appears at all is inside a refusal, at `crates/safix-core/src/error/prose.rs:244-246`, telling the operator to go run it themselves.
So safix's bridge has no way to ask clan what clan actually holds, and only a way to ask clan about one declared mapping at a time.

The consequence is exactly the one keepassxc's mirror already solved for its own store and clan's mirror has not: when a bridge mapping is removed from the declarations, the clan var it named does not stop existing, and nothing safix runs ever looks at it again.
It sits in clan's repository, live, disconnected from any current declaration, and the only way to notice is to remember to check by hand — which is what the refusal message above is already asking an operator to do reactively, one triple at a time, only after something else has already gone wrong.
`sync.rs:346-363`'s `lingering` computation is the shape that answers this for keepassxc: entries under the declared group that no mapping currently accounts for, including the companion of a mapping that is gone, reported as information on every run, deleted by nobody but a person.
`audit` — the verb whose entire purpose is comparing declarations against both stores and changing nothing (`crates/safix-core/src/audit.rs:1-19`) — has no equivalent, because it only ever iterates the mappings that are still declared and never asks clan what else is there.

## What Changes

- `Clan` gains a fourth read contract, `vars list <machine>`, alongside the three it already has (`get`, `set`, `check`).
  It is invoked once per machine that at least one currently declared mapping names, never per mapping and never against the whole clan.
- `audit`'s report gains a `lingering` field, in the same shape and under the same name as `sync::Report::lingering`: clan vars, named as `<machine> <generator>/<file>`, that no currently declared mapping's clan side accounts for.
  It is reported as information on every `audit` run where at least one mapping is declared, alongside the existing per-mapping agreement findings, and it does not change `audit`'s exit status — exactly as `lingering` does not change `sync`'s tally or `is_clean`.
- No mode this change adds deletes anything, on either side of the boundary.
  Reporting is the entire deliverable; a person still removes a var, on clan's side, with clan's own command.
- The scope of what gets enumerated is exactly the machines the bridge currently names through its mappings, not clan's whole machine inventory (`clan machines list` is deliberately not called — see design.md's D2 for why "one consumer bridges one clan" does not mean one consumer may see every machine of that clan).
  A machine whose last mapping is removed becomes invisible to this report; that is a stated, deliberate limitation and not a defect — see design.md's Risks.
- `crates/safix/tests/support/clan-stub.rs` answers `vars list` for the hermetic suite, and `modules/flake/checks/real-clan.nix` gains a var no mapping names, so the parsing this change adds is held against the real clan CLI's real output shape, not only against a stub written to agree with it.

Not in scope: deleting a lingering var on either side, on any operator's say-so or any mode's; enumerating a clan machine no mapping currently names; anything about `clan secrets users`, which `safix-bridge-real-clan` also does not exercise today and which this change does not touch; and the two-way agreement-memory mechanism `sync-clan-vars-two-way` is building, which lives entirely inside safix's own store and has no clan-side namespace footprint for this change's enumeration to interact with.

## Capabilities

### Modified Capabilities

- `bridge-transfer`: the delegation requirement gains a third kind of clan interaction (enumeration, alongside read and write), and a new requirement states that the audit verb reports clan vars no currently declared mapping accounts for, how that report is scoped, and that it never deletes anything.

## Impact

Affected code:

- `crates/safix-core/src/clan.rs` — new `Clan::list` method and its doc comment ("the two contracts" becomes three); a new error path for a machine that cannot be listed.
- `crates/safix-core/src/error/mod.rs` and `error/prose.rs` — one new `Error` variant and its message.
- `crates/safix-core/src/audit.rs` — `Report::lingering`, the machine-grouping and enumeration that fills it, and the module's own doc comment.
- `crates/safix/src/render.rs` and `crates/safix/src/usage.rs` — the lingering section of `audit`'s printed report, and its help text.
- `README.md:911-914` — the paragraph describing `audit`'s scope currently says it compares declared mappings and nothing else; it needs the same correction.
- `crates/safix/tests/support/clan-stub.rs`, `crates/safix/tests/audit.rs` — hermetic coverage of the new report field.
- `crates/safix/tests/real_clan.rs`, `modules/flake/checks/real-clan.nix` — real-clan coverage that a var no mapping names is actually reported, against the real `clan vars list` output shape.

Sequencing: this change is written assuming `sync-clan-vars-two-way` has already landed, and for a structural reason rather than a scheduling preference.
That change amends bridge-surface's current blanket refusal of a two-way relationship and is expected to give `Mapping` a third kind of relationship alongside its two directions (mirroring keepassxc's `Mode`, per the shared program contract's D2).
This change's `claimed` computation — which var ids count as accounted for — is written against `mapping.clan.machine`, `.generator`, and `.file` alone, never against `mapping.direction`, precisely so that a third relationship kind requires no change here (design.md's D3 records this as a decision rather than an accident).
Both changes touch `crates/safix-core/src/audit.rs` and `modules/flake/checks/real-clan.nix`; landing the two-way change first means this change's diff to those two files is written against their post-two-way shape once, rather than against two moving targets.
