## MODIFIED Requirements

### Requirement: sync moves declared mappings, scoped by target and narrowed by direction

Amended while proposing `rename-transfer-verbs`: this requirement is `rename-transfer-verbs`'s renamed and rewritten successor to the base spec's "Two verbs move declared mappings, one per direction", which `import`/`export` refused a two-way-declared mapping under and a third verb `bridge` would have acted on.
`rename-transfer-verbs` collapses `import`/`export` into `sync clan`'s own per-mapping direction convergence and a `--direction` filter; this change folds its own third verb into that same filter as a third accepted value, `two-way`, rather than adding a fourth CLI form.
Substance is unchanged: a `two-way` mapping still converges by `bridge-sync`'s own rule, still refuses under `--direction clan-to-safix`/`--direction safix-to-clan` by naming its actual declared direction, and that refusal is now the same generic one every direction mismatch already gets from `rename-transfer-verbs`'s filter, with no `two-way`-specific wrong-verb wording left to carry.

The command SHALL provide a `sync` verb whose `clan` target acts on the mappings declared under `flake.safix.bridge.mappings`, each mapping converging in its own declared direction — `clan-to-safix`, `safix-to-clan`, or `two-way` — scoped to one or more named mappings or to every mapping of that target when none is named, and narrowed further by an optional `--direction` option naming one of the three declared direction values.

#### Scenario: Each mapping converges in its own declared direction

- **WHEN** `sync clan` runs
- **THEN** each mapping acted on moves in the direction declared for it: a `clan-to-safix` or `safix-to-clan` mapping writes the side that lags, and a `two-way` mapping converges by the rule `bridge-sync` declares
- **AND** no option or argument overrides a mapping's own declared direction

#### Scenario: A run may be scoped by naming mappings, or complete when none is named

- **WHEN** `sync clan` is given one or more mapping names
- **THEN** it acts on exactly those mappings, of any declared direction
- **AND** when given none it acts on every mapping declared under the clan target

#### Scenario: --direction narrows the run to mappings declared with that value

- **WHEN** `sync clan --direction <value>` runs with no mapping named
- **THEN** only mappings declared with that direction are acted on, `--direction two-way` narrowing to two-way mappings the same way the two one-way values narrow to theirs
- **AND** a mapping declared with a different direction is left untouched, not refused

#### Scenario: A named mapping outside the --direction filter is told its actual direction

- **WHEN** `sync clan --direction <value>` is given the name of a mapping declared with a different direction
- **THEN** it refuses naming the direction the mapping is actually declared with — one of `clan-to-safix`, `safix-to-clan`, or `two-way`
- **AND** the refusal is distinct from the one for a name nothing declares

#### Scenario: sync's help documents the clan target's forms

- **WHEN** the command is asked for help with `sync`
- **THEN** the help text states the `clan` target, its mapping-name scoping, and the three `--direction` values it accepts, `two-way` among them

### Requirement: clan is the authority on its own store and is reached only through its command

Every read of a clan value and every write of one SHALL be performed by invoking clan's own command as a subprocess, and the runtime SHALL NOT read, write, decrypt, encrypt or parse clan's stored files.
When a mapping's placement is shared, the machine named on clan's command line to address it SHALL itself be obtained by invoking clan's command, never from a second field a consumer declares.

#### Scenario: Reading a clan value delegates

- **WHEN** a clan-side value is needed
- **THEN** it is obtained by invoking clan's command
- **AND** the value arrives on that process's standard output

#### Scenario: Writing a clan value delegates

- **WHEN** a clan-side value is written
- **THEN** it is written by invoking clan's command
- **AND** the value is supplied on that process's standard input

#### Scenario: No store implementation exists in the runtime

- **WHEN** the runtime is searched for clan's store layout, its recipient handling, or any of its backends
- **THEN** none is found
- **AND** the reason is recorded: the consumer's backend is a choice clan owns, and reimplementing one would silently support only that one

#### Scenario: The raw value is captured rather than a rendered one

- **WHEN** a clan value is read
- **THEN** the runtime establishes that it received the raw bytes rather than a rendering intended for a terminal
- **AND** the reason is recorded: clan's read command substitutes a printable form when its output is a terminal

#### Scenario: An absent clan command refuses the whole run

- **WHEN** clan's command is not available
- **THEN** `sync`, for the clan target, refuses before transferring anything, whatever mix of directions the run would have acted on
- **AND** the refusal states that clan is the authority on its own store
- **AND** the run does not proceed with a subset of its mappings

#### Scenario: A shared mapping's address is discovered from clan, not declared twice

- **WHEN** a mapping's placement is shared
- **THEN** the runtime asks clan which machines it has, and tries them in turn against the mapping's generator until one resolves it
- **AND** no option or field on the mapping names that machine

#### Scenario: An unaddressable shared placement refuses naming the generator

- **WHEN** no machine clan has resolves a shared mapping's generator
- **THEN** the mapping is refused
- **AND** the refusal names the mapping, the placement, the generator and the file, and states that no machine in clan's own fleet exposed it
