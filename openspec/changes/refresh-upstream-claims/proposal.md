## Why

The upstream survey of 2026-11-04 (`docs/notes/research/upstream-survey-2026-11.md`) found recorded safix claims that clan-core, agenix and sops-nix have since made stale or wrong.
The migration emitter refuses an agenix semantic agenix now provides, emits agenix declarations that agenix 0.16.0 and 0.17.0 cannot evaluate on nix-darwin, and the clan fixtures and bridge wording describe a clan that no longer exists at the surveyed revision.
The flake inputs are also three months old, and the docs being corrected should be checked against current upstream releases.

## What Changes

- `safix migrate` with `deploymentTarget = "agenix"` stops refusing a root-owned entry with `neededForUsers: true` and publishes it as a plain `age.secrets` entry; the flag is not written, and the receipt still records it.
  Restart and reload hooks and templates stay refused for agenix.
- The agenix emitter writes `group` for every entry, so an owner without a group evaluates under agenix 0.16.0 and 0.17.0 on nix-darwin as well as under 0.18.0.
- The migration guide, the migration plan reference and `safix migrate -h` drop the claim that agenix cannot carry early-user secrets, state an agenix floor of 0.18.0, mention the `agenix-rules.nix` rename and the `secrets.nix` name clash with the quickstart, state that nested keys in JSON destinations need sops-nix at or after commit 311ef94, and gain an agenix-rekey subsection.
- The guide's statement that every generated module carries hooks and the early-user flag is narrowed to the targets that carry them.
- The clan-shaped fixture in `modules/flake/checks/installer.nix` moves to clan's current deployed path shape `/run/secrets/per-machine/<machine>/<generator>/<file>`.
- The bridge wording that `clan vars set` commits in "the clan repository" is qualified to clan's store repository: the flake, or `CLAN_AGE_STORE_DIR` when the clan sets `vars.settings.age.externalStore`.
  The password-manager guide's clan section says that variable must be exported for `safix sync clan`.
- The hardware keys guide states that on an age-backend clan the `clan secrets users` registration grants no vars access, and that the recipient belongs in `vars.settings.recipients`, which safix does not write.
- `flake.lock` moves nixpkgs, home-manager, crane, flake-parts, treefmt-nix and advisory-db to current revisions.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-migration`: "usable declarations" gains a definition tied to the consumer versions the documentation states; the agenix target accepts root-owned early-user entries and names a group for every entry.
- `bridge-sync`: the clan-only refusal list names clan's store repository rather than "its own repository".
- `bridge-transfer`: the two scenarios that say clan's write commits in "the clan repository" name clan's store repository, which is absent when the external store is not a git repository.

## Impact

- Code: `crates/safix-core/src/migrate.rs` (`validate_deployment`, `deployment_fields`), `crates/safix/src/usage.rs` (`MIGRATE`), comments in `crates/safix-core/src/bridge.rs`.
- Tests: new cases in `crates/safix/tests/migrate.rs`; changed fixture paths in `modules/flake/checks/installer.nix`.
- Docs: `docs/guides/migrating-from-sops-nix-and-agenix.md`, `docs/reference/migration-plan.md`, `docs/guides/syncing-password-managers.md`, `docs/guides/hardware-keys.md`.
- Dependencies: every input in `flake.lock`; the rust toolchain, `sops`, `age`, `bitwarden-cli` and home-manager options may move with them.
- A plan that `safix migrate` refused before (agenix with early-user entries) now succeeds; no plan that succeeded before is refused, and the declarations it produced for agenix gain a `group` line.
