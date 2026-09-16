# A third sync target: the operator's `pass` store

## Why

`safix sync` converges a declared secret with two far sides today — a clan var (`crates/safix-core/src/bridge.rs:74-77`) and an entry in a KeePassXC database (`crates/safix-core/src/sync.rs:1-15`) — and neither is where a scripted consumer keeps a credential.
`pass` is the store a person already has on a headless machine: one gpg file per entry, no session, no database to rewrite, and a body convention that carries a username, a URL, notes and tags beside the value.
It is also the one target of the three this programme adds that can be driven for real inside a `nix build` sandbox, because it needs no network and no account — only `gpg` and a keyring — which is what makes it the target that puts a real tool behind the shared field model rather than a second model beside it.

This change depends on `extend-bridge-fields`, which lands first and which this change only consumes.
That change owns `modules/flake/safix/fields.nix` (the `fields = { username; url; notes; tags; }` submodule and `fieldValue = either str (submodule { entry = str; })`), `modules/flake/safix/reserved.nix` (the complete six-word reserved list `[ "clan" "keepassxc" "pass" "bitwarden" "1password" "all" ]` and the check asserting it equals the Rust literal), `crates/safix-core/src/endpoint.rs` (`Endpoint`, `Record`, `Existing`, `Capabilities`), `model::{Fields, FieldValue}` and `ResolvedFields`, the single `judge` over `Record`s, `Outcome::FieldsDiverged`, and `Error::{FieldUnsupported, FieldSourceInArgv}`.
The word `pass` is already reserved there, so nothing here appends to that list; this change imports it.

## What Changes

- Add a declaration, `flake.safix.pass`: a store root and per-entry mappings from a person's safix entry to an entry path inside the store, each mapping declaring a mode and the far side's fields.
  `store` is a string, defaulting to `"~/.password-store"` with the `~` expanded by the runtime, never a nix path — the reason `flake.safix.keepassxc.database` is a string (`modules/flake/safix/options.nix:516-521`) applies unchanged, and a path-typed field additionally reddens `safix-examples`, which compares the flattened `flake.safix.lib.*` records field-for-field between two examples evaluated under different roots.
- Add the four modes the keepassxc target already has (`modules/flake/safix/keepassxc.nix:33-38`), with the target word swapped: `safix-to-pass`, `pass-to-safix`, `two-way`, `backup`.
  No mode deletes an entry on either side, and `pass rm` is never run.
- Add `crates/safix-core/src/pass.rs`: one transport over the store's own command, implementing the `Endpoint` trait `extend-bridge-fields` introduces, so the converge loop, the single `judge`, `selected` and `lingering` are reused rather than copied.
  A read is `pass show <path>`; a write is `pass insert --multiline --force <path>` with the whole record body on standard input; the namespace for `list` is the `*.gpg` filenames under the store root; the store root reaches the child as `PASSWORD_STORE_DIR` in its environment, which is a path and never a value.
- Carry all four fields, because `pass` carries the whole record on one pipe: the value is the body's leading lines and the fields are a trailing block of `login:`, `url:`, `notes:` and `tags:` lines.
  So `pass` declares full capability, raises neither `Error::FieldUnsupported` nor `Error::FieldSourceInArgv`, and is the one target where a `{ entry = "<name>"; }` field source is admissible — nothing a field carries travels an argument vector here.
- Accept a multi-line value byte for byte, and state both per-transport consequences rather than inheriting them: `Error::ValueSpansLines` is a `keepassxc-cli` limitation (`crates/safix-core/src/store.rs:48-51`) and is not raised for this target, and `Secret::without_one_trailing_newline` (`crates/safix-core/src/secret.rs:313`) exists because `keepassxc-cli show` appends a byte (`store.rs:44-47`) and is not applied to `pass show`, which is byte-exact.
- Record a `two-way` mapping's last agreement in a companion entry beside the mapped one, `<path>.safix-sync-state`, under the shared `safix-sync-v1` format (`crates/safix-core/src/sync.rs:68`), written as its own second write strictly after the value's.
  Evaluation refuses a declared path carrying that suffix, and `lingering` claims each mapping's companion alongside its entry so a mapping's own memory is never reported as an unclaimed entry.
- Add `SAFIX_PASS` (default `pass`) as the program override, in the shape `SAFIX_KEEPASSXC_CLI` has (`crates/safix-core/src/enroll/custody.rs:43-44, 353-357`), which is the seam every check drives the stub through.
- Add no unlock step: `pass` shells to `gpg`, and the unlock belongs to the ambient `gpg-agent`.
  The preflight is that the store exists — a directory holding a `.gpg-id` — and a decrypt failure naming gpg is surfaced as its own refusal rather than as a generic command failure.
- Add five refusals, each with its own code, prose, reporter sample arm and two accepted snapshots: `Error::PassUnavailable`, `Error::PassLocked`, `Error::PassCommandFailed`, `Error::NoPassStore` and `Error::PassEntryAbsent`.
- Extend `sync` and `audit` to accept `pass` as a target keyword, alongside `clan` and `keepassxc`, in both verbs' dispatch, forms, usage sections and reports.
- Add `crates/safix/tests/support/pass-stub.rs` and a `refuse_a_real_store` harness guard for this target, which refuses any run whose `PASSWORD_STORE_DIR` is not under the fixture's scratch directory or whose `SAFIX_PASS` is not the stub — the machines this suite is developed on plausibly hold the operator's own store.
- Add a real-binary check, `safix-pass-cli`, over `pkgs.pass` and `pkgs.gnupg` with a keyring the check mints in its own `GNUPGHOME`, linux-only, modelled on `crates/safix/tests/store_cli.rs` and reached through `integration.runOneWith` (`modules/flake/checks/integration.nix:112-118`).
  It is what establishes that the argument vectors mean to `pass` what this runtime thinks they mean.
- **BREAKING** for a consumer holding a mapping whose id is the word `pass`: that id is refused as a target keyword.
  The refusal and the reserved list itself land in `extend-bridge-fields`; this change is the one that makes the word mean something, and states the break for the consumer who meets it.
- **BREAKING** for the check surface, not for consumers: `modules/flake/checks/pass.nix` is a new file contributing `safix-pass` and `safix-pass-drill`, and `modules/flake/safix/checks.nix` gains a `safix-pass-refusals` attribute in the record a consumer's `safixChecks` call returns.
  No existing check attribute is renamed or removed.

Not in scope: `pass otp`, `pass generate` and every other extension verb — safix mints values through its own generators, and a second producer for one value is the refusal `twoProducers` already makes.
Not in scope: `pass git` beyond the commit `pass` makes for itself on every mutation; the store's own history is the store's.
Not in scope: `pass init` and any recipient management — a `.gpg-id` is the store's own audience declaration, and writing one would be safix deciding who can read somebody else's store.
Not in scope: reading fields into safix on a pull. A safix entry is a `(file, key, audience)` placement with no slot for a URL or a tag, and fields are push-only across every target; `extend-bridge-fields` owns that property and this change inherits it.

## Capabilities

### New Capabilities

- `pass-sync`: the `flake.safix.pass` declaration, the four modes, the record layout `pass` entries are read and written under, the companion memory, the transport's own refusals, and what `sync pass` and `audit pass` report.

### Modified Capabilities

- `safix-cli`: the `--entry` requirement's attribute count grows by one (`safix.lib.pass`), and its nested-payload scenario names a pass mapping alongside the generator, bridge and keepassxc ones, so the `#[serde(deny_unknown_fields)]` structs are exercised over the new record under both evaluation paths.
- `bridge-surface`: the reserved-id requirement's refusal covers `pass` — the word is reserved by `extend-bridge-fields` and the list is one exported nix value — and the reason is restated once for a third target rather than per target.
- `behavioural-suite`: a requirement stating that a target whose tool can be driven hermetically is driven for real by at least one check, and that a stub stands only for a delegation boundary, with the pass stub and `safix-pass-cli` as the pair that discharges it.

## Impact

Affected code:

- `modules/flake/safix/pass.nix` — new: the mapping submodule, `modes`, `pullCapable`, `stateSuffix`, `companionOf`, `mappingsOf`, `entryPathOf`, `violationsOf`.
- `modules/flake/safix/options.nix` — a `pass` block beside `keepassxc` (`:507-622`).
- `modules/flake/safix/default.nix` — the lib import beside `keepassxcLib` (`:20`) and the flattened `pass` record beside `keepassxc` (`:337-345`).
- `modules/flake/safix/checks.nix` — `passMessages`, `mkPassCheck`, the `pass` default argument and the `safix-pass-refusals` attribute (`:144-151, 435-457, 482-483`).
- `modules/flake/checks/pass.nix` — new: the structural check and its drill, modelled on `modules/flake/checks/keepassxc.nix`.
- `modules/flake/checks/cli.nix` — the per-test check attributes, and `withPass` beside `withStore` (`:340-343`).
- `crates/safix-core/src/pass.rs` — new: the transport, the record layout, the companion, the listing.
- `crates/safix-core/src/{nix,workspace}.rs` — the `Attribute::Pass` arm and its two spellings (`nix.rs:52, 80, 102`), the `OnceLock` field and accessor (`workspace.rs:43, 104, 236-240`).
- `crates/safix-core/src/model.rs` — `PassSide`, `PassMapping`, `Pass`, and the mode enum's four new spellings.
- `crates/safix-core/src/{sync,audit}.rs` — the pass target's converge and compare sections, reusing the shared `Endpoint` and `judge`.
- `crates/safix-core/src/error/{mod,prose,code}.rs` — five variants, their prose, their codes.
- `crates/safix/src/{main,usage,render,reporter}.rs` — the target keyword, the two forms, the usage sections, the report section, five sample arms.
- `crates/safix/src/snapshots/` — ten new files, plus the accepted snapshots the two widened forms move.
- `crates/safix/tests/support/pass-stub.rs`, `crates/safix/tests/pass_path.rs`, `crates/safix/tests/pass_cli.rs` — new.
- `crates/safix/tests/harness/mod.rs` — `pass_env`, `pass_seed`, `pass_holds`, and `refuse_a_real_store`.
- `README.md`, `CHANGELOG.md` — the changelog entry here; the README's own five-target chapter belongs to `rewrite-readme-and-examples`, which owns that file in this programme.

Not affected, verified rather than assumed: `deny.toml`, `Cargo.toml`, `Cargo.lock` and `openspec/specs/rust-supply-chain/spec.md` — the transport is a subprocess over `std::process::Command`, the shape `crates/safix-core/src/clan.rs` and `store.rs` already have, and takes no dependency.
Every guarantee this change states gets a severity drill in `tasks.md`.
