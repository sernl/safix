# Proposal

## Why

Rotation is a ceremony an operator remembers to perform. Nothing in the declarations says how long a value may live, nothing reports a value that has lived too long, and nothing re-mints a generator-backed value on a schedule. A person who has been removed from an audience keeps every value they read until someone remembers to rotate it.

## What Changes

- Named rotation policies: `flake.safix.rotation.<policy>.every` declares an interval once; an entry opts in with `rotation = "<policy>"`.
- `safix rotation set <user> <name> <policy>` and `safix rotation unset <user> <name>` edit that declaration as text, parsed before staging and committed, the way `group add` edits a group.
- Every entry with a policy has a rotation deadline: its last write plus the interval. `safix list` and the picker show the time remaining; the picker's countdown ticks while it is open, and a row whose deadline has passed is marked due.
- `safix check` reports every entry past its deadline, naming the remedy: `safix rotate` for a generator-backed entry, `safix set` for a typed one.
- `safix rotate <user> <name>` re-mints one generator-backed entry now, with the cascade its dependents require. `safix rotate --due [<user>]` re-mints every due generator-backed entry and lists the due typed entries it cannot mint.
- `safix.rotation` on the home-manager profile installs a systemd user timer on the operator's workstation that runs `safix rotate --due --yes` against a named repository. It is off by default and requires an identity that decrypts without a prompt.

## Capabilities

### New Capabilities

- `secret-rotation`: rotation policies, deadlines, the due finding, the `rotate` and `rotation` verbs and the workstation timer.

### Modified Capabilities

- `safix-cli`: `list` shows the rotation deadline; the closed subcommand set gains `rotate` and `rotation`.
- `secret-consumption`: the home-manager profile gains the `safix.rotation` timer options.

## Impact

- `modules/flake/safix/{options.nix,types.nix,resolve.nix}`: the policy root, the entry option, validation, and the deadline carried on every placement.
- `crates/safix-core/src/{model.rs,check.rs,generate.rs,rotate.rs,stamps.rs}`: the schema field, the finding, the verb.
- `crates/safix/src/{main.rs,usage.rs,render.rs,picker/}`: the verbs, the column, the live countdown, the due tint.
- `modules/consume/home.nix`: the timer options and units.
- `examples/`: both fleets declare a policy so the projection check covers it.
- `README.md` / `docs/`: rotation becomes a first-class chapter.
- No new dependencies.
