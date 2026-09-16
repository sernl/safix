# Design: a third sync target, the operator's `pass` store

## Context

See `proposal.md` for motivation. Five facts in the tree fix the shape of this change, and none of them is negotiable here.

`extend-bridge-fields` lands first, and this change is written against what it leaves behind: `crates/safix-core/src/endpoint.rs` carrying `trait Endpoint { fn unlock(&mut self); fn read(&self, id) -> Result<Option<Record>>; fn write(&mut self, id, &Record, Existing); fn list(&self); fn capabilities(&self); }`, `Record { value: Option<Secret>, fields: ResolvedFields }`, one `judge` over `Record`s replacing the two identical copies at `crates/safix-core/src/bridge.rs:805-838` and `crates/safix-core/src/sync.rs:451-493`, `modules/flake/safix/fields.nix`, and `modules/flake/safix/reserved.nix` holding all six reserved mapping words with `pass` already among them.
This change adds one implementation of that trait and one declaration surface; it introduces no abstraction of its own.

`pass` has no field schema. `passwordstore.org`'s own data-organization section states the convention — the password on the first line, additional information on the lines after it — and the machine-readable spellings (`login:`, `url:`, `notes:`) are browserpass's, not the tool's. So unlike the other four targets, safix is *defining* the layout of a `pass` record rather than mirroring one somebody else's schema fixes. Every property of that layout is therefore this change's to state, and the specification is where it is stated.

Two per-transport invariants that live in shared code today are keepassxc's, not truths about secrets. `Error::ValueSpansLines` exists because `add --password-prompt` and `edit --password-prompt` read the entry's value as one line (`crates/safix-core/src/store.rs:48-51`); `Secret::without_one_trailing_newline` (`crates/safix-core/src/secret.rs:313`) exists because `show -s -a Password` appends a newline of its own (`store.rs:44-47`). `pass insert --multiline` reads to EOF and `pass show` prints the file, so neither applies here, and inheriting either would corrupt a value.

The two existing two-way memories live in two different name spaces for one reason each: clan's is a minted companion *safix* entry (`modules/flake/safix/bridge.nix:171`, folded into placements at `modules/flake/safix/default.nix:241-244`), keepassxc's is a companion *kdbx* entry (`crates/safix-core/src/store.rs:70-86`). `pass` has neither a custom-attribute surface nor a second field the mapped object can hold apart from its own body, so its memory is a third shape and had to be designed rather than picked.

`modules/flake/checks/integration.nix:112-118` records the rule this change's real-binary check is governed by: `extra` exists so one check can carry a tool the rest of the page has no use for, and leaving such a check out entirely "is how a claim stops being made without anybody deciding to stop making it". `pkgs.pass` is free-licensed and needs no network, which is what makes a real-tool check possible for this target and impossible for the other two.

## Goals / Non-Goals

**Goals:**

State the whole layout of a safix-written `pass` record, including what happens to a multi-line value, so the store stays readable by `pass show`, `pass -c` and browserpass rather than becoming a safix-private format.
Put a real `pass` behind the shared field model, so the four fields are exercised against a tool rather than only against a model.
Reuse the converge loop, the decision function, the selection and the lingering report through `Endpoint`, and add per-target behaviour only where the tool forces it.
Keep every byte of every value and every field on a pipe, in both directions, with nothing but public strings in an argument vector.
Make an accident against the operator's own store structurally impossible in the suite, not conventionally unlikely.

**Non-Goals:**

A second transport. `pass` has one, and the gpg-agent underneath it is not safix's to address.
Any recipient management. A store's `.gpg-id` is its own audience declaration; safix reads whether one exists and never writes one.
A field read into safix on a pull, or a per-field agreement. `extend-bridge-fields` owns the one-author property; this design does not widen it.
A new error for a field `pass` cannot carry, because there is none: the capability row for this target is all four fields, which is why `Error::FieldUnsupported` and `Error::FieldSourceInArgv` are unreachable here and are asserted unreachable rather than left unmentioned.

## Decisions

### D1. The transport is `pass` itself, and the store root travels the environment

`crates/safix-core/src/pass.rs` runs the program `SAFIX_PASS` names, defaulting to `pass`, resolved through the same `named` helper `keepassxc_cli()` uses (`crates/safix-core/src/enroll/custody.rs:353-357`).
Four invocations, and no others:

- read: `pass show <path>`, the whole file on standard output, byte-exact.
- write: `pass insert --multiline --force <path>`, the whole record body on standard input until EOF. `--multiline` is what makes the read to EOF happen; `--force` is what suppresses the overwrite prompt, so a run never blocks on a question a non-interactive check cannot answer.
- list: the `*.gpg` filenames under the store root, with the suffix removed — the namespace `pass` derives its own names from.
- preflight: the store root is a directory holding a `.gpg-id`.

The store root reaches each child as `PASSWORD_STORE_DIR` in its environment. That is a path and never a value, which is the same narrowing `BW_SESSION` needs on the Bitwarden target and does not need justifying twice — the rule is that no *value* and no *field* travels an argument vector or an environment variable, and a store's location is neither.

**Alternative rejected**: parsing `pass ls`. It is `tree(1)` output — box-drawing glyphs and colour — and `crates/safix-core/src/store.rs:39-43` was willing to parse `ls -R -f` precisely because that is a flat line-per-object format. A listing is used for one thing, the lingering report, and building it on a rendering meant for a human is how a report becomes wrong after somebody's `tree` changes.

**Alternative rejected**: `pass edit`. It needs an editor, and the nixpkgs derivation for `pass` deletes its own `t0200-edit`, `t0100-insert`, `t0020-show` and `t0300-reencryption` tests on darwin because `pass edit` needs `hdid` there. A verb that cannot run on a platform safix supports is not a transport.

**Alternative rejected**: reading the `.gpg` file and shelling to `gpg` directly. That is the sin `crates/safix/tests/support/clan-stub.rs:19-23` names for clan — reading what the delegated tool placed instead of asking it — and it would put safix in the business of knowing how `pass` encrypts, which is the whole of what `pass` is.

### D2. The record layout: value first, fields in a trailing block, separated by one blank line

A safix write emits the value's bytes, then — only when at least one field is declared — one blank line, then the field block: `login: <v>`, `url: <v>`, `notes: <v>`, `tags: <a>, <b>`, each on its own line, in that order, omitting a field that is null and the `tags` line when the list is empty.
A read is the inverse: the field block is the maximal suffix of lines every one of which matches `^(login|url|notes|tags): `, one immediately preceding blank line is consumed as the separator when it is there, and the value is every byte before that, verbatim.
A body with no such suffix is a value and nothing else.

Three properties follow, and they are why this layout rather than another.
A single-line value with no declared field round-trips as exactly its own bytes, so an entry safix wrote is indistinguishable from one a person wrote by hand.
`pass show -c` and browserpass keep working, because the value is still the first line and the field spellings are the ones they already read.
A multi-line value keeps its own last line, because the blank separator is what tells the value's end from the field block's start.

The one ambiguity this leaves is stated rather than engineered away: a *value* whose own trailing lines all look like `url: …` reads back as fields. Refusing such a value would be a safix-invented limitation on a store that carries it fine, and a private framing marker would make the record unreadable by the tools above. The blank separator is what keeps safix's own writes out of that case, and the module doc and the spec both say so.

**Alternative rejected**: refusing a multi-line value, keepassxc's shape. `pass insert -m` carries one natively; refusing it would make the target worse than the tool and would put a refusal in front of a value the operator can store by hand.

**Alternative rejected**: a JSON body, or a `safix-v1` header line. Either makes the record safix-private: `pass -c` would copy a brace, browserpass would find no password, and the store stops being the operator's and starts being safix's projection — which is the failure `modules/flake/safix/keepassxc.nix:81-86` names for field templating.

**Alternative rejected**: writing fields without the blank separator, which is browserpass's own convention. It is unambiguous only for a single-line value; for a multi-line one the parse has to guess where the value stopped. The separator costs one byte on entries that declare a field and is read as optional, so both layouts are accepted and only the unambiguous one is written.

### D3. Two-way memory is a companion entry, `<path>.safix-sync-state`

The suffix is `.safix-sync-state` — the same spelling `crates/safix-core/src/store.rs:70` reserves, because both name an entry inside a store rather than a safix entry name (clan's hyphenated `-safix-bridge-sync-state` exists for the reason `crates/safix-core/src/bridge.rs:643-647` gives).
The recorded line is `safix-sync-v1 <fingerprint>` under the shared `FORMAT` at `crates/safix-core/src/sync.rs:68`, so a memory under an unknown tag reads as no memory and takes the mapping to bootstrap rather than to a conflict on every entry.
Evaluation refuses a declared `pass.path` carrying the suffix, which is what makes the reservation structural, and `lingering` pushes each mapping's companion into `claimed` beside its entry the way `crates/safix-core/src/sync.rs:363-376` already does — without that, every two-way mapping would report its own memory as an unclaimed entry forever.
The memory is written as its own second write, strictly after the value's, for the reason `crates/safix-core/src/sync.rs:35-41` records: a memory written first and then an interrupted run leaves the next run overwriting the new value with the old one.

**Alternative rejected**: a `safix-sync-state:` line in the mapped entry's own field block. It collapses the two writes into one object, and the ordering property above then cannot exist — there is no "after" when the value and its memory are the same file. It would also show up in `pass show` and in browserpass as a field of the person's own entry, which is safix decorating somebody's record with its own bookkeeping.

**Alternative rejected**: a state file in the repository. A committed digest of a secret value is an offline oracle for confirming a guess; `crates/safix-core/src/sync.rs:29-33` states it and `keepassxc-sync`'s "No oracle lands in the repository" scenario holds it.

### D4. No unlock step; a gpg failure is its own refusal

`Endpoint::unlock` for this target checks the store and nothing else: a store root that is not a directory, or a directory with no `.gpg-id`, is `Error::NoPassStore { store }`, raised before any mapping is read so a run cannot report "unchanged" about mappings it never looked at — the role `crates/safix-core/src/sync.rs`'s pre-flight terminal check plays for keepassxc.
There is no password prompt, because `pass` shells to `gpg` and the unlock belongs to the ambient `gpg-agent`, which may answer from its own cache, from a pinentry on the operator's own terminal, or not at all.
A `pass show` that fails with gpg's own words about a passphrase, a secret key or an agent is `Error::PassLocked { entry, output }`, carrying gpg's stderr verbatim; every other non-zero exit is `Error::PassCommandFailed { entry, arguments, output }`, and `arguments` is safe to print because no value is ever in it.

**Alternative rejected**: prompting for a passphrase and feeding gpg. It would put safix between the operator and their agent, defeat a hardware-backed key, and require holding a passphrase safix has no use for. The one thing worth having from a prompt — a legible failure — is what `PassLocked`'s prose gives instead.

**Alternative rejected**: treating a gpg failure as `PassEntryAbsent`. Absence and a locked key both exit non-zero, and conflating them would let a `backup` mapping write over an entry it could not read. Absence is answered from the listing, the way `store.rs:39-43` answers it, and never from a status.

### D5. The option surface: a store and per-mapping far side

```
flake.safix.pass = {
  store = <string>;                       # default "~/.password-store"
  mappings.<id> = {
    mode  = enum [ "safix-to-pass" "pass-to-safix" "two-way" "backup" ];
    safix = { user; name; };              # bridge.safixSide, the same submodule both existing targets use
    pass  = { path; fields; };            # fields is fields.nix's submodule, on the far side only
  };
};
```

`store` is a string for `keepassxc.database`'s reason (`modules/flake/safix/options.nix:516-521`) plus one more: `modules/flake/checks/examples.nix` compares the flattened `flake.safix.lib.*` records field-for-field between two examples that evaluate under different roots, and a path-typed option stringifies to a root-dependent absolute path, which reddens `safix-examples` the moment an example declares one.
It carries a default where `database` cannot, because `pass` itself has one — `$PASSWORD_STORE_DIR`, else `$HOME/.password-store` — and a declaration that had to restate the tool's own default would be a declaration of nothing. The `~` is expanded by the runtime, not by nix, because nix has no home to expand against at evaluation time.
`path` is the entry path inside the store, with no leading slash and no `.gpg` suffix: the name `pass` itself takes.

**Alternative rejected**: `store` as a `nullOr str` with `null` meaning "the tool's default", mirroring `database`. It makes every reader answer "what happens when it is null" and every refusal name two states; the string default says the same thing once, and a consumer who wants the tool's default writes nothing.

### D6. Four modes, spelled with this target's word

`safix-to-pass`, `pass-to-safix`, `two-way`, `backup`, with `pullCapable` true for the second and third — the vocabulary at `modules/flake/safix/keepassxc.nix:26-42`, unchanged in meaning. `backup` still never overwrites a differing entry, and no mode deletes one.

**Alternative rejected**: a shared four-word enum across every target (`push`/`pull`/`two-way`/`backup`). `modules/flake/safix/keepassxc.nix:26-28` records why endpoint names win: a declaration is read by someone with no tool in hand to be relative to, and `push` is relative to whoever is speaking.

### D7. What is per-target, and why each thing is

Inside the trait: `unlock`, `read`, `write`, `list`, `capabilities`. Outside it, and documented as such in the module doc:

| Behaviour | Why it cannot be generic |
|---|---|
| the trailer parse and the blank separator | the layout is this target's, and D2 is its whole statement |
| the companion entry and its suffix | a second object in the store's own namespace, where keepassxc's is a kdbx entry and clan's a safix entry |
| the `*.gpg` listing | a coupling to the store's on-disk layout, which is the tool's own name space and is stated as a measured coupling the way `store.rs:29-51` states its own |
| no write burst | a `pass` write is one file; the deferred-pulls discipline at `sync.rs:8-15` exists for a 292 MB whole-file rewrite and has no analogue here |
| byte-exactness | `pass show` adds nothing, so neither the trailing-newline trim nor `ValueSpansLines` is applied |

### D8. Checks: three claims, three shapes

- `safix-pass` and `safix-pass-drill` in a new `modules/flake/checks/pass.nix`, mirroring `modules/flake/checks/keepassxc.nix`: every refusal asserted as the message it produces against a literal, the sound declaration asserted to produce none, and the drill running the same `refuseScript` bytes over a perturbed declaration.
- Per-test integration checks through the existing `integration.runOne`, against the stub.
- `safix-pass-cli` through `integration.runOneWith [ pkgs.pass pkgs.gnupg ]`, linux-only, over a store the check creates in its own directory with its own `GNUPGHOME` and a key minted inside the check. It is what `crates/safix/tests/store_cli.rs` is for keepassxc, and it is the one place the argument vectors meet the tool.

Linux-only rather than unconditional, because the nixpkgs derivation for `pass` disables its own `insert`/`show`/`edit`/`reencryption` tests on darwin; a check that silently did less there would state a claim it is not making.

**Alternative rejected**: no real-binary check, on the ground that the stub already answers every vector. That is exactly the failure `modules/flake/checks/integration.nix:112-118` and `crates/safix/tests/support/clan-stub.rs:19-23` both name: a model answers the vectors safix sends because it was written to, and goes on answering them after the tool changes its options.

### D9. The stub is an instrument, and the guard is structural

`crates/safix/tests/support/pass-stub.rs`, in `clan-stub.rs`'s shape: dispatch on the first word (`show`, `insert`, `ls`, `git`, `--version`), record argv, environment and stdin into a spool named by `SAFIX_PASS_STUB_SPOOL`, and refuse rather than answer where the runtime's contract says it must — an `insert` without `--multiline` is refused, the way `card-stubs.rs` refuses a write with no `--password-prompt`, and any invocation carrying a value in argv is refused.
Its store lives under the spool in a layout deliberately unlike a real store's, so a runtime that cheated by reading the tool's own files finds nothing.
`harness::refuse_a_real_store` refuses, before a process is spawned, any `sync`/`audit` run reaching this target whose `SAFIX_PASS` is not the stub or whose `PASSWORD_STORE_DIR` is not under the fixture's scratch directory — both halves, because either alone lets the accident through, which is the reasoning `refuse_a_real_database` already carries.

### D10. Five refusals, and the two that do not exist

`PassUnavailable { program, cause }`, `PassLocked { entry, output }`, `PassCommandFailed { entry, arguments, output }`, `NoPassStore { store, mappings }`, `PassEntryAbsent { mapping, entry, mode }` — the set `store.rs`'s own refusals map onto, minus the ones this target cannot reach.
`ValueSpansLines` is not raised here (D2), and `StoreLocked`'s shape is not reused because its prose is about a database password and a terminal, and printing that at somebody whose gpg-agent declined would name the wrong remedy — the same reasoning `add-view-picker` used for not reusing `Error::NoTerminal`.
`FieldUnsupported` and `FieldSourceInArgv` are unreachable for this target and asserted unreachable by a unit test over `capabilities()`, so "pass carries all four" is held by a check rather than by this sentence.

### D11. Ordering against the siblings

`extend-bridge-fields` first, then this change, then `add-bitwarden-bridge` and `add-onepassword-bridge` in either order, then `rewrite-readme-and-examples` last because it documents all five targets at once.
This change is deliberately second: it is the only one of the three targets with a real tool behind it, so the field model and the `Endpoint` trait meet a real store before two model-only targets rest on them.

## Risks / Trade-offs

A value whose trailing lines look like field lines reads back as fields → stated in the spec and the module doc as the layout's one ambiguity; safix's own writes never produce it, because they emit the blank separator.

The `*.gpg` listing couples the runtime to the store's on-disk layout → the coupling is one direction (names only, never content), recorded in the module doc the way `store.rs:29-51` records its measurements, and it is the alternative to parsing `tree(1)` output, which is worse.

`pass` auto-commits in the store's own git repository on every mutation → recorded rather than suppressed: the store's history is the store's, safix never runs `pass git`, and the report says what it wrote so a store-side commit is never a surprise.

A gpg-agent that prompts on the operator's terminal makes a run interactive in a way safix does not control → `PassLocked` is what a non-interactive run gets, carrying gpg's own words; the suite proves it through the stub's locked switch, and no check ever waits on a pinentry.

Two writes for a two-way mapping means a window between the value and its memory → the same window keepassxc and clan have, in the same safe direction: the next run sees an older memory and reports a conflict, which writes nothing.

## Open Questions

Whether `pass`'s `--multiline` insert preserves a body's final newline byte for byte on the pinned version, or normalises it. The layout in D2 is unaffected either way — the field block is parsed from the end and a trailing newline is not part of it — but the byte-exactness claim for a value ending in a newline is a measurement, and `safix-pass-cli` is where it is made; task 7.6 records the measured answer in `pass.rs`'s module doc the way `store.rs:44-47` records keepassxc's.
