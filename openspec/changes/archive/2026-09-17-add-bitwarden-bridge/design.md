# Design: a third sync target, the operator's Bitwarden vault

## Context

See `proposal.md` — Why, for the motivation and for the dependency on `extend-bridge-fields`.

Four facts in the tree fix the shape of this change, and none of them is negotiable here.

**Every target-shaped thing in this repository is duplicated per target today, and one shared piece of it is being collapsed by another change.**
`bridge::selected` (`crates/safix-core/src/bridge.rs:382-414`) and `sync::selected` (`crates/safix-core/src/sync.rs:339-356`) are the same function over different mapping types; `bridge_sync::judge` (`bridge.rs:805-838`) and `sync::two_way` (`sync.rs:451-493`) are line-for-line the same decision, and `bridge.rs:617-625` says so in a comment.
`extend-bridge-fields` collapses them behind `crates/safix-core/src/endpoint.rs` — `Record { value, fields }`, `trait Endpoint { unlock, read, write, list, capabilities }`, `Existing { Absent, Present }` — and one `judge`.
This change is therefore the first target written *against* that trait rather than beside it, and it adds no third copy of the decision.

**A far side that is a network service is new, and the existing refusal vocabulary is about paths.**
`Error::NoStoreDatabase { mappings }` (`crates/safix-core/src/error/mod.rs:1174-1177`) exists because a keepassxc mapping is unreachable without a declared path; `Error::StoreLocked { database }` (`:1190-1193`) names the database in its prose (`error/prose.rs:703-720`).
Neither sentence can be reused for a vault: there is no path to name, the thing that is locked is a client rather than a file, and a client can additionally be *unauthenticated*, which no existing variant has a word for.

**The transport rules are stated in a spec, and one of them has to be narrowed rather than bent.**
`openspec/specs/keepassxc-sync/spec.md` ("The database is a store being written, never a keyring being managed") says a secret value "SHALL travel standard input or pipes, never an argument vector or an environment variable", and `crates/safix/tests/support/card-stubs.rs:70-78` asserts the argv-and-env claim unconditionally over every recorded invocation.
Bitwarden's session key has no standard-input channel: `bw` takes it as `--session <key>` in argv or as `BW_SESSION` in the environment, and nowhere else.
So this change either puts a credential in argv, which is world-readable through `/proc/*/cmdline`, or it narrows the env claim deliberately and writes down the narrowing.

**Nothing in this repository can drive a real `bw`.**
`modules/flake/checks/integration.nix:111-167` builds every integration check in a `pkgs.runCommand`, and `runOneWith`'s own comment (`:112-118`) records the doctrine for exactly this situation: `extra` exists for the one check needing the real `keepassxc-cli`, and leaving a needed input out entirely "would make that check state an absence and pass — which is how a claim stops being made without anybody deciding to stop making it".
`bw login` and `bw unlock` need a reachable server; a nix build has no network.
The only hermetic option is a NixOS VM against `services.vaultwarden`, and decision B9 records the measurement that would have to come first.

## Goals / Non-Goals

**Goals:**

Give a fleet whose people read credentials on a phone one declared, reviewable relationship per secret and one verb that ends the drift.
Settle the vocabulary a network-backed far side needs — unreachable, locked, unauthenticated, stale, ambiguous — once, on the free-licensed client, so the fourth and fifth targets copy a settled shape.
Keep the two-way memory inside the vault, and take the simpler mechanism this target makes available rather than inheriting the one keepassxc was forced into.
Narrow exactly one transport invariant, in one place, with the narrowing asserted by an instrument rather than promised in prose.
Leave the keepassxc target byte for byte as it is.

**Non-Goals:**

Any change to `crates/safix-core/src/store.rs`, `modules/flake/safix/keepassxc.nix`, or the keepassxc specs.
This change adds a sibling; a change that also edited the sibling it is modelled on would make a regression there indistinguishable from this work.
Collections, organizations, and Bitwarden's `send` surface: a second addressing space with its own permission model, additive to `bitwardenSide` later.
Any form of vault authentication: safix unlocks what an operator logged into, as it opens a database an operator created.
A real-binary or VM check for this target; B9 records what would unblock one.
Deletion, on either side, in any mode — the rule `modules/flake/safix/keepassxc.nix:127-133` states, transferred unchanged.

## Decisions

### B1. An item is addressed by folder and name, never by its identifier

`bitwardenSide = { folder = nullOr str; item = str; fields = <the shared submodule>; }`.
`folder = null` means the vault's root, which is where an item with no folder lives.
Addressing resolves at run time: `bw list items --search <item>` filtered by the resolved folder id, then the matched item's id is used for the subsequent `get`/`edit`, memoized for the run.

**Alternative rejected**: declaring Bitwarden's own item id.
It reads as the robust choice, being what `bw get item` actually takes, and it is the fragile one.
An id is opaque to review — a diff adding `item = "3a1c…";` says nothing a reviewer can check — it is not what the person holding the item sees, and it is reissued when a vault is exported and re-imported, so a declaration written against one silently stops naming anything and every mapping reports the item absent.
Folder-and-name is the address the person uses, which is the same reason `kdbx.path` is a group-relative path rather than a kdbx UUID.

**Alternative rejected**: accepting either, an id when given and a name otherwise.
Two addressing spellings for one thing, decided by which field is set, is the "second convention beside the existing one" this repository forbids, and it makes the ambiguity refusal in B8 conditional on a declaration's shape.

### B2. `server` is optional, and the refusal is a mismatch rather than an absence

`flake.safix.bitwarden.server = nullOr str`, default `null`.
Null means the run proceeds against whatever server the operator's `bw` is configured against; a declared URL is checked against the `serverUrl` the unlocked client reports in `bw status`, and a difference is `Error::BitwardenServerMismatch { declared, reached }`, raised before any side is read.
safix never runs `bw config server`: pinning the client's server is the operator's own act, like creating the vault.

**Alternative rejected**: `Error::NoBitwardenServer { mappings }`, the direct analogue of `NoStoreDatabase`.
It requires `server` to be mandatory, and the analogy breaks where it matters: a keepassxc mapping has literally nowhere to go without a declared path, whereas a `bw` that an operator has logged in already knows its server, so requiring the declaration would add a second place for one fact to be wrong and would refuse a correctly working setup for saying nothing.
The hazard the variant was reaching for is real but different — a *declared* self-hosted URL that is not the one the session reached, which would write a fleet's secrets into the wrong vault — and that is what `BitwardenServerMismatch` names.
Recorded rather than dismissed: if a future capability needs to *create* the client's configuration, `NoBitwardenServer` is the variant to mint then, and nothing here blocks it.

**Alternative rejected**: running `bw config server <url>` to make the declaration true.
That is safix managing the operator's client rather than writing to a vault through it, and it would silently repoint a client an operator uses for other vaults.

### B3. `bw status` preflight, password on standard input, session key in the child's environment only

The unlock sequence, in order, once per run, before any mapping is read:

1. `bw --nointeraction status` → JSON `{serverUrl, lastSync, userEmail, userId, status}`.
   `status == "unauthenticated"` → `Error::BitwardenLocked { state: "unauthenticated" }`, whose prose names logging in as the operator's act.
   `status == "locked"` with no terminal → `Error::BitwardenLocked { state: "locked" }`.
   A declared `server` unequal to `serverUrl` → `BitwardenServerMismatch`.
2. `status == "locked"` with a terminal → one password prompt through the existing hidden-prompt path, then `bw --nointeraction --raw unlock --passwordfile /dev/stdin` with the password written to the child's standard input and the session key read from its standard output.
3. Every later invocation carries `BW_SESSION` in its own environment and takes `--nointeraction`.
4. `bw lock` at the end of a run that unlocked; a run that found the client already unlocked leaves it as it found it.

`--nointeraction` is on every invocation so a client that would otherwise prompt fails instead of hanging a check.

**Alternative rejected**: `--session <key>` per invocation.
It keeps the environment clean at the cost of putting the key in `/proc/<pid>/cmdline`, which every process on the machine can read, where an environment is readable by the same uid and root alone.
This is the strictly larger exposure, so it is the wrong half of the trade.

**Alternative rejected**: `bw unlock --passwordenv <VAR>`.
It moves the *master password* — the credential that outlives the run — into an environment variable, which is worse than the session key being there, and for no gain: `--passwordfile /dev/stdin` keeps it on a pipe.
`/dev/stdin` is the one measured uncertainty in this design; task group 4 measures it against the pinned client before the transport depends on it, and records the fallback if it does not behave as an ordinary file read.

**The narrowing, stated once.** The invariant becomes: no master password and no mapped value in any argument vector or any environment, on any invocation; the session key in the environment of the invocations that need it, and in no argument, no file safix writes, and no output path.
It is narrower than the unconditional claim `card-stubs.rs:70-78` asserts, it is narrower *deliberately* because this transport offers no third channel, and it is asserted by `bw-stub.rs` over every recorded invocation rather than promised here — which is the whole point of writing it down as a `behavioural-suite` requirement.

### B4. One `bw sync` before the first read, and a failed one refuses every mapping

`bw --nointeraction sync` runs exactly once per run, after the unlock and before the first read.
Failure is `Error::BitwardenStale { output }`, applied to every mapping on this target.

The reason it is a refusal rather than a warning: `bw`'s read commands answer from a local copy, and `bw sync` pulls — the docs are explicit that pushes happen on change, and that `sync` is the pull.
A stale local copy makes a comparison report agreement that does not exist, and under `backup` it makes safix write a secret into an item that already holds one, which is the exact outcome `backup`'s non-overwriting rule exists to prevent.

**Alternative rejected**: syncing per mapping.
One network round trip per mapping, and worse, two reads of one run could then see two different vaults — so a report would describe a vault state that never existed at any instant.

**Alternative rejected**: treating a failed sync as an unjudgeable outcome per mapping (`Outcome::NotJudged`).
`NotJudged` is for a fault local to one mapping; a failed refresh is a fault of the run, and reporting it once per mapping would print the same client error N times while implying the mappings were looked at individually.

### B5. The two-way memory is a hidden custom field of the item itself

`fields[] = [{ name = "safix-sync-state", value = "safix-sync-v1 <fingerprint>", type = 1 }]`, type 1 being Bitwarden's Hidden.
The format string is the one `crates/safix-core/src/sync.rs:68` already uses, so one reader understands both targets' memories, and the comparison is over the whole recorded line's bytes — a memory under an unknown tag matches neither side and degrades to a conflict rather than to a guess, exactly as `bridge.rs`'s `agrees` does.

**Alternative rejected**: a companion item named `<item>.safix-sync-state`, keepassxc's mechanism.
`openspec/specs/keepassxc-sync/spec.md`'s own amendment note (lines 77-79) records that the companion exists *only* because `keepassxc-cli` 2.7.12 has no custom-attribute write, and that the original requirement asked for a field of the entry itself.
Bitwarden can write the field.
Copying the workaround would import machinery this target does not need — a reserved name suffix, a `reservedName` evaluation refusal, and a "lingering companion whose mapping is gone" report shape — to reproduce a limitation that is not here.

**Alternative rejected**: the item's `notes` field.
`notes` is a declarable field (B7), so the memory would collide with a consumer's own declaration, and it is not hidden.

### B6. The item payload crosses as base64 JSON on standard input, and an edit round-trips the whole item

Read: `bw --nointeraction get item <id>` → the full item JSON on standard output.
Create: `bw --nointeraction create item` with base64(JSON) on standard input.
Edit: `bw --nointeraction edit item <id>` with base64(JSON) on standard input.
The base64 is computed in process; there is no `bw encode` subprocess, and the documented positional `<encodedJson>` form is never used, because a payload carrying a secret value in an argument vector is what the transport rule forbids.

`bw edit item` replaces the whole item, so every write is read-modify-write: the item JSON is fetched, the value, the declared fields and — on a two-way mapping — the memory field are set on that object, and the whole object goes back.
A write that constructed a fresh payload would delete every field of the item the declaration does not govern, and the `bitwarden-sync` requirement "An existing item is edited rather than replaced in part" is what holds that.

**Alternative rejected**: `bw encode` as a subprocess, which is the form the upstream docs pipeline shows.
It puts the plaintext payload on a pipe into a second process for no reason — the encoding is a pure function of bytes safix already holds — and doubles the number of children a value passes through.

### B7. Three fields carried, `tags` refused at evaluation, `{ entry = … }` permitted

`capabilities()` for this target declares `username`, `url` and `notes` carried, each by the standard-input JSON channel, and `tags` **not** carried.
A mapping declaring `tags` is `Error::FieldUnsupported { target: "bitwarden", field: "tags" }` at evaluation, from `bitwarden.nix`'s `violationsOf` — a returned sentence, not a throw, so a fixture can assert it against a literal.
`{ entry = "<name>"; }` is permitted for all three, because this target's fields travel standard input: `Error::FieldSourceInArgv` cannot arise here, and the `bitwarden-sync` spec says so rather than leaving its absence unexplained.

The mapping into Bitwarden's schema: `login.username` for username, `login.uris = [{ match = null; uri = <url>; }]` for url, and the item's top-level `notes` for notes.

**Alternative rejected**: mapping `tags` onto folders or collections.
Bitwarden genuinely has no tag concept; a folder is a single placement (an item is in one folder) and a collection is an organizational permission boundary.
Mapping a list of labels onto either means something the declaration did not say — declaring two tags cannot put an item in two folders — and the failure would be discovered by an operator finding their item moved.

**Alternative rejected**: accepting `tags` and dropping it silently.
This is the precise failure the `username` option's own documentation (`modules/flake/safix/keepassxc.nix:88-94`) was written to forbid: every field safix writes is a field its report and its refusals have to be able to speak about.

### B8. Ambiguity refuses; absence depends on the mode

Folder-and-name resolution that matches more than one item is `Error::BitwardenItemAmbiguous { mapping, folder, item, matched }`, and nothing picks between them — no recency rule, no id ordering.
A vault legitimately holds two items with one name, and a mirror that chose would converge a person's secret with whichever one sorted first.

Matching no item is not one outcome: a `safix-to-bitwarden` or `backup` mapping creates the item; a `bitwarden-to-safix` mapping is `Error::BitwardenItemAbsent { mapping, address, mode }`, the direct analogue of `StoreEntryAbsent`; a `two-way` mapping treats one-sided presence as bootstrap and pushes, which is what the shared `judge` already does.

### B9. One stub, a structural guard, and a written absence

`crates/safix/tests/support/bw-stub.rs`, one binary, role chosen by the first word after the program — `status`, `config`, `sync`, `unlock`, `list`, `get`, `create`, `edit`, `lock` — following `clan-stub.rs`'s argv-shape dispatch and `card-stubs.rs`'s env recording.
It stores items as JSON under its spool keyed by a generated id, so a `get` reads what a prior `create`/`edit` wrote, and the on-disk layout is deliberately unlike a real `bw`'s data directory, for the reason `clan-stub.rs:290-300` gives: a runtime that cheated by reading the tool's own files finds nothing.

It asserts, on every invocation: `--nointeraction` present; no master password and no mapped value in argv or env; the session key in env and nowhere else; a positional argument that is not valid base64 refused (which is what catches a payload handed as `<encodedJson>`); an `edit` whose payload is not a complete item refused.

The harness guard, `refuse_a_real_vault`, follows `refuse_a_real_database` (`crates/safix/tests/harness/mod.rs:2802-2821`) exactly: a bitwarden run is refused unless `SAFIX_BW` names the stub, `BITWARDENCLI_APPDATA_DIR` is under the fixture's scratch, and the configured server is loopback.
Three conditions rather than one because a developer machine plausibly holds a logged-in `bw`, and the cost of the guard being wrong is a fleet's real vault.

**No real-binary check, and the absence is recorded in `modules/flake/checks/bitwarden.nix`'s header.**
`bw` cannot authenticate without a network, and a nix build has none.

**Alternative rejected, for now**: a NixOS VM check beside `modules/flake/checks/installer-vm.nix`, with a `services.vaultwarden` node.
The machinery exists and nixpkgs ships `nixos/tests/vaultwarden.nix` with a `bitwarden-cli` client node, which is why this is recorded as deferred rather than impossible.
It is not minted here because that upstream test's `bw login`/`sync`/`list` assertions are commented out, so adopting its shape would produce a check whose green means nothing was measured — the exact failure `integration.nix:112-118` names.
What would unblock it: a measurement, on the pinned client, that `bw login --apikey` and `bw unlock` complete against a local vaultwarden with snakeoil certs.
That measurement is task 9.4, and it is recorded as a measurement rather than promised as a check.

### B10. The target's own dispatch, report section and usage text are additive, and nothing existing is renamed

`bridge::Target::Bitwarden`; `"bitwarden"` in `parse_dispatch`'s keyword match (`crates/safix/src/main.rs:717-720`); a third arm in `DirectionOnWrongTarget`'s rendering (`main.rs:770-778`), which enumerates targets explicitly today; a third `matches!` block in `sync_command` (`main.rs:827-850`); `Report.bitwarden: Option<BitwardenReport>` and a third conjunct in `Report::is_clean` (`crates/safix-core/src/audit.rs:212-234`); `Attribute::Bitwarden` → `safix.lib.bitwarden` (`crates/safix-core/src/nix.rs:51, 80, 102`); one `OnceLock` and one accessor on `Workspace` (`workspace.rs:43, 104, 236-240`).

`--direction` stays clan-only: it is the clan target's vocabulary, and a bitwarden mapping carries a mode, so `audit bitwarden --direction …` is `DirectionOnWrongTarget` exactly as `audit keepassxc --direction …` is.

## Risks / Trade-offs

**The transport is held by a model, not by a tool.**
Every claim about what `bw` does with an argument vector rests on the stub and on fetched documentation, and `clan-stub.rs:1-55` states the limit plainly: a stub cannot establish that the argv means to the tool what safix thinks.
Mitigation is honesty rather than coverage — the absence is written where the checks are, the deferred VM check names its own precondition, and every invocation the transport builds is asserted against a literal in a unit test so a change to one is visible in a diff.

**A network-backed target introduces a partial run.**
A read that fails halfway through a set of mappings leaves the earlier ones written and the later ones untouched.
This is already true of the clan target and of the shared-machine search, and the mitigation is the same: the report is per mapping and complete, an unjudgeable mapping is reported rather than skipped, and nothing is written before the value's own side is judged.
What is *not* mitigated, and is stated rather than hidden: there is no transaction across mappings, and there cannot be one over a vault that has no such operation.

**`BW_SESSION` in a child's environment is a real exposure, not a nominal one.**
Any process of the same uid can read it while a run is in flight, and the run ends with `bw lock`, which invalidates it.
The alternative was argv, which is worse; the trade is recorded in B3, asserted by the stub, and stated as a requirement so a later reader finds a decision rather than an oversight.

**`--passwordfile /dev/stdin` is the one unmeasured mechanism.**
If the pinned client stats the path rather than reading the stream, the fallback is a file inside the run's own memory-backed staging root with owner-only permissions — which `plaintext-staging` already governs — and never `--passwordenv`.
Task 4.2 measures it and task 4.3 records the outcome; the transport is not written against the unmeasured assumption.
