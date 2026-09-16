## ADDED Requirements

### Requirement: The fields read asks for the declared fields alone and can never return the value

When a mapping declares at least one field, sync and audit SHALL read that entry's fields through an invocation separate from the value read, asking for the named metadata attributes and neither the protected-attribute switch nor the password attribute, so that no value can arrive on that pipe.
A mapping that declares no field SHALL issue no fields read at all.
Both reads SHALL happen before the first write of a run, so that the single-burst discipline the whole-file save requires is unaffected.

#### Scenario: The value cannot arrive on the fields pipe

- **WHEN** the fields of an entry are read
- **THEN** the invocation names the metadata attributes it wants and does not ask for the password attribute or for protected attributes to be shown in the clear
- **AND** the value therefore cannot be returned by that read at all, rather than being discarded after arriving

#### Scenario: A mapping declaring no field reads exactly what it reads today

- **WHEN** a mapping declares no field
- **THEN** its value read is the only read of that entry
- **AND** the arguments of that read are unchanged by this capability

#### Scenario: Both reads are behind every write

- **WHEN** a run converges several mappings, some of which declare fields
- **THEN** every value read and every fields read of the run happens before its first database write
- **AND** the database writes remain one consecutive burst, with no read between two of them

## MODIFIED Requirements

### Requirement: The mirror is declared, and the naming and the mode are the consumer's

Which secrets sync, into which database, under which group and entry path, in which mode, and with which fields beside the value SHALL be declared in nix, one mapping per safix entry.
The modes SHALL be named by their endpoints — `safix-to-keepassxc`, `keepassxc-to-safix`, `two-way`, and `backup` — and evaluation SHALL refuse two mappings onto one kdbx path, a mapping whose safix side no declaration resolves, a `keepassxc-to-safix` or `two-way` mapping onto an entry a generator produces, a kdbx path carrying the suffix reserved for the entry a `two-way` mapping records its agreement in, a mapping whose id is one the command line reserves as a target keyword, and a mapping declaring a field this target cannot carry.
The fields a mapping may declare are the username, the URL, the notes and the tags of the entry, declared on the database half of the mapping; of those, this target SHALL carry the username, the URL and the notes, and SHALL refuse a declared tag, because the store's own command has no way to write one.
This target SHALL further refuse any of its fields declared as a reference to another entry, because its only channel for a field is an argument vector and a referenced value is a secret.

Amended by `extend-bridge-fields`. The declaration previously carried a single `username` on the database half, documented as the one field a mapping may set.
That option is gone rather than aliased: its meaning moved into the shared field declaration under the same name, and a declaration still carrying the old spelling fails with the module system's own unknown-option sentence naming it.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, the kdbx side by path under the declared group, its mode by endpoints, and whichever fields it carries beside the value
- **AND** nothing about the naming, the direction or the fields is invented at run time

#### Scenario: A second producer is refused where it is visible

- **WHEN** a pull-capable mapping targets a generator-produced entry
- **THEN** evaluation refuses, naming the mapping, the generator, and why two producers for one value is a race

#### Scenario: The name safix reserves cannot be declared

- **WHEN** a mapping's kdbx path carries the reserved suffix
- **THEN** evaluation refuses, naming the mapping, the entry and the suffix
- **AND** no admissible declaration can therefore name the entry a `two-way` mapping records its agreement in

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is one of the reserved target keywords — `clan`, `keepassxc`, `pass`, `bitwarden`, `1password`, or `all`
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is the one `bridge-surface` gives for the same refusal over its own mappings: `sync` and `audit` read their first argument as a target keyword or a mapping name, never both

#### Scenario: A tag declared for this target is refused

- **WHEN** a mapping for this target declares a tag
- **THEN** evaluation refuses, naming the target and the field
- **AND** the reason given is that the store's own command cannot write a tag, so accepting the declaration would mean writing less than it says

#### Scenario: A field referencing another entry is refused for this target

- **WHEN** a mapping for this target declares a username, URL or notes as a reference to another entry
- **THEN** evaluation refuses, naming the target and the field
- **AND** the reason given is this capability's own rule that a secret value travels standard input or a pipe and never an argument vector

### Requirement: Each mode converges exactly as its name says

`safix sync`, bare or with `keepassxc` as its target, SHALL read both sides of every declared keepassxc mapping — or the ones named — and converge per mode: `safix-to-keepassxc` writes the database to safix's value; `keepassxc-to-safix` writes safix to the database's value through the ordinary write path with every write-path refusal in force; `two-way` writes toward whichever side changed since the last sync; `backup` writes only where the database holds nothing.
A mapping's declared fields SHALL converge alongside its value under every mode that writes the database's side, in the same write as the value, and SHALL NOT be written by any mode that does not: a pull writes only the value into safix, and `backup` writes fields only where it writes a value, so an entry `backup` refuses to overwrite keeps its own fields.
A mapping whose value agrees and whose declared fields do not SHALL be converged by a mode that pushes and reported by a mode that does not.
No mode SHALL delete an entry on either side, and a run over agreeing mappings SHALL write nothing anywhere.

#### Scenario: Agreement writes nothing

- **WHEN** a mapping's two sides already hold the same value and the same declared fields
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
- **AND** only the value crosses, because a field is a declaration and safix's side has nowhere to hold one

#### Scenario: `sync keepassxc` narrows to the keepassxc target, and accepts more than one mapping name

- **WHEN** `sync keepassxc` is given one or more mapping names
- **THEN** it converges exactly those mappings, in each one's own declared mode
- **AND** when given none it converges every declared keepassxc mapping

#### Scenario: Bare sync converges every target, keepassxc's mappings among them

- **WHEN** `sync` runs with no target and no mapping named
- **THEN** the keepassxc mappings this requirement governs converge alongside every other target's, each in its own declared mode

#### Scenario: A field drift on an otherwise-agreeing entry is repaired under a pushing mode

- **WHEN** a `safix-to-keepassxc` or `two-way` mapping's value agrees and one of its declared fields does not
- **THEN** the entry is written once, carrying the declared fields and the value it already held
- **AND** the report says the fields were updated, naming which ones

#### Scenario: A field drift is reported rather than written where the mode does not push

- **WHEN** a `keepassxc-to-safix` mapping's declared field differs from the entry's, or a `backup` mapping's does on an entry that already exists
- **THEN** nothing is written
- **AND** the report says the fields diverged, naming which ones, and the remedy under a pulling mode is that the declaration is the author

#### Scenario: A two-way mapping's fields have one author

- **WHEN** a `two-way` mapping's declared field differs from the entry's
- **THEN** the entry converges toward the declaration
- **AND** no agreement is remembered for a field, because a field has exactly one author and there is nothing for a memory to arbitrate

### Requirement: The report names mappings and never values

Each run SHALL report per mapping — unchanged, updated, pulled, fields updated, fields diverged, conflict, or refused with the reason — and no value and no derivative of a value SHALL appear in any output path.
A report of a field SHALL name the field and never its content, because a note may itself be sensitive and a field read out of another entry is a secret.
A mapping that could not be judged SHALL be reported, never silently skipped.
An entry under the declared group that no mapping declares SHALL be reported as information, including a companion entry whose mapping is gone, and SHALL NOT be removed.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value

#### Scenario: A field is named and its content is not

- **WHEN** a run reports that a mapping's fields were updated or diverged
- **THEN** the report names the fields concerned
- **AND** no output carries what any of them holds, on either side

### Requirement: The database is a store being written, never a keyring being managed

Sync SHALL NOT create databases, change database keys, or program, reprogram, or delete a hardware slot; it SHALL reach the database through the store's own command with a single password prompt per run, and a secret value SHALL travel standard input or pipes, never an argument vector or an environment variable.
A field declared as a literal is not a secret value in the sense this requirement governs, because the declaration it comes from is evaluated into a world-readable store, and MAY therefore travel the store's own command line the way the entry path already does; a field declared as a reference to another entry is a secret value and SHALL be refused for this target rather than placed in an argument vector.
A database MAY additionally require a YubiKey challenge-response slot, a key file, or both to open, and reading that slot or that file to unlock the database is not touching it in the sense this requirement forbids: the slot number, its optional serial, and the key file's own path are public identifiers of how the database opens rather than secrets it holds, and MAY travel the store's own command line the way the database's own path already does.
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
- **AND** the refusal is this transport's own, declared as the one-line limit of its value channel rather than assumed of every store

#### Scenario: A declared slot is read, never programmed

- **WHEN** a database declares a YubiKey challenge-response slot
- **THEN** every command sync issues against that database reads the slot to answer the store's own unlock challenge
- **AND** no command sync issues anywhere creates, reprograms, or deletes that slot or any other

#### Scenario: No argument vector carries a secret field

- **WHEN** the arguments of every command a run issues are enumerated
- **THEN** each field among them came from a literal in the declaration
- **AND** no field read out of another entry is there, because such a field is refused for this target before a run reaches a write

### Requirement: audit compares the mirror without writing, scoped to the keepassxc target

`audit`, bare or with `keepassxc` as its target, SHALL read both sides of every declared keepassxc mapping — or the ones named — compare them per mode, and change nothing on either side.
Each mapping SHALL be reported as agreeing, diverged, fields diverged, or unjudgeable; a divergence of the value SHALL take precedence over a divergence of the fields, and a divergence of the fields SHALL name the fields and never their contents.
A divergence of the fields SHALL move the exit status on the same footing as a divergence of the value, because a declared field that is not there is a declaration that is not true.
The report SHALL include, as information, every entry under the declared group that no currently declared mapping accounts for, in the same shape `sync`'s own report already gives that finding, and this report SHALL NOT change what a `backup` mapping or any other mode would otherwise do, because nothing here writes.

#### Scenario: A compare-only run writes nothing

- **WHEN** `audit keepassxc` runs over any mix of agreeing and diverged mappings
- **THEN** no side of any mapping is written
- **AND** each mapping's outcome is reported as agreeing, diverged, fields diverged, or unjudgeable

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

#### Scenario: A value divergence is not reported as a field divergence

- **WHEN** a mapping's value and one of its declared fields both differ
- **THEN** the mapping is reported as diverged
- **AND** the lesser finding does not displace the greater one in the report or in the exit status
