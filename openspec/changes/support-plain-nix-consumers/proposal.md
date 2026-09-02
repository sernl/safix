# safix works for a consumer with no flake-parts, and no flake at all

Revisions are as recorded in the program's shared contract: sops-nix at `f140661`, clan-core at `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, this flake's own `nixpkgs`.
Every measured claim below was independently re-verified against the repository at those revisions, including two claims made directly in this session by evaluating `.#checks.<system>` rather than by reading source.

## Why

Every one of safix's fifteen verbs, including the fourteen that only read the resolver's answers, currently requires a flake, and `secret-catalogue`'s own spec forbids anything else: `openspec/specs/secret-catalogue/spec.md:11` states the catalogue "SHALL be an attribute set option on a flake-parts module."
That sentence is narrower than what the mechanism underneath it needs.
`lib.evalModules` merges declarations from a list of files regardless of who calls it, flake-parts included — `modules/flake/checks/portability.nix:76-83` already runs that exact call outside any flake-parts context to prove three consumption shapes agree — and the Rust runtime's own view of the projection, `flake.safix.lib.*` in `crates/safix-core/src/nix.rs:52-67`, is an ordinary attribute path that `nix eval --file` reads from a plain file with zero failures, measured directly, ten attributes as JSON and two as raw text.

Nothing about safix's opinion — one declaration per file, scattered anywhere, merged by the module system — depends on flake-parts.
What depends on it is one thing only: the option that says the catalogue must live there.
A consumer who has no flake at all — a NixOS configuration built by `nixos-rebuild --flake` against someone else's flake, a home-manager profile on a non-NixOS host with a hand-written `configuration.nix`, a tree mid-migration off channels — cannot adopt safix today, and the four changes that follow this one in the same program (two-way clan sync, a separate vault repository, keepassxc composite-key unlock, first-class clan var placement) all extend a consumer surface this change has to open first.

Two further defects belong here because they were found while establishing this one.
`crates/safix-core/src/generate.rs:213` reaches `Envelope::probe`, which needs `nix shell --inputs-from <root>` (`nix.rs:205-213`) — a flake-only operation — to resolve the sandbox's own tools; the other fourteen verbs never call it, so `generate` alone keeps a flake requirement rather than losing one.
And this session's own measurement of `.#checks.aarch64-darwin` and `.#checks.x86_64-linux` against the current tree found `safix-consumption`, `safix-consumption-ordering` and `safix-consumption-refusals` already unconditioned by `pkgs.stdenv.hostPlatform.isLinux`, and only `safix-consumption-system` gated — the narrow scoping the shared contract's darwin item asks for already exists in the checked-in file at `modules/flake/checks/consumption.nix:428`.
What is missing is not a gate to narrow; it is a check that holds the narrowing in place, since nothing today would fail if a future edit widened the gate back over the home-manager checks it does not need.

## What Changes

- Publish `lib.mkVault`, a new top-level flake output independent of flake-parts: `mkVault { modules, root } -> projection`, returning exactly the value `flake.safix.lib` returns today, by running `lib.evalModules` over `modules/flake/safix` plus the given `modules`, with `root` supplied as `_module.args.self`.
- Add a global `--entry <file>` CLI option and `SAFIX_ENTRY` environment variable to the Rust runtime. When set, every `nix eval` targets `--file <entry> <attribute>` instead of `<root>#<attribute>`; the twelve attribute spellings in `nix.rs:52-67` are unchanged.
- `generate` gains a `--nixpkgs <flake-ref>` option and `SAFIX_NIXPKGS` environment variable. Under `--entry`, with no nixpkgs reference declared, and only when the target user's `generatorPlan.order` is non-empty, `generate` refuses at evaluation naming the reason (the sandbox's tools resolve through a flake) and the remedy (drop `--entry`, or supply `--nixpkgs`). Fourteen of fifteen verbs are unaffected by `--entry`.
- Relax `secret-catalogue`'s requirement that the catalogue live on a flake-parts module: it becomes an attribute set merged by the module system, reached either through a flake-parts import or through `lib.mkVault`'s `modules` argument.
- Add `homeManagerModules` as an alias of `homeModules`, matching sops-nix's own `flake.nix:69-73`, and document that `nixosModules.safix` and `homeModules.safix` already import nothing and so are importable by a plain file path with no flake anywhere in the consumer's tree.
- Update `rust-runtime`'s evaluation-seam requirement, which currently states the runtime evaluates "the declarations the consumer's flake carries," to admit the entry-file path as well.
- Add a regression-guard check asserting the home-manager consumption checks carry no platform gate, replacing the header comment at `consumption.nix:425-427` that currently asserts this in prose alone.
- Close the one measurement gap the shared contract names undischarged: a fixture populating `Mapping`, `SyncMapping`, `Generator`/`GeneratorFile` and `PlanInput` and asserting the emitted JSON matches their `#[serde(deny_unknown_fields)]` shapes byte for byte, under both the flake and the `--entry` evaluation path.
- `flakeModules.default` and every existing flake-based consumer are unchanged. Nothing described here removes an output, narrows a type, or changes a default for a consumer who never sets `--entry`, `SAFIX_ENTRY`, or `SAFIX_NIXPKGS`.

Not in scope: the two-way clan vars agreement memory, a separate vault repository and the `vaultRoot`/`declarationRoot` split, keepassxc composite-key unlock, and first-class clan placement — each is its own change in this program and rebases onto this one's contract.
Also not in scope: any change to `secret-installation`, `secret-generators`' resolver algebra, or the bridge and keepassxc mapping types themselves — the schema-fidelity check this change owes exercises their existing shapes and changes none of them.

## Capabilities

### New Capabilities

- `flakeless-projection`: `lib.mkVault`, the function that produces the same resolver projection a flake-parts consumer's `flake.safix.lib` holds, from a plain module list and a root path, for a consumer that imports no flake-parts module at all — and how that projection reaches a consumption module through the `safix.lib` option those modules already expose.

### Modified Capabilities

- `secret-catalogue`: the catalogue's mergeability requirement no longer names flake-parts specifically; it names the module-system merge that both flake-parts and `lib.mkVault` perform.
- `consumer-integration`: gains the module entrypoint naming and no-flake import requirement — `homeManagerModules`, and that the consumption modules import without a flake.
- `safix-cli`: gains the `--entry`/`SAFIX_ENTRY` global option and the `generate`-specific `--nixpkgs`/`SAFIX_NIXPKGS` option and refusal.
- `rust-runtime`: the evaluation-seam requirement's wording is widened from "the consumer's flake" to also admit the entry file.

## Impact

Affected code:

- New: `modules/flake/lib.nix` — the flake-parts module declaring `flake.lib.mkVault`, imported into `flake.nix`'s `imports`.
- Modified: `flake.nix` — the new import, and `homeManagerModules` added beside `homeModules`.
- Modified: `crates/safix-core/src/nix.rs` — the `Nix` driver gains an entry-file mode that changes how `target()` is built, and a nixpkgs-reference field `generate`'s sandbox resolution reads.
- Modified: `crates/safix-core/src/generate.rs` — the pre-probe refusal.
- Modified: `crates/safix/src/main.rs`, `crates/safix/src/usage.rs` — the two new global options and their help text.
- Modified: `openspec/specs/secret-catalogue/spec.md:11` — the flake-parts-specific wording.
- Modified: `modules/flake/checks/consumption.nix` — a regression-guard check that the home-manager consumption checks carry no platform gate.

Affected checks: a new check under `modules/flake/checks/` proving `lib.mkVault`'s output equals `flake.safix.lib`'s output over the same fleet (mirroring `portability.nix`'s mechanism); a new or extended check in `modules/flake/checks/cli.nix` proving the twelve attribute spellings resolve identically through `--entry` and through the flake, including the four nested shapes this change's own fixture populates; a new check proving `generate`'s refusal fires and is bypassed by `--nixpkgs`.
