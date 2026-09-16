## MODIFIED Requirements

### Requirement: A global entry file replaces the flake for evaluation

The command SHALL accept a global `--entry <file>` option and a `SAFIX_ENTRY` environment variable naming a plain nix file.
When either is set, every nix evaluation SHALL target that file — `nix eval --file <file> <attribute>` — instead of `<root>#<attribute>`, using the same attribute spellings the flake path uses.
`--entry` SHALL take precedence over `SAFIX_ENTRY` when both are given.
Fifteen of the sixteen subcommands SHALL behave identically under `--entry` as against a flake; `generate` is the documented exception.
`install` is identical under either because it evaluates no nix at all.
A sync target added to the runtime adds one attribute to the set every evaluation path reads, so this requirement SHALL be stated over each attribute the runtime evaluates rather than over a count of them, which a target change would falsify without changing any behaviour.

#### Scenario: The attribute spellings are unchanged

- **WHEN** each attribute the runtime evaluates is read under `--entry`
- **THEN** the string naming it is identical to the one read against a flake
- **AND** only how the target is built — `--file <entry> <attribute>` rather than `<root>#<attribute>` — differs

#### Scenario: Nested payload shapes evaluate identically under either path

- **WHEN** an entry file's declarations include at least one generator, one bridge mapping, one keepassxc mapping, and one pass mapping declaring metadata fields
- **THEN** the JSON `--entry` evaluation emits for `generatorPlan`, `bridge`, `keepassxc`, and `pass` deserializes without an unknown-field error against the same `#[serde(deny_unknown_fields)]` structs the flake path deserializes against
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

## ADDED Requirements

### Requirement: `sync` and `audit` read `pass` as a target keyword

Both verbs SHALL accept `pass` as a first argument naming a target, alongside the target keywords they already accept, and SHALL narrow the run to that target's own mappings when it is given.
Naming a mapping of one target together with a direction or mode word that target does not use SHALL be refused naming the target, as it already is for every other target.
Both verbs' forms, help text and reports SHALL name this target in the same shape they name the existing ones, so the target list a reader sees in the help is the target list the dispatch accepts.

#### Scenario: The keyword narrows the run

- **WHEN** `sync pass` or `audit pass` is run
- **THEN** only pass mappings are acted on or compared
- **AND** no other target's mappings appear in the report

#### Scenario: The help and the dispatch agree

- **WHEN** either verb's form and help text are read
- **THEN** `pass` appears among the target keywords, with its own section stating its modes and what it converges
- **AND** the keywords the help lists are exactly the keywords the dispatch accepts

#### Scenario: A bare run still covers every target

- **WHEN** either verb is run with no target named
- **THEN** pass mappings are acted on or compared alongside every other target's
- **AND** a run over a consumer declaring no pass mapping reports nothing for this target rather than refusing
