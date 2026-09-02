## ADDED Requirements

### Requirement: A global entry file replaces the flake for evaluation

The command SHALL accept a global `--entry <file>` option and a `SAFIX_ENTRY` environment variable naming a plain nix file.
When either is set, every nix evaluation SHALL target that file — `nix eval --file <file> <attribute>` — instead of `<root>#<attribute>`, using the same attribute spellings the flake path uses.
`--entry` SHALL take precedence over `SAFIX_ENTRY` when both are given.
Fourteen of the fifteen subcommands SHALL behave identically under `--entry` as against a flake; `generate` is the documented exception.

#### Scenario: The attribute spellings are unchanged

- **WHEN** any of the twelve attributes the runtime evaluates is read under `--entry`
- **THEN** the string naming it is identical to the one read against a flake
- **AND** only how the target is built — `--file <entry> <attribute>` rather than `<root>#<attribute>` — differs

#### Scenario: Nested payload shapes evaluate identically under either path

- **WHEN** an entry file's declarations include at least one generator, one bridge mapping, and one keepassxc mapping
- **THEN** the JSON `--entry` evaluation emits for `generatorPlan`, `bridge`, and `keepassxc` deserializes without an unknown-field error against the same `#[serde(deny_unknown_fields)]` structs the flake path deserializes against
- **AND** the same fixture, evaluated against an equivalent flake, produces byte-identical JSON apart from any store path a generator's tooling reference embeds

#### Scenario: A verb unaffected by --entry works exactly as it does against a flake

- **WHEN** `list`, `get`, `set`, `check`, `fix`, `keygen`, `adduser`, `enroll`, `group`, `import`, `export`, `audit`, `sync`, or `edit` is run under `--entry`
- **THEN** its behaviour, output, and exit code match the same invocation against an equivalent flake
- **AND** neither git operations nor workspace root discovery change, because `--entry` governs only how declarations are evaluated

#### Scenario: The workspace root is still found by git

- **WHEN** `--entry` or `SAFIX_ENTRY` is set
- **THEN** the repository root a run stages and commits into is still the one git reports for the current directory
- **AND** the entry file need not be inside that repository, though every declaration it makes about paths under `root` still resolves relative to whatever `root` its own expression names

### Requirement: generate requires a flake or a declared nixpkgs reference

`safix generate` SHALL refuse, before running any generator, when it is invoked under `--entry` or `SAFIX_ENTRY`, no `--nixpkgs <flake-ref>` or `SAFIX_NIXPKGS` is given, and the target user's `generatorPlan.order` is non-empty.
The refusal SHALL name the reason — that the generator sandbox resolves its tools through a flake — and both remedies: dropping `--entry` to run against the declaring flake, or supplying `--nixpkgs`.
A user whose `generatorPlan.order` is empty SHALL succeed under `--entry` with no nixpkgs reference, unchanged from today.

#### Scenario: A declared generator refuses without a flake or a nixpkgs reference

- **WHEN** `safix generate` is run under `--entry`, the target user's generator order is non-empty, and neither `--nixpkgs` nor `SAFIX_NIXPKGS` is set
- **THEN** the command refuses before any generator runs
- **AND** the refusal names both remedies

#### Scenario: A user with no generator is unaffected

- **WHEN** `safix generate` is run under `--entry` for a user whose `generatorPlan.order` is empty
- **THEN** the command succeeds, having done nothing, exactly as it does against a flake

#### Scenario: A declared nixpkgs reference lifts the refusal

- **WHEN** `safix generate` is run under `--entry` with `--nixpkgs <flake-ref>` or `SAFIX_NIXPKGS` set, and the target user's generator order is non-empty
- **THEN** the sandbox resolves its tools against the declared reference instead of `--inputs-from <root>`
- **AND** every generator in the order runs exactly as it would against a flake

#### Scenario: Flake mode is unaffected

- **WHEN** `safix generate` is run with neither `--entry` nor `SAFIX_ENTRY` set
- **THEN** the refusal never fires, and `--nixpkgs` and `SAFIX_NIXPKGS`, if given, are ignored, because `--inputs-from <root>` already resolves the sandbox's tools
