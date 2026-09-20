# Proposal

## Why

`safix migrate` verifies every candidate before publishing, but publication spans several files and a process killed mid-way leaves a subset of outputs beside the retained sources. Today a rerun refuses those outputs as "already exists" and the operator has to inspect and delete by hand, which is exactly the judgement the tool exists to make for them.

## What Changes

- A migration writes a durable journal before it publishes anything. The journal names the plan by content digest and records every output as it lands: path, file identity and a digest of the published bytes.
- Rerunning the same plan after an interruption resumes it. Outputs the journal proves are its own are re-verified and kept; the rest are published; the journal is removed as the last step. A journal from a different plan, or an output that no longer matches its record, refuses by name.
- `safix migrate --abandon <plan.json>` removes an interrupted migration's recorded outputs and staging directories, and nothing else.
- Interruption by signal between steps rolls back the way an ordinary error does, instead of leaving the process's own cleanup unrun.
- The migrate verb gains the integration coverage it never had: a plan fixture, the spec's existing rollback scenario, resumption, abandonment and the refusals.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `secret-migration`: publication becomes recoverable — the "not crash-atomic" clause is replaced by the journal contract, resumption, abandonment and their refusals.
- `safix-cli`: `migrate` gains `--abandon`; the help text describes recovery instead of advising manual inspection.

## Impact

- `crates/safix-core/src/migrate.rs`: journal record, resume and abandon paths, signal-aware interruption.
- `crates/safix/src/main.rs`, `usage.rs`: the flag and the help text.
- `crates/safix/tests/`: a new `migrate.rs` suite with a plan fixture.
- `README.md` / `docs/`: the migration guide describes recovery; the "inspect partial outputs by hand" paragraph goes.
- No new dependencies. The journal is JSON written with the crate's existing serde setup.
