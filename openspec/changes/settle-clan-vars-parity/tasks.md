## 1. The definition record

- [ ] 1.1 Compute the digest over the canonical generator record — script, `runtimeInputs`, prompts, `files` with modes and secret flags, dependencies, validation — with a leading format-version tag (design D1, D2)
- [ ] 1.2 Write the record to `state/safix/definitions/...` in the same commit as the minted value, for mint and for regeneration alike; an interrupted mint leaves neither
- [ ] 1.3 Unit-test the digest: an edit to each field of the record changes it; a value change does not; two mints under one definition agree
- [ ] 1.4 Integration-test the commit atomicity: the record rides the mint's commit, and an interrupted run leaves no record for the value it did not commit

## 2. The drift finding

- [ ] 2.1 Add the fifth finding class to `check`: recorded digest present and unequal to the current declaration's digest — naming the entry and both remedies, carrying no value
- [ ] 2.2 Hold the out-of-scope cases quiet: no generator, no record, or an unknown record version produce no drift finding
- [ ] 2.3 Render the finding in the CLI with the existing findings-are-data split
- [ ] 2.4 Verify: a fixture minted, then its declaration edited, produces the finding; regeneration clears it; a hand-set entry and a record-less value never produce it

## 3. The stream source for set

- [ ] 3.1 Implement the stdin `ValueSource` at the CLI layer: read when standard input is not a terminal, bytes exactly as received, no prompt, no confirmation
- [ ] 3.2 Keep the refusals: empty input takes the empty-value refusal; the terminal path is byte-for-byte today's behaviour
- [ ] 3.3 Verify: piped bytes round-trip exactly (trailing newline included), the value reaches no argv and no environment, and an empty pipe refuses — extending the existing value-pipe observations to the new source

## 4. The recorded absences

- [ ] 4.1 USER-RUN: decide whether the recorded refusals of `upload` and of the plaintext dump-and-restore stand, with the committed reasoning in view (design's first open question). If either is overturned, that is a new change with its own spec delta, not an edit here
- [ ] 4.2 USER-RUN: confirm or reject the per-export placement non-goal (design's second open question); on confirmation, add the recording to this change's safix-cli delta
- [ ] 4.3 Record in the help and README that scripted writes exist, replacing nothing: the prompt path is unchanged

## 5. The record

- [ ] 5.1 CHANGELOG entries under Unreleased: the state tree, the finding class, the stream source
- [ ] 5.2 README: the daily-commands section gains the piped `set` form; the checks narrative gains the drift finding
- [ ] 5.3 Verify: `openspec validate settle-clan-vars-parity --strict` passes
- [ ] 5.4 Verify before archive: `clan-generator-contract` and `adopt-generator-sandbox` archived first (shared `secret-generators` history)
