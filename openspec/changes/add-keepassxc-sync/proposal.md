## Why

Secrets safix governs are read by tools; some are also read by a person — typed into a web login, a mobile app, another machine's prompt — and the fleet's place for person-read credentials is the KeePassXC database, which is also its root of trust.
Today a value that should exist in both places is copied by hand, which means it drifts, and nothing reports the drift.
The operator's direction: declared safix secrets sync with a KeePassXC group, the group and entry naming decided in nix code, and the relationship per mapping shaped the way Filen shapes its sync pairs — one-way in either direction, two-way, or backup — rather than one fixed direction.

## What Changes

- A declaration, `flake.safix.keepassxc`: the database, the group, and per-entry mappings from a person's safix entry to a kdbx entry path — naming is the consumer's, in code, and each mapping declares its mode.
- Four modes, named by their endpoints the way the clan bridge names direction, with Filen's pair semantics as the model and no deletion propagation in any of them:
  - `safix-to-keepassxc` — the database converges to safix's value; a database-side edit to a mapped entry is overwritten and reported.
  - `keepassxc-to-safix` — safix converges to the database's value through the ordinary write path, with every write-path refusal in force; refused at evaluation for a generator-produced entry, because the generator is that value's producer.
  - `two-way` — whichever side changed since the last sync wins; both changed is a conflict finding that writes nothing and names both one-way remedies.
  - `backup` — safix's value is written where the database has none, and an existing differing database value is never overwritten, only reported.
- A new verb, `safix sync`: for each declared mapping — or the one named — read both sides, converge per the mapping's mode, and report per mapping: unchanged, updated, pulled, conflict, or refused. No value in any output path.
- Two-way needs a memory of the last agreed state, and it lives inside the encrypted database — a protected attribute on the entry — never in the repository, because a committed digest of a secret value is an oracle for confirming guesses.
- The database is reached through the session's secret service when unlocked, `keepassxc-cli` with one password prompt when not; values travel stdin, pipes and the session bus, never argument vectors. `sync` never touches the database's own key material.
- Convergence is load-bearing on this fleet: the database is 292 MB and every save rewrites and re-uploads the whole file, so a run that would write nothing writes nothing.
- Evaluation refuses what it can see: two mappings to one kdbx path, a mapping whose safix side no entry declares, a pull or two-way mapping onto a generator-produced entry.
- **BREAKING** for nothing.

## Capabilities

### New Capabilities

- `keepassxc-sync`: the declaration, the four modes, the convergent transfer, the report, and the refusals.

### Modified Capabilities

None.

## Impact

Affected code:

- `modules/flake/safix`: the `keepassxc` declaration, the mode enum, and the evaluation refusals.
- `crates/safix-core`: the sync module — safix-side reads through the existing resolver, safix-side writes through the existing terminal-free write core, kdbx reads and writes through the shared KeePassXC path, the per-mode convergence, the findings-as-data report.
- `crates/safix`: the verb, its usage contract, the report rendering.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Dependencies: `keepassxc-cli` resolved the way runtime tools already are; the secret-service path needs a session with the database unlocked.
Ordering: shares the KeePassXC write path with `enroll-hardware-custody`, and the pull modes write through the same core `ValueSource` seam `settle-clan-vars-parity` builds its stream source on — whichever of the three lands last reuses what the others built.
The one-unlock-bootstrap design's open question about a second, small database bounds the rewrite cost; if the operator takes it, the declaration's `database` field is where the answer lands, and nothing here changes shape.
