## 1. The definition record

- [x] 1.1 Compute the digest over the canonical generator record — script, `runtimeInputs`, prompts, `files` with modes and secret flags, dependencies, validation — with a leading format-version tag (design D1, D2). `crates/safix-core/src/definition.rs` holds the canonical form and `digest.rs` the SHA-256 it is reduced through. The `files` record carries secret flags and no mode: a mode is a registry field of the entry rather than of the generator, does not travel on `Generator`, and decides where a decrypted value lands rather than what a mint produces — recorded in the module's own "What the digest covers"
- [x] 1.2 Write the record to `state/safix/definitions/...` in the same commit as the minted value, for mint and for regeneration alike; an interrupted mint leaves neither
- [x] 1.3 Unit-test the digest: an edit to each field of the record changes it; a value change does not; two mints under one definition agree. A value cannot reach the digest at all — `definition::digest` takes the generator record and nothing else — so the second claim is asserted as the two uncovered fields leaving it alone, plus the canonical form's own content
- [x] 1.4 Integration-test the commit atomicity: the record rides the mint's commit, and an interrupted run leaves no record for the value it did not commit

## 2. The drift finding

- [x] 2.1 Add the fifth finding class to `check`: recorded digest present and unequal to the current declaration's digest — naming the entry and both remedies, carrying no value. `Finding::DefinitionDrift` carries the user, the name, the entry the generator is declared on and the record's path, and no value. One finding per record rather than per carrier, so a shared entry is reported once
- [x] 2.2 Hold the out-of-scope cases quiet: no generator, no record, or an unknown record version produce no drift finding. The producer is read off the placements rather than the run plan — `Placements::producer_of`, bound to `UserPlan::producer_of` by a test — because the plan is guarded and a report that evaluated it would fall silent on exactly the trees whose declarations are wrong
- [x] 2.3 Render the finding in the CLI with the existing findings-are-data split
- [x] 2.4 Verify: a fixture minted, then its declaration edited, produces the finding; regeneration clears it; a hand-set entry and a record-less value never produce it. `safix-generate-definition-drift` runs it; two drills observed red — never reporting fails the drift assertion, always reporting fails the four silences

## 3. The stream source for set

- [ ] 3.1 Implement the stdin `ValueSource` at the CLI layer: read when standard input is not a terminal, bytes exactly as received, no prompt, no confirmation
- [ ] 3.2 Keep the refusals: empty input takes the empty-value refusal; the terminal path is byte-for-byte today's behaviour
- [ ] 3.3 Verify: piped bytes round-trip exactly (trailing newline included), the value reaches no argv and no environment, and an empty pipe refuses — extending the existing value-pipe observations to the new source

## 4. The recorded absences

- [x] 4.1 USER-RUN (answered): decide whether the recorded refusals of `upload` and of the plaintext dump-and-restore stand, with the committed reasoning in view (design's first open question). If either is overturned, that is a new change with its own spec delta, not an edit here. Decided: both refusals stand, re-examined against the custody-subjects extension the operator directed on the same day — machines and services joining the audience model changes who may be in an audience, not how values reach a machine, which stays activation reading the committed file; and the backend count stays one, so the migration scenario the dump serves still does not exist
- [x] 4.2 USER-RUN (answered): confirm or reject the per-export placement non-goal (design's second open question); on confirmation, add the recording to this change's safix-cli delta. Decided: rejected as a non-goal, and not adopted as clan's shape either — the operator extended safix's axis instead. Machines, services, groups and organizational custody become first-class subjects in `extend-custody-subjects`, which supersedes this question; no recording lands in this change's delta
- [ ] 4.3 Record in the help and README that scripted writes exist, replacing nothing: the prompt path is unchanged

## 5. The record

- [ ] 5.1 CHANGELOG entries under Unreleased: the state tree, the finding class, the stream source
- [ ] 5.2 README: the daily-commands section gains the piped `set` form; the checks narrative gains the drift finding
- [ ] 5.3 Verify: `openspec validate settle-clan-vars-parity --strict` passes
- [ ] 5.4 Verify before archive: `clan-generator-contract` and `adopt-generator-sandbox` archived first (shared `secret-generators` history)
