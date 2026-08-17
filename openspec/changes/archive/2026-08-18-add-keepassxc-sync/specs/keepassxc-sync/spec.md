## Purpose

A declared subset of safix's secrets exists, converged, in the operator's password database, under names and modes the declaration chooses — so a person-read credential has one place to drift from and a verb that ends the drift.

## ADDED Requirements

### Requirement: The mirror is declared, and the naming and the mode are the consumer's

Which secrets sync, into which database, under which group and entry path, and in which mode SHALL be declared in nix, one mapping per safix entry.
The modes SHALL be named by their endpoints — `safix-to-keepassxc`, `keepassxc-to-safix`, `two-way`, and `backup` — and evaluation SHALL refuse two mappings onto one kdbx path, a mapping whose safix side no declaration resolves, a `keepassxc-to-safix` or `two-way` mapping onto an entry a generator produces, and a kdbx path carrying the suffix reserved for the entry a `two-way` mapping records its agreement in.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, the kdbx side by path under the declared group, and its mode by endpoints
- **AND** nothing about the naming or the direction is invented at run time

#### Scenario: A second producer is refused where it is visible

- **WHEN** a pull-capable mapping targets a generator-produced entry
- **THEN** evaluation refuses, naming the mapping, the generator, and why two producers for one value is a race

#### Scenario: The name safix reserves cannot be declared

- **WHEN** a mapping's kdbx path carries the reserved suffix
- **THEN** evaluation refuses, naming the mapping, the entry and the suffix
- **AND** no admissible declaration can therefore name the entry a `two-way` mapping records its agreement in

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

A `two-way` mapping SHALL record the last-synced state as a protected field of a reserved companion entry beside the mapped one, inside the same encrypted database, and SHALL NOT record any value-derived state in the repository.
The companion's name SHALL be one no declaration can produce.
When both sides have changed since that state, the run SHALL write nothing for the mapping and report a conflict naming both one-way remedies.

Amended during apply. The requirement read "as a protected attribute of the database entry itself", and the mechanism was not implementable: `keepassxc-cli` 2.7.12 has no custom-attribute write on any verb, so a protected attribute was reachable over the session's secret service and not over the store's own command — a mechanism that exists under one transport and not the other.
`Password` is the one protected field both can write, so the memory moved onto a companion entry whose name is the mapped entry's plus a reserved suffix.
Every normative property is unchanged: the state is inside the encrypted database, the repository carries nothing value-derived, and deleting the memory converts the mapping to bootstrap semantics.

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
An entry under the declared group that no mapping declares SHALL be reported as information, including a companion entry whose mapping is gone, and SHALL NOT be removed.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value

### Requirement: The database is a store being written, never a keyring being managed

Sync SHALL NOT create databases, change database keys, or touch any hardware slot; it SHALL reach the database through the store's own command with a single password prompt per run, and values SHALL travel standard input or pipes, never an argument vector or an environment variable.
Without a terminal to ask that password on, sync SHALL refuse rather than prompt into the void.

Amended during apply. The requirement had sync reach the database "through the session's secret service when the database is unlocked, else through the store's own command"; the service is not a transport it can use.
The Secret Service collection KeePassXC publishes is its exposed group, so an item found or created through it belongs to whatever group the operator's exposure setting names and not to the group a mapping declares.
Two transports addressing different entries would make a mapping's convergence depend on which one ran, and a service read of an entry in an unexposed group is indistinguishable from the database holding no value — which would let a `backup` mapping write a secret into a group no declaration named, an outcome the report has no way to state.
Everything else in this requirement is unchanged, including the refusal arriving before any secret is read.

#### Scenario: Headless refuses

- **WHEN** sync runs with no terminal to ask the database's password on
- **THEN** it refuses before reading any secret, naming the declared database and the option that is unset when that is the defect

#### Scenario: A value the store cannot carry whole is refused rather than trimmed

- **WHEN** a mapping would write a value carrying a newline into the database
- **THEN** the mapping is refused with the reason and the remedy named
- **AND** nothing is written, because a mirror that silently drops a byte lies about what it holds
