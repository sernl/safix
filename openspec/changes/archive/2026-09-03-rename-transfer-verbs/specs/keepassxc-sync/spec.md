## MODIFIED Requirements

### Requirement: The mirror is declared, and the naming and the mode are the consumer's

Which secrets sync, into which database, under which group and entry path, and in which mode SHALL be declared in nix, one mapping per safix entry.
The modes SHALL be named by their endpoints — `safix-to-keepassxc`, `keepassxc-to-safix`, `two-way`, and `backup` — and evaluation SHALL refuse two mappings onto one kdbx path, a mapping whose safix side no declaration resolves, a `keepassxc-to-safix` or `two-way` mapping onto an entry a generator produces, a kdbx path carrying the suffix reserved for the entry a `two-way` mapping records its agreement in, and a mapping whose id is one the command line reserves as a target keyword.

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

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is `clan`, `keepassxc`, or `all`
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is the one `bridge-surface` gives for the same refusal over its own mappings: `sync` and `audit` read their first argument as a target keyword or a mapping name, never both

### Requirement: Each mode converges exactly as its name says

`safix sync`, bare or with `keepassxc` as its target, SHALL read both sides of every declared keepassxc mapping — or the ones named — and converge per mode: `safix-to-keepassxc` writes the database to safix's value; `keepassxc-to-safix` writes safix to the database's value through the ordinary write path with every write-path refusal in force; `two-way` writes toward whichever side changed since the last sync; `backup` writes only where the database holds nothing.
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

#### Scenario: `sync keepassxc` narrows to the keepassxc target, and accepts more than one mapping name

- **WHEN** `sync keepassxc` is given one or more mapping names
- **THEN** it converges exactly those mappings, in each one's own declared mode
- **AND** when given none it converges every declared keepassxc mapping

#### Scenario: Bare sync converges every target, keepassxc's mappings among them

- **WHEN** `sync` runs with no target and no mapping named
- **THEN** the keepassxc mappings this requirement governs converge alongside every other target's, each in its own declared mode

## ADDED Requirements

### Requirement: audit compares the mirror without writing, scoped to the keepassxc target

`audit`, bare or with `keepassxc` as its target, SHALL read both sides of every declared keepassxc mapping — or the ones named — compare them per mode, and change nothing on either side.
The report SHALL include, as information, every entry under the declared group that no currently declared mapping accounts for, in the same shape `sync`'s own report already gives that finding, and this report SHALL NOT change what a `backup` mapping or any other mode would otherwise do, because nothing here writes.

#### Scenario: A compare-only run writes nothing

- **WHEN** `audit keepassxc` runs over any mix of agreeing and diverged mappings
- **THEN** no side of any mapping is written
- **AND** each mapping's outcome is reported as agreeing, diverged, or unjudgeable

#### Scenario: The gap sync's always-converging loop left is filled

- **WHEN** an operator wants to know whether the keepassxc mirror has drifted without risking a write
- **THEN** `audit keepassxc` answers that question, where previously only `sync` existed and `sync` always converges what it finds

#### Scenario: Lingering entries are reported the same way sync already reports them

- **WHEN** `audit keepassxc` runs
- **THEN** an entry under the declared group that no mapping declares is reported as information, including a companion entry whose mapping is gone
- **AND** it is not removed by this report or by any other effect of running it

#### Scenario: audit's exit status answers only whether every compared mapping agreed

- **WHEN** a run finds one or more lingering entries and every compared mapping agrees
- **THEN** `audit keepassxc` still exits reporting agreement
- **AND** the lingering entries are reported alongside it as information, not as findings that change the exit status
