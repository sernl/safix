## Why

Secrets safix governs are read by tools; some are also read by a person — typed into a web login, a mobile app, another machine's prompt — and the fleet's place for person-read credentials is the KeePassXC database, which is also its root of trust.
Today a value that should exist in both places is copied by hand, which means it drifts, and nothing reports the drift.
The operator's direction: declared safix secrets sync into a KeePassXC group, with the group and entry naming decided in nix code.

## What Changes

- A declaration, `flake.safix.keepassxc`: the database, the group, and per-entry mappings from a person's safix entry to a kdbx entry path — naming is the consumer's, in code, like every other safix declaration.
- A new verb, `safix sync`: for each declared mapping, read the safix side, read the kdbx side, write only where they differ, and report per mapping — unchanged, updated, or refused — never printing a value. One direction, safix to KeePassXC: safix stays the producer, the database a mirror for human eyes.
- The database is reached the way the enrollment change reaches it: the session's secret service when unlocked, `keepassxc-cli` with one password prompt when not; values travel stdin and DBus, never argument vectors. `sync` never touches the database's own key material.
- Convergence is load-bearing on this fleet: the database is 292 MB and every save rewrites and re-uploads the whole file, so a run that would write nothing writes nothing, and a run that writes batches what it can.
- Evaluation refuses what it can see: two mappings to one kdbx path, a mapping whose safix side no entry declares, a mapping for a person outside the declaring operator's audience reach.
- **BREAKING** for nothing.

## Capabilities

### New Capabilities

- `keepassxc-sync`: the declaration, the one-way convergent transfer, the report, and the refusals.

### Modified Capabilities

None.

## Impact

Affected code:

- `modules/flake/safix`: the `keepassxc` declaration and its evaluation refusals.
- `crates/safix-core`: the sync module — safix-side reads through the existing resolver, kdbx-side reads and writes through the shared KeePassXC path, the convergent compare, the findings-as-data report.
- `crates/safix`: the verb, its usage contract, the report rendering.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Dependencies: `keepassxc-cli` resolved the way runtime tools already are; the secret-service path needs a session with the database unlocked.
Ordering: shares the KeePassXC write path with `enroll-hardware-custody`; whichever lands second reuses it.
The one-unlock-bootstrap design's open question about a second, small database bounds the rewrite cost; if the operator takes it, the declaration's `database` field is where the answer lands, and nothing here changes shape.
