# Tasks: clan-generator-contract

The three standing disciplines hold: fixture identities only, nothing deploys, and no sentence describing a guarantee is written before the code enforcing it exists in the same commit.

A fourth applies only to this change and is the one that will be tempting to relax.
This change weakens safix's central invariant.
Every commit that materializes plaintext must land with the containment that bounds it, in the same commit — never "the staging works, the tmpfs check comes next".
Stage 1 therefore builds the containment before anything writes into it, and stage 1's own verification is a drill showing the refusal fires.

Stages: 1 is containment, 2 is the executor, 3 is public outputs, 4 is the editor, 5 is the v1 removal, 6 is the record.

## 1. Containment before anything is staged

- [x] 1.1 Add a tmpfs probe to `safix-core`: given a candidate directory, ask the operating system what filesystem is mounted there and answer whether it is memory-backed. Do not infer from the path
- [x] 1.2 Implement the staging root: created mode 0700 under the verified mount, files 0600, one per run, registered with `scratch.rs`'s registry *before* creation
- [x] 1.3 Implement the refusal when no memory-backed mount is found, and the `--allow-disk-staging` acknowledgement that overrides it. Give the flag a name that states what is accepted; verify it reads that way in `usage.rs`
- [x] 1.4 Extend `scratch.rs`'s directory removal to overwrite file contents before unlinking, and confirm it is reached from the panic path and from both signal handlers as well as from return and error
- [x] 1.5 Record at the staging module the two residual exposures — a swapped page the overwrite does not reach, and same-user reachability for the run's duration — in the voice `types.nix` already uses for the equivalent 0.1 limit
- [x] 1.6 Severity drill: point the probe at a disk-backed directory and confirm the run refuses; then pass the acknowledgement and confirm it proceeds with the directory still 0700 and the removal still running
- [x] 1.7 Severity drill: kill the process mid-run with `SIGINT` and with `SIGTERM`, and confirm the staging root is gone in both cases
- [x] 1.8 Verify: `cargo test` passes, and 1.6 and 1.7 were each observed before being reverted

## 2. The executor

- [x] 2.1 Replace descriptor construction in `inputs.rs` with staging-directory construction: `in/<dependency>/<output>`, `prompts/<key>`, `out/`, working directory at the root
- [x] 2.2 Create the prompts directory only when the generator declares prompts, and leave the variable unset otherwise — matching clan's executor, so a script cannot distinguish "none declared" from "directory missing"
- [x] 2.3 Write a prompt's answer with nothing added and nothing removed, and read a dependency's output the same way. Record that this is clan's behaviour and that a newline convention here would silently corrupt a key
- [x] 2.4 Collect declared outputs from `out/` after the script exits. Refuse the whole run on a missing one, naming it and listing what the directory did contain
- [x] 2.5 Hold the presence check to completing for every output before any value is encrypted, so a partial generator refuses having written nothing
- [x] 2.6 Derive `share` on the generator from its outputs' entries, expose it read-only, and refuse a generator whose outputs disagree — naming both outputs, which side each is on, and the two-generator remedy
- [x] 2.7 Update `resolve.nix` for the derived `share` and the disagreement refusal, and add both to the message families `checks.nix` already exposes so a consumer's fixture can assert the message against a literal
- [x] 2.8 Re-read `generate.rs`'s multi-output write against 2.6: with outputs constrained to one audience the keypair case is one staged document and one rename. Record what remains non-atomic — a `--regenerate` cascade still commits per generator — rather than claiming the window closed
- [x] 2.9 Rewrite `modules/flake/checks/generators.nix` against the new interface
- [x] 2.10 Verify: the integration suite's `generate`, `generate-refusals` and `generate-isolation` tests pass against the new contract, including the claim that a script reading standard input to end of input does not consume a later prompt's answer — which is now true for a different reason and must be re-asserted rather than assumed to carry over

## 3. Public outputs

- [x] 3.1 Change `files` in `types.nix` from a list of names to an attribute set carrying `secret`, defaulting to true. Record why the default is true rather than mirroring clan's: a mistyped field that makes a value public is not recoverable by fixing the typo
- [x] 3.2 Implement the public store in `crates/safix-core/src/public.rs` at the top-level prefix design D3 names, with the shared and per-user layouts and the `value` leaf
- [x] 3.3 Route a public output's bytes to the public store and not to sops, and include public paths in the commit the run makes
- [x] 3.4 Add `.path` for every output and `.value` for public ones on the nix side. Make `.value` on an ungenerated public output fail naming the command to run, and `.value` on a secret output fail naming the entry and pointing at `.path`
- [x] 3.5 Add `safix-public-no-rule` to `checks.nix`, matching every generated rule against every public path with the existing `matches` helper, failing while any match exists
- [x] 3.6 Add the public paths to `catchAllProbes`, so a rule reaching them fails the catch-all check as well
- [x] 3.7 Severity drill: hand the rule generator a directory that would produce a rule matching a public path, and confirm both checks fail. One check passing while the other fails is also a finding — record which
- [x] 3.8 Verify: `nix flake check` passes; a fixture fleet with one public output evaluates `.value` to the file's contents; and the two drills in 3.7 were observed

## 4. The editor

- [x] 4.1 Add the `edit` verb to `main.rs` and `usage.rs`, addressing an entry by name
- [x] 4.2 Select the editor from the visual variable then the editor variable, and refuse naming both when neither is set. Add no fallback program, and record why at the selection site
- [x] 4.3 Split the editor command on whitespace and execute it directly rather than through a shell. Record that the staged path reaches argv and the value does not
- [x] 4.4 Stage the existing value — or an empty buffer when there is none — into the staging root, hand the path to the editor, and read the result back
- [x] 4.5 Implement the four outcomes: non-zero exit writes nothing and names the status; unchanged buffer commits nothing; empty buffer takes the existing empty-value refusal; changed non-empty buffer writes through the same path `set` writes through
- [x] 4.6 State in the verb's help that an editor configured to write undo history or backups outside the directory it was given has put plaintext where safix does not look
- [x] 4.7 Add refusal snapshots for the new variants under both reporters, matching the existing paired plain and graphical convention
- [x] 4.8 Verify: an integration test drives `edit` with a scripted editor through all four outcomes, and the staging root is absent after each

## 5. Removing the v1 interface

- [x] 5.1 Delete the descriptor construction path entirely. Confirm by search that no code path executes the retired interface
- [x] 5.2 Add the evaluation-time refusals in `resolve.nix`: a script referencing the retired input spelling, a validation fragment referencing the retired output-name spelling, and a script that never references the output directory. Each names this change and gives the rewrite
- [x] 5.3 Record at the refusal site why it is retained permanently rather than deleted once the fleet migrates, and why it coexists with the unbound-variable failure the shell would produce anyway
- [x] 5.4 Rewrite the generator prose in `types.nix`: the descriptor paragraphs go, the directory contract arrives, and the limit about what a fragment does with a value it was handed is kept, because it is more true under this contract than it was under the last
- [x] 5.5 Severity drill: declare a fixture generator in the retired spelling and confirm evaluation refuses with the rewrite in the message, not with an unbound-variable error at runtime
- [x] 5.6 Verify: `nix flake check` passes and 5.5 was observed

## 6. The record

- [x] 6.1 Write the invariant change into `CHANGELOG.md` as a breaking change, leading with what it costs rather than with what it enables — the comparison table in design D1 is the content
- [x] 6.2 Update `README.md`'s generator section to the new contract, including the public-output accessors and the editor verb
- [x] 6.3 USER-RUN (answered): decide the sandbox question in design's open question — whether adopting clan's default generator sandbox is its own 0.2 change or deliberately out of scope for safix. It changes what an existing network-reaching generator may do, so it is not foldable into this change. Decided: its own change, `adopt-generator-sandbox`, opened immediately rather than deferred; the resolution is recorded under design's open question
- [x] 6.4 Verify: `openspec validate clan-generator-contract --strict` passes
