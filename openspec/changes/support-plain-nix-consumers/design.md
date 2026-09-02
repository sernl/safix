# Design: safix without flake-parts, and without a flake

Revisions are as recorded in `proposal.md`.
Every line anchor below was read at one of them, or measured directly in this session against the checked-out tree, which is noted where it happens.

## Context

Two facts do all the work here and both were read rather than assumed.

`modules/flake/safix/default.nix` uses exactly one flake-parts facility: the `self` module argument, at `:12`, used at `:47` (`bound = args: args // registry // { root = self; }`) and `:269` (`publicValue = resolve.publicValueOf registry self;`), and downstream `self` is only ever concatenated as a path root — `resolve.nix:648` and `:2021`, `:2093`.
`modules/flake/checks/portability.nix:76-83` already proves the whole module evaluates outside flake-parts: `lib.evalModules { modules = [ ../safix { _module.args.self = ""; } { flake.safix = fleet; } ]; }` produces `.config.flake.safix.lib`, the identical projection a flake-parts consumer reads as `flake.safix.lib`.
So the module was already flake-parts-independent before this change; what this change does is publish that independence as a supported entrypoint rather than leaving it as an internal check fixture.

The second fact is on the runtime side.
`crates/safix-core/src/nix.rs:52-67` names twelve attribute paths under `safix.lib.*` and `safix.*`, and `Workspace::discover` (`workspace.rs:52-61`) resolves its root through git, never through `flake.nix`.
Fourteen of the fifteen verbs reach nix only through `eval_json`/`eval_raw`/`eval_raw_to`, which build `<root>#<attribute>` and hand it to `nix eval`.
`nix eval --file <path> <attribute>` reads the same attribute out of a plain expression instead of a flake output, and this session measured that against a hand-built fixture: all twelve attributes present, zero deserialization failures, ten read as `--json` and two (`policyText`, `nameRegex`) as `--raw`.
The one verb that does not fit this pattern is `generate`, whose one flake-only call is `nix shell --inputs-from <root> nixpkgs#<attr>` (`nix.rs:205-213`), reached from `Envelope::probe` at `generate.rs:213` only when the target user's `generatorPlan.order` is non-empty.

A third and fourth fact belong here because together they change what this design has to build, and both were measured directly rather than assumed from the shared contract's text — the first in this session, the second confirmed and extended by the orchestrator's own re-measurement of the same tree.

`.#checks.aarch64-darwin` and `.#checks.x86_64-linux` were each evaluated with `nix eval .#checks.<system> --apply builtins.attrNames`: 100 names on darwin, 113 on linux.
`safix-consumption`, `safix-consumption-ordering`, and `safix-consumption-refusals` are present in both; of the thirteen linux-only names, `safix-consumption-system` is the only one under `consumption.nix`.
Reading `modules/flake/checks/consumption.nix:428-511` confirms why: `lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux { safix-consumption-system = …; }` is unioned with `// { safix-module-collision = …; safix-consumption = …; … }` at `:511`, and nix's function-application-before-`//` precedence means only the first attrset is conditional.
The gate at `:428`, as checked into the tree today, already narrows to exactly the system-scope check — the shared contract's characterization of that line as over-broad does not hold against the checked-out tree, and D9 below records the correction rather than either silently complying with or silently overriding the contract.

The other twelve linux-only names are the eight `safix-installer-*` checks, `safix-bridge-real-clan`, `safix-generate-envelope`, `safix-memory-backing`, and `safix-portability` — and `safix-portability` is the real gap.
It is one check bundling all three consumption shapes' agreement — `nixos`, `homeInNixos`, `standalone` — gated entirely by `isLinux` because the `nixos` shape alone needs a real `nixosSystem` (`portability.nix:276-303`).
The other two shapes need nothing platform-specific: `homeInNixos`'s `osConfig` fixture is the plain attrset `insideNixos = { networking.hostName = hostname; }` (`:334-336`), not a real NixOS evaluation, and `standalone` passes no `osConfig` at all.
So the agreement claim this change's own Goals rely on — the standalone shape resolving identically to the shape a NixOS-hosted profile would — is currently provable on `x86_64-linux` alone, for a reason that has nothing to do with what the standalone shape itself needs.
D9 below designs the split.

## Goals / Non-Goals

**Goals:**
A consumer with no flake-parts import, and separately a consumer with no flake at all, can read every one of safix's fourteen non-generator verbs.
`generate` states plainly, at evaluation, why it still needs more than that and what supplies it.
The catalogue's scatter-and-merge property, safix's headline opinion, survives the move: `lib.mkVault`'s `modules` argument merges exactly as a flake-parts `imports` list does, because both are one call to the same function.
Every module entrypoint a consumer already reaches (`nixosModules.safix`, `homeModules.safix`) is importable with zero flake anywhere in the importing tree, which this design distinguishes from merely zero flake-parts.

**Non-Goals:**
Nothing about vault splitting (`declarationRoot` vs `vaultRoot`, D3/C5/C6), two-way clan sync (D2), keepassxc composite unlock (D5/C8), or first-class clan placement (D6) — each is a later change in this program and this change's contract is what they rebase onto.
No change to the resolver algebra, the recipient policy renderer, or any generator's own semantics: `mkVault` and `--entry` are both evaluation-path changes, not resolution changes, and their own correctness is proven by agreement with the flake-parts path they sit beside, not by a new independent implementation.
No attempt to make `generate` itself flake-independent. The sandbox's tool resolution is `nix shell --inputs-from`, which needs a flake by construction; this change gives the operator a second flake to point it at, not a way to avoid naming one when generators exist.

## Decisions

### D1. `mkVault` is published at a new top-level `lib.mkVault`, not inside `flake.safix.lib`

`flake.safix.lib` is a value: the resolved projection of whatever `flake.safix.catalogue` and the sibling records hold in the flake that computed it.
It is not a namespace a function can live inside without changing its type — `options.flake.safix.lib` is declared `type = lib.types.attrs`, `readOnly = true`, and every one of its sibling attributes (`placements`, `audiences`, `governedFiles`, …) is data, not a callable.
Putting `mkVault` there would also be circular for the exact consumer it serves: a flakeless consumer's whole reason to reach for `mkVault` is that they have no `flake.safix` namespace of their own to publish a function from, and `flake.safix.lib` is populated per-consumer by each importer's own flake-parts evaluation, not published once by safix's own flake for others to call.

A new `flake.lib.mkVault` has neither problem.
It is a plain function, reachable as `inputs.safix.lib.mkVault` from any nix expression — a flake-parts consumer, a non-flake-parts flake, a plain file reached by `--entry` — with no flake-parts machinery required to call it, which is exactly the property a flakeless consumer needs from the entrypoint itself.
safix's own `flake.nix` currently publishes no `flake.lib` at all, so this is a new file, `modules/flake/lib.nix`, added to `flake.nix`'s `imports`, declaring `options.flake.lib.mkVault` in the same `mkOption`-documented style `options.flake.safix.lib` already uses, and defining it as:

```nix
mkVault = { modules, root }:
  (lib.evalModules {
    modules = [ ./safix { _module.args.self = root; } ] ++ modules;
  }).config.flake.safix.lib;
```

Amended during apply.
This snippet read `../safix` when the change was proposed, which is correct only from a file one level below `modules/flake/`, as `modules/flake/checks/portability.nix` is.
`modules/flake/lib.nix` is a sibling of `modules/flake/safix` rather than a level below it, so `../safix` resolves to a nonexistent `modules/safix` and fails to evaluate; `./safix` is the correct path from that file.
The formula is otherwise implemented verbatim, including the `_module.args.self = root` binding and the `++ modules` composition order.

One further detail the proposal did not settle: `options.flake.lib.mkVault`'s type is `lib.types.raw`, matching the convention `modules/consume/common.nix` already uses for `safix.flake` and `safix.lib`.
`lib.types.functionTo` was tried first and rejected, because its merge wraps the value in a type-checking functor set, which makes `builtins.typeOf` report `set` and `builtins.isFunction` report false even though the value stays callable — failing task 1.2's own stated verification.

**Alternative rejected: extend `flake.safix.lib` with a callable field.**
Two problems, not one. First, it changes `flake.safix.lib`'s type from "a resolved projection" to "a resolved projection that also carries an unrelated function," which every existing reader of that value — `common.nix:285`, the consumption modules, every check that reads `config.flake.safix.lib` — would now see a stray key inside. Second, and more basically, it does not solve the problem: a flakeless consumer has no flake of their own with `flake.safix.lib` populated by anything, since flake-parts is what populates it, so an escape hatch placed inside the thing flake-parts populates is unreachable by exactly the consumer it is for.

### D2. `root` is threaded to `_module.args.self`, unchanged from what the module already reads

`self` is read at exactly two sites downstream (`resolve.nix:648`, `:2021`, `:2093`), always as a path concatenated with `+ "/…"`.
`mkVault`'s `root` argument is handed to `_module.args.self` with no transformation, so the module never learns whether it is running under flake-parts (where `self` is the flake's own store path) or under `mkVault` (where `root` can be any path value the caller names, typically `./.` inside their own entry file).
`portability.nix:80` already demonstrates the degenerate case, `_module.args.self = ""`, to get repository-relative strings out of the comparison; `mkVault`'s callers pass a real path instead, and the module does not need to know the difference.

### D3. Cross-file merging survives because `mkVault`'s `modules` is a `lib.evalModules` module list, the same mechanism flake-parts uses internally

flake-parts' own `imports` handling is sugar over `lib.evalModules`; it is not a separate merge algorithm.
`mkVault { modules = [ ./catalogue-a.nix ./catalogue-b.nix ]; root = ./.; }` merges the two files' `flake.safix.catalogue` declarations by the identical rule a flake-parts consumer's `imports = [ ./catalogue-a.nix ./catalogue-b.nix ]` would, because both paths end at one `lib.evalModules` call over one module list containing `../safix` plus the consumer's files.
This is not a new claim needing a new mechanism to hold it; it is `secret-catalogue`'s existing requirement, restated to name the mechanism rather than one caller of it, and the check that holds it (D3 in `tasks.md` group 1) is the same shape `portability.nix` already uses: one fleet, declared once through each path, compared for equality.

### D4. What `--entry` does to the twelve attribute spellings: nothing. It changes how the target is built, not what is named

`nix.rs`'s `target()` currently builds one `OsString`, `<root>#<attribute>`.
Under `--entry`, the `Nix` driver instead runs `nix eval --file <entry> <attribute>` — two arguments where there was one, but the same `Attribute::as_str()` string in the second position either way.
This is why C2's claim is falsifiable rather than definitional: the twelve strings were fixed before this change existed, and the measurement this session's fixture repeats is that a plain file, evaluated with `--file`, answers to the same twelve strings a flake answers to.

What the entry file itself must contain is a separate question, and it has a real answer rather than an obvious one.
The twelve attributes split unevenly: ten live under `safix.lib.*` (`Placements` through `Keepassxc`), and two, `OnboardingHook` and `EnrollHook`, live at `safix.onboardingHook` and `safix.enrollHook` directly — siblings of `lib`, not fields inside it, per `options.flake.safix`'s declaration.
`mkVault` returns only the `.lib` half, by D1's own contract.
So an entry file built to serve the CLI is not simply `{ safix.lib = mkVault { … }; }`; the recipe this change documents is:

```nix
{
  safix = {
    lib = (import <safix>).lib.mkVault { modules = [ ./secrets.nix ]; root = ./.; };
    onboardingHook = null; # or a literal shell fragment, set here directly
    enrollHook = null;
  };
}
```

`onboardingHook` and `enrollHook` are declared directly in the entry file rather than being threaded through `mkVault`'s `modules`, and this is a deliberate asymmetry rather than an oversight.
`flake.safix.catalogue` and `flake.safix.users` are designed to scatter — that is `secret-catalogue`'s whole first requirement — but `onboardingHook` and `enrollHook` are each a single `nullOr lines` value with no merge semantics of their own to preserve; nothing in their existing contract promises they survive being declared across files, under flake-parts or otherwise.
Reading them back out of the same `evalModules` call `mkVault` performs would mean either widening `mkVault`'s return type past what D1 settles, or running a second, redundant `evalModules` pass just to reach two scalar fields.
Setting them once, directly, in the file the CLI already reads them from, costs nothing a consumer would otherwise have paid, and keeps `mkVault`'s contract exactly what C1 states it to be.

**Alternative rejected: widen `mkVault` to return the whole `flake.safix` attrset, not just `.lib`.**
This directly breaks C1's settled contract ("It returns exactly the value `flake.safix.lib` returns today") and it breaks something a flake-parts consumer would notice: `common.nix:283-297` declares `safix.lib` as "the resolver projection," and a flake-parts consumer's own `flake.safix.lib` has never carried `onboardingHook` or `enrollHook` — they are read from `config.flake.safix.onboardingHook` directly, never through `.lib`. Widening `mkVault` to include them would make the flakeless path's `safix.lib` a different shape than the flake-parts path's, which is the opposite of D1's stated goal — that `mkVault` is a drop-in for the value a flake-parts consumer already gets.

### D5. `generate`'s refusal fires at `generate.rs:213`, before `Envelope::probe`, and names both remedies

The check `if order.is_empty() { … return Ok(0); }` at `generate.rs:201-207` already returns success before `Envelope::probe` for a user with no generator, and this change adds nothing before that check — a user with an empty order is unaffected under `--entry` exactly as the shared contract's measured fact states.
The refusal this change adds sits immediately after that check and immediately before the `Envelope::probe` call at `:213`: when the workspace is running under `--entry`/`SAFIX_ENTRY` and neither `--nixpkgs` nor `SAFIX_NIXPKGS` names a flake reference, the command returns a refusal naming the two facts an operator needs — that the generator sandbox resolves its tools through `nix shell --inputs-from`, a flake-only operation, and that the fix is either of two options: drop `--entry` and run against the declaring flake, or add `--nixpkgs <flake-ref>`.
When `--nixpkgs`/`SAFIX_NIXPKGS` is given, `nix.rs`'s `shell()` (`:205-213`) resolves `nixpkgs#<attribute>` against the declared reference directly rather than against `--inputs-from <root>`; the declared reference must itself provide `legacyPackages`/`packages` the way `nixpkgs#<attribute>` expects, exactly as this flake's own pinned `nixpkgs` input does today.

### D6. The catalogue's flake-parts-specific wording is removed, not weakened into a maybe

`secret-catalogue/spec.md:11`'s "SHALL be an attribute set option on a flake-parts module" is replaced with "SHALL be an attribute set option merged by the nix module system," naming both routes a consumer reaches that merge through.
This is not a loosening that changes what is guaranteed — the module-system merge rule is exactly what "attribute set option on a flake-parts module" already rested on, since flake-parts contributes no merge semantics of its own — it is a correction of a sentence that named one caller of a mechanism as though it were the mechanism.

### D7. Module entrypoint naming: `homeManagerModules` is a plain alias, defined once, in the same `let`

sops-nix's own `flake.nix:69-73` publishes both `homeManagerModules` and `homeModules` for the identical module value.
safix's `flake.nix` already builds `homeModules` and `nixosModules` as literal attrsets inside `outputs`; this change binds that attrset to a name in the enclosing `let` and references it twice — `homeModules = homeModules; homeManagerModules = homeModules;` — rather than duplicating the two-line `{ safix = …; default = { imports = …; }; }` attrset a second time. A second copy would be a second place for the `.safix`/`.default` split to drift out of sync; a shared binding cannot drift.

### D8. `Workspace::discover` and git-based root discovery are unchanged

Already established as a measured fact and repeated here because it bounds this design rather than being incidental to it: `--entry` changes only how declarations are evaluated, never where a run stages, commits, or discovers its repository root.
A consumer's entry file need not live inside the repository `Workspace::discover` finds, and nothing in this change makes it need to.

### D9. `safix-portability` is split: the two home shapes' agreement is held on every system, the system shape's own involvement stays linux-only

`safix-portability`'s `checks` binding at `portability.nix:449` gates the whole check — including the `homeInNixos`-vs-`standalone` half of its `agreeOn` comparisons — behind `pkgs.stdenv.hostPlatform.isLinux`, for a reason that is real for only one of the three shapes it holds.
`nixosFor` (`:276-303`) builds `inputs.nixpkgs.lib.nixosSystem`, which does not evaluate off Linux; `homeFor` (`:306-333`) builds an ordinary home-manager configuration and needs no host platform at all, and its `osConfig` argument for the `homeInNixos` shape is the synthetic `insideNixos = { networking.hostName = hostname; }` (`:334-336`), not a second `nixosSystem`.
So two of the check's three shapes, and every comparison between just those two, do not need the gate that currently covers all of them.

This change splits `safix-portability` into two checks rather than widening or removing the gate on the existing one.
`safix-portability-system` keeps every assertion that reads or compares against the `nixos` shape — `resolution.agree`, `systemIdentity`, `serviceOwnership.system`, and the `nixos` row of every per-field comparison — gated to `isLinux` exactly as today, because that gate is genuinely load-bearing: a `nixosSystem` will not evaluate on darwin, and nothing in this change claims otherwise.
`safix-portability-home` is new, ungated, and holds the comparison this change's own Goals rely on but the checked-in tree does not yet prove cross-platform: that `homeInNixos` and `standalone` agree with each other on every field `agreeOn` already covers — `person`, `machine`, `machineEntry`, `organizationEntry`, `organizationOwnedEntry`, `serviceEntry`, `servicePath`, `serviceOwnership.{homeInNixos,standalone}`, `standalone.{machine,resolvesWithoutAHostname,tagsComeFromTheDeclaration}`, and every `refusalsOf` fixture's `homeInNixos`/`standalone` pair — read off the same `shapes` binding `safix-portability-system` uses, so neither check computes a second copy of the fixture fleet.
Both checks share the `fleet`, `broken`, `shapes`, and `shapeOf`/`homeFor`/`nixosFor` bindings already in `portability.nix`'s `let`; splitting the `checks` attrset does not require splitting the file's fixture construction, only where the `isLinux` gate is applied.

This is what the shared contract's own measured fact — that the home-manager consumption shape evaluates identically on `aarch64-darwin` and `x86_64-linux`, "exit 0, empty stderr, resolved output byte-identical apart from `coreutilsOutPath` and `systemPlatform`" — turns out to support rather than contradict, once it is read against the right check.
It is not evidence that `consumption.nix:428`'s gate is over-broad, which this session's direct measurement of `.#checks.<system>` already showed to be narrow and correct as checked in.
It is evidence that the narrow gate is the right one precisely because the home-manager shape's own behaviour does not vary by platform — and it is the reason `safix-portability-home` is safe to run ungated: nothing about `homeInNixos` or `standalone` reads `pkgs.stdenv.hostPlatform`, `pkgs.system`, or any store path that would differ between the two builders, which `tasks.md` group 5 now holds as a check rather than as this paragraph's prose.

Separately, and still worth having: the regression guard on `consumption.nix:428` is kept.
Nothing today would fail red if a future edit moved a home-manager consumption check inside that file's `isLinux` block by habit, alongside `safix-consumption-system` — the file's own header comment states the split, and a comment is not what `tasks.md`'s own discipline accepts as the final form of a claim it can hold.
That check is not a fix for anything broken; it is insurance against the file drifting into the state the shared contract's original wording described.

## Risks / Trade-offs

Two flake-only mechanisms move to being consumer-choosable rather than absolute, and each is a place a consumer can now get a working evaluation that silently diverges from what a flake would have produced, if they choose badly.
`--entry` reading a hand-maintained file rather than a flake's locked `nixpkgs` means a generator or check depending on package versions could see a different `nixpkgs` than the one this flake pins — bounded by D5's refusal firing whenever a generator would actually need one, and by nothing else, because `--entry`'s other fourteen verbs read no package version at all, only nix values.
`--nixpkgs <flake-ref>` accepting an arbitrary flake reference means an operator can point `generate` at a `nixpkgs` that does not carry the tool a generator names; that failure is `nix shell`'s own "attribute not found," not a new refusal this change adds, and it is bounded the same way an operator's own typo in any flake reference already is.

Publishing `lib.mkVault` as a new top-level output is a permanent addition to safix's public surface, and unlike an internal check fixture, it is now something an external consumer's tree can come to depend on.
That is the point of this change rather than a cost of it, but it does mean `mkVault`'s contract — that it returns exactly `flake.safix.lib`'s shape — is now a promise this repository has to keep across future changes to that shape, the same way `flake.safix.lib`'s own fields already are.

`safix-portability`'s split (D9) adds a second check rather than replacing the first, so both run on every `nix flake check` from here on; the cost is one more evaluation of the same fixture fleet, already bounded by `safix-portability-home` sharing every binding `safix-portability-system` uses.
The consumption.nix finding itself is settled: the orchestrator independently re-measured `.#checks.<system>` against this tree and confirmed the gate at `:428` is narrow and correct as checked in, so `tasks.md` group 5 carries a regression guard rather than a fix.

## Migration Plan

This program is additive, and this change is where that has to be verified rather than merely stated, since the other four changes in the program depend on it holding.
`flakeModules.default` keeps working unchanged: nothing in `modules/flake/safix/` is touched by this change beyond the new sibling file `modules/flake/lib.nix`, so a consumer already importing `flakeModules.default` sees no different `flake.safix.*` surface.
`nixosModules.default` and `homeModules.default` keep importing sops-nix and `modules/consume/*.nix` exactly as before; `homeManagerModules` is a new name pointing at an unchanged value, not a rename.
`secret-catalogue`'s relaxed wording removes a constraint, and removing a constraint cannot break a consumer who was operating inside it.
`rust-runtime`'s evaluation-seam wording gains an alternative it did not name before; the case it already named — evaluating the consumer's flake — is untouched.

What would constitute a break of C3, stated so it is checkable rather than a matter of judgment: any change to `modules/flake/safix/default.nix`'s existing option surface or `flake.safix.lib`'s existing fields; any change to what `nixosModules.default` or `homeModules.default` import or in what order; any narrowing of `secret-catalogue`'s vocabulary (fields an entry may declare) rather than widening of where declarations may be merged; or `generate` refusing under any condition it does not refuse under today when no `--entry` is given. None of those appear in this change's task list, and a reviewer checking this change against C3 checks exactly that list.

There is no rollback distinct from reverting the commit: no data migrates, no file changes shape, and a consumer who adopts none of `--entry`, `SAFIX_ENTRY`, `--nixpkgs`, `SAFIX_NIXPKGS`, `lib.mkVault`, or `homeManagerModules` is running the same evaluation paths as before this change landed.

## Open Questions

None that would change the specs, the approach, or the task breakdown.
D9's finding was raised with the operator during this session rather than left open here; the operator independently re-measured the same tree, confirmed the consumption.nix gate is already correct, and identified the real gap this design now closes — `safix-portability`'s bundling of the `nixos` shape with the two platform-independent shapes. Nothing about that exchange changed this change's specs or its task breakdown beyond the split `tasks.md` group 5 now carries.
