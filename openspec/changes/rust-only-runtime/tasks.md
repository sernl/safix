# Tasks: rust-only-runtime

Three disciplines hold throughout, unchanged from 0.1.

No real recipient, no real hostname and no real user name enters this repository.
Fixtures are `ana`, `bo` and `cy`, recipients are synthetic `age1` strings, and every key a test decrypts with is minted inside that test's own scratch directory.

Nothing here deploys, switches or activates.
Every verification builds, evaluates, or runs a test against a fixture repository.

No sentence describing a guarantee is written before the code enforcing it exists in the same commit.
Here that bites on the deletions: no task removes a mode before the row of the parity table naming its successor is green, and no changelog sentence about the retired gate is written before the deletion it describes is in the same commit.

The order is not negotiable and is the whole point of the change.
Stage 1 stands up the suite's harness; stage 2 ports the eighteen behavioural claims into it; stage 3 re-homes the four single-runtime claims; stage 4 deletes; stage 5 records.
Stage 4 is unreachable while any row of stage 2 or 3 is open, and each of its tasks names the rows it waits on.

## 1. The integration harness

- [x] 1.1 Create `crates/safix/tests/` with a harness module: a scratch repository builder (git init, fixture `.sops.yaml`, minted age keys), a `nix` stub that answers placement queries and asserts the attribute path it was asked for, and a runner that invokes the built binary and captures stdout, stderr and exit status separately
- [x] 1.2 Give the harness the same backend composition `cli.nix` uses today — real `sops`, real `age`, real `git` — and record at the harness why no second one is stubbed
- [x] 1.3 Add a repository-effect helper that reports, for a run: the paths in the resulting commit, the commit message, and which of a file's ciphertext lines moved. This is the projection the retired harness computed, expressed as an assertion helper rather than a diff input
- [x] 1.4 Wire `checks.safix-integration` in `modules/flake/rust.nix` to run the suite under the same harness inputs, so the tests run in the sandbox and not only in a devshell
- [x] 1.5 Verify: the harness's own self-check runs, mints a key, writes and reads one value through real sops, and the `nix` stub's attribute assertion is observed to fail under a deliberately wrong attribute name

## 2. The eighteen behavioural claims

Each task ports one mode of `safix-selftest.sh` into the suite and asserts against the literal named in the design's parity table.
Each is verified by running the new test and by the drill named beside it, because a test nobody has seen fail is not evidence.

- [x] 2.1 `set-new`: file created through sops, recipients equal the creation rule's, value round-trips, one commit names the secret and not the value. Drill: write straight to the final path instead of the scratch file, and observe the unruled empty file
- [x] 2.2 `set-existing`: one key moves, every other key's ciphertext line is byte-identical, a re-run one second later commits nothing. Drill: drop `--idempotent` and observe the byte-identity assertion fail — and record that the test must wait out a second first, or the drill does not bite
- [x] 2.3 `refusals`: the six refusal conditions, each producing its own code and prose, none writing anything. Drill: remove the `no matching creation rules found` branch
- [x] 2.4 `recipient-drift`: refused before the rename, in both drift directions, naming which side is short
- [x] 2.5 `staged-bystander`: an unrelated staged path survives staged and uncommitted and does not make an idempotent re-run commit. Drill: drop the `-- <path>` scoping from the commit
- [x] 2.6 `abort`: SIGINT at the prompt, and a backend failure after the value was read; neither leaves a partial file, a scratch file or a created directory. Drill: leak the scratch guard
- [x] 2.7 `get-list`: a value round-trips by digest for an own secret and one shared from another owner, and both resolve one file
- [x] 2.8 `generate`: no-input, prompted and dependent generators each mint and commit; the prompt is read unechoed
- [x] 2.9 `generate-refusals`: the five refusal conditions, each with its own code, each leaving nothing written
- [x] 2.10 `generate-isolation`: a script reading standard input to end of input does not consume a later prompt's answer. This is the descriptor hazard the 0.1 proposal names; assert it rather than describe it
- [x] 2.11 `generate-cascade`: `--regenerate` lists the transitive downstream set in dependency order, confirms once, and declining writes nothing
- [x] 2.12 `governed-extras`: a consumer-named file in step with its rule is not a finding; the same file drifted is
- [x] 2.13 `adduser`: one custody record and the regenerated policy are committed; a staged bystander is not
- [x] 2.14 `adduser-refusals`: the four refusal conditions, each named, nothing written
- [x] 2.15 `adduser-hook`: `--host` with no hook is refused naming the hook; with one, the hook receives what it is promised
- [x] 2.16 `shared-placement`: both carriers' placements name one file and one key; one mints, the other reads back what was minted
- [x] 2.17 `shared-shrink`: a dropped carrier is reported as a revocation naming the file and the person
- [x] 2.18 `shared-flip`: flipping to shared over existing values is reported as a migration, not a disclosure
- [x] 2.19 Fill in the parity table in `design.md` with the test name that carries each row, and mark each row green only after the drill beside it was observed red
- [x] 2.20 Verify: `nix build .#checks.<system>.safix-integration` passes, and the eighteen tests are enumerable by name in its output

## 3. The four single-runtime claims

- [x] 3.1 Re-express `differential-abort` as `safix-abort-residue`, driving the rust binary alone and interrupting a write in each window it has
- [x] 3.2 Re-express `differential-pipes` as `safix-value-pipe`, observing the sops process and holding the value to a pipe
- [x] 3.3 Re-express `differential-strace` as `safix-syscall-proof`, keeping the linux-only condition and the non-linux placeholder that says what it did not do rather than passing silently
- [x] 3.4 Re-express `differential-drills` as `safix-channel-drills`, mutating the rust runtime once per channel and failing unless each mutation is caught. Retain every channel the retired version covered, and add the exit-status channel, which the comparative form got for free and the single-runtime form must assert
- [x] 3.5 Verify: each of the four builds and passes, and `safix-channel-drills` is observed to fail when one of its mutations is neutralized

## 4. The deletions

Nothing in this stage is reachable while any task in stages 2 or 3 is open.
Each task names the rows it waits on.

- [x] 4.1 Waits on 2.1–2.20. Delete `modules/flake/safix/safix-selftest.sh`, and repoint the eighteen `checks.safix-*` attributes in `modules/flake/checks/cli.nix` at the integration suite, keeping their names so a consumer's CI does not silently stop running a check
- [x] 4.2 Waits on 3.1–3.5. Delete `modules/flake/safix/safix-differential.sh` and reduce `modules/flake/checks/differential.nix` to the four surviving checks, renaming the file to reflect that it no longer compares anything
- [x] 4.3 Waits on 4.1 and 4.2. Delete `modules/flake/safix/safix.sh` and `modules/flake/safix/package.nix`, and remove `packages.safix-sh` from the flake's package set
- [x] 4.4 Waits on 4.3. Delete `modules/flake/safix/sops_recipients.py`, `modules/flake/safix/sops_keys.py` and `modules/flake/safix/readers.nix`, and remove the readers from every check's `nativeBuildInputs`
- [x] 4.5 Update the module doc comment at the head of `crates/safix-core/src/sops/document.rs`: it currently says the python readers "remain the differential oracle", which stops being true in 4.4 and must not outlive it
- [x] 4.6 Remove `bash` and `util-linux` from the check harnesses if nothing else needs them, and record which check still does if one does
- [x] 4.7 Verify: `nix flake check` passes; `nix eval .#packages.<system> --apply builtins.attrNames` does not contain `safix-sh`; and no python derivation appears in `nix path-info -r` over the check closures

## 5. The record

- [x] 5.1 Write the retirement into `CHANGELOG.md`: the gate was green across every subcommand, at which commit, over which nineteen modes, and that the harness is reachable in history rather than in the tree
- [x] 5.2 Rewrite the `Known differences` section in the single-runtime voice — each entry becomes what the rust runtime does, with the shell behaviour it diverged from named as history. These are decisions rather than observations and are the one part of the harness whose content outlives it
- [x] 5.3 Update `CONTRIBUTING.md`, which references `safix-selftest`, to describe the integration suite and how to run one test of it locally
- [x] 5.4 Record in `design.md` the scripting survivors as finally landed, with any survivor discovered during the work added to the table with its own justification
- [ ] 5.5 Surface, rather than fix, the two dotfiles python files (`modules/flake/secrets/sops_recipients.py`, `modules/flake/secrets/sops-recipients-check.py`): they are the same tooling under the same rule and are outside this repository. Name them in `safix-full-switch`'s impact and ask the operator whether they route through the safix bridge or are deleted with the vocabulary
- [x] 5.6 Verify: `openspec validate rust-only-runtime --strict` passes and the parity table has no open row

5.5 is open because it is an edit to a change in the dotfiles repository, and the work that landed the rest of this change was not authorized to write there.
The two files are `modules/flake/secrets/sops_recipients.py` and `modules/flake/secrets/sops-recipients-check.py`.
They are the same tooling by the same test — a wrong answer from either decides whether a write is refused — and the rule that deleted this repository's two copies reaches them.
The routing question is the operator's: through the safix bridge, or deleted with the vocabulary they serve.
