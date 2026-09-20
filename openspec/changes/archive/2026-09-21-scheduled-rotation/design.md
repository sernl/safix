# Design

## Context

See proposal.md. The resolver (`modules/flake/safix/resolve.nix`) emits one placement per held entry carrying `definitionRecord` and `stampRecord`; `stamps::touch` records `created`/`updated` on every `set` and `generate`; `UserPlan::cascade` computes the re-run set for a regeneration; `check::run` returns `Finding`s that `render.rs` prints with remedies; consuming machines only run `safix install`. Every schema struct in `model.rs` is `deny_unknown_fields`, so the nix and rust halves of a new field ship together.

## Goals / Non-Goals

**Goals:**
- One declaration decides how long a class of values lives; the runtime derives nothing but a deadline from it.
- The operator sees time remaining wherever they see the entry, and `check` says what is overdue with the remedy that fits.
- Unattended rotation runs where the keys are and touches nothing a human would not have touched.

**Non-Goals:**
- Rotating a typed value without a human. The verb refuses and names `set`.
- Pushing, rebuilding or deploying. The timer commits locally; the rest of the pipeline is the operator's.
- A per-entry inline interval. Policies are named so an interval change is one edit.

## Decisions

### D1. Policy root and entry option

`flake.safix.rotation.<policy>.every : str` matching `^[1-9][0-9]*[hdw]$`, validated by the option type so a bad interval fails with the accepted forms. `entry.rotation : nullOr str` on the shared `entry` submodule (so catalogue and private entries take it alike, and `override` may carry it per scope). The resolver refuses an entry naming an undefined policy in the same violation list generator errors use. Hours exist for drills and short-lived tokens; minutes do not, because a deadline shorter than a timer's calendar granularity is a bug.

### D2. What the placement carries

`Placement.rotation : Option<{ policy: String, every_seconds: u64 }>`. Seconds rather than the string, so the runtime never parses the unit; the policy name so findings and cells can print it. Emitted by `resolve.nix` beside `stampRecord`.

### D3. The deadline

`deadline = stamps.updated (or created) + every_seconds`; no stamp record ⇒ due. Computed in one function in `stamps.rs` (`Deadline::of(stamps, rotation, now)`) returning `Remaining(Duration) | Due(Duration overdue) | None`, used by `list`, the picker, `check` and `rotate` so the four cannot disagree.

### D4. Rendering the countdown

A ninth listing column `rotates`, after `updated`. Format: `12d 04:12:09` under one year, `due` past the deadline, `-` without a policy. The picker recomputes the cell every frame from `now` (the draw loop already runs on a 100 ms read timeout), so the countdown ticks without a keypress. A due row takes a new `Tint::Due` (red), ranked above `Newest` and `LastChosen`; the tint enum's order is the precedence. `list` prints the value once at the time of the run.

### D5. The finding

`Finding::RotationDue { user, name, policy, overdue_seconds, generator: bool }`, pushed in the same per-entry loop as `DefinitionDrift`. `render.rs` prints `safix rotate <user> <name>` or `safix set <user> <name>` by the flag. `check` stays read-only and needs no identity for this.

### D6. `rotate` reuses `generate`

`rotate <user> <name>` = `generate::run` with `regenerate` set, restricted to one producer; `rotate --due [<user>]` computes the due set through D3, takes the union of cascades in plan order, confirms once (or `--yes`), runs, then prints the due typed entries as a list with `safix set` remedies. Sharing `generate::run` keeps the pipe-only path, the sandbox and the per-generator commit exactly as they are. `generate --regenerate` stays; `rotate` is the verb the finding names because its meaning is the deadline, not the definition.

### D7. `rotation set` / `unset` scaffold the declaration

Same mechanism as `group add`: locate the entry's declaration in the file the resolver attributes it to, insert or replace one `rotation = "<policy>";` line (or remove it), parse the file with the real evaluator before staging, commit with a subject naming the act. Delegation checks apply as they do for `group`.

### D8. The timer lives in the home module

Options under `safix.rotation`: `enable`, `repository` (absolute string path), `onCalendar` (default `daily`), `environment` (attrs of str, for `SOPS_AGE_KEY_FILE` and the like). Units: `systemd.user.services.safix-rotate` (oneshot, `WorkingDirectory = repository`, `ExecStart = safix rotate --due --yes`, `Environment` from the option) and `systemd.user.timers.safix-rotate` with `Persistent = true` so a laptop that was asleep catches up. Linux only, like the existing install unit. Not in the NixOS module: a machine holds no repository and no git identity.

## Risks / Trade-offs

- [A timer runs against a repository with uncommitted changes] → `generate` already refuses a dirty tree; the unit fails visibly and rotates nothing.
- [Cascade confirmation in a unit] → `--yes` is in the ExecStart; the option documentation says so, because it is the one decision the timer answers for the operator.
- [Clock skew makes a countdown wrong by seconds] → Deadlines are days; seconds are shown for the tick, not for precision.
- [A policy assigned to a value never stamped] → Due at once (D3), which is the only reading that cannot silently extend a value's life.
- [The ninth column widens `list`] → It is behind Tab in the picker like the other optional columns; `list` gains it unconditionally because a deadline is not optional information.

## Migration Plan

Additive. Fleets declaring no policy see a new `-` column and nothing else. Both example fleets declare `quarterly` on one generated and one typed entry so the projection check covers both remedies.
