# Tasks: sync-clan-vars-two-way

Citations are as `design.md` reads them. Where a task says "hold", add a check that fails when the claim stops being true, following `modules/flake/checks/bridge.nix`'s own pattern of a perturbed fixture per refusal, not a sentence asserting it.

No real recipient, no real hostname, no real machine or user name from any fleet enters this repository; fixtures use the `alice`/`bob`/`carol` names the existing bridge and consumption fixtures already use, and synthetic clan machine names such as `nonexistent` the way `modules/flake/checks/bridge.nix:112-120` already does.

## 1. The placement model, in nix

- [ ] 1.1 Add `placement` (`shared | per-machine | per-export`, default `per-machine`) and `export` (`nullOr str`, default `null`) to `clanSide` in `modules/flake/safix/bridge.nix`, and change `machine` from `str` to `nullOr str`
- [ ] 1.2 Add `"two-way"` to `directions` and confirm `bridgeOf`'s existing `typed` harness in `modules/flake/checks/bridge.nix` accepts a mapping declaring it, without building a derivation
- [ ] 1.3 Rewrite `endpointsOf` and `targetOf` to key on placement: per-machine keeps `<machine>:<generator>/<file>`; shared becomes `shared:<generator>/<file>`; per-export becomes `export:<export>:<generator>/<file>`
- [ ] 1.4 Add the placement/machine/export consistency refusal to `violationsOf`: per-machine with no machine, per-export with no export, or a non-per-machine placement with machine set, or a non-per-export placement with export set
- [ ] 1.5 Narrow `bothDirections`'s message from "has no conflict resolution" to naming that a two-way relationship is declared once, and add a fixture asserting a single `direction = "two-way"` mapping over the same pair of endpoints produces no message
- [ ] 1.6 Verify: extend `checks.safix-bridge`'s `actual`/`expected` in `modules/flake/checks/bridge.nix` with fixtures for each new placement, the narrowed two-way message, the accepted single two-way declaration, and the consistency refusal; `nix build .#checks.x86_64-linux.safix-bridge` green
- [ ] 1.7 Severity drill: a shared-placement fixture naming a machine anyway turns 1.4's new assertion red; two mappings of one shared var naming two different machines turns the broadened 1.3 target-collision assertion red where the old machine-keyed `targetOf` would have stayed green — hold both, observed red before the fix and green after

## 2. The generator-share comparison, and two-producers broadened to two-way

- [ ] 2.1 In `violationsOf`, extend `twoProducers`'s condition from `direction == "clan-to-safix"` to `direction == "clan-to-safix" || direction == "two-way"`
- [ ] 2.2 Add the share/placement comparison: for a `safix-to-clan` mapping whose source is generator-produced, refuse when the generator's derived `share` disagrees with the mapping's `placement` (`share = true` requires `shared`; `share = false` requires `per-machine` or `per-export`)
- [ ] 2.3 Add fixtures: a shared generator paired with `shared` (accepted), a shared generator paired with `per-machine` (refused, naming both), a per-user generator paired with `shared` (refused), and a hand-set safix-to-clan source with any placement (accepted, no message)
- [ ] 2.4 Verify: `nix build .#checks.x86_64-linux.safix-bridge` green with 2.3's fixtures added to `actual`/`expected`
- [ ] 2.5 Severity drill: dropping 2.2's comparison turns the two refusal fixtures in 2.3 green when they should be red; dropping 2.1's broadening turns a two-way mapping over a generator-produced source green when it should be refused by the two-producers rule

## 3. The companion entry: reservation and minting

- [ ] 3.1 In `modules/flake/safix/resolve.nix`, for every `two-way` mapping, mint a second placement sharing the mapped entry's `file`, distinguished by a reserved key suffix (`.safix-bridge-sync-state`, distinct from `store.rs`'s `.safix-sync-state` so the two mechanisms' reservations are independently checkable)
- [ ] 3.2 Add the reservation refusal: a hand-declared entry whose name carries the suffix is refused, naming the entry, the mapping that reserves it, and the suffix, mirroring `modules/flake/safix/keepassxc.nix:208-211`'s shape
- [ ] 3.3 Confirm a mapping with no two-way declaration mints no companion, by asserting the resolved placement set for a fixture with only one-way mappings is unchanged by this change
- [ ] 3.4 Verify: a new or extended structural check (`modules/flake/checks/bridge-sync.nix` or an addition to `resolve.nix`'s own check) asserting the companion's file and audience equal the mapped entry's, and that the reservation refusal fires
- [ ] 3.5 Severity drill: dropping 3.2's refusal lets a hand-declared entry collide with a companion's name and evaluate clean; hold it red before 3.2 lands and green after

## 4. The rust model: `ClanPlacement`, and the widened `ClanSide`

- [ ] 4.1 Add `ClanPlacement` to `crates/safix-core/src/model.rs` (`Shared | PerMachine | PerExport`, `#[serde(rename = "shared" | "per-machine" | "per-export")]`), named apart from the existing `Placement` struct at `:147`
- [ ] 4.2 Change `ClanSide.machine` to `Option<String>`, add `ClanSide.export: Option<String>` and `ClanSide.placement: ClanPlacement`, keeping `#[serde(deny_unknown_fields)]`
- [ ] 4.3 Add new `Error` variants: the placement/machine/export mismatch is nix-side only and needs no rust variant; add `ClanAddressUnresolved` (no machine in the fleet resolves a shared/per-export mapping), `SyncConflict` (both sides moved), and any variant `bridge_sync`'s comparison-and-write path needs that `bridge.rs` does not already have
- [ ] 4.4 Unit test: `Placements`/`Bridge` deserialize a fixture JSON carrying `placement: "shared"` with `machine: null` and `export: null`, and one carrying `placement: "per-export"` with `export` set and `machine: null`, matching the shape 4.1/4.2 emit
- [ ] 4.5 Verify: `cargo test -p safix-core model::` green

## 5. `Clan::machines` and addressing-machine discovery

- [ ] 5.1 Add `Clan::machines(&self) -> Result<Vec<String>>` invoking `clan machines list --flake <flake>`, piped stdout, parsed one name per line, beside `probe`/`register_user`/`generator_stale` (`clan.rs:126-138,240-269,295-317`)
- [ ] 5.2 Add an addressing-search helper, memoized per run keyed on `(generator, file)`, trying each machine `machines()` returns against `clan vars get`/`set` in turn, using the existing `NO_SUCH_VAR` substring match to tell "wrong machine" apart from a genuine failure
- [ ] 5.3 Wire the search into every read/write of a shared or per-export mapping's clan side, for both `import`/`export`/`bridge`; a per-machine mapping is unaffected and still uses its declared `machine` directly
- [ ] 5.4 Add `Error::ClanAddressUnresolved` naming the mapping, the placement, the generator and the file, raised when the search exhausts every machine `machines()` returned
- [ ] 5.5 Test against `crate::clan`'s existing stub harness (`clan.rs:395-413`'s pattern): a stub `clan machines list` returning three names, one of which resolves the var; assert the search stops at the first success and does not try the remaining two
- [ ] 5.6 Test the exhaustion case: a stub where no returned machine resolves the var; assert `ClanAddressUnresolved` and that every returned machine was tried exactly once
- [ ] 5.7 Verify: `cargo test -p safix-core clan::` green

## 6. The two-way decision function

- [ ] 6.1 Add `bridge_sync::decide`, mirroring `crate::sync::two_way`'s four-way match (`sync.rs:439-481`) over `clan.read(...)` and `bridge::held_by_safix(...)`, reading the companion the same way a mapped entry is read
- [ ] 6.2 Add `bridge_sync::FORMAT = "safix-bridge-sync-v1"`, `memory_of`, and `agrees`, reusing `Secret::fingerprint()` and the byte-comparison shape of `sync.rs:492-511` under the distinct tag
- [ ] 6.3 Add `bridge_sync::push`/`pull`, reusing `Clan::write` under the identical comparison and `generator_stale` refusal `one_export` uses (`bridge.rs:355-376`), and `set::run_committing` for the safix side (`bridge.rs:303-310`), each followed by the companion write as a second, separate commit, value before agreement
- [ ] 6.4 Unit test each of the four outcome classes against a fixture `Workspace`/stub `Clan`: neither moved (unchanged, nothing written); one moved either way (converge, both writes observed, agreement updated); both moved (conflict, nothing written); one side never held a value (bootstrap converge, agreement recorded)
- [ ] 6.5 Unit test the "no memory yet, sides already agree" case separately from "no memory yet, sides disagree": the first is unchanged with no companion write; the second is conflict
- [ ] 6.6 Unit test the interruption case design.md's D8 names: a companion reflecting an older agreement, followed by a one-sided change on the side that matches the old agreement; assert the outcome is conflict, not a silent overwrite
- [ ] 6.7 Verify: `cargo test -p safix-core bridge_sync::` green

## 7. The `bridge` verb

- [ ] 7.1 Add `bridge_sync::converge` (import/export/bridge orchestration shape: `probe`, select mappings, decide then write in two passes so writes stay a single burst, mirroring `sync.rs::run`'s decide-then-write split at `sync.rs:272-307`)
- [ ] 7.2 Extend `bridge::selected`'s wrong-verb refusal (`bridge.rs:191-221`) to three directions: a two-way mapping named to import/export names `bridge`; a one-way mapping named to `bridge` names import or export
- [ ] 7.3 Register `bridge` in `crates/safix/src/main.rs`'s `VERBS` table and add `usage::BRIDGE`, following the existing entries' shape (`main.rs:147-166`)
- [ ] 7.4 Wire the report: `bridge`'s `Outcome` carries `Unchanged | UpdatedTowardClan | UpdatedTowardSafix | Conflict | Refused(Error)`, distinct from `crate::bridge::Outcome`'s four and `crate::sync::Outcome`'s six, and its renderer prints the conflict remedy named in `bridge-sync`'s spec
- [ ] 7.5 Integration test: `bridge <mapping>` on a two-way mapping named in the other direction's verb list refuses correctly in both directions; `import <mapping>`/`export <mapping>` on a two-way-declared mapping refuse naming `bridge`
- [ ] 7.6 Verify: `cargo test -p safix` green for the new subcommand's dispatch and refusal tests; `safix -h` lists `bridge` (snapshot test following `main.rs:811-836`'s pattern)

## 8. `real_clan.rs`: the four outcome classes against a real clan

- [ ] 8.1 Extend the `bridged()` fixture helper to accept a `direction` of `two-way` and a `placement`
- [ ] 8.2 Test neither-moved: seed both sides equal, run `bridge`, assert unchanged and that clan's tree digest (`real_clan.rs:562-585`) is unchanged
- [ ] 8.3 Test one-moved toward clan: change safix's side, run `bridge`, assert clan's var now matches and the companion updated
- [ ] 8.4 Test one-moved toward safix: change clan's side via a direct `clan vars set` in the test harness (not through the runtime under test), run `bridge`, assert safix's entry now matches
- [ ] 8.5 Test both-moved: change both sides independently, run `bridge`, assert conflict, nothing written on either side, and clan's tree digest unchanged
- [ ] 8.6 Extend `the_runtime_reached_clans_store_only_through_clans_command` (`real_clan.rs:543-559`) to run `bridge` in its bridged-agreement sequence, so the whole-tree digest proof also covers the two-way path, not only import/audit/import
- [ ] 8.7 Test the stale-generator refusal on a two-way push, reusing `export_refuses_a_generator_that_declares_a_validation_and_has_never_run`'s fixture shape (`real_clan.rs:511-530`) with a two-way mapping instead of safix-to-clan
- [ ] 8.8 Test the addressing-machine search against a real clan with a shared-placement generator, asserting the runtime never names a machine on the command line that the consumer did not have to declare
- [ ] 8.9 Verify: `cargo test -p safix real_clan` green where a real clan is available; each test's `no_clan_here` fallback confirmed to skip rather than fail where it is not, following the existing pattern

## 9. Documentation

- [ ] 9.1 Document `placement`/`export` on `clanSide` in `modules/flake/safix/bridge.nix`, stating the addressing-machine discovery and why `machine` is forbidden outside per-machine
- [ ] 9.2 Document the companion entry's naming, its shared file/audience, and the reservation refusal beside `resolve.nix`'s minting logic
- [ ] 9.3 Document the `bridge` verb in `usage::BRIDGE`, naming the four outcome classes and the conflict remedy
- [ ] 9.4 Verify: every guarantee stated in the new documentation names a check or test in this repository that holds it

## 10. Verification

- [ ] 10.1 `openspec validate sync-clan-vars-two-way --strict`
- [ ] 10.2 `openspec validate --all --strict`
- [ ] 10.3 `nix eval .#checks.x86_64-linux --apply builtins.attrNames` lists `safix-bridge`, `safix-bridge-drill`, and every new check named in groups 1 through 3
- [ ] 10.4 `nix flake check` green
- [ ] 10.5 `cargo test` green
- [ ] 10.6 `rg` the whole tree for any real fleet identifier, machine name, or clan flake reference and confirm none
