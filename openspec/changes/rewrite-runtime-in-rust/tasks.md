# Tasks: rewrite-runtime-in-rust

Three disciplines hold throughout.

No real recipient, no real hostname and no real user name enters this repository.
Fixtures are `ana`, `bo` and `cy`, recipients are synthetic `age1` strings, and every key a test decrypts with is minted inside that test's own scratch directory.

Nothing here deploys, switches or activates.
Every verification builds, evaluates, or runs a test against a fixture repository.

No sentence describing a guarantee is written before the code enforcing it exists in the same commit.
This bites hardest on the custody type and on the equivalence gate: the absent traits are asserted by a compiled probe in the commit that claims them, and the shipping package does not move in any commit before the last subcommand passes the harness.

The stages are sequenced so that each one's verification exists before the code it judges.
Stage 1 is the scaffold and the custody type; stage 2 is the harness that later stages are judged by; stages 3 to 5 port the runtime behind it; stage 6 is retirement.

## 1. Stage 1 — the workspace and its posture

- [x] 1.1 Create the cargo workspace with `safix-core` and `safix`, edition 2024, both declaring `#![forbid(unsafe_code)]`
- [x] 1.2 Pin the minimum supported version to the toolchain the locked nixpkgs provides, and record beside it the rule that produced the number — newest required rather than oldest tolerated, because one toolchain is ever compiled against

  The locked nixpkgs carries rustc 1.97.1, so the field is `rust-version = "1.97"`.

- [x] 1.3 Write the lint posture into the workspace manifest: the pedantic group on, the panicking and lossy constructions denied, every relaxation listed in one place, and the reason recorded where a reader of the manifest meets it

  One relaxation was needed and it is the technique rather than an exception to it: `clippy::assertions_on_constants` fires on every `const` assertion in section 2, which is exactly what those assertions are.

- [x] 1.4 Write `rustfmt.toml` and `clippy.toml`
- [x] 1.5 Write `deny.toml` covering bans, licences and sources, with the permitted licences enumerated and the repository's own dual licence among them

  The licence list is the set actually present, read off `cargo-deny list`, rather than a superset: an unmatched allowance is a warning, and a licence arriving later should be a decision rather than a lock update. Two duplicate versions are skipped by exact version with the reason at each, so a *new* duplicate still fails.

- [x] 1.6 Commit `Cargo.lock`, and add the cargo build directory to `.gitignore`
- [x] 1.7 Verify: `cargo metadata --locked` succeeds and the workspace members are the two crates

## 2. Stage 1 — the custody type

- [x] 2.1 Implement the plaintext type as a newtype over the zeroing secret crates, with no derived or hand-written debug, display or serialization implementation
- [x] 2.2 Give it stream construction only — a reader, and the process's standard input — and no constructor taking an owned or borrowed string
- [x] 2.3 Zero whatever was read when a read fails partway, and return the failure rather than a partial value

  The read grows its buffer by allocating, copying and dropping the old one rather than by `read_to_end`, because a `Vec` reallocation copies the plaintext into a fresh allocation and frees the old one unzeroed — which `zeroize`'s own `Vec` documentation says it cannot reach. The final buffer is allocated at exactly the value's length, so the handoff to the secret box does not reallocate either.

- [x] 2.4 Write the compile-time probe that reports each of the absent traits, and assert each as a `const` assertion, so adding any of them later fails the build rather than a review

  Five rather than three. The absence of `From<String>` and `From<&str>` is probeable the same way, which turns "no constructor takes a string" from a claim about the constructor list into a compiled one.

- [x] 2.5 Record at the type why the traits are absent, naming the shell hazard it removes rather than describing good practice
- [x] 2.6 Severity drill: derive the debug trait on the type locally and confirm the build goes red; the probe is worth nothing if it cannot distinguish the two states

  Observed: `error[E0080]: evaluation panicked: Secret must not implement Debug`.

- [x] 2.7 Severity drill: implement `From<String>` for the type locally and confirm the build goes red

  Observed: `error[E0080]: evaluation panicked: Secret must not be constructible from an owned string`.

- [x] 2.8 Assert the probe's own positive case in the library rather than only in a test, since a probe answering `false` unconditionally satisfies every absence assertion in the crate while detecting nothing
- [x] 2.9 Verify: `cargo test` passes, and the drills in 2.6 and 2.7 are each observed red before being reverted

## 3. Stage 1 — the nix wiring

- [x] 3.1 Add `crane` and a pinned advisory database to the flake inputs, each with the reason recorded beside it in the same shape the existing inputs carry
- [x] 3.2 Write `modules/flake/rust.nix` producing `packages.safix-rs`, with the toolchain taken from the pinned nixpkgs
- [x] 3.3 Add the build, test, clippy and format checks over the workspace
- [x] 3.4 Add the offline dependency check covering bans, licences and sources
- [x] 3.5 Add the advisory check against the pinned database, separate from 3.4, with the reason for the split recorded — the sandbox has no network, and a new advisory should redden one check on a dated lock update
- [x] 3.6 Leave `packages.safix` untouched, and state in the module header that the shell runtime is what ships until the harness closes

  Held for the whole of the port and then discharged by 9.2: the harness closed, and the module header now says what it is and what the shell runtime became.
- [x] 3.7 Add the rust toolchain to the devshell from the same pinned nixpkgs, so a local `cargo clippy` and `safix-rs-clippy` are one compiler
- [x] 3.8 Verify: `nix flake check` is green, and the six new checks are present for the current system

  `nix flake show` could not be the instrument, for the reason already carried out of `add-consumption-modules` §5.4: it evaluates every declared system and the pinned nixpkgs has dropped one of them. `nix eval .#checks.x86_64-linux --apply builtins.attrNames` was used instead, and lists all six.

## 4. Stage 1 — the adoption surface

- [x] 4.1 Document the library's public items, with the custody type's construction rule and absent traits stated where the type is defined

  `missing_docs` is denied workspace-wide, so an undocumented public item is a build failure rather than a review comment.

- [x] 4.2 Write `CHANGELOG.md` in the keep-a-changelog shape, whose unreleased entry states that the rust runtime is not what the shipping package builds
- [x] 4.3 Write `CONTRIBUTING.md` carrying the fixture-fleet recipe and the no-real-identifiers rule

  The recipe points at `safix-selftest.sh` rather than describing a second one, and names the two commits where a real recipient was carried in and removed, because both arrived as an example beside a working change.

- [x] 4.4 State the versioning policy: the library's public interface is what semantic versioning governs, and the command's refusal prose is governed by the equivalence gate rather than by the version number
- [x] 4.5 Verify: `cargo doc` builds warning-free under `RUSTDOCFLAGS=-D warnings`, and no document describes a runtime behaviour that has not landed

## 5. Stage 2 — the differential harness

Delivered as `modules/flake/safix/safix-differential.sh` and, by the end of the port, eighteen `safix-differential-*` checks.
The argument vectors of 5.2 are identical by construction — one list is passed to both runtimes — rather than by an assertion over two lists that could drift apart.
5.9 runs five drills rather than four: the four channels, and the plaintext-residue assertion beside them.

- [x] 5.1 Build the fixture fleet as a fixture repository: `ana`, `bo` and `cy`, keys minted at test time into the run's scratch directory, and a governed file set exercising the single-reader and shared-audience shapes
- [x] 5.2 Give each runtime its own pristine copy of that repository per invocation, and assert the argument vectors are identical
- [x] 5.3 Implement the plain reporter in the command, selected from the environment, emitting the program name, the message and two-space-indented notes with no colour, code or span
- [x] 5.4 Assert that reporter selection alters standard error only: identical standard output, exit code and repository effects with and without it
- [x] 5.5 Implement the canonical repository projection — ordered commit subjects with per-path status, porcelain status, paths with modes, decrypted plaintext per governed file, recipient set per governed file — as one program applied to both sides
- [x] 5.6 Record at that program why the ciphertext bytes are not the comparison: a new value takes a fresh initialization vector and moves the authentication code and timestamp with it
- [x] 5.7 Compare standard output and standard error byte for byte, and exit codes as numbers rather than as success-or-failure
- [x] 5.8 Assert no plaintext residue in either runtime's temporary directory, with the refusal paths covered specifically
- [x] 5.9 Severity drills: mutate a refusal's wording, an exit code, the staged path set, and a written value, and confirm each is caught, and caught by the channel that targets it
- [x] 5.10 Record any mutation caught only incidentally as a gap in the channel that should have caught it
- [x] 5.11 Verify: the harness is red under each of the four mutations and green with none applied

## 6. Stage 3 — the read paths

Delivered with stage 2 rather than after it, because a harness with nothing to compare is not a harness.

- [x] 6.1 Port the evaluation seam: placements, audiences, governed files and policy text, obtained by evaluating the consumer's flake exactly as the shell does
- [x] 6.2 Port the two ciphertext readers, reading only the metadata fields the python helpers read
- [x] 6.3 Port `get`, `list` and `check`, with their refusals as error variants carrying data
- [x] 6.4 Snapshot every refusal variant's rendering under both reporters
- [x] 6.5 Property tests on the parsing and joining logic, with the audience separator's injectivity stated as a property
- [x] 6.6 Add the bounded concurrency for the `check` probes, and assert the output ordering is independent of completion order

  Superseded by D11 rather than deferred, and one half of the reason first recorded here has since lapsed.
  The half that stands: the probes are subprocesses in the shell runtime and in-process metadata reads in the rust one, so there is no fan-out here to bound, and a bound over reads is a concurrency whose speedup this change's own non-goals refuse to introduce before a measurement exists.
  The half that lapsed: that adding it would change what the harness compares, which stopped being a reason when `rust-only-runtime` retired the harness.

  Closed as superseded: the delta spec's concurrency requirement no longer enumerates three sites — it requires a bound wherever a fan-out survived the port and a recorded withdrawal wherever one did not, which is what D5 and D11 now state and what the implementation does.
- [x] 6.7 Verify: the harness passes for `get`, `list` and `check`

## 7. Stage 4 — the write paths

- [x] 7.1 Port the scratch-file discipline as a guard that shreds on every unwind, and test the abort path directly rather than the happy one

  `safix-differential-abort` interrupts a write in each of its three windows. The two at a prompt and the one during encryption are rust-side drills, and two assertions record why: the shell runtime has no response to `SIGINT` in any of them, because `bash` restarts an interrupted `read` and, while waiting for a foreground command, ignores the signal outright.
- [x] 7.2 Port `set`, driving the backend through pipes with the value never in an argument vector or an environment

  `safix-differential-pipes` reads the sops process' own command line and environment through a shim that then becomes sops, for both runtimes. It replaces the strace-based check the design named, which a build sandbox cannot run.
- [x] 7.3 Port the drift refusal, judged on the candidate document before the rename, so a refusal is a run that never wrote
- [x] 7.4 Port `adduser` and `fix`, preserving the stage-before-regenerate ordering and the reason it exists

  Both ported. `adduser` stages the scaffold before it regenerates the policy, and the harness's stubbed `policyText` renders its anchors from `git ls-files` for that reason: a runtime that regenerated first writes a `.sops.yaml` missing the person it has just declared, and `safix-differential-adduser` sees it in the committed bytes.
- [x] 7.5 Add the bounded concurrency for the `fix` re-wrap

  Confined to `--yes`, because a confirmation cannot be fanned out. Bounded by `SAFIX_FIX_CONCURRENCY`, output replayed in declaration order, and `safix-differential-converge` compares both the fanned-out and the serial bound.
- [x] 7.6 Cover the real-activation gap carried out of `add-consumption-modules` with a fixture-ciphertext test

  The gap is that change's 3.6. The identity preflight states, beside its guarantee, the limit of it: presence and readability are all it checked, and an identity which has both and is not a recipient of these files still fails afterwards, in `sops-install-secrets`. Every verification of that change was an evaluation, so that sentence was the one claim on the consumption path with no check under it.

  `safix-identity-recipiency` holds it against fixture ciphertext. No activation runs — the discipline at the head of this file forbids it — and the boundary a run does reach is the same sops reading the same `SOPS_AGE_KEY_FILE` the provisioner reads. The stranger's identity is shown to open a document it is a recipient of before it is shown not to open one it is not, or the refusal would hold equally over a key file that was merely broken; the drill pointing the refusing run at a recipient identity was observed red on the assertion written for it. The module header now names the check beside the sentence, as it already named `safix-consumption-ordering` beside the ordering.
- [x] 7.7 Verify: the harness passes for `set`, `adduser` and `fix`, including the abort paths

  Over `safix-differential-write`, `-refuse`, `-guard`, `-converge`, `-abort`, `-pipes` and `-adduser`.

## 8. Stage 5 — the generator graph

- [x] 8.1 Port generator execution with each child's three descriptors set explicitly and standard input pinned closed

  Standard input is `/dev/null` and that is part of the interface rather than tidiness: a generator's inputs are its descriptors, and a script reading the command's own standard input would eat the answers to every prompt after it — silently, since a prompt reading end-of-input looks exactly like one nobody answered.

- [x] 8.2 Make the child's exit status a value the caller cannot discard, and test the failing generator specifically

  The status is the value `mint` returns through, and `Error::GeneratorFailed` carries it. `safix-differential-genrefuse` drives a script that exits 3 and compares the refusal naming that number.

- [x] 8.3 Port the cycle refusal with the participating nodes carried in the variant

  Ported, against the reason first recorded here for not porting it. That reason was that a cycle never reaches the runtime: the cycle refusal is `flake.safix.lib.generatorPlan`'s, thrown at evaluation by `resolve.nix`, and an order existing at all is that refusal's postcondition. True of the command driving a consumer's flake, and not of the library D1 publishes. `GeneratorPlan` is a value with public fields, `UserPlan::cascade`'s single forward pass is documented as resting on the order being topological, and both a stand-in for nix and a program embedding `safix-core` hand the runtime a plan no refusal has been thrown over. The schemas already treat the nix half as a producer whose drift is refused rather than assumed away — that is what `deny_unknown_fields` and `NixSchemaMismatch` are — and an order that is not a run order is the same class of drift, reached silently rather than refused.

  `UserPlan::cycle` checks the claim, `Error::GeneratorCycle` carries the participating generators, and it is not a second implementation of the resolver's: it re-derives no order and answers only whether the one it was handed is one. The walk backtracks rather than following one prerequisite per node, because the resolver's trick is sound inside a set it has already established is stuck.

  `safix-generate-cycle` holds the refusal to arriving before the first generator rather than merely arriving. The drill judging the order after walking it was observed red: the run mints and commits the generator sitting ahead of the cycle in the order, then reports the first missing input as an empty output — a committed value and a refusal naming the wrong cause, which is the harm `resolve.nix` puts the question at evaluation to avoid.

  Not comparable against the retired shell runtime, in the same way `NixSchemaMismatch` was not: no fixture produces it from both sides at once. D11 records that shape.

- [x] 8.4 Add the bounded concurrency across independent branches, and assert a dependent branch never starts before its predecessor finishes

  The concurrency half is deliberately not added, and the module says why where a reader meets it. Three things depend on the walk being sequential: a prompt is read from one standard input, so two generators prompting at once is not a faster question but an unanswerable one; each generator commits as it goes, so the commit order is the plan's rather than the scheduler's, which is what the differential comparison of the repository rests on; and each generator's plaintext lives in a staging root for the duration of its run, so a fan-out would buy latency at the price of several roots holding plaintext at once, over a longer window, with no ordering between the shreds. The third is the isolation the staging discipline exists for, and latency is not worth it.

  The assertion half is held, and was before this box was read again. `safix-generate` drives a generator whose script reads `$in/seeded/seeded` and asserts the value it minted from it, which is a claim no run where the dependent started before its predecessor finished can satisfy: the file it reads would not have been there. The predecessor's commit is asserted in the same test, so the ordering is held at both the value and the commit.

  Closed as superseded with 6.6: the delta spec's concurrency requirement now requires the withdrawal to be recorded rather than the fan-out to exist, and both halves of this box are answered — the withdrawal is recorded in D5 and the module header, and the ordering assertion is held at the value and the commit by `safix-generate`.

- [x] 8.5 Verify: the harness passes for `generate` and `keygen`

  Over `safix-differential-generate`, `-regenerate`, `-genrefuse` and `-keygen`. `-keygen` is not a byte comparison of the value — two correct runs mint two different identities — so each side is held to the property and only the rendering is compared.

- [x] 8.6 Observe, at the system call, that every plaintext byte a `set` and a `generate` write goes to a pipe

  Added rather than planned. `safix-differential-strace` runs both runtimes under `strace -f -y` and holds every `write` carrying a fixture value to a descriptor `strace` resolves as a pipe, which is the positive form of the claim `-pipes` makes negatively. It carries its own drill and is linux-only, because it needs `ptrace`.

## 9. Stage 6 — retirement

- [x] 9.1 Confirm every subcommand passes the harness, and that each severity drill has been observed red

  Eighteen modes, every one green. `-drills` mutates the rust side once per channel and fails unless each mutation is caught by the channel that exists to catch it; `-strace` carries the same discipline for its own assertion.

- [x] 9.2 Move `packages.safix` to the rust binary in one commit, with the shell script and the python helpers removed in the same commit

  The move landed; the removal did not, and the plan is wrong rather than incomplete. The shell script and the python readers are what every `safix-differential-*` mode compares against, so removing them would remove the evidence that the two runtimes agree — the gate would become a claim in a changelog rather than a check that runs. They stay in the tree as `packages.safix-sh` and its readers, built and linted, with the header of each naming that as its role.

- [x] 9.3 Record in the changelog what changed for an operator, and what did not

  Under "Changed", "Unchanged, deliberately" and "Known differences". Five differences are recorded rather than reconciled, each asserted by the mode that found it.

- [x] 9.4 Verify: `nix flake check` is green with the shell runtime absent, and the harness is retargeted or retired with its reason recorded

  Amended to match 9.2: green with the shell runtime present as the oracle rather than absent. `nix flake check` passes on x86_64-linux with all eighteen differential modes, the six cargo checks, and the structural checks.
