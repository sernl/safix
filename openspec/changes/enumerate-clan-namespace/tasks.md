# Tasks: enumerate-clan-namespace

Revisions are as named in `proposal.md`, and every clan-core line anchor below was read at that revision.
Land this after `sync-clan-vars-two-way`; `crates/safix-core/src/audit.rs` and `modules/flake/checks/real-clan.nix` are shared files, and design.md's D3 records why this change's own diff to them does not depend on what shape the two-way change gives `Mapping`.
No real fleet identifier, real hostname, or real user name enters this repository at any point; fixtures use the `ntfy`/`handover`/`scheduled` generator names `real-clan.nix` already established, and the hermetic suite's existing synthetic names.

## 1. The clan-side facts this design rests on, held before anything is built

- [ ] 1.1 Add a test or check fixture asserting that `clan vars list <machine>` at the pinned revision emits one line per var, sorted, of the shape `<generator>/<file>: <state>`, against the real clan built in `modules/flake/checks/real-clan.nix`'s sandbox — this is the fact `Clan::list`'s parser in group 2 depends on, and it is held against the real command rather than assumed from reading `clan_cli/vars/list.py`
- [ ] 1.2 In the same fixture, assert that a secret var's state is exactly `********` and that listing does not require an age identity to be present — holding design.md's D1 claim that enumeration never decrypts
- [ ] 1.3 Assert that `clan vars list` accepts the same global `--flake` flag in the same position `clan.rs`'s existing `get`/`set`/`check` calls already use, so `Clan::list`'s argument vector is exercised against the real parser rather than only against `clan_cli/cli.py:85-90` as read
- [ ] 1.4 Verify: the fixture from 1.1-1.3 passes against the real clan built in that check's sandbox

## 2. `Clan::list` and the new error path

- [ ] 2.1 Add `Clan::list(&self, machine: &str) -> Result<Vec<String>>` to `crates/safix-core/src/clan.rs`, invoking `vars list --flake <flake> <machine>` with stdin null and stdout/stderr piped, mirroring `read`'s and `write`'s spawn shape
- [ ] 2.2 Parse each non-empty stdout line by splitting on the first `": "` and keeping the left half as the var id, discarding the state half unread past that split, per design.md's D1
- [ ] 2.3 On a non-zero exit, return `Error::ClanMachineListFailed { machine, output }` carrying clan's stderr verbatim, trimmed the way `trimmed()` already trims `read`'s and `write`'s
- [ ] 2.4 Add `Error::ClanMachineListFailed` to `crates/safix-core/src/error/mod.rs` and its message to `error/prose.rs`, naming the machine and carrying clan's own output, in the shape `clan_command_failed`'s message already uses
- [ ] 2.5 Update `clan.rs`'s module doc comment from "the two contracts" to name the third, and record there that enumeration never sends a machine's vars to standard input and never reads a secret var's value
- [ ] 2.6 Verify: `cargo test -p safix-core clan::` passes, including a new hermetic test driving `Clan::list` against a stub answering `vars list` (added in group 5) for both the well-formed and the non-zero-exit cases

## 3. `audit`'s `lingering` field

- [ ] 3.1 Add `pub lingering: Vec<String>` to `audit::Report`, documented the way `sync::Report::lingering` is: information, not a finding, because no mode here deletes an entry either
- [ ] 3.2 Compute the machines to enumerate as the distinct `mapping.clan.machine` values across `selected(workspace, only)`'s result, per design.md's D5 — not across every declared mapping unconditionally, so `audit clan <mapping>` narrows enumeration the same way it narrows comparison
- [ ] 3.3 Compute the claimed set as `(machine, generator/file id)` pairs from every mapping in that same narrowed set, using `mapping.clan.machine`, `.generator`, and `.file` only — never `mapping.direction` — per design.md's D3
- [ ] 3.4 For each distinct machine, call `Clan::list`, propagating `Error::ClanUnavailable` and `Error::ClanMachineListFailed` with `?` so either stops the whole run before any mapping is compared, per design.md's D6
- [ ] 3.5 Format each unclaimed id as `format!("{machine} {id}")`, matching `audit::Finding::clan`'s existing `"<machine> <generator>/<file>"` shape, and collect into `lingering` sorted by machine then by id
- [ ] 3.6 Confirm `Report::is_clean` and any exit-status computation in `crates/safix/src/main.rs`'s `audit_command` are unchanged by `lingering`'s presence — it must not be read by either
- [ ] 3.7 Update `audit.rs`'s module doc comment to state the new report alongside the existing one, and why it lives here rather than in a new verb (design.md's D4)
- [ ] 3.8 Severity drill: a fixture whose declared mapping's id matches the only var the stub's machine reports, plus a second var the stub reports and no mapping names, must leave `lingering` naming only the second — dropping the claimed-set computation in 3.3 must turn this red on the first (a wrongly-claimed mapping's own var would also appear)
- [ ] 3.9 Verify: `cargo test -p safix-core audit::` passes, including the drill in 3.8 observed to fail before 3.3 is correct and to pass after

## 4. Rendering and usage text

- [ ] 4.1 Add a lingering section to `crates/safix/src/render.rs`'s `audit` renderer, printed after the existing findings, naming each entry and stating that nothing was written; print nothing when `lingering` is empty, matching every other empty-case branch already in that file
- [ ] 4.2 Update `crates/safix/src/usage.rs`'s `AUDIT` constant to state that the report also names clan vars no declared mapping accounts for, and that reporting them is not removing them
- [ ] 4.3 Verify: a manual `safix audit clan` render (via a unit test constructing an `audit::Report` with a non-empty `lingering` and asserting on `render::audit`'s output) shows the new section and names no value

## 5. Hermetic test coverage

- [ ] 5.1 Extend `crates/safix/tests/support/clan-stub.rs` to answer `["vars", "list", "--flake", _flake, machine]`, printing lines in the `<generator>/<file>: <state>` shape for whatever the stub's fixture has declared for that machine, and update its module doc comment's contract list to name the fourth verb
- [ ] 5.2 Add a case where the stub's machine holds a var no fixture mapping names, and a test in `crates/safix/tests/audit.rs` asserting `safix audit clan`'s output names it under the new section
- [ ] 5.3 Add a test asserting a machine with no unmapped vars produces an empty `lingering` and no new section in the rendered output
- [ ] 5.4 Add a test asserting that removing a previously-declared mapping (simulated by two fixture runs, one with the mapping and one without, both against a stub that never forgets the var it once reported) causes the var to appear in the second run's lingering section, holding the "a removed mapping's var keeps appearing" scenario
- [ ] 5.5 Add a test asserting `lingering`'s presence does not change `audit`'s exit code when every compared mapping agrees, holding the "lingering never changes the exit status" scenario
- [ ] 5.6 Add a test asserting `audit clan <mapping>` (narrowed) enumerates only that mapping's machine, by giving the stub two machines and asserting the narrowed run's lingering section names only the one the given mapping declares, holding design.md's D5
- [ ] 5.7 Verify: `cargo test -p safix` passes, covering all of 5.1-5.6

## 6. Real-clan coverage

- [ ] 6.1 In `modules/flake/checks/real-clan.nix`'s fixture, add a fourth generator or file that no bridge mapping in that check's fleet names, alongside the existing `ntfy`/`handover`/`scheduled` three, so the real clan built there has something for `Clan::list` to find and no mapping to claim it
- [ ] 6.2 Add a test to `crates/safix/tests/real_clan.rs` asserting `safix audit clan` against the real clan reports the unmapped var under the new section, naming the real machine and the real generator/file the check declares (no fleet identifier — these are the check's own synthetic names)
- [ ] 6.3 Add a test asserting that once a mapping for that var is declared and the fixture rebuilt, the same var stops appearing in `lingering` — holding that the scope is genuinely computed from current declarations and not cached across runs
- [ ] 6.4 Severity drill: temporarily removing the claimed-set check in `audit.rs` (per 3.3's drill) and re-running 6.2 against the real clan must also turn red, confirming the hermetic drill in 3.8 is not answering a question only the stub can pose
- [ ] 6.5 Verify: `nix build .#checks.x86_64-linux.safix-bridge-real-clan` green on Linux with clan-core in the closure, and its own absence-guard (per that file's existing discipline) still fires when `SAFIX_TEST_REAL_CLAN_SEED` is withheld

## 7. Documentation

- [ ] 7.1 Rewrite `README.md:911-914`'s description of `audit`'s scope to state the new report alongside the existing comparison, and that it is scoped to the machines the bridge's own mappings name
- [ ] 7.2 Verify: `rg -n 'clan vars list' README.md crates/` shows the refusal message in `error/prose.rs` and the new usage/README text, and no remaining sentence claiming clan's namespace is only ever checked by hand

## 8. Verification

- [ ] 8.1 `openspec validate enumerate-clan-namespace --strict`
- [ ] 8.2 `openspec validate --all --strict`
- [ ] 8.3 `cargo test` green
- [ ] 8.4 `nix flake check` green
- [ ] 8.5 `rg` the whole tree for any real fleet identifier introduced by this change and confirm none
