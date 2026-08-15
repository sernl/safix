# Tasks: clan-bridge

The three standing disciplines hold: fixture identities only, nothing deploys, and no sentence describing a guarantee is written before the code enforcing it exists in the same commit.

This change lands after `clan-generator-contract`, because a mapping compares a generator's `share` across the two systems and safix does not carry that field until then.

One decision is unresolved and gates stage 3. Task 0.1 is that decision, and it is USER-RUN.

Stages 1 and 2 are landed. Stage 3 onward is held at 0.1: the decision governs where the *clan-side read* comes from, and both verbs read the clan side — export compares before writing, so the read is not import's alone. Task 1.3's third refusal and task 2.2 are held with it; the reasons are in the report accompanying those commits.

Stages: 0 is the decision, 1 is the surface, 2 is the delegation, 3 is transfer and convergence, 4 is the audit, 5 is the record and the follow-up.

## 0. The decision that gates the read path

- [ ] 0.1 USER-RUN: decide the import-direction question in design D1. The brief specifies that import decrypts clan material with the operator's admin identity; the recommendation is symmetric delegation through clan's command. The evidence is that this fleet's clan sets `secretStore = "age"` at `modules/clan/vars.nix:80` in dotfiles, so direct decryption means implementing clan's age backend rather than reading a sops file. Record the answer in `design.md` with the reason
- [ ] 0.2 USER-RUN: decide question 2 in design's open questions — whether export refuses outright when the clan-side generator's definition could invalidate the exported value, or exports and lets the audit catch the loss. Refusing is safer and forbids a legitimate case

## 1. The declared surface

- [x] 1.1 Add `flake.safix.bridge` to `types.nix`: `clanFlake`, and `mappings.<id>` carrying `direction`, `clan.{machine,generator,file}` and `safix.{user,name}`
- [x] 1.2 Make `direction` an enum of `clan-to-safix` and `safix-to-clan`. Record at the option why it is not spelled `import`/`export`: the word moves values in opposite directions across this boundary depending on which tool says it
- [~] 1.3 Five of six landed in `bridge.nix`; the safix-to-clan-source-with-nothing-to-send refusal is held (see the report: safix has no evaluation-time notion of an entry having no value). Refusals: unresolvable safix side; a clan-to-safix target that also has a generator; a safix-to-clan source with nothing to send; two mappings writing one target; one endpoint pair declared in both directions; more than one `clanFlake`
- [x] 1.4 Expose the refusals through `checks.nix` as a message function and a builder over it, matching the custody and generator-tool families, and add `safix-bridge-refusals` to `mkChecks`
- [x] 1.5 Record in the option documentation that evaluation does not and cannot verify the clan half, and that a bad clan side is a run-time refusal naming the triple
- [x] 1.6 Severity drill: for each of the six refusals, perturb a fixture fleet and confirm the message names what it should. Run the drill through `refuseScript` so it executes the bytes the real check runs
- [x] 1.7 Verify: `nix flake check` passes and the six drills were observed

## 2. Delegation to clan

- [x] 2.1 Implement the clan subprocess driver in `crates/safix-core/src/bridge.rs`: resolve clan's command on PATH, invoke read with the value captured from standard output, invoke write with the value supplied on standard input
- [ ] 2.2 BLOCKED on 0.1's read path. Establish that the read captured raw bytes rather than a terminal rendering. clan's read command substitutes a printable form when its output is a terminal; assert the captured form rather than relying on a subprocess pipe never being one
- [x] 2.3 Implement the absent-command refusal: both verbs refuse before transferring anything, the refusal states that clan is the authority on its own store, and no subset of mappings runs
- [x] 2.4 Surface clan's own failures rather than reinterpreting them — a missing var, an ambiguous id, an ungenerated value each reach the operator as clan's message with safix's mapping name attached
- [x] 2.5 Add the new refusal variants and codes to `crates/safix-core/src/error/`, with paired plain and graphical snapshots
- [x] 2.6 Confirm by search that no clan store layout, backend, recipient handling or file format exists anywhere in the runtime
- [x] 2.7 Verify: `cargo test` passes; the absent-command refusal is observed; and 2.6's search is recorded

## 3. Transfer and convergence

Gated on 0.1.

- [ ] 3.1 Implement `safix import`: for each clan-to-safix mapping, read both sides, compare, and write through the existing `set` path when they differ
- [ ] 3.2 Implement `safix export`: for each safix-to-clan mapping, read both sides, compare, and invoke clan's write only when they differ. Record at the comparison why it is load-bearing rather than an optimisation — clan's write commits unconditionally and a re-encrypting backend produces fresh ciphertext for an unchanged value, so without it every run commits in the clan repository for every mapping
- [ ] 3.3 Refuse a mapping whose safix side the operator cannot decrypt, rather than writing it. Use the reasoning `check` already applies to other people's files
- [ ] 3.4 Implement the four outcomes — unchanged, updated, absent at source, refused — and hold every report, refusal and commit message to naming no value
- [ ] 3.5 Commit per mapping, naming the mapping and the direction. Add `--all` running every mapping of a direction, still committing per mapping
- [ ] 3.6 Confirm the safix-side write acquires the recipient-drift refusal, the staged write and rename, and the pipe, by driving a drifted fixture through import rather than by inspecting the call
- [ ] 3.7 Verify: an integration test runs each verb twice and requires the second run to write nothing and commit nothing
- [ ] 3.8 Severity drill: remove the pre-write comparison from export and confirm the idempotency test fails with a commit per mapping per run

## 4. The audit

- [ ] 4.1 Add bridge rows to `check`: each mapping whose two sides no longer agree is a finding naming the mapping and no value
- [ ] 4.2 Record in the export documentation the condition under which an exported value is silently discarded: changing the clan-side generator's definition invalidates clan's recorded validation, and clan's next routine generation replaces the value. State that a routine generation without a definition change does not, and that an explicit regeneration does and is the operator's action
- [ ] 4.3 Confirm by search that nothing writes clan's validation record, and record why: it would mean writing clan's store directly, and the value would be a function of clan's own definition
- [ ] 4.4 Verify: a fixture with a diverged export mapping produces the finding, and one in step produces none

## 5. Testing, the record, and the follow-up

- [ ] 5.1 Build the stub clan command as a fixture: answers reads with known bytes, records what writes received on standard input, and can be made to fail in each way the real one does. Record why stubbing clan is permitted where stubbing sops is not — sops is what safix's claims are about, clan is a boundary safix delegates across, and the delegation is what is under test
- [ ] 5.2 Add a check driving the real clan command over a throwaway clan when it is present in the check closure, absent rather than trivially green when it is not, following the shape the platform-conditional check already uses
- [ ] 5.3 Update `README.md` with the bridge surface, both verbs, and the direction vocabulary
- [ ] 5.4 Write the bridge into `CHANGELOG.md`, including the decision recorded in 0.1
- [ ] 5.5 Name the dotfiles follow-up `retire-agents-mirror` in this change's impact: `modules/flake/agents/agents.sh` loses its mirror half — its `sops set --value-stdin` write, its secret-tempfile registry and shredder, its trunk guard and its `MIRROR_SOPS_KEY` table — and keeps its remote provisioning. Do not build it here; it spans two repositories and depends on `safix-full-switch`
- [ ] 5.6 Record question 3 from design's open questions as answered or still open: the ordering constraint that this change lands after `clan-generator-contract`
- [ ] 5.7 Verify: `openspec validate clan-bridge --strict` passes
