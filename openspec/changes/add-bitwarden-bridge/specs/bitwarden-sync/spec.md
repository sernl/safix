## ADDED Requirements

### Requirement: The mirror is declared, and the naming, the addressing and the mode are the consumer's

Which secrets converge with which Bitwarden items, under which folder and item name, and in which mode SHALL be declared in nix, one mapping per safix entry.
The modes SHALL be named by their endpoints — `safix-to-bitwarden`, `bitwarden-to-safix`, `two-way`, and `backup`.
An item SHALL be addressed by an optional folder and a name, never by the vault's own item identifier, so that a declaration names the thing a person sees in their vault.
The server SHALL be declared at most once, alongside the mappings rather than per mapping, and MAY be left undeclared, in which case the server is whichever one the operator's own client is configured against.
Evaluation SHALL refuse a mapping whose safix side no declaration resolves, a `bitwarden-to-safix` or `two-way` mapping onto an entry a generator produces, two mappings naming one folder-and-item pair, a mapping whose id is one the command line reserves as a target keyword, and a mapping declaring a field this target cannot carry.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, the vault side by folder and item name, and its mode by endpoints
- **AND** nothing about the naming, the addressing or the direction is invented at run time

#### Scenario: An item identifier is not an address a declaration may use

- **WHEN** the vault side of a mapping is read
- **THEN** it carries a folder and a name, and no option accepts the vault's own item identifier
- **AND** the reason is recorded: an identifier is opaque to review, is not what the person holding the item sees, and is reissued by a restore of the vault, so a declaration written against one would silently stop naming anything

#### Scenario: A second producer is refused where it is visible

- **WHEN** a pull-capable mapping targets a generator-produced entry
- **THEN** evaluation refuses, naming the mapping, the generator, and why two producers for one value is a race

#### Scenario: Two mappings naming one item are refused

- **WHEN** two mappings name the same folder and item name
- **THEN** evaluation refuses, naming both mappings and the item they collide on

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is one of the reserved target keywords, `bitwarden` among them
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is the one `bridge-surface` gives for the same refusal over its own mappings: `sync` and `audit` read their first argument as a target keyword or a mapping name, never both

### Requirement: Each mode converges exactly as its name says

`safix sync`, bare or with `bitwarden` as its target, SHALL read both sides of every declared bitwarden mapping — or the ones named — and converge per mode: `safix-to-bitwarden` writes the vault to safix's value; `bitwarden-to-safix` writes safix to the vault's value through the ordinary write path with every write-path refusal in force; `two-way` writes toward whichever side changed since the last sync; `backup` writes only where the vault holds no such item.
No mode SHALL delete an item, a folder, or a field on either side, and a run over agreeing mappings SHALL write nothing anywhere.
A value carrying newlines SHALL be carried whole, because this target's transport is not line-oriented.

#### Scenario: Agreement writes nothing

- **WHEN** a mapping's two sides already hold the same value and the same declared fields
- **THEN** neither side is written
- **AND** the report says unchanged

#### Scenario: Backup never overwrites

- **WHEN** a `backup` mapping meets an existing item holding a different value
- **THEN** nothing is written, neither the value nor any declared field
- **AND** the divergence is reported rather than resolved

#### Scenario: A pull is an ordinary write

- **WHEN** a `bitwarden-to-safix` or `two-way` mapping converges toward the vault's value
- **THEN** the safix side is written through the same path a hand-set value takes, commits included
- **AND** a refusal that path would make is made here too

#### Scenario: A multi-line value crosses whole

- **WHEN** a mapping's value carries a newline
- **THEN** it is written and read back byte for byte
- **AND** the refusal the keepassxc target makes for the same value is not made here, because that refusal is a property of a line-oriented command rather than of a secret

#### Scenario: An existing item is edited rather than replaced in part

- **WHEN** a mapping writes to an item the vault already holds
- **THEN** every field of that item the declaration does not govern is still there afterwards, unchanged
- **AND** the reason is recorded: this target's edit is a whole-item replace, so a write that did not carry the rest of the item forward would delete it

#### Scenario: `sync bitwarden` narrows to the bitwarden target, and accepts more than one mapping name

- **WHEN** `sync bitwarden` is given one or more mapping names
- **THEN** it converges exactly those mappings, in each one's own declared mode
- **AND** when given none it converges every declared bitwarden mapping

#### Scenario: Bare sync converges every target, bitwarden's mappings among them

- **WHEN** `sync` runs with no target and no mapping named
- **THEN** the bitwarden mappings this requirement governs converge alongside every other target's, each in its own declared mode

### Requirement: The vault is unlocked, never logged into, and the session key is the one credential that may travel an environment

safix SHALL NOT log a vault in, register a device, create a vault, or change a vault's password.
Before any side is read, safix SHALL ask the client for its own state and SHALL refuse, naming the remedy, unless that state is unlocked or can be made unlocked by a password asked once on a terminal.
The master password SHALL travel the client's standard input and nothing else.
The session key the unlock returns SHALL travel to every later invocation in that invocation's own environment, and SHALL NOT appear in any argument vector, in any file safix writes, or in any output path.
No mapped value and no master password SHALL travel an argument vector or an environment on any invocation.
Without a terminal to ask the master password on, and with the client not already unlocked, safix SHALL refuse rather than prompt into the void.
Where a server is declared and the unlocked client reports reaching a different one, safix SHALL refuse before reading any side, naming both URLs.

#### Scenario: A locked or unauthenticated client refuses before anything is read

- **WHEN** the client reports that it is locked with no terminal to ask on, or that it is not logged in at all
- **THEN** safix refuses before reading any side of any mapping, naming which of the two states it found and the remedy for that state
- **AND** logging in is named as the operator's own act rather than performed

#### Scenario: The master password travels standard input

- **WHEN** the unlock invocation is enumerated
- **THEN** the password is on its standard input
- **AND** it is in no argument and in no environment variable, including the client's own password-from-environment form, which is not used

#### Scenario: The session key is in the environment and nowhere else

- **WHEN** every invocation of a run is enumerated with its arguments, its environment and its input
- **THEN** the session key appears in exactly one place, the environment of the invocations that need it
- **AND** the narrowing is recorded: an argument vector is readable by any process on the machine and an environment is readable by the same user, so the key is placed in the lesser exposure rather than in no exposure, which this transport does not offer

#### Scenario: No mapped value is ever in an argument or an environment

- **WHEN** every invocation of a run is enumerated
- **THEN** no mapped value and no master password appears in any argument vector or any environment
- **AND** the item payload each write carries travels standard input

#### Scenario: A declared server that is not the one reached is refused

- **WHEN** a server is declared and the unlocked client reports a different server
- **THEN** safix refuses before reading any side, naming the declared URL and the reached one
- **AND** nothing is written, because a write against the wrong vault is not correctable by a later run

#### Scenario: No server declared is not a defect

- **WHEN** mappings are declared and no server is
- **THEN** the run proceeds against whichever server the client is configured against
- **AND** the reason is recorded: the client already holds that configuration, and requiring a declaration to restate it would add a second place for one fact to be wrong

### Requirement: A refresh that failed is a refusal, not silent agreement

Before the first read of a run, safix SHALL ask the client to refresh its local copy of the vault, and SHALL refuse every mapping on this target when that refresh fails.
The refusal SHALL state that the local copy may predate another device's change, so a comparison against it would report agreement that is not there, and — in `backup` mode — would write into a vault that already holds the item.

#### Scenario: The refresh happens once, before the first read

- **WHEN** a run over any number of bitwarden mappings is enumerated
- **THEN** exactly one refresh invocation precedes the first read
- **AND** no further refresh happens between two mappings, because two reads of one run must see one vault

#### Scenario: A failed refresh refuses rather than proceeding

- **WHEN** the refresh fails
- **THEN** no mapping on this target is read, judged or written
- **AND** the refusal names the failure the client reported and why a stale copy cannot be compared against

### Requirement: Two-way remembers the last agreement inside the vault

A `two-way` mapping SHALL record the last-synced state as a hidden custom field of the mapped item itself, named for safix, inside the vault, and SHALL NOT record any value-derived state in the repository.
When both sides have changed since that state, the run SHALL write nothing for the mapping and report a conflict naming both one-way remedies.
The recorded state SHALL be written in the same format the keepassxc target's own memory uses, so one reader understands both.

#### Scenario: The memory is a field of the item, not a second item

- **WHEN** a `two-way` mapping records an agreement
- **THEN** it is a hidden custom field of the mapped item
- **AND** the reason is recorded: this target can write a hidden custom field, which is the one thing the keepassxc target's own command cannot, so this target needs no reserved name and no companion object, and no declaration can collide with the memory

#### Scenario: The tiebreak is the recorded state

- **WHEN** exactly one side differs from the last-synced state
- **THEN** the other side converges to it
- **AND** the recorded state is updated in the same item write

#### Scenario: Both changed is a conflict, not a guess

- **WHEN** both sides differ from the last-synced state and from each other
- **THEN** nothing is written
- **AND** the finding names the mapping and the two one-way commands that would each resolve it

#### Scenario: No oracle lands in the repository

- **WHEN** the repository is searched for sync state
- **THEN** no digest or derivative of a secret value is committed anywhere

### Requirement: Only the fields this target can carry may be declared

A mapping MAY declare a username, a URL and notes on its vault side, each written beside the value in the same item write.
A mapping declaring tags on this target SHALL be refused at evaluation, naming the target and the field.
A field sourced from another safix entry SHALL be permitted for every field this target carries, because this target's fields travel standard input rather than an argument vector.
Fields SHALL be written only, never read back into safix, on the terms `bridge-sync` states for every target.

#### Scenario: The three carried fields are written beside the value

- **WHEN** a pushing mapping declares a username, a URL or notes
- **THEN** they are written in the same item write as the value
- **AND** an item created by that write carries them from the start rather than being edited twice

#### Scenario: Tags are refused rather than approximated

- **WHEN** a mapping on this target declares tags
- **THEN** evaluation refuses, naming the mapping, the target and the field
- **AND** the reason given is that this vault has no tag concept at all — its folders and collections are placements rather than labels — so a declaration mapped onto one would mean something the declaration did not say, and a declaration silently dropped would be a mirror lying about what it holds

#### Scenario: A field sourced from another entry is permitted here

- **WHEN** a field names another entry of the same person as its source
- **THEN** it is resolved at run time and carried in the item payload on standard input
- **AND** the refusal the keepassxc target makes for the same declaration is not made here, because that refusal exists for a field whose only channel is an argument vector

### Requirement: An item addressed by folder and name that matches more than one is refused

Where the declared folder and name match more than one item, safix SHALL refuse the mapping, naming the mapping, the folder, the name and how many items matched, and SHALL NOT choose between them.
Where they match no item, the outcome SHALL depend on the mode: a mode that writes the vault creates the item, a mode that reads the vault refuses naming the mapping and the address.

#### Scenario: Ambiguity is refused, never resolved by a rule

- **WHEN** two items in the declared folder carry the declared name
- **THEN** the mapping is refused, naming the count
- **AND** no ordering, recency or identifier rule picks one, because a mirror that chose would converge a person's secret with whichever item happened to sort first

#### Scenario: An absent item is created by a writing mode and refused by a reading one

- **WHEN** the declared address matches no item
- **THEN** a `safix-to-bitwarden` or `backup` mapping creates it, and a `bitwarden-to-safix` mapping is refused naming the mapping, the address and the mode that makes the vault the source
- **AND** a `two-way` mapping treats one-sided presence as bootstrap rather than as a failure, on the terms `bridge-sync` states

### Requirement: The report names mappings and never values

Each run SHALL report per mapping — unchanged, updated, pulled, conflict, or refused with the reason — and no value and no derivative of a value SHALL appear in any output path.
A mapping that could not be judged SHALL be reported, never silently skipped.
A field divergence SHALL be reported by naming the field and never its content.
An item under a declared folder that no mapping declares SHALL be reported as information and SHALL NOT be removed.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value, a session key, or a master password

#### Scenario: A field divergence is named, never printed

- **WHEN** a declared field differs from the one the item carries
- **THEN** the report names the field
- **AND** it prints neither the declared content nor the item's, because notes may themselves be sensitive and a field sourced from another entry is a secret

### Requirement: audit compares the vault without writing, scoped to the bitwarden target

`audit`, bare or with `bitwarden` as its target, SHALL read both sides of every declared bitwarden mapping — or the ones named — compare them, and change nothing on either side.
It SHALL perform the same pre-read refresh and the same unlock contract a converging run performs, and SHALL refuse identically where either fails.
The report SHALL include, as information, every item under a declared folder that no currently declared mapping accounts for, and the exit status SHALL answer only whether every compared mapping agreed.

#### Scenario: A compare-only run writes nothing

- **WHEN** `audit bitwarden` runs over any mix of agreeing and diverged mappings
- **THEN** no side of any mapping is written, and no item is created
- **AND** each mapping's outcome is reported as agreeing, diverged, fields-diverged, or unjudgeable

#### Scenario: A comparison rests on the same refusals a write does

- **WHEN** the client is locked, or the refresh fails, or a declared server is not the one reached
- **THEN** `audit bitwarden` refuses exactly as `sync bitwarden` does
- **AND** it does not report agreement about mappings it could not look at

#### Scenario: Lingering items are information, not findings

- **WHEN** a run finds items under a declared folder that no mapping declares and every compared mapping agrees
- **THEN** it exits reporting agreement
- **AND** those items are reported alongside as information, and none of them is removed
