# Fields on the far side of a mapping, one judge, and one reserved-word list

## Why

A mapping today carries a value and, on the keepassxc target alone, a username — `flake.safix.keepassxc.mappings.<id>.kdbx.username` (`modules/flake/safix/keepassxc.nix:74-87`), written into the entry's argument vector on every push (`crates/safix-core/src/store.rs:472-475`) and never read back.
An operator who wants the URL their password manager offers to fill, or a note saying where the credential came from, has no declaration to write it in, and the option's own documentation says why that was right at the time: "every field safix writes is a field its report and its refusals have to be able to speak about" (`keepassxc.nix:81-85`).
This change is what makes a report and a refusal able to speak about four fields rather than one, and it does the work once rather than five times, because the four other transports arriving after it (`add-pass-bridge`, `add-bitwarden-bridge`, `add-onepassword-bridge`) each carry a different subset of those fields through a different channel.

Two smaller facts are folded in for the same reason.
`crates/safix-core/src/bridge.rs:805-838`'s `judge` and `crates/safix-core/src/sync.rs:451-493`'s `two_way` are line-for-line the same decision over two different far sides, and `bridge.rs:614-615`'s own doc comment says so; adding a field axis to a decision written twice would write it twice.
The reserved mapping words are one fact in three places — `bridge.rs:89`, `modules/flake/safix/bridge.nix:421-430` and `modules/flake/safix/keepassxc.nix:218-227` — and three more targets would make it six.

## What Changes

- Add a shared nix submodule, `modules/flake/safix/fields.nix`, exporting `fields` (`username`, `url`, `notes`, each `nullOr fieldValue`, and `tags`, a `listOf fieldValue`) and `fieldValue = either str (submodule { options.entry = str; })`, where the `{ entry = "<name>"; }` form names another entry of the mapping's own user and is decrypted at run time rather than interpolated at evaluation.
  The submodule sits on each target's far side only — in this change `kdbxSide.fields` — and never on `safixSide`, because a safix entry is a `(file, key, audience)` placement with no slot for a URL and no second authoring surface beside the declaration.
- **BREAKING**: delete `flake.safix.keepassxc.mappings.<id>.kdbx.username` and move its meaning into `kdbx.fields.username`.
  Two spellings of one field is the second convention beside an existing one this repository refuses, and `crates/safix-core/src/model.rs`'s `#[serde(deny_unknown_fields)]` discipline (`model.rs:1-15`) makes a half-migrated state a hard evaluation failure rather than a silent one, which is the safety this cutover wants.
- Add a per-target field capability table and refuse a declaration the target cannot honour, in both halves, the way `Mode::pulls` is already refused in both halves for the reason `model.rs:875-885` states — evaluation refuses the declaration, and the runtime refuses the projection it is handed.
  Verified against `keepassxc-cli` 2.7 this session: `show` takes repeatable `-a/--attributes` and `add`/`edit` take `-u/--username`, `--url` and `--notes` in argv and have no tags flag and no custom-attribute write.
  So keepassxc declares `username`, `url` and `notes` as argv-channel fields and `tags` as unsupported: a declared `tags` is refused naming the target and the field, and an `{ entry = … }` source for any keepassxc field is refused because a field resolved out of another entry is a secret value and `keepassxc-sync`'s own spec forbids a secret value in an argument vector.
- Widen convergence from "the values differ" to "the values differ or the declared fields differ", which gives every transport a fields read.
  On keepassxc the fields read is a second, narrower invocation — `show -q -a UserName -a URL -a Notes`, without `--show-protected` and without `Password` — issued only for a mapping that declares a field, so a mapping that declares none keeps today's argument vector byte for byte and no value can return on the fields pipe by construction.
- State the direction of a field as a property rather than a default: a pull writes only the value into safix; a push writes the declared fields beside the value in the same target write; `backup` writes fields only where it writes a value, so it never touches an existing entry's fields; and a two-way mapping's fields are push-only, because a field is a declaration and has exactly one author while a two-way value has two.
- Add two report words to `sync`, `fields updated` and `fields diverged`, and a fourth outcome to `audit`'s keepassxc comparison, `fields diverged`, beside `agreeing`, `diverged` and `unjudgeable`.
  A value divergence dominates a field divergence; a field divergence names the fields and never their contents, because a `notes` body may itself be sensitive and an `{ entry = … }`-sourced field is a secret; and it moves the exit status on the same footing as a value divergence, because a declared field that is not there is a declaration that is not true.
- Add one endpoint contract, `crates/safix-core/src/endpoint.rs`, carrying `Record { value, fields }`, `Existing { Absent, Present }`, `Capabilities`, and a trait with `unlock`, `read`, `write`, `list` and `capabilities`, and collapse `bridge.rs:805-838`'s `judge` and `sync.rs:451-493`'s `two_way` into one function over `Record`s whose verdict each target maps onto its own outcome words.
  Every per-target refusal stays outside the trait and is documented as such: clan keeps its stale-generator refusal, its shared-placement addressing and its no-commit-on-clan-side; keepassxc keeps its one-line value refusal and its write burst.
- Add `modules/flake/safix/reserved.nix` exporting the whole reserved-mapping-word list, `[ "clan" "keepassxc" "pass" "bitwarden" "1password" "all" ]`, consumed by both mapping modules, with `bridge.rs:89`'s literal widened to `[&str; 6]` and a new check asserting the two lists are equal.
- **BREAKING** for a consumer holding a mapping named `pass`, `bitwarden` or `1password`: those three ids are refused from this change onward, before the targets that need them exist, so that the three target changes consume a settled list rather than moving the same check three times.
  `op` is deliberately not reserved: one spelling per target, and `1password` is it.
- Add two refusals with their codes, prose, reporter samples and accepted snapshots: `Error::FieldUnsupported { target, field }` (`safix::field_unsupported`) and `Error::FieldSourceInArgv { target, field }` (`safix::field_source_in_argv`).

Not in scope: the three new targets themselves.
`add-pass-bridge`, `add-bitwarden-bridge` and `add-onepassword-bridge` consume `fields.nix`, `reserved.nix` and the endpoint contract this change lands and add no shared surface of their own; each states the dependency.

Not in scope: `README.md` and `examples/`.
`rewrite-readme-and-examples` owns both, and it is that change which states `fields` in prose and declares it in a consumer.
The one exception is the inline example in `modules/flake/safix/options.nix:596-603`, which is an option's own `example` attribute rather than documentation and which stops evaluating the moment `username` is gone, so it moves here.

Not in scope: a fields read on the clan target.
A clan var is a file's bytes and clan's own command offers no field beside it, so a clan mapping declares no fields at all; the absence is stated as a requirement rather than left as an omission.

Not in scope: the `{ entry = … }` field source reaching a production caller.
Its resolution and its refusal both land here, complete and unit-tested, because the refusal's meaning is a statement about the resolution; the first endpoint that can carry such a field on a pipe arrives with `add-pass-bridge`.

## Capabilities

### Modified Capabilities

- `bridge-surface`: the reserved-id refusal grows from three words to six and becomes one list both mapping modules read; two requirements are added for the field surface — that the far side of a mapping is where a field is declared, and that a field the target cannot carry is refused at evaluation naming the target and the field.
- `keepassxc-sync`: the mapping declaration gains `fields` and loses `username`; convergence widens to the declared fields under a pushing mode; the report gains two words and names a field without its content; the store-not-a-keyring requirement gains the argv refusal for a field resolved out of another entry; `audit`'s comparison gains a fourth outcome; and a new requirement states that the fields read asks for the declared fields alone and can never return the value.
- `bridge-sync`: one judgement function serves every target, with each target's per-target refusals stated as staying outside it, and a field's single authorship is stated as the reason a two-way mapping's fields are push-only.
- `bridge-transfer`: the stale-generator refusal is stated as remaining clan's own rather than a shared endpoint concern, and a requirement states that a clan mapping has no field surface and why.
- `rust-runtime`: the no-plaintext-in-argv requirement gains the case this change creates — a declared field resolved out of another entry is a plaintext value, so a target whose only channel for that field is an argument vector refuses rather than writes.

## Impact

Affected code:

- `modules/flake/safix/fields.nix` — new: `fieldValue`, `fields`, and the per-target capability vocabulary the refusals read.
- `modules/flake/safix/reserved.nix` — new: the six reserved mapping words as one exported list.
- `modules/flake/safix/keepassxc.nix` — `kdbxSide.username` deleted and `kdbxSide.fields` added; `capabilities`; `fieldUnsupported` and `fieldSourceInArgv` in `violationsOf`; `reservedId` reading `reserved.nix`.
- `modules/flake/safix/bridge.nix` — `reservedId` reading `reserved.nix`; no field surface.
- `modules/flake/safix/options.nix` — the keepassxc mapping example's `username` becomes a `fields` block.
- `crates/safix-core/src/endpoint.rs` — new: `Record`, `Existing`, `Capabilities`, `Channel`, `Field`, `ResolvedFields`, `trait Endpoint`, `resolve_fields`, and the one `judge`.
- `crates/safix-core/src/model.rs` — `Fields`, `FieldValue`, `KdbxSide::fields` replacing `KdbxSide::username`.
- `crates/safix-core/src/store.rs` — `write`'s signature takes a `&ResolvedFields`; `write_arguments` appends the declared fields; `read_fields` and `fields_arguments` added; `Capabilities` for this transport.
- `crates/safix-core/src/sync.rs` — `two_way` deleted in favour of `endpoint::judge`; `decide` widened to the fields; `Outcome::FieldsUpdated` and `Outcome::FieldsDiverged`; `Tally`'s two new counters.
- `crates/safix-core/src/bridge.rs` — `bridge_sync::judge` deleted in favour of `endpoint::judge`; `RESERVED_MAPPING_WORDS` widened to six.
- `crates/safix-core/src/audit.rs` — `KeepassxcOutcome::FieldsDiverged`; `compare_keepassxc` comparing fields after values.
- `crates/safix-core/src/error/{mod,prose,code}.rs` — two variants, their sentences, their codes.
- `crates/safix/src/reporter.rs` — two sample arms.
- `crates/safix/src/snapshots/` — four new accepted snapshots.
- `crates/safix/src/render.rs` — the two new sync words and the audit outcome, each naming fields and never contents.
- `crates/safix/tests/harness/mod.rs` — `seed_sync_mapping`'s username parameter becomes a fields record; `store_field`; `store_fields`.
- `crates/safix/tests/support/card-stubs.rs` — the modelled database records and returns the three argv fields.
- `crates/safix/tests/sync_path.rs`, `crates/safix/tests/audit.rs` — the migrated seeding and the new field claims.
- `modules/flake/checks/keepassxc.nix` — the field refusal fixtures and their literals.
- `modules/flake/checks/reserved-words.nix` — new: the nix list against the Rust literal.
- `modules/flake/checks/cli.nix` — the field claims' check attributes.
- `CHANGELOG.md` — the migration note.

Not affected, verified rather than assumed: `Cargo.toml`, `Cargo.lock`, `deny.toml` and `openspec/specs/rust-supply-chain/spec.md` — no dependency is taken; `modules/flake/safix/bridge.nix`'s `stateSuffix`, `companionsOf` and `Addressing`; and `examples/`, which declares no keepassxc mapping username today.
Every guarantee this change states gets a severity drill in `tasks.md`.
