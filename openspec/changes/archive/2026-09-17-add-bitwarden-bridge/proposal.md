# A third sync target: the operator's Bitwarden vault

## Why

`safix sync` converges two targets today — clan's vars and a local KeePassXC database — and both of them are files on a machine the operator is sitting at.
A fleet whose people read credentials on a phone keeps those credentials in Bitwarden or in a self-hosted Vaultwarden, and nothing in safix reaches there: the copy is made by hand, so it drifts silently, and the only thing that would notice is a person opening both.
Bitwarden is also the first target whose far side is a network service rather than a path, which is the shape the fourth and fifth targets share — so the vocabulary for an unreachable, locked or stale far side is worth settling here, on the free-licensed CLI, rather than on 1Password's unfree one.

This change depends on `extend-bridge-fields`, which lands first and is cited rather than restated.
From it this change consumes, unchanged: `modules/flake/safix/fields.nix` (the `fields = { username; url; notes; tags; }` submodule and `fieldValue = either str (submodule { entry = str; })`), `modules/flake/safix/reserved.nix` (the six reserved mapping words, `bitwarden` among them, already complete — this change appends nothing to it), Rust `model::{Fields, FieldValue}` and the resolved `ResolvedFields`, `crates/safix-core/src/endpoint.rs` (`Record`, `trait Endpoint`, `Existing`, `Capabilities`) with its single `judge`, `Error::{FieldUnsupported, FieldSourceInArgv}`, and `Outcome::FieldsDiverged`.
Nothing in this change creates or edits either shared nix file, the `Fields` types, the endpoint module, or the reserved list.

## What Changes

- Add a target keyword, `bitwarden`, to `sync` and `audit`, in the dispatch table position after `keepassxc`, with `bridge::Target::Bitwarden` as its arm.
  The keyword is already reserved as a mapping id by `extend-bridge-fields`, so this change adds the target that reservation was made for and does not widen the reserved list.
- Add a declaration surface, `flake.safix.bitwarden`, in `modules/flake/safix/options.nix` beside `keepassxc`:
  `server` (`nullOr str`, a self-hosted Vaultwarden URL, default `null` meaning whatever server the operator's own `bw` is configured against), and `mappings.<id> = { mode; safix = { user; name; }; bitwarden = { folder; item; fields; }; }`.
  `mode` reuses keepassxc's four words spelled for this target: `safix-to-bitwarden`, `bitwarden-to-safix`, `two-way`, `backup`.
  An item is addressed by folder and name, never by Bitwarden's own item id — decision B1 records why an opaque identifier is not a reviewable declaration.
- Add `modules/flake/safix/bitwarden.nix`: the mapping submodule, `mappingsOf`, `itemPathOf`, the field capability table, and `violationsOf` carrying the five refusals evaluation can reach — an unresolvable safix side, a pull-capable mapping onto a generator-produced entry, two mappings naming one item, a mapping id that collides with a reserved target keyword (read from `reserved.nix`, not restated), and a declared `tags` field the target cannot carry.
- Add `crates/safix-core/src/bitwarden.rs`: the transport, implementing `Endpoint` from `extend-bridge-fields`.
  `bw status` is the preflight; `bw unlock --passwordfile /dev/stdin --raw` takes the master password on the child's standard input, prompted once on the terminal exactly as the database's password already is; the session key it prints travels to every later child as `BW_SESSION` **in that child's environment and nowhere else**; `bw sync` runs once before the first read.
  Item JSON crosses as base64 on standard input in both directions, encoded in process — never the documented `bw encode` subprocess, and never the positional `<encodedJson>`, which would put the value in an argument vector.
- **Record a deliberate narrowing of an existing invariant.** The suite asserts today, unconditionally, that no credential travels a child's argument vector *or* its environment.
  Bitwarden's session key has no standard-input channel, and `--session <key>` in argv is world-readable through `/proc/*/cmdline` where an environment variable is readable by the same uid alone.
  The narrowed invariant this change asserts in its place is exact: no master password and no mapped value in argv or env on any invocation, and the session key in env and nowhere else.
  Decision B3 records the choice; the `behavioural-suite` delta makes it a requirement rather than leaving it in a design document.
- Add the two-way agreement memory as a **hidden custom field named `safix-sync-state` on the mapped item itself**, in the shared `safix-sync-v1 <fingerprint>` format.
  Bitwarden can write a hidden custom field, which is the one thing `keepassxc-cli` cannot — so this target needs no reserved name suffix, no companion item, and no "lingering companion whose mapping is gone" report shape.
- Add seven refusals, each with its variant, its code, its prose, its reporter sample arm and its two accepted snapshots: `BitwardenUnavailable` (`safix::bitwarden_unavailable`), `BitwardenLocked` (`safix::bitwarden_locked`), `BitwardenCommandFailed` (`safix::bitwarden_command_failed`), `BitwardenServerMismatch` (`safix::bitwarden_server_mismatch`), `BitwardenItemAmbiguous` (`safix::bitwarden_item_ambiguous`), `BitwardenItemAbsent` (`safix::bitwarden_item_absent`) and `BitwardenStale` (`safix::bitwarden_stale`).
  `NoBitwardenServer` is deliberately **not** among them: `server` is optional, so there is no state in which a declared mapping has no server to reach, and the hazard that variant was gesturing at is a declared self-hosted URL differing from the URL the unlocked session actually reached — which is `BitwardenServerMismatch`.
  Decision B2 records it.
  `SyncSourceEmpty` is reused rather than duplicated for a safix side holding nothing, because that refusal is about safix's own half and says nothing about a target.
- Add `crates/safix/tests/support/bw-stub.rs` and the `safix-bw-stub` support binary, dispatching on the first word of its argument vector — `status`, `config`, `sync`, `unlock`, `list`, `get`, `create`, `edit`, `lock` — recording argv, env and stdin to a spool, refusing a positional that is not base64, refusing an invocation missing `--nointeraction`, and carrying failure switches for the locked, unknown-item, ambiguous-item, refusing and sync-failure drills.
- Add a harness guard, `refuse_a_real_vault`, beside `refuse_a_real_database` (`crates/safix/tests/harness/mod.rs:2802`): a bitwarden run is refused unless `BITWARDENCLI_APPDATA_DIR` is under the fixture's own scratch **and** the configured server is `127.0.0.1` or `localhost`.
  The dev machine plausibly holds a real, logged-in `bw`, and the guard is what keeps a test run from reaching it.
- Add `modules/flake/checks/bitwarden.nix` (the structural refusals and their drill) and `modules/flake/safix/checks.nix`'s `mkBitwardenCheck`/`bitwardenMessages`, plus integration entries in `modules/flake/checks/cli.nix` over the stub.
  **There is no real-binary check for this target, and that absence is recorded in `modules/flake/checks/bitwarden.nix`'s own header**, per the doctrine `modules/flake/checks/integration.nix:112-118` states: a check that states an absence and passes is how a claim stops being made without anybody deciding to stop making it.
  Decision B9 records why a vaultwarden NixOS VM check is not minted here either.
- **BREAKING** on the nix surface, in two ways, both consequences of the target existing rather than of anything being renamed.
  First, `flake.safix.bitwarden` is a new option block: a consumer who set no such option is unaffected, and a consumer who had an unrelated `bitwarden` attribute under `flake.safix` would now be typechecked against this one.
  Second, `safix sync` with no target named now converges bitwarden mappings too — a consumer who declares them and expects `sync` to mean "clan and keepassxc" gets a third target in the same run.
  No existing option is renamed or removed by this change, and no existing check attribute is renamed or removed.

Not in scope: mapping `tags` onto Bitwarden folders or collections.
Bitwarden has no tag concept — folders and collections are the only grouping, and both are placements rather than labels — so a `tags` declaration on this target is refused at evaluation rather than approximated, for the reason the keepassxc mirror already refuses a value it cannot carry whole instead of trimming it.
Not in scope: organizations and collections as an addressing space.
A mapping addresses a personal-vault item under an optional folder; an organizational collection is a second addressing space with its own permission model, and adding it later is additive to `bitwardenSide` rather than a change to anything settled here.
Not in scope: `bw login`.
safix never authenticates a vault — it unlocks one an operator has already logged into, exactly as it never creates a KeePassXC database — and a run against a `bw` reporting `unauthenticated` is `BitwardenLocked` with logging in named as the remedy.
Not in scope: item deletion, in any mode, ever.
Not in scope: a real-binary or VM check for this target; decision B9 records the measurement that would have to be performed first.

## Capabilities

### New Capabilities

- `bitwarden-sync`: the declaration surface and its refusals, the four modes and what each converges, the unlock-and-session contract with the narrowed no-credential-in-env rule, the two-way memory in a hidden custom field, the field capability table with `tags` refused, folder-and-name addressing with ambiguity refused rather than guessed, the staleness refusal when the pre-read `bw sync` fails, the value-free report, and `audit bitwarden`'s compare-only scope.

### Modified Capabilities

- `safix-cli`: the `--entry` requirement's scenario enumerating the attribute payloads that must deserialize identically under either evaluation path gains `bitwarden`, so a new target's nix value is reachable without a flake on the day it exists rather than a release later.
- `bridge-surface`: the requirement that a bridge relationship is declared rather than passed as arguments gains a scenario stating that a network-backed target's endpoint — the server — is declared once alongside the mappings, and that neither it nor a session key is ever a run argument.
- `behavioural-suite`: the requirement about what the suite drives and what it may stub gains the delegation-boundary stub for a vault that cannot be authenticated in a hermetic sandbox, and the narrowed credential-channel invariant that stub asserts.

## Impact

Affected code:

- `modules/flake/safix/bitwarden.nix` — new: the mapping submodule, the capability table, `mappingsOf`, `itemPathOf`, `violationsOf`.
- `modules/flake/safix/options.nix` — a `bitwarden` block beside `keepassxc` (`options.nix:507-621`).
- `modules/flake/safix/default.nix` — the `bitwarden` lib import, and the flattened `lib.bitwarden` record beside `lib.keepassxc` (`default.nix:337-345`).
- `modules/flake/safix/checks.nix` — `bitwardenMessages`, `mkBitwardenCheck`, the default-argument record, the exported attribute (following `keepassxc` at `checks.nix:24, 139-151, 435, 457, 482-483`).
- `modules/flake/checks/bitwarden.nix` — new: the structural refusals, their drill, and the recorded absence of a real-binary check.
- `modules/flake/checks/cli.nix`, `modules/flake/checks/integration.nix` — the integration entries, and `SAFIX_TEST_BW_STUB` in `runOneWith`'s environment.
- `crates/safix-core/src/bitwarden.rs` — new: the transport and its `Endpoint` implementation.
- `crates/safix-core/src/model.rs` — `BitwardenSide`, `BitwardenMapping`, `Bitwarden`, and the mode spellings, all under the existing `#[serde(deny_unknown_fields)]` discipline.
- `crates/safix-core/src/bridge.rs` — `Target::Bitwarden`.
- `crates/safix-core/src/nix.rs` — `Attribute::Bitwarden` and its two spellings (`nix.rs:51, 80, 102`).
- `crates/safix-core/src/workspace.rs` — the `OnceLock` field and the accessor (`workspace.rs:43, 104, 236-240`).
- `crates/safix-core/src/sync.rs` — the target's own decide-and-act pass over the shared `judge`.
- `crates/safix-core/src/audit.rs` — `BitwardenReport` on `Report`, and the conjunct in `Report::is_clean` (`audit.rs:212-234`).
- `crates/safix-core/src/error/{mod,prose,code}.rs` — seven variants, their prose, their codes.
- `crates/safix/src/main.rs` — the `bitwarden` keyword in `parse_dispatch` (`main.rs:717-720`), the `DirectionOnWrongTarget` arm (`main.rs:770-778`), and the `sync_command` block (`main.rs:827-850`).
- `crates/safix/src/usage.rs` — the `AUDIT`/`SYNC` forms, a bitwarden section in each, the two verb-table rows (`usage.rs:904-905`), and the "what sync's bitwarden target is" paragraph beside keepassxc's (`usage.rs:977`).
- `crates/safix/src/render.rs` — the bitwarden report's own section and detail lines.
- `crates/safix/src/reporter.rs` — seven `sample` arms.
- `crates/safix/src/snapshots/` — fourteen new files.
- `crates/safix/tests/support/bw-stub.rs`, `crates/safix/Cargo.toml` — new support binary.
- `crates/safix/tests/harness/mod.rs` — `bitwarden_env`, `bitwarden_seed`, `bitwarden_holds`, and `refuse_a_real_vault`.
- `crates/safix/tests/bitwarden.rs` — new.
- `README.md`, `CHANGELOG.md` — the target's section and the entry.

Not affected, verified rather than assumed: `modules/flake/safix/reserved.nix` and `modules/flake/safix/fields.nix` (both `extend-bridge-fields`'s, consumed here unedited), `crates/safix-core/src/endpoint.rs` (likewise), `crates/safix-core/src/store.rs` and `modules/flake/safix/keepassxc.nix` (the keepassxc transport and its declaration are untouched: nothing here changes what that target does), `Cargo.toml`, `Cargo.lock`, `deny.toml` and `openspec/specs/rust-supply-chain/spec.md` (no dependency is taken; the transport is `std::process` over the existing JSON support), and `flake.nix`'s system list (`bitwarden-cli` never enters the flake's closure, because no check in this change runs a real `bw`).
Every guarantee this change states gets a severity drill in `tasks.md`.
