# Tasks: support-plain-nix-consumers

Revisions and line anchors are as recorded in `proposal.md` and `design.md`.
"Hold" means add a check that fails when the claim stops being true, not add a sentence asserting it — the same discipline `own-secret-installer`'s tasks used.
No real fleet identifier enters this repository; fixtures use `alice`, `bob`, `carol` and synthetic `age1` strings, matching the existing fixtures in `modules/flake/checks/`.

## 1. `lib.mkVault`

- [ ] 1.1 Add `modules/flake/lib.nix` declaring `options.flake.lib.mkVault`, `mkOption`-documented in the style of `options.flake.safix.lib`, and add it to `flake.nix`'s `imports`
- [ ] 1.2 Define `mkVault = { modules, root }: (lib.evalModules { modules = [ ../safix { _module.args.self = root; } ] ++ modules; }).config.flake.safix.lib;`, verified by `nix eval .#lib.mkVault --apply builtins.isFunction`
- [ ] 1.3 Add a check asserting `mkVault`'s return value equals `flake.safix.lib`'s return value over one fleet declared twice — once as `flake.safix.*` in a flake-parts module fixture, once as the same fields passed through `mkVault`'s `modules` — mirroring `portability.nix:76-83`'s own mechanism
- [ ] 1.4 In the same check, assert a fleet scattered across two or more fixture files passed to `mkVault`'s `modules` resolves identically to the same fleet in one file, and that two of those files declaring the same catalogue name with different fields is a module-system evaluation error naming the option
- [ ] 1.5 Assert a module in `mkVault`'s `modules` declaring an option outside `flake.safix` is refused by the same mechanism `modules/flake/checks/namespace.nix` holds for the flake-parts path, by pointing that check's own scan at a fixture reached through `lib.evalModules` directly rather than through `flake-parts.lib.mkFlake`
- [ ] 1.6 Assert `mkVault`'s return value carries no `onboardingHook` or `enrollHook` key, holding D4's stated boundary between `mkVault`'s contract and the entry-file recipe
- [ ] 1.7 Severity drill: changing `mkVault`'s module list order in the fixture and re-declaring the same fleet turns nothing red, which is the evidence order does not matter; dropping `../safix` from the list turns 1.3 red on a missing `flake.safix.lib`
- [ ] 1.8 Verify: `nix build .#checks.x86_64-linux.safix-vault-projection` (or the chosen check name) green, and the drill in 1.7 observed

## 2. Module entrypoints

- [ ] 2.1 In `flake.nix`, bind the existing `homeModules` attrset to a `let`-bound name and add `flake.homeManagerModules = homeModules;` beside `flake.homeModules = homeModules;`, so both names reference one definition
- [ ] 2.2 Add a check asserting `homeManagerModules.safix` and `homeManagerModules.default` are the same value as `homeModules.safix` and `homeModules.default`, by `==` over the evaluated attrsets
- [ ] 2.3 Add a check that evaluates `modules/consume/home.nix` and `modules/consume/nixos.nix` by direct `import <path>` with no flake input anywhere in the evaluation — a bare `lib.evalModules` fixture with no `inputs` in scope — and asserts each evaluates and declares `options.safix.lib`
- [ ] 2.4 Assert in the same check that neither `nixosModules.safix` nor `homeModules.safix`'s import list names anything under `inputs.sops-nix`, distinguishing the `.safix` forms' zero-flake property from the `.default` forms', which do
- [ ] 2.5 Severity drill: pointing `homeManagerModules` at a fresh copy of the module rather than at `homeModules` turns 2.2 red; adding an import to `modules/consume/home.nix` turns 2.4 red
- [ ] 2.6 Verify: `nix build .#checks.x86_64-linux.safix-module-entrypoints` (or the chosen check name) green, and both drills in 2.5 observed

## 3. The `--entry` / `SAFIX_ENTRY` evaluation path

- [ ] 3.1 Add an `entry: Option<PathBuf>` field to `Nix` (`crates/safix-core/src/nix.rs`), constructed from a new `Nix::from_environment` branch reading `SAFIX_ENTRY`, and a builder or second constructor an explicit `--entry` value can set, with `--entry` overriding `SAFIX_ENTRY` when both are present
- [ ] 3.2 Change `target()`'s callers (`eval_json`, `eval_raw`, `eval_raw_to`) to branch: with `entry` set, run `nix eval --file <entry> <attribute>`; without it, the existing `<root>#<attribute>` form, keeping `Attribute::as_str()` identical either way
- [ ] 3.3 Parse a leading `--entry <file>` global option in `crates/safix/src/main.rs`'s `run()`, before subcommand dispatch, alongside the existing `-h`/`--help`/`--version` handling, and thread the resulting `Nix` into `Workspace::at` in place of `Nix::from_environment()`
- [ ] 3.4 Add `usage::SCAFFOLD` and the global-option section of `-h` text documenting `--entry` and `SAFIX_ENTRY`, matching the existing usage-text-is-a-contract convention in `usage.rs`
- [ ] 3.5 Add a hermetic check in `modules/flake/checks/cli.nix` that evaluates a fixture entry file — `{ safix = { lib = mkVault { … }; onboardingHook = null; enrollHook = null; }; }` — through each of the twelve `Attribute` spellings via the stubbed `nix`, and asserts every one succeeds with the same shape a flake-mode stub returns
- [ ] 3.6 Extend the same fixture entry file to declare at least one generator, one bridge mapping under `flake.safix.bridge`, and one keepassxc mapping under `flake.safix.keepassxc` — the residual inference this program owes — and assert the emitted `generatorPlan`, `bridge`, and `keepassxc` JSON deserializes against `Generator`, `GeneratorFile`, `Mapping`, `SyncMapping`, and `PlanInput` with no `#[serde(deny_unknown_fields)]` rejection
- [ ] 3.7 Assert the same fixture's flake-mode evaluation (the equivalent declared as `flake.safix.*` in a flake-parts fixture) produces byte-identical JSON for the three nested attributes, apart from any generator tooling store path
- [ ] 3.8 Assert `Workspace::discover`'s root is unaffected by `--entry` being set — a fixture run with `--entry` pointed outside the discovered git root still stages and commits into that root
- [ ] 3.9 Severity drill: setting only `SAFIX_ENTRY` and a conflicting `--entry` turns 3.x red on whichever the test expects to win (`--entry`); dropping one field from the nested fixture in 3.6 turns 3.6 red rather than passing silently
- [ ] 3.10 Verify: `nix build .#checks.x86_64-linux.safix-cli` (or wherever the extended check lands) green, and both drills in 3.9 observed; `cargo test` covering the new `Nix` branching green

## 4. `generate`'s flakeless refusal

- [ ] 4.1 Add a `nixpkgs: Option<String>` field threaded the same way `entry` is in group 3, read from a `--nixpkgs <flake-ref>` global option and `SAFIX_NIXPKGS`
- [ ] 4.2 In `crates/safix-core/src/generate.rs`, immediately after the `order.is_empty()` early return at `:201-207` and before the `Envelope::probe` call at `:213`, add the refusal: fires when `entry` is set and `nixpkgs` is not, naming the sandbox's flake-only tool resolution and both remedies
- [ ] 4.3 Change `Nix::shell` (`nix.rs:205-213`) to resolve `nixpkgs#<attribute>` against the declared `nixpkgs` reference directly when `entry` is set and `nixpkgs` is given, instead of `--inputs-from <root>`
- [ ] 4.4 Add a new `Error`/`Refusal` variant carrying the two remedy strings, rendered by `reporter.rs` the way every other refusal is, with a snapshot held the way `rendering is pinned` (`rust-runtime` spec) requires
- [ ] 4.5 Add a check or `cargo test` fixture: a user with a non-empty generator order, run under `--entry` with no `--nixpkgs`, refuses before any generator's script runs; the same user with `--nixpkgs` set against this flake's own pinned `nixpkgs` runs its generator and produces the same output flake mode would
- [ ] 4.6 Add a fixture for a user with an empty generator order, run under `--entry` with no `--nixpkgs`, and assert it succeeds unchanged
- [ ] 4.7 Severity drill: removing the `order.is_empty()` guard's placement ahead of the new refusal would refuse an empty-order user; assert directly that it does not, since this is the drill for the ordering rather than for the refusal's presence
- [ ] 4.8 Verify: the checks/tests in 4.5-4.6 green, and the drill in 4.7 observed

## 5. Darwin: splitting the real gap, and guarding the gate that is already correct

Confirmed in `design.md` D9 by direct measurement (`nix eval .#checks.<system> --apply builtins.attrNames`, both in this change's own research and independently by the orchestrator): `consumption.nix:428`'s gate is already narrow and correct as checked in. The work here is the `safix-portability` split plus a guard against the gate that is already right, not a fix to either.

- [ ] 5.1 In `modules/flake/checks/portability.nix`, split the `checks` attrset at `:449` into `safix-portability-system` (every assertion reading or comparing against the `nixos` shape: `resolution.agree`, `systemIdentity`, `serviceOwnership.system`, and the `nixos` row of `machineEntry`/`organizationEntry`/`organizationOwnedEntry`/`serviceEntry`/`servicePath`/`refusals`), kept under `lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux`, and `safix-portability-home` (the `homeInNixos`-vs-`standalone` half of the same fields, plus `standalone.{machine,resolvesWithoutAHostname,tagsComeFromTheDeclaration}`), ungated — both reading the same `shapes`, `fleet`, `broken`, `shapeOf`/`homeFor`/`nixosFor` bindings already in the file's `let`
- [ ] 5.2 Verify `safix-portability-home` needs no `nixosFor` call anywhere in its own `actual`/`expected` construction, by grepping the new check's source for `nixosFor` and asserting no match, holding D9's claim that the two home shapes need nothing platform-specific
- [ ] 5.3 Assert `safix-portability-home` builds and evaluates on `aarch64-darwin`: `nix build .#checks.aarch64-darwin.safix-portability-home` succeeds, which is the first time this repository holds the standalone-shape agreement claim on a non-Linux builder
- [ ] 5.4 Severity drill: dropping `laptop-token` from `homeInNixos`'s side of a per-field comparison in the new check (while leaving `standalone`'s side alone) turns `safix-portability-home` red on that field alone, and `safix-portability-system` stays green, which is the evidence the split actually separated the two claims rather than merely renaming one check
- [ ] 5.5 Severity drill: reverting 5.1's split — folding `safix-portability-home`'s assertions back under the `isLinux` gate — turns nothing red on `x86_64-linux` and makes 5.3 inapplicable on `aarch64-darwin` (the check no longer exists there), which is the state this group exists to move away from
- [ ] 5.6 Add a regression-guard check asserting that the attribute names present in each system's `checks` set, compared against `x86_64-linux`'s, differ only by `safix-consumption-system`'s and `safix-portability-system`'s presence — read via `builtins.attrNames` across `flake.nix`'s fixed `systems` list, the way this session's own measurement was taken — so a future edit that moves a home-manager check inside either file's `isLinux` block reddens this check rather than passing silently
- [ ] 5.7 Severity drill: moving `safix-consumption` or `safix-portability-home` inside either file's `lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux { … }` block turns 5.6 red
- [ ] 5.8 Verify: `nix build .#checks.x86_64-linux.safix-portability-system .#checks.x86_64-linux.safix-portability-home .#checks.aarch64-darwin.safix-portability-home .#checks.x86_64-linux.safix-consumption-gate-guard` (or the chosen names) all green, and all three drills (5.4, 5.5, 5.7) observed

## 6. Documentation

- [ ] 6.1 Document `lib.mkVault` in `README.md`: signature, what it returns, the `root`/`_module.args.self` relationship, and the `onboardingHook`/`enrollHook` boundary from D4
- [ ] 6.2 Document `--entry`, `SAFIX_ENTRY`, `--nixpkgs`, and `SAFIX_NIXPKGS`, including the full entry-file recipe from D4 and the fourteen-of-fifteen-verbs boundary
- [ ] 6.3 Document `homeManagerModules` beside the existing `homeModules`/`nixosModules` documentation, and the `.safix`-forms-import-nothing property now load-bearing for zero-flake use
- [ ] 6.4 Update any README prose that currently states or implies flake-parts is required for evaluation, and cite the corrected `secret-catalogue` requirement
- [ ] 6.5 Verify: every guarantee stated in the new README prose names a check or test in this repository that holds it

## 7. Verification

- [ ] 7.1 `openspec validate support-plain-nix-consumers --strict`
- [ ] 7.2 `openspec validate --all --strict`, compared against the baseline recorded when this change was proposed
- [ ] 7.3 `nix eval .#checks.x86_64-linux --apply builtins.attrNames` and `nix eval .#checks.aarch64-darwin --apply builtins.attrNames` both list every new check named in groups 1, 2, 3, 5
- [ ] 7.4 `nix flake check` green
- [ ] 7.5 `cargo test` green
- [ ] 7.6 `rg` the whole tree for any real fleet identifier and confirm none, matching the discipline every prior change in this program held
