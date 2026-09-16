# Design: fields on the far side of a mapping, one judge, and one reserved-word list

## Context

Five facts in the tree fix the shape of this change, and none of them is negotiable here.

`keepassxc-cli` 2.7's `add` and `edit` take `-u/--username`, `--url` and `--notes` in the argument vector, have no tags flag, and have no custom-attribute write; `show` takes a repeatable `-a/--attributes` and a `-s/--show-protected`, read off `keepassxc-cli show --help` this session.
So the field channel on this transport is argv for three fields and nothing at all for the fourth, and `openspec/specs/keepassxc-sync/spec.md`'s "The database is a store being written, never a keyring being managed" states that "a secret value SHALL travel standard input or pipes, never an argument vector or an environment variable".
A field whose value is a literal in a declaration is not a secret value — the declaration is evaluated into the world-readable nix store — and a field resolved out of another safix entry is one.
That single distinction is what the whole capability apparatus exists to enforce, and it is why `FieldValue` has two shapes rather than one.

`Secret` (`crates/safix-core/src/secret.rs`) has no egress that yields `&str` or `&[u8]`; it can only be written to a stream.
A literal field therefore cannot be a `Secret`, because `write_arguments` has to put it in a `Vec<String>`; and an entry-sourced field has to be a `Secret`, because it is plaintext read out of sops.
The provenance split is forced by the type, not chosen for tidiness.

`Database::read` asks for exactly `--show-protected --attributes Password` and its doc comment (`store.rs:426-431`) states the reason: "a summary would print the title and the username beside it, which the report has no use for and which would then be on this pipe".
Widening that same invocation to carry the fields would put the value and the fields on one pipe and delete the property that comment records.

`crates/safix-core/src/model.rs:1-15` makes every model struct `#[serde(deny_unknown_fields)]` so that "adding an option to the nix half requires a matching field here, in the same change".
A `username` left on the nix side after `KdbxSide::username` is gone is a hard evaluation failure, and so is the reverse; there is no half-migrated state to land in.

`bridge.rs:805-838`'s `judge` and `sync.rs:451-493`'s `two_way` are the same four-way decision over two different far sides, and `bridge.rs:614-615` says so in prose.
They differ only in what they return: `sync::Outcome` has `Updated`/`Pulled`/`NotJudged`, and `bridge::bridge_sync::Outcome` has `UpdatedTowardClan`/`UpdatedTowardSafix` and no unjudged variant at all (`bridge.rs:658-671`).
A shared judge therefore cannot return either outcome type; it has to return a verdict each target words for itself.

## Goals / Non-Goals

**Goals:**

Give a mapping's far side a field declaration whose every outcome — written, diverged, refused — has a word in the report and a sentence in the refusals.
Make "safix quietly did less than the declaration said" unreachable, by refusing a field the target cannot carry at evaluation rather than dropping it at run time.
Keep a value and a field on separate pipes and separate invocations, so that widening the read cannot widen what a value read exposes.
Delete two duplications while adding the axis, rather than adding the axis twice.
Leave the three target changes a settled shape: one fields submodule, one reserved list, one endpoint contract, one judge.

**Non-Goals:**

A field on `safixSide`. Decision F2 records why the asymmetry is the design rather than an omission.
A per-field two-way memory. Decision F4 records that the alternative is unimplementable with one companion password and that push-only is the property, not the shortcut.
A generic per-target error hierarchy. Each target keeps its own refusal variants; only `FieldUnsupported` and `FieldSourceInArgv` are shared, and they are shared because they are parameterised by the target rather than specialised to one.
Deleting `Database` in favour of the trait. `Keepassxc` implements `Endpoint` over the `Database` it already has; a rewrite of the transport is not in this change.

## Decisions

### F1. `fieldValue` is either a literal or a named entry, and provenance decides the channel

`fieldValue = either str (submodule { options.entry = str; })`.
A bare string is a literal and may travel any channel, including argv, because an evaluated declaration is already world-readable in the nix store.
`{ entry = "<name>"; }` names another entry of the mapping's own user, is decrypted when the mapping is converged, and may travel only a pipe.

In Rust this is `FieldValue { Literal(String), Entry { entry: String } }` on the declaration side and `Field { Literal(String), Resolved(Secret) }` on the resolved side, with `ResolvedFields` carrying one `Option<Field>` per scalar field and a `Vec<Field>` for tags.
`Field::Resolved` holds a `Secret`, so nothing can put it in a `Vec<String>`; `Field::Literal` holds a `String`, so `write_arguments` can.
The type system is what enforces the channel rule, and `Error::FieldSourceInArgv` is what turns the resulting impossibility into a sentence before a run reaches it.

**Alternative rejected**: one `String` for both, with a boolean marking secrecy.
It makes the argv rule a runtime `if` that a later edit can forget, in a crate whose entire `Secret` design exists to make that class of forgetting impossible.

**Alternative rejected**: `{ entry = …; }` resolved at evaluation.
It would put a decrypted value into the nix store, which is the one thing the whole project is built not to do.

### F2. Fields live on the far side only

`kdbxSide.fields` exists; `safixSide.fields` does not, on any target, ever.
A safix entry is a placement — a file, a key inside it, and an audience (`model.rs`'s `Placement`) — and it has no slot for a URL, a note or a tag.
Inventing one would create a second authoring surface for the same information beside the declaration that already carries it, and then a pull would have to decide which of the two wins.

The consequence, stated as a property rather than discovered as a bug: a pull writes only the value into safix.
`sync.rs:591-598`'s `set::run_committing` call is unchanged by this entire change, which is the mechanical evidence.

**Alternative rejected**: fields on both sides with the mode deciding the author.
It doubles the surface to double the number of conflict cases, and the second surface has nowhere to live.

### F3. Convergence widens to "value differs or fields differ", and the fields read is its own invocation

`decide` (`sync.rs:378-443`) today compares values. It gains a second comparison, asked in this order and no other.

The value verdict is computed first, by the shared `judge` (F5).
When the verdict writes the target's side, the declared fields ride along in the same `write` — one `add` or one `edit`, one whole-file save.
When the verdict is `Unchanged` and the mode pushes, the declared fields are compared against the fields read back, and a difference upgrades the outcome to `FieldsUpdated` through a `write` that carries the same value the target already holds.
When the verdict is `Unchanged` and the mode does not push — `keepassxc-to-safix`, or `backup` over an entry that already exists — a field difference is reported as `FieldsDiverged` and nothing is written.

The fields read is a second invocation, `show -q -a UserName -a URL -a Notes` plus the composite-key arguments, issued only for a mapping that declares at least one field.
Three properties fall out, all of them worth more than the round trip they cost.
A mapping declaring no field issues exactly the argument vectors it issues today, so `store.rs`'s `the_read_asks_for_the_protected_password_attribute_and_nothing_else` stays true and unedited.
The fields invocation passes neither `--show-protected` nor `Password`, so the value cannot come back on it by construction rather than by care.
And the burst discipline is unharmed, because both reads happen in the decide phase, which is already entirely before the first write (`sync.rs:272-294`).

**Alternative rejected**: one widened read, `show -s -a Password -a UserName -a URL -a Notes`.
It saves one process per mapping and puts the value and the fields on one pipe, deleting the property `store.rs:426-431` was written to state.

**Alternative rejected**: no fields read at all, writing the declared fields on every push.
That is what `username` does today (`sync.rs:559`), and it is why a field drift on an otherwise-agreeing entry is never repaired and never reported.
It also makes `audit` unable to say anything about a field, which is the half of this change an operator actually reads.

### F4. A field has one author, so fields are push-only, and that is a property

A two-way mapping's value has two authors and a remembered agreement to tell an ordinary one-sided change from a conflict (`sync.rs:445-493`).
A field has exactly one author — the declaration — so there is nothing for a memory to arbitrate, and there is nowhere to put one anyway: the companion entry holds one `Password` (`keepassxc-sync`'s own amendment records that `Password` is the only protected field either transport can write).

So a two-way mapping's fields converge toward the declaration, always.
`backup` is the one mode where a push does not repair a field: `backup` never overwrites an existing entry, and writing its fields while refusing its value would make `backup` half a mode.
An existing `backup` entry whose fields differ is therefore `FieldsDiverged`, exactly as its differing value is `Conflict`.

**Alternative rejected**: a second companion entry holding a per-field agreement.
Three entries per mapping, a second reserved suffix, and a whole-file save per field, to arbitrate an author that does not exist.

### F5. One `judge`, one `Verdict`, and each target words its own outcome

`endpoint::judge(ours: Option<&Secret>, theirs: Option<&Secret>, remembered: Option<&Secret>, agrees: &dyn Fn(&Secret, &Secret) -> bool) -> Verdict` where `Verdict { Unchanged, Conflict, PushValue { remember: bool }, PullValue { remember: bool } }`.
The `agrees` closure is passed rather than derived, because the two targets record their agreement under different format tags — `crate::sync::FORMAT` is `safix-sync-v1` and `crate::bridge::bridge_sync::FORMAT` is `safix-bridge-sync-v1`, deliberately distinct so that neither mechanism can read the other's memory (`bridge.rs:633-639`) — and a shared judge that built the memory line itself would have to know which target it was judging for, which is the coupling it exists to remove.
`sync::decide` maps a `Verdict` onto `sync::Outcome`; `bridge_sync::decide` maps the same `Verdict` onto its own five classes.
`sync.rs:451-493`'s `two_way` and `bridge.rs:800-835`'s `judge` are deleted, not wrapped.

`judge` takes references and returns a verdict rather than taking values and returning a `Decision` carrying them, because the two callers own their values differently: `sync`'s `Decision::Pull` defers to a second phase (`sync.rs:288-292`) and `bridge_sync`'s does not.
A verdict is the whole of what is shared; moving the values through the shared function would force one caller's ownership on the other.

`selected` and `lingering` are deliberately left duplicated.
`bridge::selected` refuses `MappingWrongDirection` and `sync::selected` does not, and their refusal variants name different declared lists (`bridge.rs:382-414` against `sync.rs:339-352`); unifying them means one generic function with a per-target refusal closure and a per-target mapping trait, which is more apparatus than the six lines it deletes.
Recorded as a decision so that its absence is not read as an oversight: what `Endpoint` exists to delete here is the judge, and the judge is what it deletes.

**Alternative rejected**: `judge` returning each target's own `Outcome` behind a generic parameter.
The outcome types are not merely differently spelled — `bridge_sync::Outcome` has no `NotJudged` at all — so the generic bound would have to be "can express five classes or six", which is a bound with one purpose and no meaning.

### F6. Capabilities are declared in both halves, and refused in both halves

`Capabilities` names, per field, one of three channels: `Argv`, `Stdin`, `Unsupported`; plus `multiline: bool` for the value itself.
keepassxc declares `username`, `url` and `notes` as `Argv`, `tags` as `Unsupported`, and `multiline: false`.

The same table is written in `modules/flake/safix/fields.nix` as a nix attribute set and consumed by each target's `violationsOf`, producing two message rules: a declared field whose channel is `Unsupported`, and an `{ entry = … }` source for a field whose channel is `Argv`.
The Rust half carries `Error::FieldUnsupported { target, field }` and `Error::FieldSourceInArgv { target, field }` for the same two facts.

This doubling is the one `model.rs:875-885` already documents for `Mode::pulls`: "evaluation refuses the declaration, and the runtime decides which half of a converging run may write".
It earns its keep twice over here.
The nix half is what an operator meets — a message, at evaluation, before a database is opened, asserted against a literal in `modules/flake/checks/keepassxc.nix` — and it is the only half reachable for a declaration that never runs.
The Rust half is what a projection reaches: `safix --entry` and an embedder both hand the runtime a `Keepassxc` record that no `violationsOf` ever saw, and a run that wrote three of four declared fields and said nothing would be exactly the silent-shortfall failure this change exists to end.

`multiline: false` is where `ValueSpansLines` now comes from, rather than from a hard-coded branch in `Database::write` (`store.rs:256-260`).
The refusal, its sentence and its code are unchanged and stay keepassxc's; what changes is that the reason is a declared capability of this transport instead of an unexplained special case, which is what lets `pass`, `bw` and `op` declare `multiline: true` without touching the refusal.

**Alternative rejected**: the nix half only.
It cannot see a projected record, and `--entry` is a supported way to run every verb.

**Alternative rejected**: the Rust half only.
It moves the refusal from evaluation to the middle of a run, after the password prompt, for a mistake that is entirely local to the declaration — which is the disposition `keepassxc.nix:140-144` and every other `violationsOf` in the tree exist to reject.

### F7. `username` is migrated, not supplemented

`kdbx.username` is deleted in the same change that adds `kdbx.fields.username`.
`deny_unknown_fields` means the two cannot coexist accidentally, and one meaning with two spellings is what this repository refuses on sight.

The migration note goes in `CHANGELOG.md` under `[Unreleased]`, naming the old option, the new one, and the fact that the value is unchanged.
No compatibility shim, no alias, no deprecation window: a consumer's evaluation fails with the module system's own "unknown option" sentence naming `kdbx.username`, which is a better error than any shim would produce.

**Alternative rejected**: accept both for one release.
The shim's own refusal — what happens when both are declared — is a third case to specify, test and document, for a field whose only consumer is a declaration a person edits once.

### F8. The reserved words become one list, complete, now

`modules/flake/safix/reserved.nix` exports `ids = [ "clan" "keepassxc" "pass" "bitwarden" "1password" "all" ]`, and `bridge.rs:89`'s `RESERVED_MAPPING_WORDS` becomes `[&str; 6]` with the same six words in the same order.
`modules/flake/checks/reserved-words.nix` renders the nix list and extracts the Rust literal from `crates/safix-core/src/bridge.rs`, and fails on any difference, modelled on `safix-installer-schema`'s diff-against-a-committed-artifact shape (`modules/flake/checks/installer.nix:1225-1257`).

All six land now, before three of the targets exist.
The alternative is that each target change appends one word and moves the same check, which is three opportunities for the list and the literal to disagree and three reviews of the same diff.
Reserving a word for a target that does not answer yet costs a consumer nothing but the name, and `sync pass` refusing with "no such target" rather than converging a mapping called `pass` is the better of the two confusions.

Each module keeps its own sentence rather than importing one, because each sentence names its own option path (`flake.safix.bridge.mappings.<id>` against `flake.safix.keepassxc.mappings.<id>`); what is shared is the list, which is the part that was duplicated.

`op` is not reserved. One spelling per target is D3's rule, `1password` is the spelling, and reserving a two-letter word an operator might reasonably name a mapping buys nothing.

**Alternative rejected**: exporting a message builder as well.
It would parameterise two sentences by an option path to save one line each, and put the wording of a refusal a check asserts against a literal one indirection away from the module it belongs to.

### F9. `Endpoint` is introduced now, with keepassxc as its first and only implementation

`crates/safix-core/src/endpoint.rs` carries `Record`, `Existing`, `Channel`, `Field`, `ResolvedFields`, `Capabilities`, `Verdict`, `judge`, `resolve_fields`, and `trait Endpoint`.
`Keepassxc` (a new thin struct over `Database` in `store.rs`) implements it: `unlock` performs the terminal check and `Database::open`, `read` performs the value read and, when the mapping declares a field, the fields read, `write` performs the burst-safe single write, `list` is `under(group)`, and `capabilities` is the table from F6.

Introducing the trait with one implementation is defensible here for exactly one reason: it arrives with the deletion of the duplication it exists to prevent, rather than in anticipation of a fourth target.
`judge` is written once in this change and read by two callers in this change.
The trait itself is what the three target changes implement instead of copying `sync.rs`, which is the whole economy of landing this first.

`resolve_fields(workspace, user, &Fields, &Capabilities, target) -> Result<ResolvedFields>` performs F6's two refusals and, for a `Stdin`-channel field declared as `{ entry = … }`, reads it through `bridge::held_by_safix` — the same read `sync`'s own safix side takes.
On keepassxc every `Stdin` arm is unreachable, because keepassxc declares no `Stdin` field; the arm is complete and unit-tested against a fixture `Capabilities` whose channels are all `Stdin`, and its first production caller is `add-pass-bridge`.
Writing the refusal without the resolution would make the refusal a statement about nothing.

**Alternative rejected**: introduce `Endpoint` with the third target, per `local://design-1.md`'s recommendation 5.
Overridden by the settled decision D2, and the override is the right way round: the fields axis is what would otherwise be written into `sync.rs` and `bridge.rs` separately and then into three more files, and one judge over `Record`s is the thing that stops that.

### F10. A field divergence is its own word, names the field, and moves the exit status

`sync::Outcome` gains `FieldsUpdated` ("the value already agreed; the declared fields were written") and `FieldsDiverged(Vec<&'static str>)` ("the declared fields differ and this mode does not write them").
`audit::KeepassxcOutcome` gains `FieldsDiverged(Vec<&'static str>)`.
`Tally` gains `fields_updated` and `fields_diverged`.

Three properties, each stated because each has a wrong answer that looks reasonable.
A value divergence dominates: a mapping whose value and fields both differ is `Conflict` or `Updated`, never `FieldsDiverged`, because the field is repaired by the same write and reporting the lesser fact would bury the greater one.
The report names the field and never its content: `notes` may carry a sentence about where a credential came from, and a `{ entry = … }`-sourced field is a secret, so `Vec<&'static str>` holds `"username"`, `"url"`, `"notes"`, `"tags"` — `&'static str`, so no run-time string can reach it.
And `FieldsDiverged` fails the run, on the same footing as `Diverged`: a declared field that is not there is a declaration that is not true, and `audit`'s whole purpose is to answer whether the declarations are true.

**Alternative rejected**: reporting a field divergence as `Diverged`.
Its remedy line and its exit status are a secret divergence's, and a URL drift failing CI in the same word as a diverged credential makes the word useless.

**Alternative rejected**: `FieldsDiverged` as information, like `lingering`.
`lingering` is information because no declaration claims the entry; a declared field is claimed by a declaration by definition.

## Risks / Trade-offs

**A second process per mapping that declares a field.**
A fields read is one more `keepassxc-cli show` per mapping, in the decide phase.
Bounded by the mapping count, off the write path entirely, and paid only by a mapping that declares a field.
The alternative that avoids it (F3's rejected one-read form) costs the value read's narrowness, which is worth more.

**`tags` is declarable and refused on the one target this change ships.**
An operator reading `fields.nix` sees four fields and finds that one of them is refused everywhere they can currently write.
Accepted rather than worked around: the submodule is shared by five targets, three of which carry tags, and a per-target submodule would be five submodules differing by one option each.
The refusal names the target and the field and arrives at evaluation, which is the best available version of this.

**The `Stdin` field-resolution arm has no production caller in this change.**
Recorded rather than hidden, with its unit test and its first caller named (F9).
The alternative is shipping a refusal whose subject does not exist.

**Six reserved words before three of the targets exist.**
A consumer with a mapping named `pass` is broken by a change that does not mention pass.
Accepted, stated as BREAKING in the proposal, and cheaper than three changes each moving the same equality check.

**The endpoint trait has one implementation until `add-pass-bridge` lands.**
If the three target changes were abandoned, this change would leave a trait serving one type — but it would still leave the judge deduplicated, the fields axis written once, and the reserved list in one place, all three of which stand on their own.
