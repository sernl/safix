## 1. The declaration

- [ ] 1.1 Add `flake.safix.keepassxc`: database path (no default — each consumer names one), group, and per-mapping safix side (person, entry), kdbx side (path under the group, optional username), and mode (`safix-to-keepassxc`, `keepassxc-to-safix`, `two-way`, `backup`), with descriptions in the types' voice
- [ ] 1.2 Evaluation refusals: two mappings onto one kdbx path; a mapping whose safix side no declaration resolves; a pull-capable mode onto a generator-produced entry — each listing every violation at once
- [ ] 1.3 Module-evaluation tests for the refusals and for a well-formed declaration's projection

## 2. The store path

- [ ] 2.1 One module, two transports: secret-service reads and writes over DBus when the database is unlocked; `keepassxc-cli` with password on stdin otherwise; values on stdin, pipes, or the bus only
- [ ] 2.2 The locked-and-headless refusal, before any secret is read, naming both ways to provide the database
- [ ] 2.3 Share the module with `enroll-hardware-custody`'s PIN mirror step (whichever lands second reuses; a task in the second change records the reuse)
- [ ] 2.4 Unit-test transport selection and the refusal; integration-test the CLI transport against a scratch database

## 3. The verb

- [ ] 3.1 `safix sync [<mapping>]`: resolver read, kdbx read, byte comparison, then per-mode convergence — push writes the database, pull feeds the database's value to the ordinary write core through an in-process `ValueSource`, backup writes only into absence
- [ ] 3.2 Two-way: the last-synced state as a protected attribute on the entry, updated in the converging write; one-side-changed converges toward the change; both-changed is a conflict finding naming both one-way remedies and writing nothing; a missing state attribute takes bootstrap semantics
- [ ] 3.3 Findings-as-data report: unchanged, updated, pulled, conflict, refused-with-reason, not-judged; every declared mapping present; no value in any output path; mappings no longer declared whose entries linger reported as information
- [ ] 3.4 Batch consecutive writes in one burst; steady state writes nothing anywhere
- [ ] 3.5 Verify: fixtures per mode — agreeing, push-diverged, pull-diverged, backup-diverged (reported, not written), two-way one-side, two-way both-sides (conflict), undecryptable — produce exactly their outcomes; a second run writes nothing; a pull lands as a commit indistinguishable in shape from a hand-set write; protected reads never reach stdout
- [ ] 3.6 Verify the no-oracle property: after every fixture run, the repository contains no digest or derivative of any fixture value

## 4. The record

- [ ] 4.1 Usage text in the scaffold order: the four modes, the no-deletion rule, the conflict semantics, the no-keyring-management rule
- [ ] 4.2 README: one section — the declaration, the modes, the verb; CHANGELOG under Unreleased
- [ ] 4.3 Verify: `openspec validate add-keepassxc-sync --strict` passes
