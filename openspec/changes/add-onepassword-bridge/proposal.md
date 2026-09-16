# safix mirrors declared secrets into 1Password

## Dependency

This change lands after `extend-bridge-fields` and assumes that change applied.
That is: `modules/flake/safix/fields.nix` exports the `fields` submodule — `username`, `url`, `notes` and `tags`, each `nullOr fieldValue` with `tags` a `listOf`, where `fieldValue = either str (submodule { entry = str; })` names another entry of the mapping's own user — and `modules/flake/safix/reserved.nix` exports the complete six-word reserved-mapping-word list `[ "clan" "keepassxc" "pass" "bitwarden" "1password" "all" ]` asserted equal to `crates/safix-core/src/bridge.rs`'s `RESERVED_MAPPING_WORDS`.
On the rust side it assumes `model::{Fields, FieldValue}` and the resolved `ResolvedFields`, `endpoint::{Record, Endpoint, Existing, Capabilities}` in `crates/safix-core/src/endpoint.rs`, one shared `judge` over `Record`s, `Error::FieldUnsupported { target, field }`, `Error::FieldSourceInArgv { target, field }` and the audit outcome's `FieldsDiverged(Vec<&'static str>)`.
This change creates neither `fields.nix` nor `reserved.nix` and appends nothing to the reserved list: the list ships complete, and this change only reads it.
Nothing here depends on `add-pass-bridge` or `add-bitwarden-bridge`, and the three target changes are independent of each other; each adds one `Target` arm, one nix module, one runtime transport, one stub and one capability.

## Why

`safix sync` and `safix audit` reach exactly two targets today — `Target { Clan, Keepassxc }` (`crates/safix-core/src/bridge.rs:74-78`) — so a fleet whose people keep their person-read credentials in 1Password has no declared relationship at all and converges them by hand.
The keepassxc mirror already establishes everything the shape needs: one mapping per safix entry, four modes named by their endpoints, evaluation refusing what is local to the consumer's own declarations, a run reading both sides before writing either, and a report that names mappings and never values (`openspec/specs/keepassxc-sync/spec.md`).
What it cannot do is carry metadata: `keepassxc-cli` 2.7 has no `--tags` and puts `--url`/`--notes` in an argument vector, which `keepassxc-sync`'s own "a secret value SHALL travel standard input or pipes, never an argument vector or an environment variable" forbids for a field whose source is another safix entry.
1Password is the target where that constraint vanishes: `op item create`/`op item edit` take the whole item as JSON on standard input, so all four declared fields cross on a pipe, a multi-line value crosses whole, and the two-way memory lives in a concealed field of the item itself rather than in a companion object beside it.

The cost is the one thing this change has to state rather than discover: the 1Password CLI is proprietary.
`_1password-cli` at this flake's pinned nixpkgs is `license = lib.licenses.unfree`, so a reference to it from `perSystem.checks` or the devshell would make `nix flake check` fail at *evaluation* for every consumer without `allowUnfree`.
So this target ships a declaration surface, a runtime transport and a stub, and no check anywhere drives a real `op` — an absence that is written into the module header and into the capability, because `modules/flake/checks/integration.nix:112-116` records exactly what happens when one is left unstated: "a check stating an absence and passing … is how a claim stops being made without anybody deciding to stop making it."

## What Changes

- New nix surface `flake.safix.onepassword`, declared in `modules/flake/safix/options.nix` beside `bridge` (`:454-505`) and `keepassxc` (`:507-622`), with the mapping submodule, its refusals and its derived surfaces in a new `modules/flake/safix/onepassword.nix` modelled on `modules/flake/safix/keepassxc.nix`:
  `account = nullOr str` (a 1Password account shorthand or sign-in address, `null` meaning whatever account `op` itself resolves), and `mappings.<id> = { mode; safix = { user; name; }; onepassword = { vault; item; fields; }; }`.
  `mode` is keepassxc's four words with this target's endpoints substituted: `safix-to-1password`, `1password-to-safix`, `two-way`, `backup`.
  `vault` and `item` are plain strings and neither is a nix path, for the reason `keepassxc.database` is a string (`options.nix:516-521`) and `bridge.clanFlake` is not: a path interpolated into a declaration is copied into the world-readable store on every evaluation, and here it would also make the two `safix-examples` consumers resolve to two different absolute strings.
- New runtime transport `crates/safix-core/src/onepassword.rs`, one `Endpoint` implementation over the `op` binary:
  `unlock()` runs `op whoami --format json` — plus `--account <shorthand>` when one is declared — before any mapping's side is read, in the position `sync.rs`'s `enroll::terminal_present()` check occupies today (`crates/safix-core/src/sync.rs:240-336`), so a signed-out run refuses without having decrypted safix's side of every mapping;
  `read()` runs `op item get <item> --vault <vault> --format json --reveal`;
  `write()` runs `op item create --vault <vault> --category login --title <item> -` for an absent item and `op item edit <item> --vault <vault> -` for a present one, the whole item as JSON on standard input in both cases;
  `list()` runs `op item list --vault <vault> --format json`, which is what makes lingering items reportable.
  No verb this transport issues deletes anything, and `op item delete` appears nowhere in the tree.
- Values and fields travel standard input only.
  safix never spells an assignment statement — `password=…`, `username=…`, or any other `field=value` argv word — because 1Password's own documentation states that assignment statements are logged in the shell's history and can be visible to other processes.
  Consequently this target declares `Capabilities` carrying all four fields with no argv-bound channel: `Error::FieldUnsupported` and `Error::FieldSourceInArgv` are both unreachable for it, and a `{ entry = … }`-sourced field is admissible on every one of the four.
- An edit is a round-trip, never a template.
  `write()` against a present item starts from that item's own JSON as `read()` returned it, replaces only the fields safix declares plus the value and — for `two-way` — the memory, and writes the whole object back.
  This is what keeps 1Password's documented template-edit danger from reaching an operator's item: a JSON edit assembled from a template overwrites passkeys on the item, and safix's edit cannot, because it never assembles one.
- The `two-way` memory is a custom `CONCEALED` field named `safix-sync-state` on the mapped item itself, carrying the shared `safix-sync-v1 <fingerprint>` line (`crates/safix-core/src/sync.rs:68`).
  There is therefore no companion object, no reserved suffix and no reserved-name refusal for this target, and no "lingering companion whose item is gone" report shape.
- New program override `SAFIX_OP`, default `op`, following `KEEPASSXC_OVERRIDE`'s shape exactly (`crates/safix-core/src/enroll/custody.rs:44,352-357`).
  Session material reaches the child in the child's environment or not at all: `OP_SERVICE_ACCOUNT_TOKEN`, or an `op signin` session the operator already established, inherited from safix's own environment.
  safix runs no `op signin` of its own and passes no `--session <token>` in an argument vector.
- New refusals in `crates/safix-core/src/error/`, each with a variant, a prose function, a `safix::snake_case` code, one `reporter::sample` arm and two snapshots:
  `OnePasswordUnavailable { program, cause }` → `safix::onepassword_unavailable`;
  `OnePasswordSignedOut { account, output }` → `safix::onepassword_signed_out`;
  `OnePasswordCommandFailed { item, arguments, output }` → `safix::onepassword_command_failed`;
  `OnePasswordItemAbsent { mapping, vault, item, mode }` → `safix::onepassword_item_absent`;
  `OnePasswordVaultAbsent { mapping, vault, output }` → `safix::onepassword_vault_absent`.
  `UnknownSyncMapping` and `SyncSourceEmpty` (`error/mod.rs:1179-1186,1226-1239`) are reused rather than duplicated per target: both name a mapping and a declaration set and say nothing about a database.
  `ValueSpansLines` does not generalise here and is not reused: `op` carries a multi-line value whole, so this target has no value-shape refusal at all.
- **BREAKING** (nix surface, and the command's own first argument): `sync` and `audit` read `1password` as a target keyword, so a mapping of any target whose id is `1password` stops evaluating.
  The word arrives with `extend-bridge-fields`' complete reserved list; what this change adds is the refusal firing over `flake.safix.onepassword.mappings` too, and the target keyword the refusal's sentence is about.
  `op` is deliberately **not** accepted as a target keyword: one spelling per target, and `op` is the binary's name rather than the store's.
- **BREAKING** (nix surface): a bare `safix sync` and a bare `safix audit` now reach this target as well.
  For a consumer declaring no `flake.safix.onepassword.mappings` nothing changes — the target contributes an empty section and issues no command — but a consumer who declares mappings gets network round-trips from the bare form, which previously reached only a local database and another flake's command.
- New structural checks `safix-onepassword` and `safix-onepassword-drill` in a new `modules/flake/checks/onepassword.nix`, mirroring `modules/flake/checks/keepassxc.nix`, plus `safix-onepassword-refusals` through `modules/flake/safix/checks.nix`'s `mkOnepasswordCheck` beside `mkKeepassxcCheck` (`checks.nix:139-152,435-457,479-485`).
- New integration coverage: `crates/safix/tests/onepassword_path.rs`, registered one test at a time in `modules/flake/checks/cli.nix` the way `sync_path.rs` is (`cli.nix:777-836`), driven against a new stub `crates/safix/tests/support/op-stub.rs` exported as `SAFIX_TEST_OP_STUB` from `modules/flake/checks/integration.nix:122-132`.
  The stub refuses any invocation carrying an argv word containing `=`, so "no value in an argument vector" is enforced by the instrument rather than by review.
  The harness gains a `refuse_a_real_store` guard for this target beside `refuse_a_real_database` (`crates/safix/tests/harness/mod.rs:2802-2835`): `SAFIX_OP` must name the stub, `OP_SERVICE_ACCOUNT_TOKEN` must be the fixture's fake, and `OP_ACCOUNT` must be unset.
- No real-binary check for this target, ever, and the flake never references `_1password-cli` from any check, package or devshell.
  Stated in `modules/flake/safix/onepassword.nix`'s header, in `crates/safix-core/src/onepassword.rs`'s header, and as a requirement of the new capability, because an unstated absence is a claim nobody decided to stop making.

Not in scope: a NixOS VM check with a 1Password server in it.
There is no self-hostable 1Password server, every authentication path needs the network, and the package is unfree — three independent reasons, each sufficient, and none of them expires.

Not in scope: `op read`/`op inject` secret-reference URIs (`op://<vault>/<item>/<field>`) as a declaration surface.
See design decision 1.

Not in scope: a `tags`-driven item listing.
`list()` enumerates the declared vault and nothing else, because a tag safix writes is a declared field rather than a selector safix may then trust to be exhaustive.

## Capabilities

### New Capabilities

- `onepassword-sync`: the declared mirror between safix entries and 1Password items — what a mapping names, how each of the four modes converges, where the two-way memory lives, how the value and the four fields cross on standard input, whose session the run uses, what an edit preserves, what the report says including a run that stops partway, what `audit` does instead, and the real-binary check that deliberately does not exist.

### Modified Capabilities

- `safix-cli`: "A global entry file replaces the flake for evaluation" — the attribute count becomes count-free rather than re-counted, the way `own-the-installer` made the `--entry` verb count count-free when two verbs landed in one batch, and the nested-payload scenario names a `onepassword` mapping alongside the generator, the bridge mapping and the keepassxc mapping, so `flake.safix.lib.onepassword`'s JSON is held to deserializing under `--entry` against the same `#[serde(deny_unknown_fields)]` structs the flake path uses.
- `behavioural-suite`: "The suite drives real backends and stubs only the evaluator" — the requirement's own sentence says the suite stubs nothing other than `nix`, which has not been true since clan's command and the store's command were stubbed; it becomes a statement that the suite stubs the evaluator and the programs standing at a delegation boundary a hermetic build may not cross, names the 1Password command as one of them, and gains a scenario for the two things that make this stub honest rather than convenient — it refuses an assignment statement, and the real-binary check it does not stand in for is named as absent.
- `rust-runtime`: "Every external program the runtime invokes is selectable by a named variable" — `SAFIX_OP` joins the set, and the requirement gains a scenario stating that a program no check may ever run for real still has its variable, because the variable is what lets the stub stand where the program would.

Not modified: `bridge-surface`.
Its reserved-mapping-word scenario is `extend-bridge-fields`' to move, since that change ships the complete list; this change's own reserved-id scenario lives in `onepassword-sync`, exactly as `keepassxc-sync`'s does (`openspec/specs/keepassxc-sync/spec.md:31-35`).

Not modified: `rust-supply-chain`.
Its licence requirement is about the locked cargo graph reviewed offline by `cargo-deny` (`openspec/specs/rust-supply-chain/spec.md:68-82`), and `_1password-cli` is a nixpkgs derivation rather than a crate.
The unfree statement therefore belongs to this target's own capability and its module headers; see design decision 12.

## Impact

Affected nix:

- `modules/flake/safix/onepassword.nix` — new: `modes`, `pullCapable`, `mapping`, `opSide`, `mappingsOf`, `itemPathOf`, `violationsOf`, and the header recording that no check drives a real `op`.
- `modules/flake/safix/options.nix` — the `flake.safix.onepassword` block, after `keepassxc`'s.
- `modules/flake/safix/default.nix` — the lib import and the flattened `lib.onepassword` record beside `lib.bridge` and `lib.keepassxc`.
- `modules/flake/safix/checks.nix` — `onepasswordMessages`, `mkOnepasswordCheck`, the default-argument record and the exported attribute set (`:139-152,435-457,479-485`).
- `modules/flake/checks/onepassword.nix` — new: the structural check and its drill.
- `modules/flake/checks/cli.nix` — one check per claim of `onepassword_path.rs`.
- `modules/flake/checks/integration.nix` — `SAFIX_TEST_OP_STUB`, and nothing added to `backends`.

Affected rust:

- `crates/safix-core/src/onepassword.rs` — new.
- `crates/safix-core/src/lib.rs` — the module registration.
- `crates/safix-core/src/model.rs` — `OpSide`, `OnepasswordMapping`, `Onepassword`, and this target's `Mode` spellings, every struct `#[serde(deny_unknown_fields)]` per `model.rs:1-15`.
- `crates/safix-core/src/bridge.rs:74-78` — `Target::OnePassword`.
- `crates/safix-core/src/nix.rs:32-52,79-80,101-102` — `Attribute::Onepassword` and its two spellings.
- `crates/safix-core/src/workspace.rs:43,104,236-240` — the `OnceLock` field and the accessor.
- `crates/safix-core/src/sync.rs`, `crates/safix-core/src/audit.rs:146-273` — this target's arms of the shared loop and of the report.
- `crates/safix-core/src/error/{mod.rs,prose.rs,code.rs}` — five variants, five sentences, five codes.
- `crates/safix/src/main.rs:695-850` — the target keyword, `DirectionOnWrongTarget`'s rendering, and `sync_command`/`audit_command`'s per-target arms.
- `crates/safix/src/usage.rs:82-141,205-,904-905,977-` — the two verb forms, this target's own section, and the verb table.
- `crates/safix/src/render.rs:626-800` — this target's audit and sync sections.
- `crates/safix/src/reporter.rs:237-675` — five sample arms.
- `crates/safix/src/snapshots/` — ten new files, `plain-` and `graphical-` per code.
- `crates/safix/tests/onepassword_path.rs`, `crates/safix/tests/support/op-stub.rs`, `crates/safix/tests/harness/mod.rs` — new target, new stub, new guard and the fixture's `onepassword_env`.
- `crates/safix/tests/snapshots/upload__safix_help.snap` — regenerated from the verb forms.

Affected documentation: `CHANGELOG.md` under `[Unreleased]`, including the migration note for a mapping id spelled `1password`.
`README.md` is `rewrite-readme-and-examples`' to restructure; this change assumes the five-target shape and adds no section of its own.

Every guarantee this change states gets a severity drill in `tasks.md`.
