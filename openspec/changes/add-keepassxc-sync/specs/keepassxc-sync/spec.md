## Purpose

A declared subset of safix's secrets exists, converged, in the operator's password database, under names and modes the declaration chooses — so a person-read credential has one place to drift from and a verb that ends the drift.

## ADDED Requirements

### Requirement: The mirror is declared, and the naming and the mode are the consumer's

Which secrets sync, into which database, under which group and entry path, and in which mode SHALL be declared in nix, one mapping per safix entry.
The modes SHALL be named by their endpoints — `safix-to-keepassxc`, `keepassxc-to-safix`, `two-way`, and `backup` — and evaluation SHALL refuse two mappings onto one kdbx path, a mapping whose safix side no declaration resolves, and a `keepassxc-to-safix` or `two-way` mapping onto an entry a generator produces.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, the kdbx side by path under the declared group, and its mode by endpoints
- **AND** nothing about the naming or the direction is invented at run time

#### Scenario: A second producer is refused where it is visible

- **WHEN** a pull-capable mapping targets a generator-produced entry
- **THEN** evaluation refuses, naming the mapping, the generator, and why two producers for one value is a race

### Requirement: Each mode converges exactly as its name says

`safix sync` SHALL read both sides of every declared mapping — or the one named — and converge per mode: `safix-to-keepassxc` writes the database to safix's value; `keepassxc-to-safix` writes safix to the database's value through the ordinary write path with every write-path refusal in force; `two-way` writes toward whichever side changed since the last sync; `backup` writes only where the database holds nothing.
No mode SHALL delete an entry on either side, and a run over agreeing mappings SHALL write nothing anywhere.

#### Scenario: Agreement writes nothing

- **WHEN** a mapping's two sides already hold the same value
- **THEN** neither side is written
- **AND** the report says unchanged

#### Scenario: Backup never overwrites

- **WHEN** a `backup` mapping meets a database entry holding a different value
- **THEN** nothing is written
- **AND** the divergence is reported rather than resolved

#### Scenario: A pull is an ordinary write

- **WHEN** a `keepassxc-to-safix` or `two-way` mapping converges toward the database's value
- **THEN** the safix side is written through the same path a hand-set value takes, commits included
- **AND** a refusal that path would make — the empty value, the recipient drift — is made here too

### Requirement: Two-way remembers the last agreement inside the encrypted store

A `two-way` mapping SHALL record the last-synced state as a protected attribute of the database entry itself, and SHALL NOT record any value-derived state in the repository.
When both sides have changed since that state, the run SHALL write nothing for the mapping and report a conflict naming both one-way remedies.

#### Scenario: The tiebreak is the recorded state

- **WHEN** exactly one side differs from the last-synced state
- **THEN** the other side converges to it
- **AND** the recorded state is updated in the same write

#### Scenario: Both changed is a conflict, not a guess

- **WHEN** both sides differ from the last-synced state and from each other
- **THEN** nothing is written
- **AND** the finding names the mapping and the two one-way commands that would each resolve it

#### Scenario: No oracle lands in the repository

- **WHEN** the repository is searched for sync state
- **THEN** no digest or derivative of a secret value is committed anywhere

### Requirement: The report names mappings and never values

Each run SHALL report per mapping — unchanged, updated, pulled, conflict, or refused with the reason — and no value and no derivative of a value SHALL appear in any output path.
A mapping that could not be judged SHALL be reported, never silently skipped.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value

### Requirement: The database is a store being written, never a keyring being managed

Sync SHALL NOT create databases, change database keys, or touch any hardware slot; it SHALL reach the database through the session's secret service when the database is unlocked, else through the store's own command with a single password prompt, and values SHALL travel standard input, pipes, or the session bus, never an argument vector or an environment variable.
Without a terminal and without an unlocked session store, sync SHALL refuse rather than prompt into the void.

#### Scenario: Locked and headless refuses

- **WHEN** sync runs with no unlocked session store and no terminal
- **THEN** it refuses before reading any secret, naming the two ways to provide the database
