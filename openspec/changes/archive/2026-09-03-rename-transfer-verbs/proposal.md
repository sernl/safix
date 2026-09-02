# sync and audit become target-scoped, and import/export retire into sync's clan target

Revisions are safix's own working tree on `propose-flakeless-and-first-class-integrations`; clan-core is pinned at `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, read at `/home/sernl/ghq/git.clan.lol/clan/clan-core`.

## Why

safix has two verbs that each move one direction of the clan bridge (`import`, `export`), one verb that reports the clan bridge's drift and nothing else (`audit`), and one verb that converges an entirely different boundary, the keepassxc mirror (`sync`).
An operator who wants both bridge directions converged today runs two commands; an operator who wants to know whether the keepassxc mirror has drifted without writing to it has no verb for that at all, because `sync` always converges.
Nothing about `audit`'s report or `sync`'s convergence loop is boundary-specific: both already read both sides of a declared relationship, compare, and either report or write.
The split is purely which boundary each verb happens to have been written against first.

Two words carry an inherited ambiguity on top of that split.
`import` and `export` are named from clan's own vocabulary, where `clan vars import`/`clan vars export` move a machine's whole plaintext vars folder across clan's own store boundary (`pkgs/clan-cli/clan_cli/vars/cli.py`, `clan_lib/vars/generate.py:_generate_vars_for_machine`'s `MachineVarsForceUpdate.always` and `.only-if-secrets-not-empty` do not name this path; `clan_lib/import.py`'s `import_sops_vars` does).
`clan vars export <dir>` writes exactly the bulk plaintext dump `openspec/specs/safix-cli/spec.md`'s "Absent verbs are recorded rather than left mysterious" requirement already refuses safix ever producing, and `clan vars import` reads one back in.
safix's `import`/`export` never did that: they move one declared mapping, encrypted, through the ordinary write path, one direction at a time.
The names promise clan's bulk plaintext operation and deliver something else, and `openspec/specs/bridge-surface/spec.md`'s own "Direction is written as its endpoints, not relative to a tool" requirement already states the reason a direction cannot be spelled as a verb borrowed from the tool on the other side of it: the word means one thing when clan says it and the opposite when safix does.

## What Changes

- **BREAKING**: `import` and `export` are removed as subcommands, with no alias and no deprecation period.
  `sync` gains a `clan` target: `safix sync clan [<mapping>...]` converges every declared bridge mapping, or the ones named, each moving in its own declared direction — `safix-to-clan` and `clan-to-safix` mappings in the same run, where reaching both previously took two commands.
  A new `--direction clan-to-safix|safix-to-clan` option narrows a `sync clan` run to mappings declared with that value, replacing what running `import` alone or `export` alone used to mean.
  `pull`/`push` are not offered as spellings for this narrowing: `bridge-surface`'s existing requirement already refuses a direction word whose meaning depends on which tool is speaking, and `--direction` keeps to the same two endpoint-named values that requirement already declares.
- `sync` gains a `keepassxc` target: `safix sync keepassxc [<mapping>...]` is exactly today's `safix sync [<mapping>...]`, renamed to make room for the clan target beside it.
  Bare `safix sync` with no target and no mapping names converges every declared relationship on every target, each mapping in its own declared direction or mode — the one spelling that means "everything"; there is no `all` keyword, because a second spelling for the same run invites the two to drift.
- `audit` gains the same two targets, generalizing what was already clan-only: `safix audit clan [<mapping>...]` is today's `audit`, renamed; `safix audit keepassxc [<mapping>...]` is new — a read-only comparison of the keepassxc mirror that writes nothing, filling a gap `sync`'s always-converging loop left, and it reports lingering database entries the same way `sync`'s own report already does.
  Bare `safix audit` compares every target.
- Mapping names become variadic (`[<mapping>...]`) on both verbs, on both targets, where every one of `import`, `export`, `audit` and `sync` today accepts at most one.
  A target argument is required to name any mapping; there is no bare `safix sync <mapping>` that guesses which target's namespace the name belongs to, because a mapping id can be declared under both `flake.safix.bridge.mappings` and `flake.safix.keepassxc.mappings` without conflict today, and a guess would be ambiguous exactly when a name happens to collide.
- The mapping ids `clan`, `keepassxc` and `all` are refused at evaluation, in both `flake.safix.bridge.mappings` and `flake.safix.keepassxc.mappings`, so the first word after `sync` or `audit` is always unambiguously a target keyword or a mapping name, never both.
- `keygen` gains `--show`: prints the operator's own public recipient, derived from the identity already minted on this machine, and mints nothing.
- `import` is retired with a recorded reservation rather than a silent removal: the word is reserved for a future, unbuilt feature — ingesting a value from an external plaintext source one entry at a time, analogous to clan's own `import-sops`, plaintext only in memory and never written to a tree — and the help text records the reservation so the absence reads as a decision rather than an oversight.
  `export` is retired permanently, with no reservation: the operation clan's own word names, a bulk plaintext dump, is the one safix's design refuses to build, on either side of the boundary.

## Capabilities

### Modified Capabilities

- `bridge-transfer`: the two-verbs-one-per-direction requirement becomes a target-scoped, direction-narrowed `sync clan` requirement; every requirement that named `import` or `export` by name is reworded to the safix-to-clan/clan-to-safix directions `sync` now carries in one run; the audit requirement is scoped explicitly to the clan target and cross-references the keepassxc target's own comparison.
- `keepassxc-sync`: `sync`'s declared-mapping requirement gains the `keepassxc` target's own scoping language, alongside `clan`'s; a new requirement adds the read-only `audit keepassxc` comparison, including the lingering report `sync` already produces; the declaration requirement's evaluation refusals gain the reserved mapping ids.
- `bridge-surface`: the mapping declaration requirement gains the reserved mapping ids alongside its existing evaluation-time refusals, and the direction option's own rationale scenario is reworded away from a safix `export` verb this change retires, toward clan's own `vars export`.
- `safix-cli`: a new requirement states the target-and-direction dispatch grammar shared by `sync` and `audit`; the absent-verbs requirement's `import`/`export` scenario is replaced with the recorded absences for both words, one retired and one reserved; the `keygen` requirement gains the `--show` scenario.

## Impact

Affected nix: `modules/flake/safix/bridge.nix` (`violationsOf`'s reserved-id refusal); `modules/flake/safix/keepassxc.nix` (`violationsOf`'s reserved-id refusal, mirroring bridge.nix's).

Affected rust: `crates/safix/src/main.rs` (`VERBS` drops `import`/`export`, both `audit_command` and `sync_command` gain target-and-mapping-list parsing and `--direction`); `crates/safix/src/usage.rs` (`IMPORT`/`EXPORT` constants removed, `AUDIT`/`SYNC`/`KEYGEN` rewritten for targets, `--direction` and `--show`, `expected_verbs()`'s snapshot shrinks by two); `crates/safix-core/src/bridge.rs` (`import`/`export` free functions collapse into one clan-target `sync` entry point iterating every mapping in its own direction, with an optional direction filter; the `MappingWrongDirection` refusal becomes a filter-mismatch refusal); `crates/safix-core/src/audit.rs` (target dispatch, a keepassxc comparison path, lingering); `crates/safix-core/src/sync.rs` (exposed as the keepassxc-target implementation `sync_command` now calls); `crates/safix-core/src/keygen.rs` (`--show`); `crates/safix-core/src/error/{mod.rs,prose.rs,code.rs}` (renamed and new refusal variants: mapping-id reservation, direction-filter mismatch, `--direction` on the wrong target).

Affected checks and tests: `modules/flake/checks/bridge.nix` and `modules/flake/checks/keepassxc.nix` (reserved-id fixtures); `crates/safix/tests/bridge.rs`, `audit.rs`, `real_clan.rs`, `store_cli.rs` (renamed and target-dispatch coverage); `crates/safix/tests/support/` snapshot fixtures for `import`/`export`/`audit`/`sync` help text.

Affected docs: `README.md`'s "The bridge to clan" and "Mirroring into your password database" sections (roughly lines 590-1027 as read on this branch) and the two verb-count sentences at roughly lines 905 and 1114.

Sequencing: this change is written to apply before `sync-clan-vars-two-way` and `enumerate-clan-namespace`, both of which are amended alongside it (see their `design.md` amendment notes) so their deltas read against this change's post-archive shape of `bridge-transfer` rather than against `import`/`export`.
`sync-clan-vars-two-way`'s third verb, `bridge`, folds into `sync clan --direction two-way` once that change lands; this change does not add `two-way` as a direction value on its own, because that value does not exist in `bridge-surface` until `sync-clan-vars-two-way` declares it.

Every claim gets a severity drill in `tasks.md`, following `modules/flake/checks/bridge.nix`'s own established pattern of a perturbed fixture per refusal.
