# Tasks: enumerate-clan-namespace

Revisions are as named in `proposal.md`, and every clan-core line anchor below was read at that revision.
Land this after `sync-clan-vars-two-way`; `crates/safix-core/src/audit.rs` and `modules/flake/checks/real-clan.nix` are shared files, and this change's own diff to them is now written directly against two-way's placement model — design.md's D2 and D3 record the enumeration and claimed-set logic that model requires.
No real fleet identifier, real hostname, or real user name enters this repository at any point; fixtures use the `ntfy`/`handover`/`scheduled` generator names `real-clan.nix` already established, and the hermetic suite's existing synthetic names.

## 1. The clan-side facts this design rests on, held before anything is built

- [x] 1.1 Add a test or check fixture asserting that `clan vars list <machine>` at the pinned revision emits one line per var, sorted, of the shape `<generator>/<file>: <state>`, against the real clan built in `modules/flake/checks/real-clan.nix`'s sandbox — this is the fact `Clan::list`'s parser in group 2 depends on, and it is held against the real command rather than assumed from reading `clan_cli/vars/list.py`
- [x] 1.2 In the same fixture, assert that a secret var's state is exactly `********` and that listing does not require an age identity to be present — holding design.md's D1 claim that enumeration never decrypts
- [x] 1.3 Assert that `clan vars list` accepts the same global `--flake` flag in the same position `clan.rs`'s existing `get`/`set`/`check` calls already use, so `Clan::list`'s argument vector is exercised against the real parser rather than only against `clan_cli/cli.py:85-91` as read
- [x] 1.4 Assert that a shared-placement generator's var appears with the identical id in `clan vars list <machine>` for every machine that declares it, holding design.md's Context finding that `vars list`'s underlying selector (`<machine>.config.clan.core.vars.generators.*`) reaches a `Shared`-placed generator through every declaring machine's own configuration
- [x] 1.5 Verify: the fixture from 1.1-1.4 passes against the real clan built in that check's sandbox (log: `logs/1a-safix-bridge-real-clan-retry2-20260903-072743.log`, 24 passed)

## 2. `Clan::list` and the new error path

- [x] 2.1 Add `Clan::list(&self, machine: &str) -> Result<Vec<String>>` to `crates/safix-core/src/clan.rs`, invoking `vars list --flake <flake> <machine>` with stdin null and stdout/stderr piped, mirroring `read`'s and `write`'s spawn shape
- [x] 2.2 Parse each non-empty stdout line by splitting on the first `": "` and keeping the left half as the var id, discarding the state half unread past that split, per design.md's D1
- [x] 2.3 On a non-zero exit, return `Error::ClanMachineListFailed { machine, output }` carrying clan's stderr verbatim, trimmed the way `trimmed()` already trims `read`'s and `write`'s
- [x] 2.4 Add `Error::ClanMachineListFailed` to `crates/safix-core/src/error/mod.rs` and its message to `error/prose.rs`, naming the machine and carrying clan's own output, in the shape `clan_command_failed`'s message already uses
- [x] 2.5 Update `clan.rs`'s module doc comment from "the three contracts" (read, write, machine discovery — `sync-clan-vars-two-way`'s own addition) to name the fourth, and record there that enumeration never sends a machine's vars to standard input and never reads a secret var's value
- [x] 2.6 Verify: `cargo test -p safix-core clan::` passes, including a new hermetic test driving `Clan::list` against a stub answering `vars list` (added in group 5) for both the well-formed and the non-zero-exit cases

## 3. `audit`'s `lingering` field

- [x] 3.1 Add `pub lingering: Vec<String>` to the clan section of `audit::Report` — the per-target report structure `rename-transfer-verbs` gives `audit` (its task 3.1), alongside that section's existing findings and separate from the keepassxc section's own `lingering` field (`rename-transfer-verbs`'s task 3.2) — documented the way `sync::Report::lingering` is: information, not a finding, because no mode here deletes an entry either
- [x] 3.2 Compute the machines to enumerate from the selected set of mappings (the mapping-name list `rename-transfer-verbs`'s `audit clan [<mapping>...]` accepts, narrowing exactly as it narrows comparison, per design.md's D5): `mapping.clan.machine` directly for every per-machine-placement mapping in that set, and the addressing machine `sync-clan-vars-two-way`'s search resolves (`Clan::machines` plus the `get`/`set` trial) for every shared-placement mapping in that set
- [x] 3.3 Compute the claimed set from that same narrowed set, per design.md's D3: a `(machine, id)` pair for every per-machine-placement mapping, using its declared `mapping.clan.machine`; an `id` alone, matched machine-insensitively, for every shared-placement mapping — `mapping.direction` never enters either comparison
- [x] 3.4 For each distinct machine, call `Clan::list`, propagating `Error::ClanUnavailable` and `Error::ClanMachineListFailed` with `?` so either stops the whole run before any mapping is compared, per design.md's D6
- [x] 3.5 Format each unclaimed id as `format!("{machine} {id}")`, matching `audit::Finding::clan`'s existing `"<machine> <generator>/<file>"` shape — `machine` is the listed machine itself: the declared one for a per-machine var, the resolved addressing machine for a shared one — and collect into the clan section's `lingering` sorted by machine then by id
- [x] 3.6 Confirm the clan section's contribution to `Report::is_clean` and any exit-status computation in `crates/safix/src/main.rs`'s `audit_command` are unchanged by `lingering`'s presence — it must not be read by either
- [x] 3.7 Update `audit.rs`'s module doc comment to state the new report alongside the existing one, and why it lives here rather than in a new verb (design.md's D4)
- [x] 3.8 Severity drill: a fixture whose declared mapping's id matches the only var the stub's machine reports, plus a second var the stub reports and no mapping names, must leave `lingering` naming only the second — dropping the claimed-set computation in 3.3 must turn this red on the first (a wrongly-claimed mapping's own var would also appear)
- [x] 3.9 Verify: `cargo test -p safix-core audit::` passes, including the drill in 3.8 observed to fail before 3.3 is correct and to pass after

## 4. Rendering and usage text

- [x] 4.1 Add a lingering section to `crates/safix/src/render.rs`'s `audit` renderer, printed after the clan section's existing findings, naming each entry and stating that nothing was written; print nothing when the clan section's `lingering` is empty, matching every other empty-case branch already in that file
- [x] 4.2 Update `crates/safix/src/usage.rs`'s `AUDIT` constant's clan-target text to state that the report also names clan vars no declared mapping accounts for, and that reporting them is not removing them
- [x] 4.3 Verify: a manual `safix audit clan` render (via a unit test constructing an `audit::Report` with a non-empty `lingering` and asserting on `render::audit`'s output) shows the new section and names no value

## 5. Hermetic test coverage

- [x] 5.1 Extend `crates/safix/tests/support/clan-stub.rs` to answer `["vars", "list", "--flake", _flake, machine]`, printing lines in the `<generator>/<file>: <state>` shape for whatever the stub's fixture has declared for that machine, and update its module doc comment's contract list to name the fourth verb
- [x] 5.2 Add a case where the stub's machine holds a var no fixture mapping names, and a test in `crates/safix/tests/audit.rs` asserting `safix audit clan`'s output names it under the new section
- [x] 5.3 Add a test asserting a machine with no unmapped vars produces an empty `lingering` and no new section in the rendered output
- [x] 5.4 Add a test asserting that removing a previously-declared mapping (simulated by two fixture runs, one with the mapping and one without, both against a stub that never forgets the var it once reported) causes the var to appear in the second run's lingering section, holding the "a removed mapping's var keeps appearing" scenario
- [x] 5.5 Add a test asserting `lingering`'s presence does not change `audit`'s exit code when every compared mapping agrees, holding the "lingering never changes the exit status" scenario
- [x] 5.6 Add a test asserting `audit clan <mapping>` (narrowed) enumerates only the machines that mapping's placement resolves to — its declared machine for a per-machine mapping, or the addressing machine two-way's search resolves for a shared one — by giving the stub two machines and asserting the narrowed run's lingering section names only the one the given mapping declares or resolves, holding design.md's D5
- [x] 5.7 Add a test asserting a shared-placement mapping's var is claimed by id alone: a stub reporting the same generator/file id on two machines' listings, with the claim recognized regardless of which one two-way's addressing search happens to resolve to and enumerate, holding design.md's D3's machine-insensitive comparison for shared placement
- [x] 5.8 Verify: `cargo test -p safix` passes, covering all of 5.1-5.7

## 6. Real-clan coverage

- [x] 6.1 In `modules/flake/checks/real-clan.nix`'s fixture, add a fourth generator or file that no bridge mapping in that check's fleet names, alongside the existing `ntfy`/`handover`/`scheduled` three, so the real clan built there has something for `Clan::list` to find and no mapping to claim it
- [x] 6.2 Add a test to `crates/safix/tests/real_clan.rs` asserting `safix audit clan` against the real clan reports the unmapped var under the new section, naming the real machine and the real generator/file the check declares (no fleet identifier — these are the check's own synthetic names)
- [x] 6.3 Add a test asserting that once a mapping for that var is declared and the fixture rebuilt, the same var stops appearing in `lingering` — holding that the scope is genuinely computed from current declarations and not cached across runs
- [x] 6.4 Severity drill: temporarily removing the claimed-set check in `audit.rs` (per 3.3's drill) and re-running 6.2 against the real clan must also turn red, confirming the hermetic drill in 3.8 is not answering a question only the stub can pose (red: `logs/6.4-drill-red-20260903-073503.log`, the perturbation fails the check one derivation upstream, in `safix-integration`'s own hermetic `audit.rs` suite that `safix-bridge-real-clan` depends on, 18/22 passed there; green after `git checkout -- crates/safix-core/src/audit.rs`: `logs/6.4-drill-green-20260903-073812.log`, 24/24 `against_a_real_clan::*` tests passing; drill-ledger comment in `modules/flake/checks/real-clan.nix` updated with both this drill and the corrected twenty-four-test count)
- [x] 6.5 Verify: `nix build .#checks.x86_64-linux.safix-bridge-real-clan` green on Linux with clan-core in the closure, and its own absence-guard (per that file's existing discipline) still fires when `SAFIX_TEST_REAL_CLAN_SEED` is withheld (log: `logs/1a-safix-bridge-real-clan-retry2-20260903-072743.log`, 24 passed; two real-clan fixture defects fixed first, see the two `fix(real-clan):` commits on this branch)

## 7. Documentation

- [x] 7.1 Rewrite the paragraph beginning "`safix audit` is the report over the same declarations" in `README.md`'s "The bridge to clan" section (`rename-transfer-verbs` rewrites this section; locate by that sentence, not by line number, since anchors drift) to state the new report alongside the existing comparison, and that it is scoped to the machines the bridge's own mappings name or resolve
- [x] 7.2 Add a `CHANGELOG.md` entry under `## [Unreleased]`, following the file's existing style, naming the new lingering report and the scope this design gives it
- [x] 7.3 Verify: `rg -n 'clan vars list' README.md CHANGELOG.md crates/` shows the refusal message in `error/prose.rs`, the new usage/README/CHANGELOG text, and no remaining sentence claiming clan's namespace is only ever checked by hand

## 8. Verification

- [x] 8.1 `openspec validate enumerate-clan-namespace --strict`
- [x] 8.2 `openspec validate --all --strict`
- [x] 8.3 `cargo test` green (log: `logs/cargo-test-workspace-20260903-074055.log`, `cargo test --locked --workspace`, 33 test binaries all `ok`, 0 failed)
- [ ] 8.4 `nix flake check` green
- [x] 8.5 `rg` the whole tree for any real fleet identifier introduced by this change and confirm none
