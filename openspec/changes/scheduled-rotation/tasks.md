# Tasks

## 1. Declarations

- [ ] 1.1 Add `flake.safix.rotation.<policy>.every` with the interval type and `entry.rotation`/`override.rotation`; verify option-type tests reject `90`, `1.5d`, `0d` and accept `12h`, `30d`, `4w`.
- [ ] 1.2 Emit `rotation = { policy; everySeconds; }` on every placement and refuse an undefined policy in the resolver's violation list; verify in the custody structural check with one valid and one undefined policy fixture.
- [ ] 1.3 Declare `quarterly` in both example fleets on one generated and one typed entry; verify `safix-examples` still compares the two projections equal and covers the field.

## 2. Runtime model and deadline

- [ ] 2.1 Add `Placement.rotation` to `model.rs`; verify the placement deserialization test with and without the field.
- [ ] 2.2 Add `Deadline::of` in `stamps.rs` (remaining, due with overdue duration, none; no record ⇒ due); verify unit tests at the boundary second, before, after and with no record.

## 3. Display

- [ ] 3.1 Add the `rotates` column to `render::listing_header`/`listing_row` with the `12d 04:12:09` / `due` / `-` format; verify render tests for each state.
- [ ] 3.2 Recompute the column every frame in the picker and add `Tint::Due` with the highest precedence; verify tests that the cell changes across a simulated second and that a due row carries the tint over the newest tint.

## 4. Findings and verbs

- [ ] 4.1 Add `Finding::RotationDue` in the per-entry loop and its renderer with the generator/set remedy; verify a check test for each remedy and that a rotated entry clears it.
- [ ] 4.2 Add `safix rotate <user> <name>` and `safix rotate --due [<user>] [--yes]` over `generate::run` with merged cascades, the typed-entries list and the nothing-due exit; verify integration tests for one entry, a due set with a typed entry, nothing due, and the typed-entry refusal.
- [ ] 4.3 Add `safix rotation set|unset` as a declaration scaffold with the parse-before-stage and delegation checks; verify integration tests for assign, replace, unset, unknown policy and unknown entry.
- [ ] 4.4 Register both verbs in the dispatch table and help; verify the discoverable-verbs test.

## 5. Timer

- [ ] 5.1 Add `safix.rotation.{enable,repository,onCalendar,environment}` to the home module and the `safix-rotate` service and persistent timer; verify a home-manager evaluation test that the units exist when enabled with the expected ExecStart, and nothing exists when disabled.
- [ ] 5.2 Document at the options that the identity must decrypt without a prompt, that the timer commits and pushes nothing, and that machines pick up values on rebuild; verify the option descriptions render in the evaluation test.

## 6. Verification

- [ ] 6.1 Run the workspace tests and the structural, examples and consumption checks; verify all pass and clippy is clean.
- [ ] 6.2 With a disposable repository and a `1h` policy, set an entry's stamp an hour back, run `safix check` (finding), `safix rotate --due --yes` (rotated), `safix list` (countdown restarted), and open the picker for two seconds (cell ticks); verify each by observation and record it in the change.
