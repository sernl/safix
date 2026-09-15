## Purpose

The command gains a sixteenth verb, `install`, which reads a manifest and installs its entries.
It is the first verb in the table that no operator types: an activation script runs it.
Three requirements move as a result — the one that enumerates the verbs, the one that records what each word in the vocabulary means, and the one that counts how many verbs behave identically under a plain nix entry file, since `install` evaluates no nix at all.

## MODIFIED Requirements

### Requirement: One command covers the lifecycle, by name and never by file

The package SHALL provide a single command named `safix` with the subcommands `set`, `edit`, `get`, `view`, `list`, `generate`, `check`, `fix`, `audit`, `sync`, `keygen`, `adduser`, `enroll`, `group`, `upload`, and `install`.
Every subcommand that addresses a secret SHALL address it by name, and SHALL NOT require the operator to name a file.
Where a subcommand offers the operator a choice among the entries a user holds, that choice SHALL be among names, and choosing SHALL be equivalent to having named the chosen entry.
`install` is the one exception and SHALL name a file, because a manifest is its whole input and no declaration names it.

#### Scenario: Addressing a secret

- **WHEN** an operator sets, reads, or generates a value
- **THEN** they name the secret
- **AND** the file and the key within it are resolved from the declarations

#### Scenario: Choosing is a way of naming

- **WHEN** an operator chooses an entry from those offered rather than typing its name
- **THEN** the run proceeds exactly as though the chosen name had been given as an argument
- **AND** no file is named at any point

#### Scenario: The subcommand set is closed

- **WHEN** an unrecognised subcommand is given
- **THEN** the command fails naming the subcommands it accepts
- **AND** the list it names is derived from the verb table rather than restated, so a verb added without the refusal's own accepted output moving is a failing test

#### Scenario: Runtime dependencies are pinned into the command

- **WHEN** the command runs
- **THEN** the tools it invokes come from its own closure
- **AND** none of them is inherited from the caller's environment

#### Scenario: The one verb that names a file, and why

- **WHEN** the help for `install` is read
- **THEN** it states that the manifest path is its argument because a manifest is a build product, not a declaration, and nothing in the declarations names it
- **AND** it states that the verb resolves no secret by name, so the by-name rule has nothing to apply to

### Requirement: Retired and reserved verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a word this package's own vocabulary once used is retired outright, the help SHALL say so; where one is reserved for a feature not yet built, the help SHALL say that instead, so a reservation is never mistaken for an oversight.
Where a verb exists but is invoked by a machine rather than by an operator, the help SHALL say that too.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no verb exists for ongoing secret delivery, because activation already delivers what it would
- **AND** it states that no export verb exists, naming clan's own `export` as the bulk plaintext dump safix's design refuses to build on either side of the boundary

#### Scenario: A reserved absence is told apart from a retired one

- **WHEN** the help for `import` is read
- **THEN** it states that `import` is reserved for a future, unbuilt feature — ingesting a value from an external plaintext source one entry at a time — rather than retired outright
- **AND** it states that this is distinct from `export`'s retirement, which is permanent

#### Scenario: `upload` exists under a name clan also uses, with narrower semantics

- **WHEN** the help for `upload` is read
- **THEN** it states that it moves only a machine's own host identity, once, before that machine's first activation
- **AND** states that it is not clan's ongoing vars-delivery verb of the same name, because activation already delivers what this package resolves for a machine that already has its identity
- **AND** it names this package's own `install` verb as what performs that delivery, rather than naming the framework this package no longer depends on

#### Scenario: A machine-facing verb is marked as one

- **WHEN** the help enumerates the verbs
- **THEN** `install` is described as the verb an activation runs rather than one an operator types
- **AND** it is the only verb so described, so the mark means something

### Requirement: A global entry file replaces the flake for evaluation

The command SHALL accept a global `--entry <file>` option and a `SAFIX_ENTRY` environment variable naming a plain nix file.
When either is set, every nix evaluation SHALL target that file — `nix eval --file <file> <attribute>` — instead of `<root>#<attribute>`, using the same attribute spellings the flake path uses.
`--entry` SHALL take precedence over `SAFIX_ENTRY` when both are given.
Fifteen of the sixteen subcommands SHALL behave identically under `--entry` as against a flake; `generate` is the documented exception.
`install` is identical under either because it evaluates no nix at all.

#### Scenario: The attribute spellings are unchanged

- **WHEN** any of the twelve attributes the runtime evaluates is read under `--entry`
- **THEN** the string naming it is identical to the one read against a flake
- **AND** only how the target is built — `--file <entry> <attribute>` rather than `<root>#<attribute>` — differs

#### Scenario: Nested payload shapes evaluate identically under either path

- **WHEN** an entry file's declarations include at least one generator, one bridge mapping, and one keepassxc mapping
- **THEN** the JSON `--entry` evaluation emits for `generatorPlan`, `bridge`, and `keepassxc` deserializes without an unknown-field error against the same `#[serde(deny_unknown_fields)]` structs the flake path deserializes against
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

### Requirement: `install` takes a manifest, a check mode, and two switches

`install` SHALL accept a manifest path, `--check-mode=off|manifest|document`, `--ignore-passwd`, and `--dry-run`, and SHALL treat the host's dry-activation signal in the environment as implying `--dry-run`.
`--check-mode=manifest` SHALL validate structure without reading any ciphertext; `--check-mode=document` SHALL additionally decrypt each named document and verify each declared key resolves in it.

#### Scenario: The two checking tiers are distinguishable

- **WHEN** `install --check-mode=manifest` is run against a manifest whose documents are absent or undecryptable
- **THEN** it succeeds, because it read none of them
- **WHEN** `install --check-mode=document` is run against the same manifest
- **THEN** it fails naming the document it could not read
- **AND** the two are exercised over one manifest, so the tiers are held apart rather than asserted

#### Scenario: A check mode installs nothing

- **WHEN** either checking mode runs
- **THEN** no file is written into the store, no filesystem is mounted, and no symlink moves
- **AND** the mode is therefore usable inside a build, which is what the manifest derivation's own check phase does with it

#### Scenario: The passwd switch skips resolution rather than guessing

- **WHEN** `--ignore-passwd` is given
- **THEN** no user, group, or keys-group lookup is performed and every entry's owner and group are zero
- **AND** the reason is recorded: a sandboxed build has no such users, so a mode that guessed them would be validating a resolution it cannot perform
