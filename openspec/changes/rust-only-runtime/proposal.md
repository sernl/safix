# One language in the tooling: retire the shell and python runtimes and the harness that judged them

## Why

The operator's rule is unconditional: "all tools should be written in rust ... hard requirement ... If there are any tooling in safix that is still in python or shell, unless it's simply for scripting they should be moved entirely to rust. Toolings that were in other languages after migration to rust should be deleted entirely."

The migration already happened.
`packages.safix` is the rust binary, `crates/safix-core/src/sops/document.rs` is the port of both python readers and says so in its own module documentation, and the differential gate that permitted the switch is green across every subcommand.
What remains is the second half of the rule, which is the half that has not been done: the migrated tooling is still in the tree.

Six thousand two hundred and five lines of it, in five files:

| File | Lines | What it is |
|---|---|---|
| `modules/flake/safix/safix.sh` | 2149 | the shell runtime, built as `packages.safix-sh` |
| `modules/flake/safix/safix-differential.sh` | 2153 | the harness comparing that runtime against the rust one |
| `modules/flake/safix/safix-selftest.sh` | 1741 | the behavioural suite, driving the shell runtime |
| `modules/flake/safix/sops_recipients.py` | 81 | ported to `sops/document.rs` |
| `modules/flake/safix/sops_keys.py` | 81 | ported to `sops/document.rs` |

Keeping them is not free, and the cost is not disk.
`packages.safix-sh` is built and shellchecked on every evaluation, so every dependency it pins is a dependency this repository still carries; `readers.nix` keeps python3 and pyyaml in the closure of a project whose runtime has neither; and a reader arriving at `modules/flake/safix/` meets three implementations of the same subcommand set and no marker saying which one an operator runs.

There is one honest reason to hesitate and it is not the one the retirement note in `package.nix` gives.
That note says retiring the oracle "would retire the evidence that the two agree", which is true and insufficient — the evidence that they agreed is a fact about a moment, and it is preserved by git history at the merge, not by keeping the subject alive.
The real reason to hesitate is a topology this change discovered and the 0.1 artifacts do not record.

`modules/flake/checks/cli.nix` drives `SAFIX_SH` and nothing else.
All eighteen of its `safix-*` checks — `set-new`, `refusals`, `abort`, `generate-cascade`, `shared-flip` and the rest — run `safix-selftest.sh` against the shell script.
`SAFIX_RS` appears in exactly one place in the whole tree, `modules/flake/checks/differential.nix:73`.

So the rust binary has no end-to-end behavioural check that is independent of the shell oracle.
Every claim about what `safix set` does to a repository is currently asserted either against the shell runtime directly, or against the *pair*.
Delete the oracle naively and both families collapse at once: the eighteen selftest checks lose their subject and the nineteen differential checks lose their comparand, and what survives is sixty-six in-crate unit tests, no `tests/` directory in either crate, and a shipped binary with no integration suite at all.

That is the sequencing constraint this change is built around.
Deletion is the last step, not the first, and it is gated on a rust integration suite that asserts each of the eighteen behavioural claims against a literal expectation rather than against another runtime.

## What Changes

- A rust integration suite is written first, at `crates/safix/tests/`, driving the built binary against throwaway fixture repositories with real sops, real age and real git, and a stubbed `nix` — the same harness composition `cli.nix` uses today, minus the shell.
  Each of the eighteen selftest modes becomes one integration test asserting against a literal oracle: the value that should be at that key, the paths that should be in that commit, the files that should not exist after that abort.
- Coverage parity is itemized mode by mode and recorded in `design.md` as a table, and the deletion tasks are individually gated on their row being green.
  A mode is at parity when its rust equivalent asserts the same claim against a literal, not when a test of a similar name exists.
- The nineteen differential modes are triaged rather than ported.
  Most assert nothing that is not already the selftest claim in comparative dress, and they die with the oracle.
  Four do not: `abort`, `pipes`, `strace` and `drills` each hold the *rust* runtime to something in its own right — no residue after an interrupt, the value travelling only down a pipe, that fact observed at the syscall boundary, and every channel shown to fail under the mutation it exists to catch.
  Those four are re-expressed as single-runtime checks and keep their names without the `differential-` infix.
- Then, and only then, the deletions.
  `safix.sh`, `safix-selftest.sh`, `safix-differential.sh`, `sops_recipients.py`, `sops_keys.py`, `readers.nix` and `package.nix` are removed; `packages.safix-sh` disappears from the flake; the differential harness's python, bash and util-linux inputs leave the check closures.
- `CHANGELOG.md` records the retirement, and the "Known differences" section — which enumerates the places the two runtimes were deliberately pinned apart rather than held to agreeing — is rewritten in the single-runtime voice: each entry becomes a statement of what the rust runtime does, with the shell behaviour it diverged from named as history.
- Scripting survivors are enumerated and justified individually rather than exempted as a class.

Not in scope: any change to what the rust runtime does.
This change adds tests and removes files.
A behavioural difference discovered while porting a mode is a finding to surface, not a fix to land here.

## Capabilities

### New Capabilities

- `single-language-tooling`: the rule that safix's own tooling is rust, the boundary between tooling and scripting that makes the rule decidable, and the enumerated survivors on the scripting side of it.
- `behavioural-suite`: the rust integration suite — where it lives, what it drives, what it is allowed to stub, and the parity obligation that governs whether a claim may be deleted from one place before it exists in another.

### Modified Capabilities

None.

### Removed Capabilities

- `runtime-equivalence`: the differential gate. It is removed rather than modified because its subject is a pair of runtimes and there will be one. Its four single-runtime claims are re-homed into `behavioural-suite` rather than lost, and the fact that the gate was green across every subcommand at the merge is recorded in `CHANGELOG.md` and in git history rather than kept alive as a check.

## Impact

Affected code:

- New: `crates/safix/tests/` — the integration suite and its fixture support.
- Modified: `modules/flake/checks/cli.nix` — the eighteen selftest checks become the rust suite's invocation; the shell harness inputs go.
- Modified: `modules/flake/checks/differential.nix` — reduced to the four surviving single-runtime checks, and renamed to reflect that.
- Modified: `modules/flake/safix/default.nix`, `flake.nix` — `package.nix` and `readers.nix` leave the import list.
- Modified: `CHANGELOG.md` — the retirement, and the Known differences rewrite.
- Deleted: `modules/flake/safix/safix.sh`, `safix-selftest.sh`, `safix-differential.sh`, `sops_recipients.py`, `sops_keys.py`, `readers.nix`, `package.nix`.

Affected checks: `safix-differential-*` goes from nineteen attributes to four renamed ones; the eighteen `safix-*` cli checks keep their names and change their subject; `packages.safix-sh` ceases to exist.

Affected closures: python3 and pyyaml leave the repository entirely; bash and util-linux leave the check harnesses.
