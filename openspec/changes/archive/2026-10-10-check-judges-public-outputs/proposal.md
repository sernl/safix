# Proposal

## Why

`check` asks every declared name whether the document its placement points at holds a value for it. A public output has no such document — that is what `files.<name>.secret = false` means — so the answer is "no" for every public output in every tree, whether it has been minted or not.

The report that follows is wrong twice. It says a converged entry is valueless, and because the generator is declared on a *sibling* entry rather than on this one, the entry carries no generator of its own and the remedy offered is `safix set <user> <name>`: a prompt for a value no operator can type, being a function of a private half they have never seen.

A fleet of 23 public outputs was reported this way, every one of them present under `public/safix/users/<user>/<name>/value` and every one of them read correctly at evaluation by `publicValue` at the same moment `check` called it missing. A report that is wrong about a converged tree is a report an operator learns to skip.

## What Changes

- `check` judges a public output by the file named on its placement's `public` field, which is where the value is and which the resolver already computes for exactly these entries — in vault mode as well as outside it, so nothing reads `logicalPublic`.
- A public output holding bytes produces no finding.
- A public output holding nothing produces `Finding::ValuelessPublic`, which names its own plaintext path and the entry the generator writing it is declared on, and whose remedy is `safix generate <user> <producer>`. There is no `set` remedy, because there is no `set` for a public output.

## Capabilities

### Modified Capabilities

- `public-outputs`: the drift report's reading of a public output becomes a stated guarantee rather than an accident of which file a placement names.
- `safix-cli`: `check` gains one finding with one remedy.

## Impact

- `crates/safix-core/src/check.rs`: the `Finding::ValuelessPublic` variant, the branch in the valueless-name walk, and `public_finding`.
- `crates/safix/src/render.rs`: the paragraph and the remedy.
- `crates/safix/tests/generators.rs` and `modules/flake/checks/cli.nix`: the check, both directions.
- No nix change. `placements.<user>.<name>.public` already carries the path in both naming modes; a consumer bumping the input changes no declaration.
- No new dependencies.
