## Context

See proposal.md — Why.
Measured facts (research 2026-08-17, keepassxc-cli 2.7.12 probed empirically):

- `keepassxc-cli add`/`edit`/`show -s`/`db-create` work non-interactively with the password on stdin; every db-opening command takes `-y slot[:serial]`. No CLI verb enrolls or changes database keys — GUI-only, and sync never wants it.
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
So the last-synced state is a protected custom attribute on the kdbx entry itself, updated in the same write that converges the entry; the repository carries no value-derived state, and the property is spec-held.
Consequence accepted: losing the database loses the memory, and every two-way mapping then reports as if newly bootstrapped — safe, because bootstrap semantics write only where one side is empty and report everything else.

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

## Risks / Trade-offs

- [Two writers on the kdbx file (sync verb and the Filen client)] → KeePassXC's save is atomic per write; the burst discipline keeps the window one save wide; named, not solved, exactly as every GUI edit already bears it.
- [A two-way pull can import a weaker value into safix] → it arrives through the full write path and its commit names the mapping, so review sees it like any hand-set change; the generator-produced refusal keeps minted values out of pull's reach entirely.
- [The state attribute makes safix a writer of entry metadata] → one reserved attribute, protected, documented in the usage text; a person deleting it converts the mapping to bootstrap semantics, which is the safe direction.
- [Byte equality over protected attributes requires reading them] → the read is a pipe into the comparison and is discarded, the audit's posture.

## Migration Plan

Additive: a declaration nobody has written yet, a verb nobody calls yet.
First use: declare one `safix-to-keepassxc` mapping, run `safix sync`, watch one entry appear; graduate mappings to `two-way` as wanted.
Rollback is removing the declaration and the verb; entries and their state attributes are database content and stay.

## Open Questions

None; the operator settled the verb name (`sync`), the mode vocabulary (Filen's), and the database default (none — each consumer names one) on 2026-08-17.
