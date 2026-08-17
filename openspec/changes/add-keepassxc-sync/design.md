## Context

See proposal.md — Why.
Measured facts (research 2026-08-17, keepassxc-cli 2.7.12 probed empirically):

- `keepassxc-cli add`/`show -s`/`db-create` work non-interactively with the password on stdin; every db-opening command takes `-y slot[:serial]` for the challenge-response factor. No CLI verb enrolls or changes database keys — that is GUI-only, which is fine here: sync never wants it.
- The fleet's database is 292 MB, synchronized two-way on a minutely timer, saved after every change; each entry write through the CLI is one whole-file rewrite and re-upload. The one-unlock-bootstrap design records a second-small-database option as open.
- The Secret Service exposure boundary travels inside the kdbx (`FdoSecretsSettings` reads it from database custom data), so a session with the database unlocked can be written through `secret-tool`/DBus with no password handling at all.
- safix's resolver already answers "which entries does this person hold, in which file, under which key", and the bridge's transfer/report machinery (`Outcome`/findings-as-data, value-free reports, converge-before-write) is the established shape for exactly this kind of verb.

## Goals / Non-Goals

Goals: one producer for person-read credentials; drift visible and converged by one verb; naming in code; write cost bounded by convergence.

Non-goals:
- Two-way sync. A database edit flowing back into safix would be a second producer, which is the race the bridge's `twoProducers` refusal exists to prevent. If a value should change, it changes in safix.
- Database lifecycle: no `db-create`, no key changes, no group deletion. Sync writes entries under the declared group and nothing else.
- Deleting database entries for retired mappings. A removed mapping stops being synced; its last value stays until a person removes it, and the report names it as no longer declared rather than silently orphaning it. Deletion is a person's act in a person's store.
- Syncing into the exposed group as a way of publishing to Secret Service consumers. The exposure boundary is the database's own setting; sync writes where the declaration says and takes no position on exposure.

## Decisions

### D1. The verb reads through the resolver and reports like the audit

The safix side is the ordinary resolver read (same custody rules: the runner decrypts only what their identity opens), the kdbx side is `show -s` over the declared path, the comparison is byte equality, and the output is findings-as-data rendered in the CLI — the audit's shape, because the question is the audit's question pointed at a different far side.
A mapping the runner cannot decrypt is reported as not judged, never skipped: same reasoning, recorded once in the audit's design and inherited here.

### D2. Writes batch behind the comparison, and the comparison is why the 292 MB cost is acceptable

Only differing mappings write, so steady state writes nothing.
When several differ in one run, writes are issued consecutively in one process group so the sync client sees one burst rather than a trickle; a per-entry save is still a whole-file rewrite, and the design does not pretend otherwise — the real bound is the second-small-database decision, which belongs to the operator's dotfiles change and lands here as nothing more than a different `database` path in the declaration.

### D3. The session path is preferred, the CLI path is the fallback, and both are one module

Secret service (DBus) when the database is unlocked: no password anywhere, entries labelled from the declaration.
`keepassxc-cli` with one hidden password prompt otherwise.
The module is shared with `enroll-hardware-custody`'s PIN custody step — one implementation of "write an entry into the operator's store" with two transports, so the two changes cannot drift apart on how the store is reached.

### D4. Attribute mapping is fixed and small

The safix value lands as the entry's password attribute; the declaration may set a username string per mapping (a person-read credential usually pairs with an account name); the entry title is the last path segment.
Arbitrary attribute templating was rejected: every field added to the declaration is a field the report and the refusals must speak about, and nothing asked for more than these.

## Risks / Trade-offs

- [The database file may be mid-sync (Filen) when sync writes] → KeePassXC's own save is atomic per write and the sync client watches the file either way; sync adds no new failure mode beyond what every GUI edit already has. Not solved here; named.
- [Byte equality over protected attributes requires reading them] → `show -s` prints protected values; the read is a pipe into the comparison and is discarded, the same posture the audit takes for the safix side.
- [A mapping for another person's entry] → refused at evaluation where visible (the declaring flake knows its users) and reported as not judged at run time where only decryption can answer.

## Migration Plan

Additive: a declaration nobody has written yet, a verb nobody calls yet.
First use: declare one mapping, run `safix sync`, watch one entry appear; the report is the verification.
Rollback is removing the declaration and the verb; entries already written are a person's store content and stay.

## Open Questions

- USER-RUN: the verb's name. `sync` is proposed (single word, table style); `mirror` is the alternative if `sync` reads as two-way to you. Decide before the usage text is written.
- USER-RUN: whether the declaration's `database` default should be the fleet's main kdbx or deliberately have no default, forcing each consumer to name one. No default is the safer read of "naming decided in code"; confirm.
