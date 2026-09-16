# Design: the README is rewritten, and the examples grow to cover the surface it documents

## Context

See `proposal.md` — Why.
Six facts about the current worktree fix the shape of this change; each was read rather than assumed.

The binary has sixteen verbs, in one table, derived nowhere else: `crates/safix/src/main.rs:121-204` holds `set`, `edit`, `get`, `view`, `list`, `generate`, `check`, `fix`, `audit`, `sync`, `keygen`, `adduser`, `enroll`, `group`, `upload`, `install`, and `expected_verbs()` (`main.rs:219-226`) derives the unknown-subcommand refusal from that table so the refusal cannot name a verb the binary lacks.
The README has no such mechanism and carries four disagreeing counts plus a thirteen-entry console block.

Exactly one check reads the examples today: `checks.safix-examples`, `modules/flake/checks/examples.nix:100-138`, registered at `flake.nix:104`.
It executes `examples/plain-nix/entry.nix` for real inside a sandbox holding only `examples/plain-nix`, the top-level `lib/`, and `modules/flake/safix` (`examples.nix:117-121`), and reads `examples/dendritic` through `lib.evalModules` over `../safix` plus every `.nix` file under `dendritic/modules` (`:65-76`).
`treefmt` covers both examples with `flakeCheck = true` (`modules/flake/treefmt.nix:8-15`), so every example file is nixfmt-clean or the flake check is red.

The comparison is over ten fields — `placements`, `audiences`, `governedFiles`, `recipients`, `delegation`, `policyText`, `generatorPlan`, `nameRegex`, `bridge`, `keepassxc` (`examples.nix:50-63`) — which is the set `crates/safix-core/src/nix.rs` reads, minus the function-valued members.
A feature whose only evidence is a function (`mkChecks`, `resolveSet`, `resolveNames`, `materialize`, `publicValue`, `outputPath`, `policyPlan`) is invisible to this check, which is why the profile surface needs a check of its own rather than a field in this one.

`_module.args.self` differs between the two halves by construction: `dendriticRoot` on the dendritic side (`examples.nix:73`) and `root = ./.` inside the sandbox on the plain-nix side (`entry.nix:20`).
`bridge.clanFlake` and `vault.root` are both `lib.types.path` and both reach the projection as strings (`modules/flake/safix/default.nix:326`, `:64`), so a path declared relative to each example's own root stringifies to two different absolute paths.
No spelling makes the two agree; the comparison itself has to stop comparing that part of the string.

The hooks are compared with themselves.
`plainNixHooks = import (plainNixRoot + "/hooks.nix")` (`examples.nix:81`) supplies both the expected value (`:93-96`) and, through `entry.nix:17`'s `safix = hooks // { … }`, the actual one.
`examples/dendritic` declares neither hook.

Two lists are kept in agreement by hand: `dendriticModules` discovers files with `lib.filesystem.listFilesRecursive` (`examples.nix:65-67`) while `examples/dendritic/flake.nix:16-40` names twenty-two paths literally, and nothing evaluates that flake — `examples.nix:25-28` declines to, because `flake.nix:5`'s `inputs.safix.url = "path:../.."` would mean resolving this repository's whole input closure a second time inside a network-less sandbox.

## Goals / Non-Goals

**Goals:**

One document that states each thing once, in the order a user meets it, with no sentence a reader has to re-parse and no name only a contributor can use.
Every option and every verb reachable from exactly one place in it, so that adding either has exactly one documentation site.
An examples tree that declares every feature of the namespace, at both scopes, with coverage held by a check rather than claimed by a sentence.
A comparison between the two consumers that fails on a real divergence and cannot be satisfied by both sides losing the same feature.

**Non-Goals:**

Any change to an option, a verb, a refusal, or a check outside `modules/flake/checks/examples.nix` and the one new `examples-profiles.nix` (§D9, §D13).
A `docs/` tree (§D1).
Evaluating `examples/dendritic/flake.nix` as a flake (§D11).
A prose-linting check in the flake (§D7).

## Decisions

### D1. Thirteen chapters, ordered by a user's first hour, written from scratch

The outline is fixed before a line is written, and every existing section maps into exactly one chapter of it:

```
# safix                                              (~12 lines)
## Install it and declare one secret                 (~45)
## How safix thinks                                  (~35)
## Declaring what someone holds                      (~110)
## The subjects that can hold a key                  (~130)
## Generators                                        (~100)
## Everyday verbs                                    (~90)
## Syncing to other stores                           (~120)
## Where files go                                    (~50)
## Establishing secrets in a profile                 (~110)
## Fitting safix to a tree you already have          (~60)
## The opinions safix will not bend                  (~25)
## Where the pieces live                             (~25)
## License                                           (~10)
```

The ordering principle is that a user's first hour is install, declare, `fix`/`set`/`get`, look at it, share it.
Today the document reaches `safix set` at `:85` but the verb inventory only at `:660`, the profile options at `:841`, and the storage roots at `:1271`.

**Alternative rejected**: incremental repair — fix `:687`, complete the verb block, reconcile the four counts, delete `## Status`.
It leaves the ordering, and the ordering is what produces the duplication: revocation is restated seven times (`:127-131`, `:152-156`, `:231-232`, `:246-248`, `:280`, `:310-311`, `:364-368`, `:1344-1345`) because each of seven sections reaches the topic before the model chapter has earned the right to state it once.
Repairing contradictions in a document whose structure generates them buys one green reading and the same defect class next quarter.

**Alternative rejected**: split into `docs/` — a tutorial, a reference, and a threat-model note.
Three documents to keep true where the finding is that one already is not, and the option surface would end up in the reference while the chapter that motivates it stays in the tutorial, which is the duplication this change is removing wearing a directory.

### D2. One verb table, sixteen rows, and no count in prose

`## Everyday verbs` opens with one table: verb, what it does, what it reads, what it writes, whether it needs a terminal.
Sixteen rows, `install` included and marked as the one nobody types (`:954`).
No sentence anywhere in the README states how many verbs there are.

A count in prose is a second encoding of the table, and the four current spellings are what a second encoding does.
The table itself cannot disagree with the binary silently in the same way: a missing row is a verb with no documentation, which the coverage task checks by name.

**Alternative rejected**: keep the console block at `:662-677`, completed to sixteen lines.
A console block cannot carry the "reads / writes / needs a terminal" columns, which is the information an operator actually wants before running an unfamiliar verb, and its comment column is where the current omissions hid.

**Alternative rejected**: generate the table from `safix --help` at build time.
The README is a committed file a reader meets on a forge page; generating part of it means a generator, a committed generated region, and a drift check — three mechanisms to hold sixteen rows that change about once a quarter.

### D3. No check name appears in the body

The README names no check.
The worst site today is the upload section (`:790-805`), which spells thirteen check names in twenty-seven lines; `:924`, `:955`, `:962`, `:967-968`, `:978`, `:983`, `:995`, `:1041-1049`, `:1064`, `:1069` and `:1390-1406` carry the rest, roughly sixty in total.

A check name is an instruction a reader cannot use: they do not run the checks, `nix flake check` prints the names itself, and the claim a check holds is exactly the claim the surrounding prose already makes — so the name adds a fact about this repository to a document about the tool.
`## Fitting safix to a tree you already have` keeps `mkChecks` as an option surface a consumer calls, which is a published function and not a check name.

**Alternative rejected**: an appendix table mapping guarantee to check.
It is genuinely useful — to a contributor, which is who `CONTRIBUTING.md:25-40` is for. Put it there if anyone wants it, not in the operator's document.

**Alternative rejected**: keeping the names as evidence that a stated guarantee is mechanical.
The evidence a reader can act on is the guarantee stated precisely, including what it does not cover.
"The value never reaches an argument vector" is checkable by them; `safix-value-source` is not.

### D4. Each load-bearing rule is stated once and cross-referenced

Two propositions carry the document and are restated twelve times between them: that narrowing an audience does not retroactively revoke what was already encrypted, and that safix reads no option outside its own namespace.
Each gets one statement — the first in `## How safix thinks`, the second in `## Fitting safix to a tree you already have` — and every later mention is a clause naming the chapter.
The twelve "Think of it as …" framings (`:91`, `:105`, `:120`, `:143`, `:164`, `:196`, `:243`, `:260`, `:274`, `:322`, `:429`, `:646`) reduce to at most two.

**Alternative rejected**: repetition as emphasis, on the argument that a reader landing mid-document needs the caveat where they are.
A reader landing mid-document needs a link, which costs one clause; seven statements of one rule cost the reader the question "is this the same rule, or a narrower one?" seven times, and that question is why the eighth site (`:1344-1345`) reads as an opinion rather than as the same fact.

### D5. One sync chapter, five same-shaped subsections, the shape stated once

`## Syncing to other stores` opens with what a mapping is, the four modes and their directions, how conflicts are judged, what the sync-state companion records, and the rule that nothing is deleted on either side — once, for all five targets.
Then five subsections, each with the same five headings: what it addresses, how it is declared, what it can carry, how it unlocks, and what it refuses.
A subsection states only what differs from the shared shape.

This replaces `## The bridge to clan` (`:1132-1209`) and `## The mirror in your password database` (`:1211-1269`), and it is where the six "two targets" sites (`:671`, `:689`, `:1320`, `:1399`, plus the two chapter headings) stop existing rather than becoming "five".

**Alternative rejected**: five chapters, one per target.
Five copies of the mode table, the conflict rule and the companion-entry explanation, which is the shape that produced two divergent copies at `:1132` and `:1211` from two targets.

**Alternative rejected**: one table with five columns and no prose per target.
The differences are not tabular: clan addresses a machine and a generator inside another flake, keepassxc addresses a path inside an encrypted database, pass a path under a store root, bitwarden a folder plus an item name, 1password a vault plus an item — and each unlocks differently.
A table would have one cell per pair and force the reader to reconstruct five paragraphs.

### D6. `## Status` moves to `CONTRIBUTING.md`, not to the bin

`README.md:1388-1406` becomes `CONTRIBUTING.md`'s `## Where the suite stands`, after `## Running the checks` (`CONTRIBUTING.md:25-40`) and before `## The fixture fleet` (`:41`).
Its content — which platforms the sandboxed checks run on, which need user namespaces, which are darwin-conditional, what was retired — is exactly what a new contributor needs before their first `nix flake check`, and none of it changes what an operator types.

**Alternative rejected**: delete it.
The platform conditionality is real and hard-won knowledge; a contributor who does not have it reads a skipped check as a passing one.

### D7. The README's prose contract is checked by a recorded command, not by a flake check

Three of the contract's clauses are mechanical: no check name in the body, no sentence over forty words, every verb and every option named exactly once.
Each is verified by a one-line command recorded in `CONTRIBUTING.md` and run in this change's tasks, with the drill being a planted violation that the command reports.

**Alternative rejected**: a `safix-readme` flake check.
The proposal's non-goal forbids new code outside the examples and their checks, and the cost is worse than the scope: a word-count check's failure mode is rewording to please a counter, and a "names every option" check needs the option list, which means reading the module system from a check that exists to read prose.
A command a contributor can run, named where contributors look, holds the same three clauses without adding a mechanism that must itself be kept true.

### D8. A third example directory for the profiles, over the one fleet the other two declare

`examples/profiles/` holds `nixos.nix` (a system-scope profile serving the machine `deck`), `home.nix` (a user-scope profile serving `alice`), `relocated.nix` (a storage-root and vault overlay) and a `README.md`.
It declares no fleet of its own: both profiles bind `safix.lib` to `(import ../../lib { inherit lib; }).mkVault { modules = [ ../plain-nix/fleet.nix ]; root = ../plain-nix; }`, so the repository holds exactly one fleet and three consumers of it.
That binding is also the documented flakeless one (`modules/consume/common.nix:455-469`: `safix.lib` set directly, rather than defaulted from `safix.flake`), with the one-line flake form shown beside it in a comment.

`safix-examples-profiles` evaluates `nixos.nix` through a real `nixosSystem` and `home.nix` through `home-manager`'s own library, the way `modules/flake/checks/portability.nix:60-78` and `modules/flake/checks/installer.nix` already do, and compares the materialized result against literals through `modules/flake/checks/mk-structural-check.nix`.

The profile surface is unreachable from `safix-examples`: `safix.secrets` is a materialization of a projection under a scope, and the fields that check compares are the pre-scope projection.
A profile therefore needs a second evaluation, with a host module system and home-manager in scope, and `safix-examples`' sandbox deliberately copies neither (`examples.nix:117-121`).
It is also where three features of the declaration surface become visible at all: an entry's `mode` and its `path` are not placement fields (`modules/flake/safix/resolve.nix:810-866` emits neither), and `path` is `functionTo str` (`modules/flake/safix/types.nix:382-387`), so it cannot be JSON-serialized and cannot be compared by `safix-examples` — it is compared applied, here.

**Alternative rejected**: a fleet of its own under `examples/profiles/`.
A third copy of the declarations, kept in agreement with the other two by nothing: `safix-examples` compares the first two to each other and would not see the third move.

**Alternative rejected**: adding the profiles to `examples/plain-nix/`.
`entry.nix` must stay flakeless and its sandbox must stay self-contained (`examples.nix:119-121`), and a profile evaluation needs nixpkgs' module system plus home-manager — neither of which is in that sandbox, and putting them there would make the flakeless example depend on a flake's inputs.

**Alternative rejected**: one profile rather than two.
"One declaration serves both system and user scope" is the requirement (`openspec/specs/consumer-integration/spec.md:120-140`); one profile demonstrates half of it, and the half with the ownership axis is exactly the half whose user-scope refusal would then stay unexampled.

**Alternative rejected**: declaring `flake.safix.storage` and `flake.safix.vault` in the shared fleet.
Both are fleet-wide: relocating the roots moves every path in every compared field, and a naming key makes every name opaque, so the one fleet the README documents would stop showing the readable layout it describes.
As an overlay merged only for the profiles check's third evaluation, both features are exercised where their consequence is visible — what a profile reads — and the shared fleet stays the readable default the document is written against.

### D9. Root-dependent absolute strings are elided from the comparison, not dropped

`safix-examples` maps every compared field through one function before diffing: any string beginning with the example's own root — `/build/repo/examples/plain-nix` on the executed side, the `/nix/store/…` dendritic path on the other — has that prefix replaced by the literal `<example-root>`.
The rest of the string is compared as-is, so `bridge.clanFlake` compares as `<example-root>` on both sides, and a divergence in the tail — one example naming a subdirectory the other does not — still fails.

**Alternative rejected**: dropping `bridge` from the comparison.
`bridge` is one of the fields the runtime reads (`crates/safix-core/src/nix.rs:52-67`), and dropping it means the clan mapping the examples newly declare is compared nowhere — the exact condition that lets the two examples drift. The same elision is what lets `safix-examples-profiles` compare a `vault.root` declared relative to `examples/profiles`.

**Alternative rejected**: declaring a path that stringifies identically on both sides, such as an absolute `/nonexistent`.
`lib.types.path` accepts it, but the example then documents a declaration no consumer would write, and the two roots differ by construction (`examples.nix:73` versus `entry.nix:20`) so no relative spelling can agree.

**Alternative rejected**: eliding by replacing any store path.
Too wide: a store path appearing where one is not expected is itself a finding, and the elision would hide it.

### D10. Both hooks are declared on both sides, compared across them, and pinned by one literal

`examples/dendritic/modules/hooks.nix` declares `onboardingHook` and `enrollHook` with the same values as `examples/plain-nix/hooks.nix`.
`safix-examples` takes the expected hooks from the dendritic evaluation and the actual ones from the executed `entry.nix`, so the two sides are compared.
Beside that, one literal row asserts the onboarding hook contains the fixture's marker text and that `enrollHook` is null, which is what keeps a both-sides-empty regression from being vacuously green.

**Alternative rejected**: preserving the asymmetry and documenting it.
The asymmetry is not a decision anybody made; it follows from `plainNixHooks` being used for both operands (`examples.nix:81`, `:93-96`), and it means a dendritic hook could be wrong in any way at all without the check noticing.

**Alternative rejected**: comparing both sides to a literal in the check file only.
That is a third copy of the hook text to keep true, and the property here is that the two consumers agree; the literal's job is anti-vacuity, so one marker substring plus the null does it.

### D11. `examples/dendritic/flake.nix` globs its modules; the check asserts there is no hand list

`flake.nix`'s `imports` becomes `[ safix.flakeModules.default ] ++ lib.filesystem.listFilesRecursive ./modules`, which is what `safix-examples` already does on its side (`examples.nix:65-67`).
`safix-examples` then asserts that `flake.nix` contains no `./modules/` literal, so the hand list cannot come back and cannot silently shrink.

This is also the honest dendritic pattern: a tree that scatters one declaration per file does not enumerate them, and the current twenty-two-line list is the part of the example a reader would copy least.

**Alternative rejected**: keeping the hand list and asserting equality against `listFilesRecursive`.
Comparing them means extracting an import list from nix source, which is either a parser with one caller or a `grep` over formatting — and the `grep` form breaks when `nixfmt` moves a path onto another line.

**Alternative rejected**: evaluating the flake and reading `imports` from it.
`flake.nix:5` names `path:../..`, so the sandbox would resolve this repository's own input closure with no network — the obstacle `examples.nix:16-23` records.

### D12. Coverage is probed by literal, because a field-for-field diff holds agreement rather than coverage

`safix-examples` gains `coverageProbes`: one literal row per feature family, asserted against the compared projection.
An organization audience appears in `audiences`; the `ownerOf.<m>` grant appears in `placements` with the machine owner's key; the generator prompt, dependency, validation and public output appear in `generatorPlan`; each of the five targets' mapping sets is non-empty in its own field; `recoveryRecipients` shows as an extra recipient on the entries it covers; `extraGovernedFiles` shows in `governedFiles`.

Without this, "the examples cover forty features" is a sentence in a README that nothing holds, and the failure mode is silent in the worst direction: deleting a feature from *both* examples keeps the diff green.

**Alternative rejected**: trusting the field-for-field diff.
It holds that the two declaration styles agree, which is a different proposition from the one this change is about.

**Alternative rejected**: a coverage table in `examples/README.md` alone.
That is the claim, not the evidence; the table stays, and each of its rows names the probe that holds it.

### D13. The sync-target half of the examples is ordered after the four sibling changes

`examples/*/…` cannot declare `flake.safix.pass`, `flake.safix.bitwarden` or `flake.safix.onepassword` before those options exist, and an example declaring an undeclared option fails evaluation rather than degrading.
Task group 6 and the five target subsections of the README therefore depend on `add-pass-bridge`, `add-bitwarden-bridge`, `add-onepassword-bridge` and, for the `fields` submodule every mapping's far side carries, `extend-bridge-fields`.
Every other group in this change is independent of all four.

The option spellings the examples are written against are those changes' own:
`flake.safix.pass = { store; mappings.<id> = { mode; safix = { user; name; }; pass = { path; fields; }; }; }`,
`flake.safix.bitwarden = { server; mappings.<id> = { mode; safix; bitwarden = { folder; item; fields; }; }; }`,
`flake.safix.onepassword = { account; mappings.<id> = { mode; safix; onepassword = { vault; item; fields; }; }; }`,
each `mode` one of `safix-to-<target>`, `<target>-to-safix`, `two-way`, `backup`.

**Alternative rejected**: landing the examples first with the mappings commented out.
A commented declaration is the thing this repository's examples exist not to be — nobody evaluates it, so it rots exactly as `examples/README.md:12` did.

**Alternative rejected**: making this change own the target options.
Four changes already own them, and a fifth definition of the same submodule is the divergence risk those changes were split to avoid.

## Risks / Trade-offs

Rewriting 1417 lines from scratch loses any correct sentence not carried into the outline.
Mitigation: the outline maps every current section into a chapter, and the rewrite is ordered chapter by chapter with the absorbed sections named per chapter in `tasks.md`, so a dropped section is a task that was not done rather than an omission nobody can see.

The examples grow from twenty-five files to roughly sixty, and every feature must be added to both examples in the same evaluation-visible way or `safix-examples` fails.
That cost is the check working as intended, and it is why the dendritic side globs (§D11): adding a file there becomes one file rather than one file plus one import line.

`examples/profiles/` introduces a check that evaluates home-manager, which the flake already takes as an input for `modules/flake/checks/installer.nix`'s activation-order proof (`flake.nix:28-31`), so no new input and no new closure.

## Migration

None for a consumer: no option, default, verb, or refusal changes.
A reader following a link into a README anchor may find it renamed; the chapter list in `## Where the pieces live` is the replacement map.
