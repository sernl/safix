## Purpose

A declared subset of safix's secrets exists, converged, in the operator's `pass` store, under paths, modes and metadata fields the declaration chooses — so a credential a script and a person both read has one place to drift from, one verb that ends the drift, and a record layout the store's own tools can still read.

## ADDED Requirements

### Requirement: The store and the mapping are declared, and the naming is the consumer's

Which secrets sync into a `pass` store, which store, under which entry path, in which mode, and carrying which metadata fields SHALL be declared in nix, one mapping per safix entry.
The store SHALL be declared as a string naming a location on the machine the verb runs on, never as a nix path, for the reason the password database's own path is a string: a nix path interpolated into a declaration is copied into the world-readable store on every evaluation.
The store's declaration SHALL carry a default, because the tool itself has one, and the runtime SHALL expand a leading `~` in it against the invoking operator's home.
The modes SHALL be named by their endpoints — `safix-to-pass`, `pass-to-safix`, `two-way`, and `backup` — and evaluation SHALL refuse two mappings onto one entry path, a mapping whose safix side no declaration resolves, a `pass-to-safix` or `two-way` mapping onto an entry a generator produces, an entry path carrying the suffix reserved for a `two-way` mapping's recorded agreement, and a mapping whose id is one the command line reserves as a target keyword.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, the store side by a path inside the declared store, its mode by endpoints, and each metadata field it sets
- **AND** nothing about the naming, the direction or the fields is invented at run time

#### Scenario: The store is a string with a default

- **WHEN** the store option is read
- **THEN** it is a string rather than a nix path, and its documentation states that a nix path would copy the store into the world-readable nix store on every evaluation
- **AND** it defaults to the location the tool itself defaults to, so a consumer who wants that location declares nothing
- **AND** a leading `~` is expanded by the runtime, because evaluation has no home to expand against

#### Scenario: A second producer is refused where it is visible

- **WHEN** a pull-capable mapping targets a generator-produced entry
- **THEN** evaluation refuses, naming the mapping, the generator, and why two producers for one value is a race

#### Scenario: The name safix reserves cannot be declared

- **WHEN** a mapping's entry path carries the reserved suffix
- **THEN** evaluation refuses, naming the mapping, the entry and the suffix
- **AND** no admissible declaration can therefore name the entry a `two-way` mapping records its agreement in

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is one of the words `sync` and `audit` read as a target — `pass` among them
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reserved words come from one exported list shared by every mapping module rather than a copy per module, so the words this refusal covers and the words the command line reads cannot come to disagree

### Requirement: The record layout is safix's own, and the store's own tools can still read it

Unlike every other sync target, the store fixes no metadata schema: its convention is that the value is the record's first line and further information follows it.
safix SHALL therefore define the layout, and that layout SHALL keep an entry readable by the store's own value-reading verbs.
A write SHALL emit the value's bytes, then — only when at least one field is declared — one blank line, then one line per declared field under the field spellings the store's ecosystem already reads, omitting a field that is unset.
A read SHALL take the field block to be the longest suffix of lines each of which is one of those field spellings, consume one immediately preceding blank line as a separator when it is present, and take every byte before that as the value, verbatim.
A body with no such suffix SHALL be a value and nothing else.
The one ambiguity this leaves — a value whose own trailing lines are spelled like fields — SHALL be stated in the capability's documentation rather than refused, because refusing it would be a limitation safix invented for a store that carries such a value fine, and safix's own writes never produce it.

#### Scenario: An entry with no fields is its own bytes

- **WHEN** a mapping declaring no field writes a single-line value
- **THEN** the record's whole body is that value
- **AND** an entry safix wrote is indistinguishable from one written by hand

#### Scenario: The value stays where the store's own tools look for it

- **WHEN** a record safix wrote is read by the store's own value-reading verb, or by a client reading the ecosystem's field spellings
- **THEN** the value is the first line and each declared field is found under the spelling that ecosystem reads
- **AND** no safix-private marker, header or encoding stands between them and the record

#### Scenario: A multi-line value keeps its own last line

- **WHEN** a mapping declaring at least one field writes a value spanning several lines
- **THEN** the blank separator is what marks the end of the value, so the value's own final line is not read as a field
- **AND** the value read back is byte-for-byte the value written

#### Scenario: A value the store carries whole is not refused

- **WHEN** a mapping writes a value carrying a newline
- **THEN** it is written whole and read back whole
- **AND** the refusal the password-database target makes for such a value is not made here, because that refusal is a property of that tool's own one-line prompt rather than of secrets

#### Scenario: Nothing is added to or trimmed from a value

- **WHEN** a value is read from the store
- **THEN** its bytes are exactly what the store holds
- **AND** the trailing-byte removal the password-database target performs is not performed here, because this store's read adds no byte

#### Scenario: The ambiguity is documented rather than hidden

- **WHEN** the layout's documentation is read
- **THEN** it states that a value whose trailing lines are spelled like fields reads back as fields
- **AND** it states that safix's own writes emit the blank separator, which is what keeps them out of that case

### Requirement: Every field this target carries crosses on a pipe, and all four are carried

The store's write SHALL take the whole record body on standard input, so the value and every field cross on a pipe and nothing but the entry path appears in an argument vector.
This target SHALL therefore declare that it carries all four of the shared metadata fields, and the two field refusals the shared field model defines — a field a target cannot carry, and a field whose value is a secret on a target that could only carry it in an argument vector — SHALL be unreachable here.
A field whose declared source is another entry of the same person SHALL be admissible on this target, resolved at run time, and its resolved value SHALL never appear in an argument vector, an environment variable, a report or a commit message.

#### Scenario: The whole record travels standard input

- **WHEN** a mapping writes a value and any field
- **THEN** the body carrying both reaches the store's own command on standard input
- **AND** the argument vector carries the entry path and nothing else

#### Scenario: All four fields are carried

- **WHEN** this target's field capability is read
- **THEN** all four shared fields are carried
- **AND** the two field refusals are unreachable for this target, which is held by a check rather than asserted in prose

#### Scenario: A secret-sourced field is admissible here

- **WHEN** a field's declared source names another entry the same person holds
- **THEN** the field is resolved at run time and written in the record body
- **AND** no output path, argument vector, environment variable or commit message contains its value

### Requirement: Each mode converges exactly as its name says

`safix sync`, bare or with `pass` as its target, SHALL read both sides of every declared pass mapping — or the ones named — and converge per mode: `safix-to-pass` writes the store to safix's value; `pass-to-safix` writes safix to the store's value through the ordinary write path with every write-path refusal in force; `two-way` writes toward whichever side changed since the last sync; `backup` writes only where the store holds nothing.
No mode SHALL delete an entry on either side, and a run over agreeing mappings SHALL write nothing anywhere.
Fields SHALL move only in the pushing direction: a pull writes the value alone into safix, a `backup` that does not overwrite a value writes no field either, and a `two-way` mapping's fields have one author because a declaration has one author.

#### Scenario: Agreement writes nothing

- **WHEN** a mapping's two sides already hold the same value and the same declared fields
- **THEN** neither side is written
- **AND** the report says unchanged

#### Scenario: Backup never overwrites

- **WHEN** a `backup` mapping meets a store entry holding a different value
- **THEN** nothing is written, neither the value nor any field
- **AND** the divergence is reported rather than resolved

#### Scenario: A pull is an ordinary write

- **WHEN** a `pass-to-safix` or `two-way` mapping converges toward the store's value
- **THEN** the safix side is written through the same path a hand-set value takes, commits included
- **AND** a refusal that path would make — the empty value, the recipient drift — is made here too
- **AND** only the value is written into safix, because a safix entry is a placement with no slot for a field

#### Scenario: `sync pass` narrows to this target, and accepts more than one mapping name

- **WHEN** `sync pass` is given one or more mapping names
- **THEN** it converges exactly those mappings, in each one's own declared mode
- **AND** when given none it converges every declared pass mapping

#### Scenario: Bare sync converges every target, this one's mappings among them

- **WHEN** `sync` runs with no target and no mapping named
- **THEN** the pass mappings this requirement governs converge alongside every other target's, each in its own declared mode

### Requirement: Two-way remembers the last agreement inside the store, in a companion entry

A `two-way` mapping SHALL record the last-synced state in a reserved companion entry beside the mapped one, inside the same store, and SHALL NOT record any value-derived state in the repository.
The companion's name SHALL be one no declaration can produce, and SHALL be the mapped path plus the reserved suffix.
The recorded state SHALL carry a format tag, and a memory under a tag this version does not write SHALL be read as no memory at all rather than as a conflict.
The memory SHALL be written after the value it describes, never before it.
When both sides have changed since that state, the run SHALL write nothing for the mapping and report a conflict naming both one-way remedies.

#### Scenario: The memory is a second object, not a field of the record

- **WHEN** a `two-way` mapping's agreement is recorded
- **THEN** it is written to the companion entry beside the mapped one
- **AND** the mapped record itself carries no safix bookkeeping field, so the person's own entry is not decorated with safix's state

#### Scenario: The tiebreak is the recorded state

- **WHEN** exactly one side differs from the last-synced state
- **THEN** the other side converges to it
- **AND** the recorded state is updated after that write, so a run interrupted between the two leaves the older memory and the next run reports a conflict rather than overwriting the newer value

#### Scenario: Both changed is a conflict, not a guess

- **WHEN** both sides differ from the last-synced state and from each other
- **THEN** nothing is written
- **AND** the finding names the mapping and the two one-way commands that would each resolve it

#### Scenario: No oracle lands in the repository

- **WHEN** the repository is searched for sync state
- **THEN** no digest or derivative of a secret value is committed anywhere

#### Scenario: A mapping's own memory is never reported as unclaimed

- **WHEN** a run reports entries in the store that no mapping accounts for
- **THEN** each mapping accounts for its own entry and for the companion beside it
- **AND** a companion whose mapping is gone lingers exactly as its entry does

### Requirement: The store is reached through its own command, and its keys are never touched

Sync SHALL reach the store only through the store's own command, SHALL NOT read or decrypt the store's files itself, and SHALL NOT create a store, write or edit a store's recipient declaration, or remove an entry.
A secret value and every metadata field SHALL travel standard input or a pipe, never an argument vector or an environment variable; the store's location MAY travel the child's environment, because a location is not a value.
There SHALL be no unlock step and no passphrase prompt: the store delegates decryption to the operator's own agent, and safix SHALL NOT interpose on it.
Before any mapping is read, the run SHALL refuse when the declared store is not a store — absent, or carrying no recipient declaration — naming the declared location, so a run cannot report agreement about mappings it never looked at.
A decrypt the agent declined SHALL be its own refusal, carrying the tool's own words, and SHALL be distinguishable from an entry that is absent and from any other command failure.

#### Scenario: No store is a refusal before anything is read

- **WHEN** a run's declared store is absent or carries no recipient declaration
- **THEN** the run refuses before reading any mapping, naming the declared location
- **AND** no mapping is reported as unchanged

#### Scenario: A declined decrypt is told apart from an absent entry

- **WHEN** the operator's agent declines to decrypt an entry
- **THEN** the run refuses for that mapping with the tool's own words
- **AND** the refusal is not the absent-entry refusal, so a `backup` mapping never writes over an entry it merely could not read

#### Scenario: Absence is answered from the store's own names

- **WHEN** a mapping's entry is not in the store
- **THEN** its absence is established from the store's own listing of names rather than from a command's exit status
- **AND** the report says the entry is absent, naming the mapping, the entry and the mode that made the store the source

#### Scenario: The store's keys and recipients are left alone

- **WHEN** every command a run issues is enumerated
- **THEN** none of them creates a store, writes a recipient declaration, re-encrypts the store, or removes an entry
- **AND** the store's own history, including any commit the store makes for itself on a write, remains the store's

#### Scenario: Nothing but the location leaves the pipe

- **WHEN** a run's commands, their argument vectors and their environments are observed
- **THEN** no value and no field is in any argument vector or environment variable
- **AND** the store's location is in the child's environment, which the documentation states as a location rather than an exception to the value rule

### Requirement: The report names mappings and never values

Each run SHALL report per mapping — unchanged, updated, pulled, conflict, fields diverged, or refused with the reason — and no value, no field content and no derivative of either SHALL appear in any output path.
A mapping that could not be judged SHALL be reported, never silently skipped.
An entry in the store that no mapping declares SHALL be reported as information, including a companion entry whose mapping is gone, and SHALL NOT be removed.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value or the content of a field

#### Scenario: A field divergence is named, and its content is not

- **WHEN** a mapping's value agrees and a declared field does not
- **THEN** the report names the mapping and the field
- **AND** it does not print either side's content, because a field may itself be a secret

### Requirement: audit compares this store without writing, scoped to the pass target

`audit`, bare or with `pass` as its target, SHALL read both sides of every declared pass mapping — or the ones named — compare them, and change nothing on either side.
The report SHALL include, as information, every entry in the store that no currently declared mapping accounts for, in the same shape `sync`'s own report gives that finding, and SHALL NOT change what a `backup` mapping or any other mode would otherwise do, because nothing here writes.

#### Scenario: A compare-only run writes nothing

- **WHEN** `audit pass` runs over any mix of agreeing and diverged mappings
- **THEN** no side of any mapping is written
- **AND** each mapping's outcome is reported as agreeing, diverged, fields diverged, or unjudgeable

#### Scenario: Lingering entries are reported the same way sync reports them

- **WHEN** `audit pass` runs
- **THEN** an entry in the store that no mapping declares is reported as information, including a companion entry whose mapping is gone
- **AND** it is not removed by this report or by any other effect of running it

#### Scenario: audit's exit status answers only whether every compared mapping agreed

- **WHEN** a run finds one or more lingering entries and every compared mapping agrees
- **THEN** `audit pass` still exits reporting agreement
- **AND** the lingering entries are reported alongside it as information, not as findings that change the exit status
