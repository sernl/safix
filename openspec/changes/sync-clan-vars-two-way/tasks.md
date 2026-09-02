# Tasks: sync-clan-vars-two-way

Citations are as `design.md` reads them. Where a task says "hold", add a check that fails when the claim stops being true, following `modules/flake/checks/bridge.nix`'s own pattern of a perturbed fixture per refusal, not a sentence asserting it.

No real recipient, no real hostname, no real machine or user name from any fleet enters this repository; fixtures use the `alice`/`bob`/`carol` names the existing bridge and consumption fixtures already use, and synthetic clan machine names such as `nonexistent` the way `modules/flake/checks/bridge.nix:112-120` already does.

## 1. The placement model, in nix

- [x] 1.1 Add `placement` (`shared | per-machine`, default `per-machine`) to `clanSide` in `modules/flake/safix/bridge.nix`, and change `machine` from `str` to `nullOr str` (null exactly when placement is shared)
- [x] 1.2 Add `"two-way"` to `directions` and confirm `bridgeOf`'s existing `typed` harness in `modules/flake/checks/bridge.nix` accepts a mapping declaring it, without building a derivation
- [x] 1.3 Rewrite `endpointsOf` and `targetOf` to key on placement: per-machine keeps `<machine>:<generator>/<file>`; shared becomes `shared:<generator>/<file>`
- [x] 1.4 Add the placement/machine consistency refusal to `violationsOf`: per-machine with no machine, or shared with machine set
- [x] 1.5 Narrow `bothDirections`'s message from "has no conflict resolution" to naming that a two-way relationship is declared once, and add a fixture asserting a single `direction = "two-way"` mapping over the same pair of endpoints produces no message
- [x] 1.6 Verify: extend `checks.safix-bridge`'s `actual`/`expected` in `modules/flake/checks/bridge.nix` with fixtures for each new placement, the narrowed two-way message, the accepted single two-way declaration, and the consistency refusal; `nix build .#checks.x86_64-linux.safix-bridge` green
- [x] 1.7 Severity drill: a shared-placement fixture naming a machine anyway turns 1.4's new assertion red (`sharedMachineSetMessages`); two shared mappings of one generator/file colliding by generator/file alone turns the broadened 1.3 target-collision assertion red where the old machine-keyed `targetOf` would have stayed green (`sharedDuplicateTargetMessages`) — both observed red before the fix and green after

## 2. The generator-share comparison, and two-producers broadened to two-way

- [x] 2.1 In `violationsOf`, extend `twoProducers`'s condition from `direction == "clan-to-safix"` to `direction == "clan-to-safix" || direction == "two-way"`
- [x] 2.2 Add the share/placement comparison: for a `safix-to-clan` mapping whose source is generator-produced, refuse when the generator's derived `share` disagrees with the mapping's `placement` (`share = true` requires `shared`; `share = false` requires `per-machine`)
- [x] 2.3 Add fixtures: a shared generator paired with `shared` (accepted), a shared generator paired with `per-machine` (refused, naming both), a per-user generator paired with `shared` (refused), and a hand-set safix-to-clan source with any placement (accepted, no message)
- [x] 2.4 Verify: `nix build .#checks.x86_64-linux.safix-bridge` green with 2.3's fixtures added to `actual`/`expected`
- [x] 2.5 Severity drill: dropping 2.2's comparison turns the two refusal fixtures in 2.3 green when they should be red; dropping 2.1's broadening turns a two-way mapping over a generator-produced source green when it should be refused by the two-producers rule (`twoWayTwoProducersMessages`)

## 3. The companion entry: reservation and minting

- [x] 3.1 In `modules/flake/safix/bridge.nix`, for every `two-way` mapping, mint a second placement (`companionsOf`, folded into `flake.safix.lib.placements` by `default.nix` rather than into `resolve.nix`'s own algebra) sharing the mapped entry's `file`, distinguished by a reserved key suffix drawn from the same alphabet `wellFormedName` requires of every declarable entry name (`-safix-bridge-sync-state`, not a dot-prefixed form: `resolve.nix`'s own name check refuses any declared name outside `[a-z0-9][a-z0-9_-]*`, so a dot-prefixed suffix could never collide with a hand declaration and the reservation refusal below would be unreachable; distinct from `store.rs`'s `.safix-sync-state`, which names a kdbx path rather than a safix entry and is under no such constraint, so the two mechanisms' reservations are independently checkable)
- [x] 3.2 Add the reservation refusal: a hand-declared entry whose name carries the suffix is refused, naming the entry, the mapping that reserves it, and the suffix, mirroring `modules/flake/safix/keepassxc.nix:208-211`'s shape
- [x] 3.3 Confirm a mapping with no two-way declaration mints no companion, by asserting the resolved placement set for a fixture with only one-way mappings is unchanged by this change (`checks/bridge-sync.nix`'s `oneWayCompanions`/`placementsUnchangedByOneWay`)
- [x] 3.4 Verify: a new structural check (`modules/flake/checks/bridge-sync.nix`, imported from `flake.nix`) asserting the companion's file and audience equal the mapped entry's, and that the reservation refusal fires; `nix build .#checks.x86_64-linux.safix-bridge-sync .#checks.x86_64-linux.safix-bridge-sync-drill` green
- [x] 3.5 Severity drill: `checks/bridge-sync.nix`'s drill runs `refuseScript` over a fleet whose alice has hand-declared the reserved companion name; dropping `reservedCompanionName` from `violationsOf`'s list turns the drill's `grep` red — observed red before the fix and green after

## 4. The rust model: `ClanPlacement`, and the widened `ClanSide`

- [x] 4.1 Add `ClanPlacement` to `crates/safix-core/src/model.rs` (`Shared | PerMachine`, `#[serde(rename = "shared" | "per-machine")]`), named apart from the existing `Placement` struct at `:147`
- [x] 4.2 Change `ClanSide.machine` to `Option<String>`, add `ClanSide.placement: ClanPlacement`, keeping `#[serde(deny_unknown_fields)]`
- [x] 4.3 Add new `Error` variants: the placement/machine mismatch is nix-side only and needs no rust variant; added `ClanAddressUnresolved` (no machine in the fleet resolves a shared mapping) and `ClanMachinesListFailed` (the `clan machines list` subprocess itself refuses). `SyncConflict` was NOT added: `bridge_sync::Outcome::Conflict` carries no `Error`, mirroring `sync::Outcome::Conflict`'s own precedent exactly (a conflict is a finding the decision function reaches, not a failed write), so no refusal exists for a code table entry to attach to
- [x] 4.4 Unit test: `Placements`/`Bridge` deserialize a fixture JSON carrying `placement: "shared"` with `machine: null`, and one carrying `placement: "per-machine"` with `machine` set, matching the shape 4.1/4.2 emit
- [x] 4.5 Verify: `cargo test -p safix-core model::` green

## 5. `Clan::machines` and addressing-machine discovery

- [x] 5.1 Add `Clan::machines(&self) -> Result<Vec<String>>` invoking `clan machines list --flake <flake>`, piped stdout, parsed one name per line, beside `probe`/`register_user`/`generator_stale`
- [x] 5.2 Add an addressing-search helper (`bridge::Addressing`, memoized per run keyed on `(generator, file)`), trying each machine `machines()` returns against `clan vars get` in turn, using the existing `NO_SUCH_VAR`-derived `Error::ClanVarUnknown` to tell "wrong machine" apart from a genuine failure
- [x] 5.3 Wire the search into every read/write of a shared mapping's clan side: `bridge::one_import`/`one_export` and `audit::compare` now go through `Addressing` rather than `Clan` directly; a per-machine mapping is unaffected and still uses its declared `machine` directly (`Addressing::resolve`'s first branch)
- [x] 5.4 Add `Error::ClanAddressUnresolved` naming the mapping, the generator and the file, raised when the search exhausts every machine `machines()` returned
- [x] 5.5 Test against a stub `clan` command, built the way `Clan::for_tests` (`clan.rs`) lets `bridge::tests` construct one: `machines list` returns three names, one of which resolves the var; the search stops at the first success and does not try the remaining one (`bridge.rs::tests::a_shared_addressing_search_stops_at_the_first_machine_that_resolves`)
- [x] 5.6 Test the exhaustion case: a stub where no returned machine resolves the var; `ClanAddressUnresolved` is raised and every returned machine was tried exactly once (`bridge.rs::tests::exhausting_every_machine_is_refused_naming_the_mapping_and_tries_each_once`)
- [x] 5.7 Verify: `cargo test -p safix-core bridge::` green (anchor corrected from `clan::`: `Addressing` and its tests are `bridge.rs`'s own, and `Clan::for_tests` is the one addition `clan::` tests gain)

## 6. The two-way decision function

- [x] 6.1 Added `bridge_sync::decide`, split into an impure outer wrapper (reads both sides through `addressing.read`/`bridge::held_for`, then the companion only when both sides are present and differ) and a pure inner `judge`, mirroring `crate::sync::decide`/`two_way`'s own split (`sync.rs:378-493`) exactly — `judge` is the direct successor of `two_way`'s four-way match, named apart from it because `bridge_sync` has only the two-way case and no `mapping.mode` to dispatch on first
- [x] 6.2 Added `bridge_sync::FORMAT = "safix-bridge-sync-v1"`, `memory_of`, and `agrees`, reusing `Secret::fingerprint()` and the byte-comparison shape of `sync.rs:512-523` under the distinct tag
- [x] 6.3 Added `bridge_sync::push`/`pull`, reusing `Addressing::write` under the identical comparison (already made in `decide`) and `generator_stale` refusal `one_export` uses, and `set::run_committing` for the safix side the way `one_import` does, each followed by the companion's own write (`remember_agreement`) as a second, separate commit, value before agreement
- [x] 6.4 Unit tested the four outcome classes against `judge` directly (no `Workspace`/stub `Clan` fixture: safix-core carries no such fixture anywhere in its own test suite, and `sync.rs`'s own precedent likewise unit-tests only its pure `two_way` core, never `decide`/`push`/`pull`'s I/O) — neither moved, one moved either way, both moved, one side never held a value. "Both writes observed" for `push`/`pull` themselves is exercised by the group 7 integration suite (`tests/bridge_sync.rs`), which supplies a real `Workspace` and a stub `Clan` the way `tests/bridge.rs` already does for the one-way writes
- [x] 6.5 Unit tested "no memory yet, sides already agree" (`agreeing_values_are_unchanged_whatever_the_companion_says`, including with an unrelated companion present) separately from "no memory yet, sides disagree" (`disagreeing_with_no_agreement_recorded_yet_is_a_conflict`)
- [x] 6.6 Unit tested the interruption case D8 names (`a_stale_companion_from_an_interrupted_write_never_resolves_the_next_divergence_by_a_guess`): a companion holding an agreement older than either side's current bytes, where one side's current bytes are what an interrupted write already landed and the other changed again afterward — reconstructed concretely since D8's own prose states the property rather than a literal fixture; the test's doc comment records the reconstruction and why it exercises a distinct branch from 6.5's "no memory" case (`remembered` is `Some` but matches neither side, rather than `None`)
- [x] 6.7 Verify: `cargo test -p safix-core bridge_sync::` green (10 passed)

## 7. `two-way` in `sync clan`'s `--direction` filter

- [x] 7.1 Added `bridge_sync::converge`, decide-then-write in two passes, reached from `main.rs::sync_command` alongside `bridge::sync` for the clan target on the same `direction`/`only`
- [x] 7.2 Added `two-way` to `--direction`'s accepted values in `main.rs`'s parser and to `Direction`'s deserialization (already landed in group 4); the generic filter-mismatch refusal fires unmodified in both directions (`tests/bridge.rs::a_two_way_mapping_and_a_one_way_filter_are_told_apart_in_both_directions`)
- [x] 7.3 Extended `usage::SYNC`'s clan-target section with `two-way`, its four-outcome summary, the companion's hyphenated suffix, and the addressing-machine discovery sentence
- [x] 7.4 Wired the report: `bridge_sync::Outcome` carries the five classes; `render::bridge_sync` prints `converged <mapping>` for both settled-write outcomes (T5: both `UpdatedTowardClan`/`UpdatedTowardSafix` render identically, no arrow, per task 7.4's own literal phrasing and the bridge-sync spec's "Requirement: The report names mappings and their outcome" paragraph — the spec wins, applied as written) and the conflict remedy naming the two one-way `--direction` overrides plus the nix-declaration edit D8/keepassxc-sync precedent requires
- [x] 7.5 Integration tests: direction-filter mismatch both ways (`tests/bridge.rs`), and `tests/bridge_sync.rs` (new target) covers all four outcome classes, the stale-generator refusal, the two-separate-commits ordering, and shared-placement discovery over the stubbed clan
- [x] 7.6 Verify: `cargo test -p safix` green (`bridge` 21/21, `bridge_sync` 7/7); `safix sync -h` names `two-way` (`sync_appears_in_the_help_with_all_three_directions`)

## 8. `real_clan.rs`: the four outcome classes against a real clan

- [ ] 8.1 Extend the `bridged()` fixture helper to accept a `direction` of `two-way` and a `placement`
- [ ] 8.2 Test neither-moved: seed both sides equal, run `sync clan --direction two-way`, assert unchanged and that clan's tree digest (`real_clan.rs:562-585`) is unchanged
- [ ] 8.3 Test one-moved toward clan: change safix's side, run `sync clan --direction two-way`, assert clan's var now matches and the companion updated
- [ ] 8.4 Test one-moved toward safix: change clan's side via a direct `clan vars set` in the test harness (not through the runtime under test), run `sync clan --direction two-way`, assert safix's entry now matches
- [ ] 8.5 Test both-moved: change both sides independently, run `sync clan --direction two-way`, assert conflict, nothing written on either side, and clan's tree digest unchanged
- [ ] 8.6 Extend `the_runtime_reached_clans_store_only_through_clans_command` (`real_clan.rs:543-559`) to run a two-way `sync clan` in its bridged-agreement sequence, so the whole-tree digest proof also covers the two-way path, not only the two one-way directions
- [ ] 8.7 Test the stale-generator refusal on a two-way push, reusing `export_refuses_a_generator_that_declares_a_validation_and_has_never_run`'s fixture shape (`real_clan.rs:511-530`) with a two-way mapping instead of safix-to-clan
- [ ] 8.8 Test the addressing-machine search against a real clan with a shared-placement generator, asserting the runtime never names a machine on the command line that the consumer did not have to declare
- [ ] 8.9 Verify: `cargo test -p safix real_clan` green where a real clan is available; each test's `no_clan_here` fallback confirmed to skip rather than fail where it is not, following the existing pattern


## 9. Documentation

- [x] 9.1 Already satisfied: `placement`'s and `machine`'s own `description`s in `modules/flake/safix/bridge.nix` (group 1) state the addressing-machine discovery and the shared/per-machine consistency rule; confirmed against the literal task text
- [x] 9.2 Already satisfied: the `stateSuffix`/`companionOf`/`companionsOf` comment block in `bridge.nix` (group 3) documents the naming, the shared file/audience, and the reservation refusal; not literally beside `resolve.nix` (the minting itself lives in `bridge.nix`, by group 3's own design decision) but adjacent to the minting logic it describes
- [x] 9.3 Done in group 7.3: `usage::SYNC`'s `\u{2500}\u{2500} two-way, across the clan boundary \u{2500}\u{2500}` section names the unchanged/converge/conflict outcomes, the remedy, and the companion's naming and discovery
- [ ] 9.4 Verify: every guarantee stated in the new documentation names a check or test in this repository that holds it

## 10. Verification

- [ ] 10.1 `openspec validate sync-clan-vars-two-way --strict`
- [ ] 10.2 `openspec validate --all --strict`
- [ ] 10.3 `nix eval .#checks.x86_64-linux --apply builtins.attrNames` lists `safix-bridge`, `safix-bridge-drill`, and every new check named in groups 1 through 3
- [ ] 10.4 `nix flake check` green
- [ ] 10.5 `cargo test` green
- [ ] 10.6 `rg` the whole tree for any real fleet identifier, machine name, or clan flake reference and confirm none
