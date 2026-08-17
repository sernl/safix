## 1. The declaration

- [ ] 1.1 Add `flake.safix.keepassxc`: database path, group, and per-mapping safix side (person, entry) and kdbx side (path under the group, optional username), with descriptions in the types' voice
- [ ] 1.2 Evaluation refusals: two mappings onto one kdbx path; a mapping whose safix side no declaration resolves; each listing every violation at once
- [ ] 1.3 Module-evaluation tests for the refusals and for a well-formed declaration's projection

## 2. The store path

- [ ] 2.1 One module, two transports: secret-service write over DBus when the database is unlocked; `keepassxc-cli` with password on stdin otherwise; values on stdin or the bus only
- [ ] 2.2 The locked-and-headless refusal, before any secret is read, naming both ways to provide the database
- [ ] 2.3 Share the module with `enroll-hardware-custody`'s PIN custody step (whichever lands second reuses; a task in the second change records the reuse)
- [ ] 2.4 Unit-test transport selection and the refusal; integration-test the CLI transport against a scratch database

## 3. The verb

- [ ] 3.1 `safix sync [<mapping>]`: resolver read, kdbx read, byte comparison, write only on difference, safix side never modified
- [ ] 3.2 Findings-as-data report: unchanged, updated, refused-with-reason, not-judged; every declared mapping present; no value in any output path
- [ ] 3.3 Report mappings no longer declared whose entries linger, as information rather than action
- [ ] 3.4 Batch consecutive writes in one burst; steady state writes nothing
- [ ] 3.5 Verify: a fixture with one agreeing, one differing, one absent, and one undecryptable mapping produces exactly the four outcomes; a second run writes nothing; protected reads never reach stdout

## 4. The record

- [ ] 4.1 USER-RUN: settle the verb's name (`sync` proposed) and the no-default-database question, design's two open questions
- [ ] 4.2 Usage text in the scaffold order; the one-direction rule and the no-keyring-management rule stated
- [ ] 4.3 README: one section, the declaration and the verb; CHANGELOG under Unreleased
- [ ] 4.4 Verify: `openspec validate add-keepassxc-sync --strict` passes
