# The README is rewritten, and the examples grow to cover the surface it documents

## Why

`README.md` is 1417 lines that contradict themselves and the shipped binary.
`README.md:687` states "`upload` does not exist here, and `safix --help` records why", ninety-two lines above `## Seeding a machine's host identity: safix upload` (`:779`), and the verb block at `:662-677` lists thirteen commands where the dispatch table carries sixteen (`crates/safix/src/main.rs:121-204`: `set`, `edit`, `get`, `view`, `list`, `generate`, `check`, `fix`, `audit`, `sync`, `keygen`, `adduser`, `enroll`, `group`, `upload`, `install`).
The verb arithmetic disagrees with itself in four places — "fifteen of its sixteen" (`:1031`), "Fourteen of safix's fifteen" followed by fourteen names (`:1066`), "all fifteen subcommands" (`:1399`), "Fourteen of safix's fifteen" again (`examples/README.md:21`) — and the target set is enumerated as two in six places (`:671`, `:689`, `:1320`, `:1399`, and the two target chapters at `:1132` and `:1211`), which is exactly where three new sync targets have to land.
Roughly sixty bare check names sit in the body, `## Status` (`:1388-1406`) narrates CI, and the same two propositions are restated seven and five times respectively; a dozen sentences run past eighty words.

The examples are worse placed than the prose they illustrate.
`examples/plain-nix/` and `examples/dendritic/` declare seventeen of the roughly forty declarable features, and neither declares a consumption profile at all — so the option surface the README spends its longest chapter on (`:831-995`, `modules/consume/common.nix:417-617`, `modules/consume/installer.nix:283-531`, `modules/consume/home.nix:322-469`) has no worked example anywhere in this repository.
`flake.safix.bridge` and `flake.safix.keepassxc` are two of the ten fields `safix-examples` compares (`modules/flake/checks/examples.nix:50-63`) and both are empty in both examples, so the comparison holds nothing about either target.
Two hand-maintained lists are already drifting apart: `modules/flake/checks/examples.nix:65-67` discovers the dendritic modules with `lib.filesystem.listFilesRecursive` while `examples/dendritic/flake.nix:16-40` names all twenty-two by hand, and nothing evaluates that flake (`examples.nix:25-28` declines to).
The hooks are compared against themselves: `examples.nix:81`, `:93-96` take `onboardingHook` and `enrollHook` from `plain-nix/hooks.nix` for both the expected and the actual side, so dendritic's hooks — which do not exist — are not compared.
Two statements in `examples/README.md` are already false: `:12` says `entry.nix` "reaches `lib.mkVault` through `builtins.getFlake`", which `entry.nix:1-14` deliberately no longer does, and `:34` claims every file under both examples is read by the check, which `examples/dendritic/flake.nix` is not.

## What Changes

- Rewrite `README.md` from scratch against a fixed outline of thirteen chapters, in plain English, at roughly 850 lines.
  Every verb and every option is documented exactly once; the sixteen verbs arrive as one table rather than as a console block plus four disagreeing counts.
  No check name appears in the body: a check name is contributor evidence, `nix flake check` already prints them, and `CONTRIBUTING.md:25-40` is where a contributor looks.
  No sentence runs past forty words.
  "Narrowing an audience is not revocation of what was already encrypted" is stated once, in the model chapter, and cross-referenced from the seven places that restate it today (`:127-131`, `:152-156`, `:231-232`, `:246-248`, `:280`, `:310-311`, `:364-368`, `:1344-1345`).
  "safix reads no option outside its own namespace" is likewise stated once (`:811-816`, `:833-839`, `:922`, `:989-990`, `:1347-1348` collapse into one).
- The two target chapters (`## The bridge to clan`, `## The mirror in your password database`) become one `## Syncing to other stores` chapter with five same-shaped target subsections — clan, keepassxc, pass, bitwarden, 1password — each with the same five headings, carrying only what differs from the shared mapping shape stated once above them.
- **BREAKING** for a reader, not for an evaluation: `## Status` (`:1388-1406`) moves to `CONTRIBUTING.md` as a new `## Where the suite stands` section.
  Nothing in it changes what an operator types, and all of it is about this repository's own CI.
- Rewrite `examples/` to declare every feature of the namespace, at both scopes.
  `examples/plain-nix/fleet.nix` and `examples/dendritic/modules/` grow from seventeen covered features to forty: `sharedWith."ownerOf.<m>"`, an organization audience, a legal `perHost.<h>.add`, machine, service and nested-group members, a second silo group, delegation (`organizations.<o>.managers` with `users.<u>.managedBy`), `recoveryRecipients`, entry `mode`, generator `dependencies`, `prompts`, `validation`, `network`, a public output (`files.<n>.secret = false`), `storage`, `vault`, `extraGovernedFiles`, and a mapping for each of the five sync targets.
- Add a third example directory, `examples/profiles/`, holding a NixOS system profile and a home-manager profile that consume the same fleet, and a new check `safix-examples-profiles` that evaluates both.
  This is the first worked example of `safix.enable`, `safix.flake`, `safix.lib`, `safix.user`, `safix.machine`, `safix.hostname`, `safix.tags`, `safix.secrets`, `safix.identity.*`, `safix.identityPreflight` and `safix.installer.*` in the repository.
- Fix the three defects in `modules/flake/checks/examples.nix` that let the examples drift.
  `onboardingHook` and `enrollHook` are read from the dendritic side for the expected value and from `plain-nix/hooks.nix` through the evaluated entry for the actual one, so the two are compared rather than one being compared with itself.
  `examples/dendritic/flake.nix` stops naming its twenty-two module files and reads its own module directory the way the check does, and the check asserts that the file contains no per-file path under `modules/`, so the hand list cannot come back and cannot silently shrink.
  Every compared field is passed through one elision that replaces a root-dependent absolute string with a stable marker, so an example may declare `bridge.clanFlake` and `vault.root` — both `lib.types.path`, both stringified into the projection (`modules/flake/safix/default.nix:326`, `:64`) against two different roots (`examples.nix:73`'s `dendriticRoot` versus `entry.nix:20`'s `root = ./.`) — without the comparison failing on the store path.
  The compared set grows from the ten data fields plus two hooks to every attribute the runtime reads that serializes — adding `subjects`, `vaultDeclared` and `vaultCreationRulesText` (`crates/safix-core/src/nix.rs:31-64`), and, once the sibling target changes land, one field per new target — so a declared feature can no longer sit in a field nothing compares.
  The check also gains `coverageProbes`, one literal row per feature family asserted against the compared projection, because a comparison between two consumers holds that they agree and stays green when a feature is deleted from both.
- Rewrite `examples/README.md` against what the two checks actually read, deleting the `builtins.getFlake` sentence and the claim that every file under `dendritic/` is evaluated.

Not in scope: any code change outside `examples/` and `modules/flake/checks/examples.nix`, plus the one new check file `modules/flake/checks/examples-profiles.nix` and its registration line in `flake.nix`.
No option is added, renamed, or retyped by this change; every feature the examples newly declare is one that already exists.
Also not in scope: moving reference material into `docs/`.
The outline demotes the threat-model half of `### The envelope a fragment runs in` (`:486-529`) and the flakeless-projection reference (`:1028-1073`) to shorter forms in place; a `docs/` tree of its own is a second document to keep true and this change's whole argument is that one is already too many.
Also not in scope: evaluating `examples/dendritic/flake.nix` as a flake.
It names `inputs.safix.url = "path:../.."` (`flake.nix:5`), so evaluating it inside the check sandbox means resolving this repository's own transitive input closure a second time with no network — the obstacle `examples.nix:16-23` records.
Asserting its import list against the files on disk buys the one thing that evaluation would buy here, which is that the list cannot silently shrink.

## Capabilities

### New Capabilities

- `documentation`: the README's contract as a document — that every verb and every option is documented exactly once, that no check name appears in it, that its prose is plain (no sentence over forty words, each load-bearing rule stated once and cross-referenced), that the sync chapter carries one same-shaped subsection per target, and that contributor-facing status lives with the contributors.

### Modified Capabilities

- `consumer-integration`: gains a requirement that the namespace is exercised end to end by worked examples, including a consumption profile at both scopes, and that the examples are evaluated rather than merely written.
  Its existing requirement "One declaration serves both system and user scope" gains a scenario stating that the same declaration is consumed by a worked profile at each scope and that a check evaluates both.
  Its existing requirement "A consumer bridges its own vocabulary with an adapter it owns" gains a scenario stating that the worked example the documentation carries is one a check evaluates.
- `behavioural-suite`: gains a requirement that the two worked consumers are compared to each other on every field the runtime reads, with root-dependent absolute strings elided rather than compared, and both hooks compared across the two sides; and a requirement that a hand-maintained list of example modules is asserted equal to the set on disk.

## Impact

Affected docs:

- `README.md` — rewritten whole, 1417 lines to roughly 850.
- `CONTRIBUTING.md` — gains `## Where the suite stands`, absorbing `README.md:1388-1406`.
- `examples/README.md` — rewritten, 34 lines.

Affected examples:

- `examples/plain-nix/fleet.nix` — grows from 103 lines to the full feature set; `examples/plain-nix/hooks.nix` unchanged in shape; `examples/plain-nix/entry.nix` unchanged (it must stay flakeless — no `config.flake`, no `inputs.safix`, no flake-parts, no `builtins.getFlake`, per `entry.nix:1-14`).
- `examples/dendritic/modules/` — grows from twenty-two files to the matching set, one declaration per file; `examples/dendritic/flake.nix` stops naming them individually and reads its own module directory instead.
- `examples/profiles/` — new: `nixos.nix`, `home.nix`, `relocated.nix`, `README.md`, all three binding `safix.lib` to the one fleet `examples/plain-nix/fleet.nix` declares.

Affected checks:

- `modules/flake/checks/examples.nix` — the hooks comparison (`:78-96`), the elision of root-dependent strings applied to `tenFields` (`:50-63`), the import-list assertion beside `dendriticModules` (`:65-67`), and the module header's own count of dendritic files (`:25`).
- `modules/flake/checks/examples-profiles.nix` — new, registered at `flake.nix:104-105`, modelled on `modules/flake/checks/portability.nix:60-78` for the projection and on `modules/flake/checks/mk-structural-check.nix` for the comparison.

Every guarantee this change states gets a severity drill in `tasks.md`.
