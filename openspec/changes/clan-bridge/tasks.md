# Tasks: clan-bridge

The three standing disciplines hold: fixture identities only, nothing deploys, and no sentence describing a guarantee is written before the code enforcing it exists in the same commit.

This change lands after `clan-generator-contract`, because a mapping compares a generator's `share` across the two systems and safix does not carry that field until then.

Both gating decisions are resolved. They are recorded in `design.md` under "The three decisions", together with a third ruling that deletes an evaluation refusal the spec carried and replaces it with a runtime one.

Stages: 0 is the decisions, 1 is the surface, 2 is the delegation, 3 is transfer and convergence, 4 is the audit, 5 is the record and the follow-up.

## 0. The decisions that gated the read path

- [x] 0.1 USER-RUN: decide the import-direction question in design D1. Decided: symmetric delegation. Every clan-side read is `clan vars get` captured on a pipe and every clan-side write is `clan vars set` fed on standard input; the runtime reads, writes, decrypts, encrypts and parses none of clan's stored files, in either direction. The evidence is that this fleet's clan sets `secretStore = "age"` at `modules/clan/vars.nix:80` in dotfiles, so direct decryption means implementing clan's age backend rather than reading a sops file. Recorded in `design.md`
- [x] 0.2 USER-RUN: decide whether export refuses outright when the clan-side generator's definition could invalidate the exported value. Decided: refuse, with its own code and message naming both remedies, and no override flag in 0.2. The comparison is delegated to `clan vars check --generator` rather than made against clan's recorded hash, because reading that record would break 0.1 to enforce this. Recorded in `design.md`
- [x] 0.3 Delete the bridge-surface requirement that evaluation refuse a `safix-to-clan` mapping "whose source entry has neither a generator nor a declared value". It has no referent at evaluation: an entry declares where a value lives rather than that one is there, and a hand-set entry with no generator is the ordinary export. Replace it with a runtime refusal — export refuses when the source key is absent from the source file, naming `safix set` and `safix generate` — and keep `handSetExportMessages` in `modules/flake/checks/bridge.nix` asserting the evaluation silence

## 1. The declared surface

- [x] 1.1 Add `flake.safix.bridge` to `types.nix`: `clanFlake`, and `mappings.<id>` carrying `direction`, `clan.{machine,generator,file}` and `safix.{user,name}`
- [x] 1.2 Make `direction` an enum of `clan-to-safix` and `safix-to-clan`. Record at the option why it is not spelled `import`/`export`: the word moves values in opposite directions across this boundary depending on which tool says it
- [x] 1.3 Five refusals in `bridge.nix`: unresolvable safix side; a clan-to-safix target that also has a generator; two mappings writing one target; one endpoint pair declared in both directions; more than one `clanFlake`. The sixth — a safix-to-clan source with nothing to send — is deleted by 0.3 rather than held, and its runtime sibling is 3.9
- [x] 1.4 Expose the refusals through `checks.nix` as a message function and a builder over it, matching the custody and generator-tool families, and add `safix-bridge-refusals` to `mkChecks`
- [x] 1.5 Record in the option documentation that evaluation does not and cannot verify the clan half, and that a bad clan side is a run-time refusal naming the triple
- [x] 1.6 Severity drill: for each of the six refusals, perturb a fixture fleet and confirm the message names what it should. Run the drill through `refuseScript` so it executes the bytes the real check runs
- [x] 1.7 Verify: `nix flake check` passes and the six drills were observed

## 2. Delegation to clan

- [x] 2.1 Implement the clan subprocess driver in `crates/safix-core/src/bridge.rs`: resolve clan's command on PATH, invoke read with the value captured from standard output, invoke write with the value supplied on standard input
- [x] 2.2 Establish that the read captured raw bytes rather than a terminal rendering. clan's read command substitutes a printable form when its output is a terminal; assert the captured form rather than relying on a subprocess pipe never being one
- [x] 2.3 Implement the absent-command refusal: both verbs refuse before transferring anything, the refusal states that clan is the authority on its own store, and no subset of mappings runs
- [x] 2.4 Surface clan's own failures rather than reinterpreting them — a missing var, an ambiguous id, an ungenerated value each reach the operator as clan's message with safix's mapping name attached
- [x] 2.5 Add the new refusal variants and codes to `crates/safix-core/src/error/`, with paired plain and graphical snapshots
- [x] 2.6 Confirm by search that no clan store layout, backend, recipient handling or file format exists anywhere in the runtime
- [x] 2.7 Verify: `cargo test` passes; the absent-command refusal is observed; and 2.6's search is recorded

## 3. Transfer and convergence

- [x] 3.1 Implement `safix import`: for each clan-to-safix mapping, read both sides, compare, and write through the existing `set` path when they differ
- [x] 3.2 Implement `safix export`: for each safix-to-clan mapping, read both sides, compare, and invoke clan's write only when they differ. Record at the comparison why it is load-bearing rather than an optimisation — clan's write commits unconditionally and a re-encrypting backend produces fresh ciphertext for an unchanged value, so without it every run commits in the clan repository for every mapping
- [x] 3.3 Refuse a mapping whose safix side the operator cannot decrypt, rather than writing it. Use the reasoning `check` already applies to other people's files
- [x] 3.4 Implement the four outcomes — unchanged, updated, absent at source, refused — and hold every report, refusal and commit message to naming no value
- [x] 3.5 Commit per mapping, naming the mapping and the direction. Landed without an `--all` flag: a bare verb converges every mapping of its direction and a mapping id narrows the run to one. A flag would be the only spelling of what the verb is for, and a verb whose bare form does nothing is one an operator has to remember a flag for. The export direction commits nothing here, because nothing in this repository changed — clan commits what it wrote, one invocation per mapping, so the single-intent discipline holds across the boundary rather than being restated on this side of it
- [x] 3.6 Confirm the safix-side write acquires the recipient-drift refusal, the staged write and rename, and the pipe, by driving a drifted fixture through import rather than by inspecting the call
- [x] 3.7 Verify: an integration test runs each verb twice and requires the second run to write nothing and commit nothing
- [x] 3.8 Severity drill: remove the pre-write comparison from export and confirm the idempotency test fails with a commit per mapping per run
- [x] 3.9 Implement 0.3's runtime sibling: export refuses when the source key is absent from the source file, naming the entry, the file, and both remedies. Assert it against a literal beside the evaluation silence it replaces
- [x] 3.10 Implement 0.2's refusal: before writing, ask clan whether the mapping's generator has an outdated recorded validation, and refuse the mapping when it has. Read no recorded hash and compute none. The message names the machine, the generator, and both remedies
- [x] 3.11 Bare `safix import` and `safix export` converge every mapping of their direction, reporting changed, unchanged and failed per mapping; a mapping id narrows the run to one. An unknown id is refused naming what is declared

## 4. The audit

- [ ] 4.1 USER-RUN: decide before building. Adding bridge rows to `check` breaks two properties `check` currently has and documents. `check` answers every question from the structure of the ciphertext and decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not have; comparing a mapping's two sides requires decrypting the safix side. And `check` needs no clan, where a bridge row needs one per mapping. Three shapes are available: bridge rows only for mappings the caller can decrypt, silently skipping the rest; a separate verb; or rows behind a flag. The comparison itself is written and tested — it is the transfer's own — so this is a question about `check`'s contract rather than about the code
- [ ] 4.1a Add bridge rows to `check` in whichever shape 4.1 settles: each mapping whose two sides no longer agree is a finding naming the mapping and no value
- [x] 4.2 Record in the export documentation the condition under which an exported value is silently discarded: changing the clan-side generator's definition invalidates clan's recorded validation, and clan's next routine generation replaces the value. State that a routine generation without a definition change does not, and that an explicit regeneration does and is the operator's action
- [x] 4.3 Confirm by search that nothing writes clan's validation record, and record why: it would mean writing clan's store directly, and the value would be a function of clan's own definition
- [ ] 4.4 Verify: a fixture with a diverged export mapping produces the finding, and one in step produces none

## 5. Testing, the record, and the follow-up

- [x] 5.1 Build the stub clan command as a fixture: answers reads with known bytes, records what writes received on standard input, and can be made to fail in each way the real one does. Record why stubbing clan is permitted where stubbing sops is not — sops is what safix's claims are about, clan is a boundary safix delegates across, and the delegation is what is under test
- [ ] 5.2 Add a check driving the real clan command over a throwaway clan when it is present in the check closure, absent rather than trivially green when it is not, following the shape the platform-conditional check already uses. A miniature clan *was* built with the real CLI outside the sandbox and every contract this change rests on was confirmed against it — see the note at the end of `design.md` — but that clan needs a locked flake, an age identity and a recipient, none of which a build sandbox has. Landing it as a check is its own piece of work
- [x] 5.3 Update `README.md` with the bridge surface, both verbs, and the direction vocabulary
- [x] 5.4 Write the bridge into `CHANGELOG.md`, including the decision recorded in 0.1
- [ ] 5.5 Name the dotfiles follow-up `retire-agents-mirror` in this change's impact: `modules/flake/agents/agents.sh` loses its mirror half — its `sops set --value-stdin` write, its secret-tempfile registry and shredder, its trunk guard and its `MIRROR_SOPS_KEY` table — and keeps its remote provisioning. Do not build it here; it spans two repositories and depends on `safix-full-switch`
- [ ] 5.6 Record question 3 from design's open questions as answered or still open: the ordering constraint that this change lands after `clan-generator-contract`
- [x] 5.7 Verify: `openspec validate clan-bridge --strict` passes
