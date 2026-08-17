# Design: retiring the other two languages without retiring the evidence

## The problem this change actually has

Deleting five files is not the hard part.
The hard part is that the checks proving safix behaves are attached to the files being deleted, and this was not visible from the 0.1 artifacts.

`modules/flake/checks/cli.nix:143` reads:

```
SAFIX_SH=${../safix/safix.sh} bash ${../safix/safix-selftest.sh} ${mode}
```

There is no `SAFIX_RS` on that line and none anywhere else in `cli.nix`.
The eighteen checks that file defines — the ones whose comments carry this project's most careful reasoning about what a secret tool must not do — drive the shell script.
`SAFIX_RS` occurs once in the tree, at `differential.nix:73`, where it is one of two runtimes under comparison.

The consequence is stated plainly because it governs everything below.
Today, if the shell files were deleted and nothing else changed, the rust binary that operators run would be covered by sixty-six in-crate unit tests and zero integration tests.
There is no `crates/safix/tests/` and no `crates/safix-core/tests/`.
The claim "a run which aborts leaves neither a partial file nor a plaintext value behind" would have no executable form anywhere in the repository.

So this change is a test-authoring change with a deletion at the end of it.

## D1. The differential gate is retired honestly, not quietly

The retirement note in `package.nix` argues the oracle must stay because "retiring it would retire the evidence that the two agree".

That reasoning does not survive examination, and the reason it does not is worth writing down.
A differential test's evidence is indexed by time.
It established that at commit `8409f15`, over that fixture fleet, the two runtimes agreed on every compared channel for every subcommand.
That is a fact about a past state of the tree, and facts about past states are what version control holds.
Keeping the oracle alive does not preserve that fact; it produces a *new* fact on each run, about a pair of runtimes only one of which anyone uses.

The thing the gate genuinely bought — that a rust implementation written from a specification could not quietly drift from the behaviour the specification was reverse-engineered from — was spent when the port completed.
It cannot be spent twice.

What is therefore recorded, and where:

- `CHANGELOG.md` states that the gate was green across every subcommand at the merge, names the commit, and names the nineteen modes.
- The `Known differences` section, which today enumerates the places the two runtimes were pinned apart rather than held to agreeing, is rewritten as statements about the rust runtime with the shell divergence named as history. This is the one part of the differential harness whose *content* is load-bearing beyond the moment: those entries are decisions, not observations.
- Git history at the merge holds the harness itself, and this design names the revision so a reader is not left to search.

What is not done: the harness is not kept "just in case", and no `packages.safix-sh` survives under a deprecated name.
A retained oracle that nobody re-derives against is a 2149-line liability that appears in every dependency audit.

## D2. Parity is per-mode, itemized, and gated

The rule the operator's requirement implies but does not state: a claim may not be deleted from one place before it exists in another.
This is made operational as a table, and each deletion task in `tasks.md` names the rows it depends on.

Column meanings.
*Rust coverage today* is what exists in the tree right now, honestly characterized — unit coverage of a function that participates in the claim is not coverage of the claim.
*Port* says what the integration test must assert, phrased as the literal oracle it asserts against, because a test that re-derives its expectation through the production path proves nothing.

### The eighteen behavioural modes (`safix-selftest.sh`, driven by `cli.nix`)

| Mode | Rust coverage before this change | Port: the literal the test asserts against | Carried by |
|---|---|---|---|
| `set-new` | `set.rs` unit tests (3) cover path resolution, not the write | the file exists, `sops -d` yields exactly the bytes written, the recipients equal the creation rule's, one commit names the secret and not the value | `write_path::set_new_creates_the_file_through_the_creation_rules` |
| `set-existing` | none end-to-end | every other key's ciphertext line is byte-identical after the write, and a re-run one second later changes nothing | `write_path::set_existing_moves_one_key_and_leaves_the_others_byte_identical` |
| `refusals` | `error/` unit tests (8) cover variants and codes, not the conditions | each of the six refusal conditions produces its own code and its own prose, and no file is written under any of them | `write_path::refusals_each_have_their_own_code_and_leave_the_tree_alone` |
| `recipient-drift` | `check.rs` unit tests (6) cover the diff, not the pre-write refusal | a drifted file is refused before the rename, in both drift directions, naming which side is short | `write_path::recipient_drift_is_refused_before_anything_is_written` |
| `staged-bystander` | none | an unrelated staged path survives the run staged and uncommitted, and does not make an idempotent re-run commit | `write_path::a_staged_bystander_survives_the_run_and_does_not_make_it_commit` |
| `abort` | `scratch.rs` unit tests (3) cover the guard's drop | after a SIGINT at the prompt and after a backend failure past the read: no partial file, no scratch file, no created directory | `write_path::an_aborted_run_leaves_no_file_no_scratch_and_no_value` |
| `get-list` | `table.rs` unit tests (8) cover rendering | a value round-trips by digest for an own secret and for one shared from another owner, and both resolve one file | `read_path::get_round_trips_a_value_and_list_reports_where_it_lives` |
| `generate` | none end-to-end | no-input, prompted and dependent generators each mint and commit; the prompt is read unechoed | `generators::generate_mints_in_dependency_order_and_commits_each_generator` |
| `generate-refusals` | none | five refusal conditions, each with its own code, each leaving nothing written | `generators::generate_refusals_each_have_their_own_code_and_write_nothing` |
| `generate-isolation` | `inputs.rs` unit tests (4) cover descriptor construction | a script reading standard input to end of input does not consume a later prompt's answer | `generators::one_generator_sees_neither_the_stdin_nor_the_descriptors_of_another` |
| `generate-cascade` | none | `--regenerate` lists the transitive downstream set in dependency order, confirms once, and declining writes nothing | `generators::a_rotation_carries_its_downstream_set_and_nothing_else` |
| `governed-extras` | none | a consumer-named file in step with its rule is not a finding; the same file drifted is | `read_path::a_governed_extra_is_held_to_its_rule_and_not_to_the_declarations` |
| `adduser` | `adduser.rs` unit tests (7) cover the record and the nix rendering | one custody record and the regenerated policy are committed, and a staged bystander is not | `custody::adduser_commits_the_scaffold_and_the_policy_that_saw_it` |
| `adduser-refusals` | `adduser.rs` covers name validation | four refusal conditions, each named, nothing written | `custody::adduser_refusals_leave_the_tree_as_they_found_it` |
| `adduser-hook` | none | `--host` with no hook configured is refused naming the hook; with one, the hook receives what it is promised | `custody::host_attachment_is_refused_without_a_hook_and_handed_to_one_after_the_commit` |
| `shared-placement` | `model.rs` unit tests (12) cover audience resolution | both carriers' placements name one file and one key; one mints, the other reads back what was minted | `shared_entries::both_carriers_resolve_one_file_and_read_one_value` |
| `shared-shrink` | none | a dropped carrier is reported as a revocation naming the file and the person | `shared_entries::a_dropped_carrier_is_reported_as_a_revocation_naming_the_file_and_the_person` |
| `shared-flip` | none | flipping to shared over existing values is reported as a migration, not a disclosure | `shared_entries::a_flip_to_shared_over_existing_values_is_reported_as_a_migration` |

Every row's coverage before this change was "none end-to-end".
That was the finding, and no row was marked done by pointing at the unit tests in column two.

Every row is now carried, and the eighteen attributes in `modules/flake/checks/cli.nix` run exactly the test named beside them.
The runner reads the result line rather than the exit status, because libtest exits zero having run nothing when a filter names no test; pointing `safix-set-new` at a name that does not exist was observed to fail with "no test ran" rather than passing.

One claim outside these eighteen lost its only pin in the triage below and was rewritten rather than dropped: `safix --version`, which `differential-unknown` held and which `read_path::version_is_answered_on_standard_output_and_is_not_an_unknown_subcommand` now holds.

### The nineteen comparative modes (`safix-differential.sh`)

Triage rather than port.
A comparative mode whose claim is "both runtimes said the same thing about X" has no successor once there is one runtime; the claim about X itself lives in the behavioural row above.

| Mode | Disposition |
|---|---|
| `clean`, `missing`, `drift`, `orphan`, `unknown`, `norule` | die with the oracle; the read-path claims are `get-list`, `refusals` and `recipient-drift` above |
| `write`, `refuse`, `guard`, `converge` | die; the write-path claims are `set-new`, `set-existing`, `staged-bystander` and `refusals` |
| `generate`, `regenerate`, `genrefuse` | die; claims are `generate`, `generate-cascade`, `generate-refusals` |
| `keygen`, `adduser` | die; `keygen.rs` unit tests plus the `adduser` behavioural rows |
| `abort` | **survived** as `safix-abort-residue`, carried by `abort_residue.rs` (4 tests). It was never a comparison — it interrupts a write in each window it has and holds the run to leaving nothing behind. | `write_path::an_aborted_run_leaves_no_file_no_scratch_and_no_value` |
| `pipes` | **survived** as `safix-value-pipe`, carried by `value_pipe.rs` (3 tests). Also never a comparison: it observes the sops process and holds the value to travelling down a pipe and no other way. |
| `strace` | **survived** as `safix-syscall-proof`, carried by `syscall_proof.rs` (3 tests), linux-only for the same ptrace reason. The non-linux placeholder moved from a derivation that said it observed nothing into the suite itself, so the attribute exists on both platforms and the test says what it did not do. |
| `drills` | **survived** as `safix-channel-drills`, carried by `channel_drills.rs` (5 tests). It mutates the runtime on purpose, once per channel, and fails unless each mutation is caught. This is the severity evidence for the whole suite and is the last thing that should be deleted, not the first. |

`drills` surviving is the load-bearing decision here.
Without it the new integration suite is eighteen assertions nobody has shown can fail, which is the failure mode a green suite is best at hiding.

## D3. Tooling versus scripting, made decidable

The operator's rule carries an exemption — "unless it's simply for scripting" — and an exemption without a test is an exemption that swallows the rule.

The test used here: *does an operator's secret depend on this code being right?*

Tooling is code on the path between an operator's intent and a secret's ciphertext, or between a ciphertext and a claim about it.
Scripting is code that arranges for tooling to run, or that constructs a fixture the tooling is then judged against.
The distinguishing property is that scripting has no privileged relationship to plaintext and no authority over what is asserted: if it is wrong, a build fails or a fixture is malformed, and the failure is loud.

Under that test, `safix.sh` is tooling — it read plaintext.
`sops_recipients.py` is tooling — its answer decided whether a write was refused.
The survivors, each named individually:

| Survivor | Why it is scripting |
|---|---|
| `.github/workflows/check.yml` | CI orchestration. Names which nix commands run on which runner. No relationship to plaintext; a mistake fails the workflow. |
| `modules/flake/devshell.nix` | A package list. Contains no script text at all today, and the exemption is recorded so it stays that way. |
| `modules/flake/safix/checks.nix` `refuseScript` | A six-line `writeShellScript` that exits non-zero while a message file is non-empty. It asserts nothing itself — the messages are computed in nix — and it is deliberately shared so a severity drill runs the same bytes the real check runs. |
| `modules/flake/checks/mk-structural-check.nix` and the fixture builders under `modules/flake/checks/` | Inline `runCommand` text that assembles fixture repositories. They construct the subject; the rust suite makes the claims. A malformed fixture fails loudly at build. |
| `crates/safix/tests/` fixture setup shelling out to `git`, `age`, `sops` | Arranging real backends over a throwaway repository. The assertions are rust. |
| `modules/flake/checks/integration.nix` `runOne` | Inline `runCommand` text that invokes one test binary and reads its result line. It arranges for the rust assertions to run and asserts nothing about a secret. Its one judgment — whether a test actually ran — exists because libtest exits zero over a filter that named nothing, and it fails closed: a wrong answer here reddens a check rather than greening one. |
| `modules/flake/rust.nix` `installPhaseCommand` | Two `jq` expressions selecting the compiled binaries out of cargo's own artifact messages, and four existence assertions over the result. A mistake fails the build. |
| The operator-authored `generator.script` and `generator.validation` fragments | Not safix's code. They are data safix executes on the operator's behalf, and making them rust would be making safix a compiler. |

Two survivors this change *discovered* are on that list rather than exempted as a class: the check runner and the artifact-installing install phase, both added when the eighteen attributes were repointed.
Two things it might have written in shell it wrote in rust instead, for the rule's own reason — each decides whether an assertion is shown to fail, which puts it on the tooling side of the test: `crates/safix/tests/support/nix-stub.rs`, which asserts the attribute path the runtime asked for, and `crates/safix/tests/support/shim.rs`, which records what sops was handed, interrupts the runtime from inside a window, and damages one channel at a time.
The retired harness spelled all three as shell functions.

Two files are deliberately *not* on this list and must be surfaced rather than assumed: `modules/flake/secrets/sops_recipients.py` and `modules/flake/secrets/sops-recipients-check.py` live in the dotfiles repository, not here.
They are the same tooling by the same test and the same rule reaches them, but they are outside this change's repository and are named in `safix-full-switch`'s impact as an open routing question.

## D4. What the integration suite is allowed to stub

One thing: `nix`.
A flake evaluation is what a build sandbox cannot do, and the stub also asserts the attribute name the command reads, so renaming `flake.safix.lib.placements` fails in the suite rather than at an operator's terminal.

Nothing else.
Real sops, real age, real git, keys minted inside each test's own scratch directory.
The reasoning is `cli.nix`'s and it is adopted verbatim: standing a stub in for sops is what lets a check stay green over a command calling something the tree no longer contains.

## D5. Where the suite lives, and why not `safix-core`

`crates/safix/tests/`, driving the built binary as a subprocess.

The alternative — library-level integration tests in `safix-core/tests/` — would test the library and leave the binary's argument parsing, its terminal interaction and its exit codes uncovered, and three of the eighteen modes (`refusals`, `adduser-refusals`, `generate-refusals`) are exactly about what an operator sees and what the process exits with.
`safix-core` keeps its unit tests where they are.

## Risks

The suite is written against the rust runtime's *current* behaviour, which is the behaviour the differential gate certified.
If a port reveals that a mode's claim does not actually hold of the rust binary, the correct response is to stop and surface it, because the gate said it did and one of the two is wrong.
That is a finding about 0.1, not a task in this change.
