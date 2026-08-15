# Design: a bridge that declares, delegates, and converges

## What was read

clan-core at `/nix/store/skwb0795vlb7ymhl8zkc9cdx2cm3mf9d-source`:

- `clan_cli/vars/get.py` and `clan_lib/vars/get.py` — the read path.
- `clan_cli/vars/set.py` and `clan_lib/vars/set.py` — the write path.
- `clan_lib/vars/export_vars.py` and `import_vars.py` — clan's own bulk dump and restore.
- `clan_lib/vars/generator.py` — placement, share, and the validation hash.

And in the dotfiles repository: `modules/clan/vars.nix` (the backend this fleet actually uses), `modules/flake/agents/agents.sh` (the prototype being replaced), and `docs/notes/architecture/clan-vars-sops-agenix-bridge.md` (an adversarially-reviewed prior investigation of exactly this question).

Four facts from that reading govern the design.

**One.** `clan vars get <machine> <generator>/<file>` writes the value to standard output. It writes raw bytes when standard output is not a terminal and a *printable* rendering when it is. A subprocess capture is never a terminal, so safix gets raw bytes — but this must be asserted rather than assumed, because the printable branch would silently substitute a rendered form for the value.

**Two.** `clan vars set <machine> <generator>/<file>` reads the value from standard input. It is the whole CLI surface for writing one var, and it commits what it wrote.

**Three.** This fleet's clan uses `secretStore = "age"`, set at `modules/clan/vars.nix:80`. Reading clan's store directly means implementing clan's age backend, not reading a sops file.

**Four.** clan's own `export_vars` and `import_vars` are a bulk dump to a directory and a restore from one — a backup mechanism, not a bridge to another tool. They are not reusable here, but `import_vars` contains one thing worth stealing, discussed in D5.

## D1. Both directions delegate to the clan CLI

The requirement "clan stays the authority on its own store" is a symmetric statement.
It is not "clan owns writes"; it is "clan owns its store".
Reading a store by reimplementing its layout is a claim of authority over that layout, and the claim breaks the moment the layout moves.

The concrete cost of the alternative is not hypothetical for this fleet.
There is no sops file to read on the clan side; there are age-encrypted vars under clan's own directory scheme with recipient sidecars.
Implementing that inside safix would mean safix carries a second decryption backend, for a store it does not own, whose layout is versioned by someone else, and whose *other* possible backends (`password-store`, and whatever clan adds) it would silently not support.

Delegating gives:

- backend independence — the bridge works over sops, age, password-store, and anything clan adds, with no code in safix;
- one code path per direction rather than one per backend per direction;
- a pipe on both legs, since `get` writes to stdout and `set` reads from stdin, which is worth noting after `clan-generator-contract` made staging files necessary elsewhere;
- and a refusal surface that is clan's own, so a missing var, an ambiguous id, or an ungenerated value produces clan's message rather than safix's guess at one.

The recommendation is therefore symmetric delegation, and it deviates from the brief in the import direction.

**Decided: symmetric delegation.** See "The three decisions" below. The committed spec's prohibition — the runtime reads, writes, decrypts, encrypts and parses none of clan's stored files — stands unqualified and covers the import direction too.

### Refusing when clan is absent

If the clan CLI is not on PATH, both verbs refuse before doing anything, and the refusal says why: clan is the authority on its own store and safix will not read or write it directly.
This is deliberately not a soft failure that skips clan-side mappings — a bridge run that quietly does half its mappings is worse than one that does none, because the report would say "unchanged" for the half it never looked at.

## D2. The mapping is a declaration

The alternative is CLI arguments: `safix import --machine sundog --generator ntfy --file token --user sernl --name ntfy-token`.
That is rejected because a bridge is a standing relationship, not an event.

A declaration is diffable, so adding a mapping shows up in review as a line naming both endpoints.
It is repeatable, so a run has no arguments to get wrong and no operator has to remember the pairs.
It is checkable, so evaluation refuses a mapping whose safix side does not exist.
And it is enumerable, so `safix check` can report the whole bridge's state without being told what the bridge is.

The shape:

```nix
flake.safix.bridge.clanFlake = ./.;

flake.safix.bridge.mappings.ntfy-token = {
  direction = "clan-to-safix";
  clan = { machine = "sundog"; generator = "ntfy"; file = "token"; };
  safix = { user = "sernl"; name = "ntfy-token"; };
};
```

`clanFlake` is declared once per consumer rather than per mapping. A consumer with two clans is not a case this supports, and the refusal says so rather than silently taking the first.

The mapping's attribute name is an identifier for the mapping itself — it appears in reports, in commit messages, and in refusals. It is not derived from either side, because deriving it from one side would make a report about the other side read wrongly.

### Direction is absolute

`direction` takes `clan-to-safix` or `safix-to-clan`, not `import` or `export`.

The reason is a genuine ambiguity rather than pedantry.
`clan vars export <dir>` moves values *out of* clan. `safix export` moves values *into* clan.
Both words are correct relative to the tool that says them, and a reader of a declaration does not have a tool in hand to be relative to.
Writing the endpoints removes the question.

The verbs stay `import` and `export` because they are safix's verbs on safix's command line, where safix is the relative frame and every other verb already assumes it.

## D3. What evaluation can refuse, and what it cannot

Half of every mapping lives in another flake. This asymmetry is stated rather than papered over.

Evaluation refuses, using the resolver machinery custody and generator checks already use:

- a safix side naming a user who does not exist, or a name that user does not carry;
- a `clan-to-safix` mapping whose safix target is also produced by a generator — two producers for one value, refused by the same rule that already refuses two generators naming one output, because the winner is whichever ran last;
- a `safix-to-clan` mapping whose safix source has neither a generator nor a declared value path, so there is nothing to export;
- two mappings writing the same target;
- the same clan-side triple and safix-side pair appearing in two mappings with opposite directions — which is a two-way sync spelled as two declarations, and a two-way sync with no conflict resolution is the shape the prior dotfiles investigation already refuted;
- a `direction` outside the two permitted values;
- more than one `clanFlake`.

Evaluation cannot refuse a clan side that does not exist. That is checked at run time, on the first transfer, by asking clan — and the refusal names the machine, the generator and the file, so a typo is a sentence rather than an empty value.

The messages go into `checks.nix`'s message-function-plus-builder split, as the other families do, so a consumer's fixture can assert a message against a literal and a severity drill can run the same `refuseScript` bytes the real check runs.

## D4. Convergence is a comparison, not a write

Both verbs read both sides before writing either.

For `clan-to-safix`: read the clan value through `clan vars get`, read the safix value through the existing decrypt path, compare bytes. Equal means nothing is written and nothing is committed.

For `safix-to-clan`: the same comparison in the same order, and the comparison is *load-bearing* rather than an optimisation.
`clan vars set` writes unconditionally and commits what it wrote; clan's age backend re-encrypts on every write, producing fresh ciphertext for an unchanged value.
Without the read-first comparison, every `safix export` run would produce a commit in the clan repository for every mapping, forever.
That is the finding that makes convergence a requirement rather than a nicety.

A mapping whose safix side cannot be decrypted by the operator running the command is refused rather than written.
The reasoning is the one `check` already uses for other people's files: writing a value the operator cannot read means writing a value they cannot verify, and on the export side it would mean pushing an unverifiable value into another store.

The report is per mapping and says which of four things happened: unchanged, updated, absent at source, or refused with the reason.
"Absent at source" is a distinct outcome rather than an error because a clan var that has not been generated yet is a normal state during bootstrap, and a bridge run during bootstrap should say so and continue.

Idempotency is asserted rather than claimed: an integration test runs each verb twice and requires the second run to write nothing and commit nothing.

## D5. Commits are single-intent, and the validation hash is a known hazard

Each transferred mapping produces its own commit, naming the mapping and never the value, exactly as `set` commits per secret today.
A bulk commit is rejected on two grounds: its message cannot say what it did without naming values, and reverting one mapping should not revert eleven others.

`--all` runs every mapping of a direction and still commits per mapping.

### The validation hash

clan records a `validationHash` per generator and regenerates when the recorded hash does not match the definition's.
`clan vars set` does not update it — read `clan_lib/vars/set.py`, which commits the var's paths and nothing else.
clan's own `import_vars` *does* restore it, and its comment says exactly why: "otherwise the generator counts as outdated and the next 'clan vars generate' overwrites what we just imported."

For `safix export` the consequence is precise and needs stating rather than guessing at:

- A routine `clan vars generate` after an export does not overwrite the exported value, because the recorded hash still matches the definition and the generator is not considered outdated.
- `clan vars generate --regenerate` does overwrite it. That is an explicit operator action and is out of safix's hands.
- A change to the clan-side generator's *definition* invalidates the hash, and the next routine `clan vars generate` then regenerates and silently discards the exported value.

The third is the real hazard: it is silent, and it is triggered by editing a nix file rather than by running a command.

Three responses. Two are taken and one is refused.

Taken: `safix export` refuses the mapping outright when clan already considers the generator's recorded validation stale, before writing anything. This is the decision recorded in "The three decisions" below, and it converts a silent later loss into a refusal at the moment the operator asks for the write.

Taken: `safix check` reports an export mapping whose clan-side value no longer matches the safix-side value, which catches a loss that happened between two runs and names it. This costs one `clan vars get` per export mapping and is the same comparison the transfer already does. It remains necessary after the refusal, because a definition can change *after* a successful export.

Not taken: writing clan's validation hash from safix. It would prevent the loss, and it is refused because it means writing clan's store directly — the one thing this change exists to avoid — and because the hash it would need to write is a function of clan's definition, which safix would then be computing.

Whether clan should grow a "this var is externally supplied" concept, and whether that is worth raising upstream, is recorded as a question rather than answered.

### How the comparison is made without reading clan's store

The recorded validation hash is a file in clan's store — `VALIDATION_HASH_NAME` under the generator's directory, written by `StoreBase.set_validation` and read by `StoreBase.get_validation` in `clan_lib/vars/_types.py`. Reading it directly is exactly what D1 forbids, so the comparison is delegated like every other clan-side read.

`clan vars check <machine> --generator <generator>` is the surface. `clan_lib/vars/check.py` runs the comparison safix would otherwise have to run — `hash_is_valid(generator.key, generator.validation())` against both stores, where `validation()` is the `validationHash` nix exported for that generator's definition — and reports a generator that fails it under "outdated invalidation hash", at the default log level, on standard error.

So safix computes no hash, reads no hash, and writes no hash. It asks clan the question clan already answers for itself, and refuses when the answer is that this generator is stale.

The coupling is to clan's wording, and it is the same coupling `clan.rs` already carries for the ungenerated-var and unknown-var lines, recorded there with the same reasoning: the alternative is treating clan's exit status alone as the answer, and that status is also non-zero for a var that has not been generated yet — which is the ordinary state of a var about to be exported into for the first time, and not a refusal.

## D6. The dotfiles mirror becomes a consumer

`modules/flake/agents/agents.sh` currently moves service tokens from clan vars into `secrets/safix/users/sernl/secrets.yaml`, using its own `sops set --value-stdin` write, its own secret-tempfile registry and shredder, its own trunk-branch guard, and a hardcoded `MIRROR_SOPS_KEY` table for the mapping.

Under this change every one of those is redundant: the mapping is a declaration, the write is `safix import`, the shredder and the guard are safix's, and the tempfile registry is `scratch.rs`.
What remains of `agents.sh` is provisioning the tokens on the remote host, which is genuinely its own job.

That work is a dotfiles follow-up change, `retire-agents-mirror`, named here and deliberately not built here — it depends on this change having landed and on `safix-full-switch` having settled the dotfiles secret vocabulary, and folding it in would make this change span two repositories.

## Testing without a clan

The integration suite drives a stub clan CLI whose behaviour is asserted, not assumed: it answers `vars get` with known bytes, records what `vars set` received on standard input, and can be made to fail in each way the real one does.
The stub is a fixture, and the same reasoning that forbids stubbing sops does not apply — sops is the thing safix's claims are *about*, whereas clan is a boundary safix delegates across, and what is being tested is the delegation.

One further check drives the real clan CLI over a throwaway clan if it is present in the check closure, and is absent rather than trivially green when it is not — the same shape the linux-only syscall check uses.

## What the real clan confirmed

Every contract above was read out of clan-cli's source. Before the transfer verbs were written, each was also confirmed against the real `clan` command driven over a miniature clan built with that command — a machine, one `age`-backed generator, a generated identity and a recipient. What that established, and could not have been established by reading alone:

- `clan vars get <machine> <generator>/<file> --flake .` writes the raw value to a pipe with no trailing byte added. The fixture value came back as exactly its own bytes.
- `clan vars set` fed on standard input succeeds and *commits in clan's repository*, naming the files it wrote: the var's `.age` file and its `.recipients` sidecar. This is the fact D4's comparison is load-bearing because of.
- A var whose generator is declared but which holds nothing answers `Var <id> has not been generated yet`; an id nothing declares answers `Couldn't find var: <id> for machine: <machine>`. The two are different states with different remedies, and every first export writes into the first — so a runtime that treated them alike would refuse every first export.
- `clan vars check <machine> --generator <g>` exits zero with "All vars are present and valid." while the recorded validation matches, and after the generator's definition changes exits non-zero with `Generator '<g>' in machine <machine> has outdated invalidation hash.` on standard error at the default log level. This is the surface decision two rests on.
- With the definition changed, `clan vars get` still returns the old value. That is the hazard stated precisely: nothing observable has gone wrong yet, and the loss happens at the next routine generation.
- clan's store is `secrets/clan-vars/per-machine/<machine>/<generator>/<file>/<file>.age` with a `.recipients` sidecar beside it — the layout decision one exists in order not to implement.

The suite drives a stub rather than this, and `crates/safix/tests/support/clan-stub.rs` states why: what is under test is the delegation, and a stub can be asked what it was handed. The stub's behaviour is written against the findings above rather than against a reading of the source, which is the point of having made them.

Landing this as a check is task 5.2 and is not done: the miniature clan needs a locked flake, a generated identity and a declared recipient, none of which a build sandbox has.

## The three decisions

The two questions that gated stage 3 are answered, and one refusal the spec carried is replaced. All three are recorded here as the resolved decision rather than as a recommendation.

### One. Symmetric delegation

Both directions reach clan's store only through clan's own command, and the runtime reads, writes, decrypts, encrypts and parses none of clan's stored files.

Every clan-side read is `clan vars get` run as a subprocess with its standard output captured on a pipe, and the capture is asserted to be the raw-value path rather than assumed to be: `clan_cli/vars/get.py` branches on `sys.stdout.isatty()` and prints `var.printable_value` on the terminal branch, so a `get` that inherited a terminal would hand back a rendering in place of the value. Every clan-side write is `clan vars set` with the value on standard input and nowhere else.

This settles the deviation D1 argued for. The brief's asymmetry — import decrypting clan material with the operator's admin identity — is not taken, and the committed spec's prohibition stands unqualified in both directions. The consequence accepted with it is that a consumer without clan-cli cannot import either: both verbs refuse before touching any mapping, with a named refusal saying that clan is the authority on its own store.

What this buys beyond principle is that the bridge is backend-agnostic. It works over `age`, which is what this fleet sets at `modules/clan/vars.nix:80` in dotfiles, over `sops`, over `password-store`, and over whatever clan adds, with no code in safix and no second decryption path to hold correct for a store safix does not own.

### Two. Refuse on definition drift

`safix export` refuses a mapping whose clan-side generator clan already considers stale, rather than writing a value clan's next routine generation would silently discard.

The refusal carries its own code and its own message, and the message names both remedies: update the clan-side definition so the recorded validation matches it again, or declare the mapping `clan-to-safix` instead, which is the right shape when clan's generator is the producer.

There is no override flag in 0.2. A flag would be the third state — "export anyway, knowing the value is scheduled for replacement" — and 0.2 has no way to record that intent anywhere the next `clan vars generate` would read it, so the flag would amount to a switch that turns a refusal into a silent loss. Whether 0.2 was right to omit it is a question the audit's findings will answer with evidence rather than one to settle now.

The comparison itself is delegated, for the reason given at the end of D5: the recorded hash lives in clan's store, and reading it would break decision one to enforce decision two. Nothing safix runs writes clan's validation record.

### Three. The eval refusal with no referent, replaced by its runtime sibling

The bridge-surface spec required evaluation to refuse "a `safix-to-clan` mapping whose source entry has neither a generator nor a declared value". That requirement is deleted, because at evaluation it has no referent.

A safix entry is a declaration that a name is held, a file it lives in, and a key inside that file. It is not a declaration that the key holds anything. An entry with no generator is exactly the hand-set case — `safix set` writes it, and it is the ordinary thing to export — so "has no declared value" describes every hand-set entry before its first write and none of them after, and nothing at evaluation can tell the two apart. Refusing on it would refuse the ordinary export.

`modules/flake/checks/bridge.nix` already asserts this: `handSetExportMessages` holds a `safix-to-clan` mapping over `ana`'s hand-set `tok` and expects no message. That assertion stays true and is correct.

What replaces it is a runtime refusal with a real referent: export refuses when the source key is absent from the source sops file, naming `safix set` and `safix generate` as the two remedies. The question "does this entry hold a value" is answerable exactly once, at the moment something tries to read it, and that is where the refusal now lives.

The two are siblings rather than a move: the eval check asserts that the hand-set export produces no evaluation message, and the runtime check asserts that the same mapping over an unwritten entry refuses when a transfer reaches it. Neither is redundant, and the first would be vacuous without the second.

## Open questions for the operator

1. Whether the ordering constraint is acceptable: this change depends on `clan-generator-contract` for `share` agreement between the two systems, so it lands after it.
2. Whether clan should grow a "this var is externally supplied" concept, so that an exported value survives a definition change rather than requiring the refusal decision two installs. This is an upstream question and is not safix's to answer.
