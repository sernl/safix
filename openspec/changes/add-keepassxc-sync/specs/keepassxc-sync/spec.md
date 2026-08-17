## Purpose

A declared subset of safix's secrets exists, converged, in the operator's password database, under names the declaration chooses — so a person-read credential has one producer and no hand copies.

## ADDED Requirements

### Requirement: The mirror is declared, and the naming is the consumer's

Which secrets sync, into which database, under which group and entry path SHALL be declared in nix, one mapping per safix entry, with the kdbx path chosen by the declaration.
Evaluation SHALL refuse two mappings onto one kdbx path and a mapping whose safix side no declaration resolves.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, and the kdbx side by path under the declared group
- **AND** nothing about the naming is invented at run time

#### Scenario: Collisions and dangling references refuse at evaluation

- **WHEN** two mappings share a kdbx path, or a mapping names an undeclared safix entry
- **THEN** evaluation refuses, naming the mappings and the collision or the missing entry

### Requirement: Sync moves values one way and converges

`safix sync` SHALL read both sides of every declared mapping — or the one named — and write only mappings whose sides differ, safix to database, never the reverse.
A run over agreeing mappings SHALL write nothing, and a second run immediately after a successful one SHALL change nothing.

#### Scenario: Agreement writes nothing

- **WHEN** a mapping's two sides already hold the same value
- **THEN** the database file is not written for it
- **AND** the report says unchanged

#### Scenario: Difference converges toward safix

- **WHEN** the sides differ, or the database entry does not exist yet
- **THEN** the database entry is created or updated to safix's value
- **AND** the safix side is never modified

### Requirement: The report names mappings and never values

Each run SHALL report per mapping — unchanged, updated, or refused with the reason — and no value and no derivative of a value SHALL appear in any output path.
A mapping that could not be judged SHALL be reported, never silently skipped.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value

### Requirement: The database is a store being written, never a keyring being managed

Sync SHALL NOT create databases, change database keys, or touch any hardware slot; it SHALL reach the database through the session's secret service when the database is unlocked, else through the store's own command with a single password prompt, and values SHALL travel standard input or the session bus, never an argument vector or an environment variable.
Without a terminal and without an unlocked session store, sync SHALL refuse rather than prompt into the void.

#### Scenario: Locked and headless refuses

- **WHEN** sync runs with no unlocked session store and no terminal
- **THEN** it refuses before reading any secret, naming the two ways to provide the database
