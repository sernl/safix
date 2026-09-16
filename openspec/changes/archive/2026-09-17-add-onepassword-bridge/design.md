# Design: add-onepassword-bridge

## Context

This change adds the fifth target of `safix sync` and `safix audit`, and the second one whose far side is a network service.
Everything structural is settled before it starts: `extend-bridge-fields` ships the `fields` submodule, the complete reserved-word list, the `Endpoint` trait in `crates/safix-core/src/endpoint.rs`, one `judge` over `Record`s, and the two field refusals.
What remains is this target's own answers — the option shape, the authentication posture, the JSON crossing, the memory's location, the edit's discipline, the report's honesty about a run that stops partway, and the checks that can and cannot exist.

Grounding, read in the tree while designing this change:
the declaration surface this one is modelled on is `modules/flake/safix/keepassxc.nix` (modes `:34-41`, `pullCapable` `:43`, `stateSuffix` `:55`, `companionOf` `:57`, `kdbxSide` `:59-91`, `mapping` `:92-142`, `mappingsOf` `:146`, `entryPathOf` `:149`, `violationsOf` `:151-232`) and `modules/flake/safix/options.nix:507-622`;
the converge loop is `crates/safix-core/src/sync.rs` (`Outcome` `:75`, `Decision` `:215`, `run` `:240`, `decide` `:378`, `two_way` `:451`, `recorded` `:500`, `agrees` `:512`, `push` `:543`, `pull` `:570`, `remembered_after` `:633`, `commit_subject` `:654`, memory format `:68`);
the report is `crates/safix-core/src/audit.rs` (`KeepassxcOutcome` `:146`, `KeepassxcFinding` `:170`, `KeepassxcReport` `:184`, `Report` `:213`, `run` `:258`, `lingering` `:345`, `run_keepassxc` `:387`, `compare_keepassxc` `:437`) and `crates/safix/src/render.rs:626-800`;
the refusals are `crates/safix-core/src/error/mod.rs:1146-1250`, `error/prose.rs:646-760`, `error/code.rs:178-188`;
the program-override seam is `crates/safix-core/src/enroll/custody.rs:44,352-357`;
the stub doctrine is `crates/safix/tests/support/clan-stub.rs` and `card-stubs.rs`, the harness guard is `crates/safix/tests/harness/mod.rs:2748-2835`, and the "an absence must be stated" rule is `modules/flake/checks/integration.nix:112-116`.

## Decisions

### 1. A mapping names a vault and an item, not an `op://` reference

`onepassword = { vault = str; item = str; fields; }`, both strings, `vault` required and `item` required.
`vault` is required rather than defaulted because a service-account token cannot reach the built-in Private, Personal or Employee vault at all and `op item get` requires `--vault` under one, so a default would be a value that fails for the most likely automation posture.

Rejected: a single `reference = "op://<vault>/<item>/<field>"` string.
The reference is 1Password's own addressing spelling and reads well, but it collapses three named things into one opaque token: a refusal could not say which half of the address is wrong, `list()` would have to parse it back apart to enumerate a vault, and the field set the reference's last segment names would be a second field surface beside `fields`.

Rejected: a nix path for either.
`keepassxc.database` is a string for a measured reason (`options.nix:516-521`) and the same reason applies twice here — a path is copied into the world-readable store on every evaluation, and a root-dependent absolute string makes the two `safix-examples` consumers resolve to two different values.

### 2. The session is the operator's, and the preflight comes before any read

`unlock()` runs `op whoami --format json`, with `--account <shorthand>` when `account` is declared, and refuses with `OnePasswordSignedOut` carrying `op`'s own words when it does not answer.
It runs before the first mapping's side is read — the position `sync.rs:240-336` gives `enroll::terminal_present()`, whose whole point is that a headless run refuses without having decrypted safix's side of every mapping first.
Authentication material is whatever the operator's environment already holds: `OP_SERVICE_ACCOUNT_TOKEN`, or a session `op signin` established in that shell.

Rejected: safix running `op signin --raw` itself.
It needs the desktop application's CLI integration turned on, it prompts through a channel safix does not own, and the token it prints would then be safix's to hold, pass and zero — a credential-management role this repository refuses everywhere else (`keepassxc-sync`: "The database is a store being written, never a keyring being managed").

Rejected: `--session <token>` in the argument vector.
`/proc/<pid>/cmdline` is world-readable; the child's environment is readable by the same uid or root.
The environment is strictly the lesser exposure, and it is where a session already is when the operator signed in.

### 3. `account` is nullable and there is no "no account declared" refusal

`account = nullOr str`, default `null`, passed as `--account <shorthand>` on every invocation when set and omitted entirely when not.
There is deliberately no analogue of `NoStoreDatabase`: `op` resolves its own default account, and a service-account token names one implicitly, so an undeclared account is a working configuration rather than a missing one.
When the account is the defect, it is `op`'s own sentence that says so, carried verbatim by `OnePasswordSignedOut`.

Rejected: requiring `account` whenever a mapping is declared.
It would make the common single-account and service-account cases carry a value safix does nothing with but pass through.

### 4. Every byte crosses on standard input, and that is what makes the capability row full

Read: `op item get <item> --vault <vault> --format json --reveal`.
Write, absent: `op item create --vault <vault> --category login --title <item> -`.
Write, present: `op item edit <item> --vault <vault> -`.
List: `op item list --vault <vault> --format json`.
Each write's whole payload is one JSON object on standard input; no `field=value` assignment statement is ever spelled, because 1Password's own documentation states such statements are logged in shell history and can be visible to other processes.

The four declared fields map onto the item's own JSON: `username` is the built-in field with `purpose: "USERNAME"`, `notes` is `notesPlain` with `purpose: "NOTES"`, `url` is the item's `urls[]` autofill entry, and `tags` is the item's own `tags` array.
So `capabilities()` for this target claims all four with a stdin channel for each, `Error::FieldUnsupported` and `Error::FieldSourceInArgv` are unreachable for it, and a `{ entry = … }`-sourced field is admissible on all four — the asymmetry against keepassxc, whose `url`/`notes` are argv-bound and whose `tags` cannot be written at all, is the reason the capability table exists rather than a default.

Rejected: the `url` custom-field type instead of `urls[]`.
1Password's documentation is explicit that the `url` field type is not used for autofill; the autofill website is the `urls[]` entry, and a declared `url` that did not appear where a person's browser looks would be a field safix wrote and nobody sees.

### 5. The two-way memory is a concealed field of the mapped item

A custom field named `safix-sync-state`, type `CONCEALED`, on the item itself, carrying the shared `safix-sync-v1 <fingerprint>` line (`sync.rs:68`), compared byte-for-byte by the shared `agrees` (`sync.rs:512`) so a line under an unknown tag matches neither side and degrades to a conflict rather than a guess.

Rejected: a companion item beside the mapped one.
That shape exists for keepassxc only because `keepassxc-cli` 2.7.12 cannot write a custom attribute on any verb — the amendment recorded at `openspec/specs/keepassxc-sync/spec.md:77-79`.
`op` can, so a companion here would add a second object, a reserved name, a refusal keeping that name out of a consumer's reach, and a "lingering companion whose item is gone" report shape, all to work around a limitation this transport does not have.

Consequence to state rather than leave implied: this target reserves no name, so `flake.safix.onepassword` has no `reservedName` violation and `violationsOf` carries four rules where keepassxc's carries five.

### 6. An edit is a round-trip; safix never assembles a template

`write()` against `Existing::Present` starts from the item JSON `read()` already fetched, replaces the value field, the declared fields and — under `two-way` — the memory field, and writes that object back.
Every other member of the object survives untouched.

This is a refusal made unnecessary rather than a refusal added.
1Password's documentation carries an explicit danger that an item edited from a JSON template loses the passkeys on the item; safix cannot reach that outcome, because it never sends a template.
The check that holds it: the stub's stored item carries a `passkey` member and an unrecognised custom section, and both are still there byte-identically after a `safix-to-1password` run rewrote the value.

Rejected: refusing an item that carries a passkey.
It would be a refusal for a hazard the round-trip already removes, and it would make a person's ordinary login item — passkey and password together, which is what 1Password encourages — unmappable.

### 7. A run that fails partway reports per mapping and continues

There is no write burst here and no deferred pull.
keepassxc's two-phase shape exists because a kdbx save rewrites and re-uploads the whole file (`sync.rs:8-16`); an `op` write is one item over the network, so the phases would buy nothing and would hold a decision from before the write against a far side that may have moved.

A network or command failure on one mapping produces that mapping's own refusal — `OnePasswordCommandFailed`, `OnePasswordItemAbsent` or `OnePasswordVaultAbsent` — and the loop continues to the next mapping, exactly as `decide`'s `Outcome::NotJudged` already does for a side that cannot be read (`sync.rs:378-448`).
The report is therefore complete over the declarations whatever happened, and the run exits non-zero.

Rejected: aborting the whole run on the first failure.
It would make the report a function of the ordering of the declarations, and it would leave an operator unable to see which of thirty mappings converged.

Rejected: a retry.
A retried write is a second write of the same value under a report line that says one happened; a transient failure is a refusal an operator re-runs, which is what every other refusal in this runtime already is.

### 8. The stub enforces the argv rule, and no check ever runs the real binary

`crates/safix/tests/support/op-stub.rs`, one binary, dispatching on `whoami` / `vault` / `item` + subcommand, following `clan-stub.rs`'s shape: a spool directory named by `SAFIX_OP_STUB_SPOOL`, recording argv, environment and standard input per invocation, with failure switches `SAFIX_OP_STUB_SIGNED_OUT`, `SAFIX_OP_STUB_UNKNOWN_ITEM`, `SAFIX_OP_STUB_UNKNOWN_VAULT` and `SAFIX_OP_STUB_REFUSES` for the refusal drills.

Two things make it an instrument rather than a convenience.
It exits non-zero on any argv word containing `=`, so "no value in an argument vector" is checked by the thing safix actually runs.
And it requires `--vault` on `item get`, `item create`, `item edit` and `item list`, which is service-account semantics, so a transport that omitted one would fail here rather than at an operator's terminal against a personal vault.

The harness guard, beside `refuse_a_real_database` (`harness/mod.rs:2802-2835`): `SAFIX_OP` must name the stub, `OP_SERVICE_ACCOUNT_TOKEN` must be the fixture's fake, and `OP_ACCOUNT` must be unset.
Both halves are needed for the same reason the keepassxc guard needs both — a machine that develops this suite plausibly holds a real, signed-in `op`.

There is no real-binary check for this target and there never will be.
`_1password-cli` at this flake's pin is `license = lib.licenses.unfree`, so naming it from `perSystem.checks` or the devshell makes `nix flake check` fail at evaluation for every consumer without `allowUnfree`; there is no self-hostable server to point a VM node at; and no authentication path works without the network, which no `nix build` and no hermetic VM node has.
Each reason alone is sufficient and none expires, which is why the absence is written into the module header, the transport's header and the capability rather than inferred from a missing file.

### 9. The refusals this target owns, and the two it reuses

Five new variants, each with a prose function, a code, a `reporter::sample` arm and two snapshots:
`OnePasswordUnavailable { program, cause }` (the binary could not be run — the analogue of `StoreUnavailable`, `error/mod.rs:1146-1154`);
`OnePasswordSignedOut { account, output }` (the preflight did not answer);
`OnePasswordCommandFailed { item, arguments, output }` (one item's command refused, carrying `op`'s own standard error and an argument vector that by construction holds no value);
`OnePasswordItemAbsent { mapping, vault, item, mode }` (the far side is the source and holds nothing);
`OnePasswordVaultAbsent { mapping, vault, output }` (the declared vault is not one this session can see, which is a distinct fault from an absent item and has a distinct remedy — a service account's permissions).

`UnknownSyncMapping` and `SyncSourceEmpty` are reused.
Both already name a mapping, a declaration set and safix's own side, and neither says anything about a database; duplicating them per target would be five spellings of one sentence.

`ValueSpansLines` is not reused and has no analogue.
It is a `keepassxc-cli` limitation (`--password-prompt` reads one line), not a truth about secrets, and `op`'s stdin JSON carries a multi-line value whole — which the capability states positively, so nobody adds the refusal back by analogy.

### 10. One spelling per target: `1password`, never `op`

The target keyword, the mode words and the reserved mapping word are all `1password`.
`op` is the binary's name, selectable through `SAFIX_OP`, and is not accepted as a target keyword anywhere.

Rejected: accepting both.
An alias makes two spellings of one target appear in reports, in refusals, in the reserved list and in a consumer's muscle memory, and the reserved-word refusal would then have to reserve both to stay honest.

### 11. audit reuses the shared outcome, and the remedy names this target

`audit 1password` compares both sides of every declared mapping and writes nothing, reporting agreeing, diverged, fields-diverged or unjudgeable per mapping through the shared per-target outcome `extend-bridge-fields` establishes.
Value divergence dominates field divergence.
A diverged mapping's remedy line is `safix sync 1password <mapping>` under a pushing mode, and "the declaration is the author" under `1password-to-safix`.
The report names a diverged field and never its content, because a `notes` body and an `{ entry = … }`-sourced field are themselves secrets.
Items in a declared vault that no mapping declares are reported as information, never removed, and never move the exit status — the rule `audit.rs:345` and `keepassxc-sync`'s "audit's exit status answers only whether every compared mapping agreed" already state.

### 12. The unfree statement's home is this capability, not `rust-supply-chain`

`rust-supply-chain`'s licence requirement is about the locked cargo graph reviewed offline by `cargo-deny` (`openspec/specs/rust-supply-chain/spec.md:68-82`); `_1password-cli` is a nixpkgs derivation and appears in no lock file.
Putting the statement there would make a rust-dependency requirement answer a question about the flake's own closure, and the check that would hold it is not the dependency check.
So the claim lives where its subject lives: a requirement of `onepassword-sync`, the header of `modules/flake/safix/onepassword.nix`, and the header of `crates/safix-core/src/onepassword.rs`.

## Risks

1. **The transport is measured against a model, not against the tool.**
   Every claim about `op`'s JSON shape and flags comes from 1Password's published CLI reference, and no check can confirm it.
   Mitigation, and it is a narrowing rather than a fix: the stub emits the documented `fields[]{id,type,purpose,label,value}` shape and the documented `tags`/`urls` members, so the deserializer is exercised against the documented schema; and `crates/safix-core/src/onepassword.rs`'s header records, in the shape `crates/safix-core/src/store.rs:1-52` records measured `keepassxc-cli` behaviour, that these are *documented* rather than measured facts and what a contributor with a licensed `op` should re-measure.
2. **A vault-wide `list()` reads more than the mapped items.**
   Nothing it returns reaches a report beyond a name, but an operator's vault may hold items whose *names* are sensitive.
   Mitigation: lingering items are reported as names under the vault the declaration itself names, which is exactly the scope `keepassxc`'s group-wide listing already has, and no field of an unmapped item is ever requested.
3. **The bare `sync` form now performs network round-trips.**
   Mitigation: only for a consumer who declared mappings, and the preflight refuses once rather than per mapping, so a signed-out fleet costs one invocation.
4. **A concealed field named `safix-sync-state` can be edited by a person.**
   Mitigation: the same one keepassxc's companion has — an unreadable or unrecognised memory is treated as absent, which converts the mapping to bootstrap semantics rather than to a refusal (`sync.rs:500-524`).
