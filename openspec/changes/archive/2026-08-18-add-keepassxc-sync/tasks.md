## 1. The declaration

- [x] 1.1 Add `flake.safix.keepassxc`: database path (no default — each consumer names one), group, and per-mapping safix side (person, entry), kdbx side (path under the group, optional username), and mode (`safix-to-keepassxc`, `keepassxc-to-safix`, `two-way`, `backup`), with descriptions in the types' voice
  - `database` is `nullOr str` rather than a nix path: a path is copied into the world-readable store when it is interpolated, and the fleet's database is 292 MB. `group` carries a default (`safix`) where `database` cannot, because a group has to exist for a path to be under and the default names safix rather than inventing a taxonomy. The safix half is `bridge.nix`'s own `safixSide`, exported for the purpose, because the runtime deserializes both declarations' safix endpoint through one type.
- [x] 1.2 Evaluation refusals: two mappings onto one kdbx path; a mapping whose safix side no declaration resolves; a pull-capable mode onto a generator-produced entry — each listing every violation at once
  - Plus the reserved-name refusal the apply-time amendment added: a declared path carrying `.safix-sync-state`, which is what makes the companion entry's name structural rather than conventional. Judged over every declared mapping rather than the sound ones, so a mapping with two faults hears about both. Mappings with no `database` produce no evaluation message — that is a run-time refusal, because a tree mid-way through being written still has to evaluate.
- [x] 1.3 Module-evaluation tests for the refusals and for a well-formed declaration's projection
  - `modules/flake/checks/keepassxc.nix`: `safix-keepassxc` holds every message against a literal and the well-formed declaration against silence, and `safix-keepassxc-drill` runs `refuseScript` over a perturbed declaration. `safix-keepassxc-refusals` is the consumer-facing check, instantiated over the fixture fleet by `checks/exported.nix` through `mkChecks`.

## 2. The store path

- [x] 2.1 One module, two transports: secret-service reads and writes over DBus when the database is unlocked; `keepassxc-cli` with password on stdin otherwise; values on stdin, pipes, or the bus only
  - Amended during apply to one transport, with the operator's approval: the Secret Service collection KeePassXC publishes is its exposed group, so it cannot address the group and path a mapping declares, and a service read of an unexposed entry is indistinguishable from the database holding nothing — which would let a `backup` mapping land a secret in a group no declaration named. `crates/safix-core/src/store.rs` and design D7 record it. Values still travel standard input and pipes only.
- [x] 2.2 The locked-and-headless refusal, before any secret is read, naming both ways to provide the database
  - `Error::StoreLocked`, raised before the password is asked for and before any side is read. It names one way rather than two, per the same amendment, and says why the service is not a second.
- [x] 2.3 Share the module with `enroll-hardware-custody`'s PIN mirror step (whichever lands second reuses; a task in the second change records the reuse)
  - Enrollment landed first, so this change reuses: `custody::keepassxc_cli` for the `SAFIX_KEEPASSXC_CLI`-named program, `custody::DatabasePassword` for the one password prompt, `enroll::terminal_present` for the terminal probe, and the stdin protocol `custody::write` records (one newline between values, none after the last). The card stub's `keepassxc-cli` role is extended rather than duplicated, and its enrollment shape is matched exactly first so that path's checks keep testing what they tested. What is deliberately not reused is `custody::choose`: enrollment's entry is safix-owned and addressed by attribute, for which the exposed group is the right home, and sync's is a declared group and path.
- [x] 2.4 Unit-test transport selection and the refusal; integration-test the CLI transport against a scratch database
  - `store.rs`'s own tests hold the argument vectors, the reserved suffix, and that absence is answered from the listing rather than by spawning. `crates/safix/tests/store_cli.rs` drives the real command against a database it creates with `db-create` in a directory it made, and found what no model would have: `ls -R -f` prints `[empty]` for a database holding nothing.

## 3. The verb

- [x] 3.1 `safix sync [<mapping>]`: resolver read, kdbx read, byte comparison, then per-mode convergence — push writes the database, pull feeds the database's value to the ordinary write core through an in-process `ValueSource`, backup writes only into absence
- [x] 3.2 Two-way: the last-synced state as a protected attribute on the entry, updated in the converging write; one-side-changed converges toward the change; both-changed is a conflict finding naming both one-way remedies and writing nothing; a missing state attribute takes bootstrap semantics
  - The state is the password of a reserved companion entry beside the mapped one, per the apply-time amendment recorded in D2 and in the delta spec. Written after the value it is about, never before: the other order records an agreement on a value only one side holds, and the next run would converge the wrong way and overwrite the new value with the old.
- [x] 3.3 Findings-as-data report: unchanged, updated, pulled, conflict, refused-with-reason, not-judged; every declared mapping present; no value in any output path; mappings no longer declared whose entries linger reported as information
  - The lingering line is computed from `ls -R -f` and covers companions, which read differently: a companion holds no value, only a digest of one, and removing it is safe.
- [x] 3.4 Batch consecutive writes in one burst; steady state writes nothing anywhere
  - Two phases: every mapping is read and decided, then every database write is issued consecutively, then the pulls, because a pull commits in this repository and a commit between two database writes is a commit inside the window the burst keeps one save wide.
- [x] 3.5 Verify: fixtures per mode — agreeing, push-diverged, pull-diverged, backup-diverged (reported, not written), two-way one-side, two-way both-sides (conflict), undecryptable — produce exactly their outcomes; a second run writes nothing; a pull lands as a commit indistinguishable in shape from a hand-set write; protected reads never reach stdout
- [x] 3.6 Verify the no-oracle property: after every fixture run, the repository contains no digest or derivative of any fixture value
  - The search is for the value, for its SHA-256 taken by `sha256sum` rather than by the code under test, and for the record's own format tag. The first attempt at the drill for this wrote into a directory that does not exist and swallowed the failure, so nothing turned red and the assertion was still unproven; the second wrote where the run's working directory is and three checks went red.

## 4. The record

- [x] 4.1 Usage text in the scaffold order: the four modes, the no-deletion rule, the conflict semantics, the no-keyring-management rule
- [x] 4.2 README: one section — the declaration, the modes, the verb; CHANGELOG under Unreleased
- [x] 4.3 Verify: `openspec validate add-keepassxc-sync --strict` passes
