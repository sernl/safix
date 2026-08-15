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

Delivered as `modules/flake/safix/safix-differential.sh` and the seven `safix-differential-*` checks.
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
- [ ] 6.6 Add the bounded concurrency for the `check` probes, and assert the output ordering is independent of completion order

  Superseded by D11 rather than deferred. The probes are subprocesses in the shell runtime and in-process reads in the rust one, so there is no fan-out here to bound; adding concurrency would be a change to what the harness compares rather than an optimisation under it.
- [x] 6.7 Verify: the harness passes for `get`, `list` and `check`

## 7. Stage 4 — the write paths

- [ ] 7.1 Port the scratch-file discipline as a guard that shreds on every unwind, and test the abort path directly rather than the happy one
- [ ] 7.2 Port `set`, driving the backend through pipes with the value never in an argument vector or an environment
- [ ] 7.3 Port the drift refusal, judged on the candidate document before the rename, so a refusal is a run that never wrote
- [ ] 7.4 Port `adduser` and `fix`, preserving the stage-before-regenerate ordering and the reason it exists
- [ ] 7.5 Add the bounded concurrency for the `fix` re-wrap
- [ ] 7.6 Cover the real-activation gap carried out of `add-consumption-modules` with a fixture-ciphertext test
- [ ] 7.7 Verify: the harness passes for `set`, `adduser` and `fix`, including the abort paths

## 8. Stage 5 — the generator graph

- [ ] 8.1 Port generator execution with each child's three descriptors set explicitly and standard input pinned closed
- [ ] 8.2 Make the child's exit status a value the caller cannot discard, and test the failing generator specifically
- [ ] 8.3 Port the cycle refusal with the participating nodes carried in the variant
- [ ] 8.4 Add the bounded concurrency across independent branches, and assert a dependent branch never starts before its predecessor finishes
- [ ] 8.5 Verify: the harness passes for `generate` and `keygen`

## 9. Stage 6 — retirement

- [ ] 9.1 Confirm every subcommand passes the harness, and that each severity drill has been observed red
- [ ] 9.2 Move `packages.safix` to the rust binary in one commit, with the shell script and the python helpers removed in the same commit
- [ ] 9.3 Record in the changelog what changed for an operator, and what did not
- [ ] 9.4 Verify: `nix flake check` is green with the shell runtime absent, and the harness is retargeted or retired with its reason recorded
