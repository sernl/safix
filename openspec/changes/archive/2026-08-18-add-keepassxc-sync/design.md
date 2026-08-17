## Context

See proposal.md — Why.
Measured facts (research 2026-08-17, keepassxc-cli 2.7.12 probed empirically):

- `keepassxc-cli add`/`edit`/`show -s`/`db-create` work non-interactively with the password on stdin; every db-opening command takes `-y slot[:serial]`. No CLI verb enrolls or changes database keys — GUI-only, and sync never wants it.
- Measured again during apply, against scratch databases created for the purpose, and each of these constrains what sync can promise:
  - The writable per-entry surface is `--title`, `--username`, `--url`, `--notes` and `--password-prompt`. There is no custom-attribute write, protected or otherwise; `show -s -a <name>` reads one. See D2's amendment.
  - The entry password is line-oriented in both directions. `add -p` and `edit -p` read one line, so a value carrying a newline cannot be stored — what lands is the bytes before it — and `show -s -a Password` appends a newline of its own. Removing exactly one on the way back is exact whatever the entry holds. See D7.
  - A group must exist before an entry can be added under it, and `mkdir` creates one level: `mkdir <db> a/b` on a database with no `a` refuses, and so does creating a group that is already there.
  - A wrong database password and an absent entry both exit non-zero with nothing distinguishing them in the status, so the database is opened once with `ls -R -f` and absence is answered from that listing rather than from the tool's wording. The listing marks groups with a trailing slash and prints `[empty]` for a database holding nothing.
- The fleet's database is 292 MB, synchronized two-way on a minutely timer, saved after every change; each entry write is one whole-file rewrite and re-upload.
- The Secret Service exposure boundary travels inside the kdbx, so a session with the database unlocked can be read and written over DBus with no password handling.
- The model for the modes is the fleet's own Filen declaration: sync pairs each carry a mode — two-way, one-way in either direction, backup — and the operator asked for the same vocabulary here.
- safix's resolver answers the read side; `safix_core::set::run` takes `&mut dyn ValueSource` and is terminal-free, so a pull is an in-process source feeding the ordinary write path — commits, refusals and all. The bridge's transfer/report machinery is the established shape for a converge-and-report verb, and its `twoProducers` refusal is the established answer to a second producer.

## Goals / Non-Goals

Goals: one verb that ends drift in the declared direction per mapping; naming and mode in code; write cost bounded by convergence; pulls indistinguishable from hand-set writes.

Non-goals:
- Deletion propagation, in any mode. A removed mapping stops being synced; its last database value stays until a person removes it, reported as no longer declared. Filen's mirror modes do propagate deletions; that is the one part of the model deliberately not taken, because an accidental deletion of a secret is not a state a sync should be able to reach.
- Database lifecycle: no `db-create`, no key changes, no group deletion.
- Conflict auto-resolution. Two-way's both-changed case is a finding with two named remedies, never a heuristic winner: last-writer-wins over secrets rewards whichever clock lied best.
- Syncing into the exposed group as a publication mechanism. The exposure boundary is the database's own setting; sync writes where the declaration says.

## Decisions

### D1. Modes are per mapping, endpoint-named, with Filen's semantics minus deletion

`safix-to-keepassxc` and `keepassxc-to-safix` are Filen's one-way pairs; `two-way` is its twoWay; `backup` is its backup shape — write where absent, never overwrite, report divergence.
Endpoint naming rather than push/pull follows the bridge's direction decision: a declaration is read with no tool in hand to be relative to.
The mode is declared, not passed: a remembered flag on a verb is exactly the drifting operational knowledge the declaration model exists to end.

### D2. Two-way's memory lives inside the encrypted store, and that is a security decision

Three-way convergence needs the last agreed state.
A committed digest of a secret value — in `state/safix/` or anywhere in the repository — is an oracle: anyone holding the tree can confirm a guessed value offline.
So the last-synced state lives inside the encrypted database, updated in the write that converges the entry; the repository carries no value-derived state, and the property is spec-held.
Consequence accepted: losing the database loses the memory, and every two-way mapping then reports as if newly bootstrapped — safe, because bootstrap semantics write only where one side is empty and report everything else.

Amended during apply: the memory is a protected field of a reserved companion entry beside the mapped one, rather than a protected custom attribute of the mapped entry.
The measurement is D0's: `keepassxc-cli` 2.7.12 has no custom-attribute write on any verb, so the attribute mechanism was reachable through the secret service and not through the command — a mechanism that exists under one transport and not the other, which a requirement must not be.
`Password` is the one protected field both surfaces can write, so the companion carries the digest as its own password, at the mapped entry's path plus `.safix-sync-state`.
That suffix is structurally reserved: the companion of a declared path is that path plus the suffix, and evaluation refuses a declared path carrying it, so no admissible declaration can name any companion.
Every normative sentence above is unchanged — the state is inside the encrypted database, the repository carries nothing value-derived, and deleting the memory converts the mapping to bootstrap semantics.
Two alternatives were rejected: the entry's `notes` field, which is writable and on the entry but is not protected and would destroy whatever a person wrote there; and an attachment, which would put the digest in a file on disk on its way in.

The memory is written only as part of a converging write, and after the value it is about.
Writing it first loses data: it would record an agreement on a value only one side holds, and the next run would read the side holding the new value as the one that had not changed and converge the other way, overwriting the new value with the old.
Writing it on its own — to refresh a memory that is absent or stale while the two sides agree — is refused by the spec's own sentence that agreement writes nothing anywhere, so a two-way mapping whose sides agreed before safix ever ran has no memory and its first divergence is a conflict rather than a guess.
A run interrupted between the two writes lands in the same state. Both are the safe direction.

### D3. A pull is the ordinary write path wearing a different source

`keepassxc-to-safix` and the pulling half of `two-way` feed the database's value to `safix_core::set::run` through an in-process `ValueSource`.
Everything the write path does happens: the empty-value refusal, the recipient-drift refusal, the staged write, the rename, the commit naming the mapping and never the value.
This is the same seam `settle-clan-vars-parity`'s stream source uses at the CLI layer; the two changes share it rather than each building a second write path.

### D4. The verb reads through the resolver and reports like the audit

The safix side is the ordinary resolver read, the kdbx side is `show -s` (or the DBus read), the comparison is byte equality, and the output is findings-as-data rendered in the CLI — the audit's shape, extended with `pulled` and `conflict` outcomes.
A mapping the runner cannot decrypt is reported as not judged, never skipped.

### D5. Writes batch behind the comparison, and the 292 MB cost is bounded by convergence

Only differing mappings write; steady state writes nothing; consecutive writes in one run are issued as one burst.
The real bound remains the second-small-database decision, which is the operator's dotfiles question and lands here as nothing more than a different `database` path.

### D6. Attribute mapping is fixed and small

The safix value is the entry's password; the declaration may set a username per mapping; the title is the last path segment; the two-way state attribute is safix's own reserved name.
Arbitrary attribute templating was rejected: every added field is a field the report and refusals must speak about, and nothing asked for more.

### D7. One transport, because the other cannot address what a mapping declares

Amended during apply. The original decision had sync reach the database through the session's secret service when it is unlocked and through the store's own command otherwise, and the convenience claimed for the first was "no password handling at all".
That is withdrawn rather than softened.

The Secret Service collection KeePassXC publishes *is* its exposed group: an item found through the service is an entry in whatever group the operator's exposure setting names, and an item created through it lands there.
So the service cannot address `<group>/<path>`, which is the thing a mapping declares.
Two consequences, and the second is a correctness fault rather than a limitation.
Two transports addressing different entries would make a mapping's convergence depend on which one ran.
And a service read of an entry in a group the operator has not exposed returns nothing, indistinguishable from "the database holds no value here" — so a `backup` mapping would read absence, write, and land a secret in a group no declaration named, which is an outcome the report has no way to state and therefore an outcome sync may not have.

So sync's store path is `keepassxc-cli` alone.
What replaces the convenience: one password prompt per run, held for the run, with the cost bounded by the same convergence that bounds the rewrite — a run over agreeing mappings opens the database, reads, and writes nothing — and by the operator's own second-small-database option, which lands here as a different `database` path and nothing else.
Without a terminal to ask on, sync refuses before reading any secret.

`enroll --mirror-to-store` keeps both transports, and the asymmetry is not an inconsistency.
Its entry is safix's own, addressed by an attribute rather than by a path, so the exposed group is the right home for it and the service is the better of the two there: it prompts for nothing.
Sync addresses a group and a path the consumer chose, which the service structurally cannot.

Recorded as a future option and deliberately not built: the database open could take the store's other key factors — `-y slot[:serial]`, a key file — through a declaration field, should a prompt-free flow be wanted.
Every db-opening verb already accepts them. It changes nothing normative here.

## Risks / Trade-offs

- [Two writers on the kdbx file (sync verb and the Filen client)] → KeePassXC's save is atomic per write; the burst discipline keeps the window one save wide; named, not solved, exactly as every GUI edit already bears it.
- [A two-way pull can import a weaker value into safix] → it arrives through the full write path and its commit names the mapping, so review sees it like any hand-set change; the generator-produced refusal keeps minted values out of pull's reach entirely.
- [The state attribute makes safix a writer of entry metadata] → one reserved attribute, protected, documented in the usage text; a person deleting it converts the mapping to bootstrap semantics, which is the safe direction.
- [Byte equality over protected attributes requires reading them] → the read is a pipe into the comparison and is discarded, the audit's posture.
- [A value the store cannot carry whole] → the entry password is one line, so a value carrying a newline is refused per mapping with the reason and the remedy named, never trimmed to fit: a mirror that silently drops a byte lies about what it holds, and the byte-exact comparison would then rewrite the whole database on every run forever. Uniform across the surface rather than transport-dependent, and invisible at evaluation, which is why it is a run-time refusal.

## Migration Plan

Additive: a declaration nobody has written yet, a verb nobody calls yet.
First use: declare one `safix-to-keepassxc` mapping, run `safix sync`, watch one entry appear; graduate mappings to `two-way` as wanted.
Rollback is removing the declaration and the verb; entries and their state attributes are database content and stay.

## Open Questions

None; the operator settled the verb name (`sync`), the mode vocabulary (Filen's), and the database default (none — each consumer names one) on 2026-08-17.
