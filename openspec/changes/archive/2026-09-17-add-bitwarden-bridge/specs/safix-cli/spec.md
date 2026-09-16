## MODIFIED Requirements

### Requirement: A global entry file replaces the flake for evaluation

The command SHALL accept a global `--entry <file>` option and a `SAFIX_ENTRY` environment variable naming a plain nix file.
When either is set, every nix evaluation SHALL target that file — `nix eval --file <file> <attribute>` — instead of `<root>#<attribute>`, using the same attribute spellings the flake path uses.
`--entry` SHALL take precedence over `SAFIX_ENTRY` when both are given.
Fifteen of the sixteen subcommands SHALL behave identically under `--entry` as against a flake; `generate` is the documented exception.
`install` is identical under either because it evaluates no nix at all.
Every attribute the runtime evaluates SHALL be reachable under `--entry` on the day it exists, including one a new sync target introduces.

#### Scenario: The attribute spellings are unchanged

- **WHEN** any attribute the runtime evaluates is read under `--entry`
- **THEN** the string naming it is identical to the one read against a flake
- **AND** only how the target is built — `--file <entry> <attribute>` rather than `<root>#<attribute>` — differs
- **AND** the enumeration is "every attribute" rather than a count, because a count in a scenario no check holds goes stale the first time an attribute is added and says nothing true about the one that was

#### Scenario: Nested payload shapes evaluate identically under either path

- **WHEN** an entry file's declarations include at least one generator, one bridge mapping, one keepassxc mapping, and one bitwarden mapping
- **THEN** the JSON `--entry` evaluation emits for `generatorPlan`, `bridge`, `keepassxc`, and `bitwarden` deserializes without an unknown-field error against the same `#[serde(deny_unknown_fields)]` structs the flake path deserializes against
- **AND** the same fixture, evaluated against an equivalent flake, produces byte-identical JSON apart from any store path a generator's tooling reference embeds

#### Scenario: A verb unaffected by --entry works exactly as it does against a flake

- **WHEN** `list`, `get`, `set`, `check`, `fix`, `keygen`, `adduser`, `enroll`, `group`, `audit`, `sync`, `upload`, or `edit` is run under `--entry`
- **THEN** its behaviour, output, and exit code match the same invocation against an equivalent flake
- **AND** neither git operations nor workspace root discovery change, because `--entry` governs only how declarations are evaluated

#### Scenario: A verb that evaluates nothing is trivially unaffected

- **WHEN** `install` is run with `--entry` or `SAFIX_ENTRY` set
- **THEN** it behaves exactly as without them, because its whole input is the manifest path it was given
- **AND** it performs no workspace discovery either, so a manifest installs on a machine that holds no declaring repository at all

#### Scenario: The workspace root is still found by git

- **WHEN** `--entry` or `SAFIX_ENTRY` is set
- **THEN** the repository root a run stages and commits into is still the one git reports for the current directory
- **AND** the entry file need not be inside that repository, though every declaration it makes about paths under `root` still resolves relative to whatever `root` its own expression names
