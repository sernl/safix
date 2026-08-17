## Context

See proposal.md — Why.
What the approach rests on:

- clan records a validation hash per generator and reports `invalid_generators` from `check` when the recorded hash no longer matches the definition (`clan_lib/vars/check.py:21,43-45`, mechanism `clan_lib/vars/_types.py:366-409`, at the pinned rev `56e35624`). safix's `validation` field is a different concept — a script judging a candidate value — and covers none of this.
- `safix_core::set::run` already takes `&mut dyn ValueSource` and is terminal-free (`crates/safix-core/src/set.rs`); the prompt is one source implementation at the CLI layer. `clan vars set` reads stdin when it is not a tty (`clan_lib/vars/set.py:59-70`), and safix's bridge feeds clan's `set` on exactly that contract (`crates/safix-core/src/clan.rs`).
- The repository's path contracts: everything under `secrets/` is encrypted, without qualification; everything under `public/` is a declared public output. A plaintext definition record fits neither.

## Goals / Non-Goals

Goals: definition drift detectable from the tree alone; a scripted write path with the same custody properties as the prompt; every remaining absence recorded rather than mysterious.

Non-goals: reading clan's validation record (decision one of the bridge design stands — nothing safix runs reads clan's store, and clan's record covers only bridged entries anyway); reporting drift for hand-set entries; a migration that backfills records for values minted before this change (a backfilled record would assert a mint this repository never observed).

## Decisions

### D1. safix records its own digest rather than reusing clan's validation hash

A safix generator exists whether or not any bridge mapping names its entry, so a drift record that lived in clan would cover only the bridged subset and would be read through the very store-reading the bridge design forbids.
The digest is safix's own, computed over the canonical form of the generator record the runtime already receives — script, `runtimeInputs`, prompts, `files` with their modes and secret flags, dependencies, and the validation script — so any edit that would change what a mint does changes the digest.
Where a bridge mapping's clan-side generator drifts, clan remains the authority and `export`'s existing refusal already answers; the two records answer different stores and neither reads the other.

### D2. The record lives in a third top-level tree, because both existing trees' meanings exclude it

`state/safix/definitions/<user>/<name>` (and `state/safix/definitions/shared/<audience>/<name>` for shared entries), one line of digest per file, committed.
Under `secrets/` it would break the everything-here-is-encrypted contract; under `public/` it would dilute declared-public-output into "plaintext things safix wrote".
Two alternatives were considered and refused.
A reserved key inside the sops document would put the record behind decryption — making `check` decrypt to answer a question about declarations, which is the property `check` exists to not have — and would carve a reserved name out of the entry namespace.
Deriving drift from git history (declaration file changed after the value's last commit) needs no record at all, but a refactor that moves or reformats a declaration would report drift that is not there, and a rename of the defining file would hide drift that is.

### D3. The stdin path drops the confirmation, and that is the point rather than a concession

The double prompt exists to catch a mistyped invisible value; a piped value has no typist.
Bytes are stored exactly as received — `echo` pipes a trailing newline and `printf` does not, the same doctrine the generator contract already states — and the empty-value refusal holds, because an empty pipe is the state a failed upstream command leaves behind.
Detection is the terminal test on standard input, exactly clan's branch, so `safix set` and `clan vars set` are scriptable by the same calling code.

### D4. The finding's remedies are the two edits that end the disagreement

A drift finding means the tree holds a value and a declaration that disagree about how the value comes to be.
Regenerating adopts the declaration; reverting the edit adopts the value; the finding names both and recommends neither.
`--regenerate`'s existing cascade semantics make the first remedy safe for dependents.

## Risks / Trade-offs

- [A new top-level `state/` tree is a consumer-visible surface change] → it is additive, plaintext, and small; the alternative locations each break a stated contract. The name says what it is: recorded state about the tree, not secrets and not outputs.
- [Existing generated values have no record, so drift that predates this change is invisible] → accepted and stated in the spec: no record, no claim. The record population grows at the natural mint rate; `--regenerate` adopts an entry immediately when the operator wants it covered.
- [A digest over the canonical record changes if the canonicalization changes] → the canonical form is versioned with a leading tag in the record file, so a format change reads as unknown-version rather than as universal drift.
- [The stdin path removes a human checkpoint] → only where no human is present; the terminal path is untouched, and the empty refusal plus exact-bytes semantics are the checks a pipe can actually have.

## Migration Plan

Additive; no existing behaviour changes until a mint writes the first record.
`check` gains a finding class that cannot fire on any tree minted before this change.
Rollback is removing the `state/safix/definitions` tree and the code that writes it.
Archive order: after `clan-generator-contract` and `adopt-generator-sandbox`, which hold earlier unarchived deltas on `secret-generators`.

## Open Questions

- USER-RUN: the operator asked for `upload` and clan-style plaintext `import`/`export`; the committed safix-cli spec refuses all three with reasons (`extract-safix-from-dotfiles` spec:132-133, restated by the bridge's reconciling delta). Overturning either is a deliberate spec change to make with that reasoning in view, and it is the operator's, not this change's default. Task 4.1 carries it.
  Answered: both refusals stand, conditionally on the custody-subjects extension not disturbing them — and it does not, for the reason recorded at 4.1: audiences change who reads, not how values arrive.
- USER-RUN: whether the absence of clan's flake-level (per-export) generator placement is confirmed as a non-goal — safix's axis is people and custody, not machines or service exports. If confirmed, it is recorded in the safix-cli delta; if not, it becomes its own future change. Task 4.2 carries it.
  Answered: rejected in the larger direction — the axis itself extends. `extend-custody-subjects` makes machines, services, groups and organizational custody first-class, and the per-export question dissolves into it rather than being recorded as an absence.
