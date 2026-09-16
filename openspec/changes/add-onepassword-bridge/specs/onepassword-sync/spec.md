## Purpose

A declared subset of safix's secrets exists, converged, in the operator's 1Password vaults — under the vault, item and mode the declaration chooses, with the metadata the declaration names beside each value — so a person-read credential kept in 1Password has one place to drift from and a verb that ends the drift, on a target whose own command no check of this repository may ever run.

## ADDED Requirements

### Requirement: The mirror is declared, and the vault, the item and the mode are the consumer's

Which secrets sync, into which 1Password vault, under which item name, in which mode, and with which of the four declared fields beside the value SHALL be declared in nix, one mapping per safix entry.
The modes SHALL be named by their endpoints — `safix-to-1password`, `1password-to-safix`, `two-way`, and `backup`.
The vault and the item SHALL be declared as plain strings rather than as paths, for the reason the mirrored database's own location already is one: a path interpolated into a declaration is copied into the world-readable store on every evaluation.
Evaluation SHALL refuse a mapping whose safix side no declaration resolves, a `1password-to-safix` or `two-way` mapping onto an entry a generator produces, two mappings naming one item in one vault, and a mapping whose id is one the command line reserves as a target keyword.
Evaluation SHALL NOT refuse anything about the far side: whether the vault exists, whether the item does, and whether either side holds a value are run-time questions that need a session.

#### Scenario: The declaration names everything

- **WHEN** a mapping is declared
- **THEN** it names the safix side by person and entry, the far side by vault and item, its mode by endpoints, and any field it sets by that field's own name
- **AND** nothing about the naming, the direction or the fields is invented at run time

#### Scenario: A second producer is refused where it is visible

- **WHEN** a pull-capable mapping targets a generator-produced entry
- **THEN** evaluation refuses, naming the mapping, the generator, and why two producers for one value is a race

#### Scenario: Two mappings naming one item are refused

- **WHEN** two mappings name the same item in the same vault
- **THEN** evaluation refuses, naming both mappings and the item
- **AND** two mappings naming the same item name in two different vaults are accepted, because they are two items

#### Scenario: A mapping id reserved for a target keyword is refused

- **WHEN** a mapping's id is one of the reserved target keywords, `1password` among them
- **THEN** evaluation refuses, naming the mapping and the word it collides with
- **AND** the reason given is the one every other target gives for the same refusal: `sync` and `audit` read their first argument as a target keyword or a mapping name, never both

#### Scenario: The target has exactly one spelling

- **WHEN** `sync` or `audit` is given this target's keyword
- **THEN** `1password` names it
- **AND** `op` does not, because that is the name of the program the runtime invokes rather than of the store, and an alias would put two spellings of one target into reports, refusals and the reserved list

#### Scenario: The far side is not an evaluation question

- **WHEN** the evaluation-time refusals are documented
- **THEN** they state that the vault and the item are content of a remote service and are not among them
- **AND** they state that a declared vault a session cannot see is refused when a run reaches the mapping, naming the mapping and the vault

### Requirement: Each mode converges exactly as its name says

`safix sync`, bare or with `1password` as its target, SHALL read both sides of every declared mapping of this target — or the ones named — and converge per mode: `safix-to-1password` writes the item to safix's value; `1password-to-safix` writes safix to the item's value through the ordinary write path with every write-path refusal in force; `two-way` writes toward whichever side changed since the last agreement; `backup` writes only where the item does not exist or holds nothing.
No mode SHALL delete an item, a field or a vault, and a run over agreeing mappings SHALL write nothing anywhere.
A value carrying newlines SHALL be carried whole, because this target's own transport carries one.

#### Scenario: Agreement writes nothing

- **WHEN** a mapping's two sides already hold the same value and the same declared fields
- **THEN** neither side is written
- **AND** the report says unchanged

#### Scenario: Backup never overwrites

- **WHEN** a `backup` mapping meets an item holding a different value
- **THEN** nothing is written, neither the value nor any field
- **AND** the divergence is reported rather than resolved

#### Scenario: A pull is an ordinary write

- **WHEN** a `1password-to-safix` or `two-way` mapping converges toward the item's value
- **THEN** the safix side is written through the same path a hand-set value takes, commits included
- **AND** a refusal that path would make — the empty value, the recipient drift — is made here too

#### Scenario: A multi-line value crosses whole

- **WHEN** a mapping's value carries newlines
- **THEN** it is written and read back byte-identically
- **AND** no refusal about the value's shape exists for this target, because the transport that would have needed one is not the transport in use

#### Scenario: `sync 1password` narrows to this target, and accepts more than one mapping name

- **WHEN** `sync 1password` is given one or more mapping names
- **THEN** it converges exactly those mappings, in each one's own declared mode
- **AND** when given none it converges every declared mapping of this target

#### Scenario: Bare sync converges every target, this one's mappings among them

- **WHEN** `sync` runs with no target and no mapping named
- **THEN** the mappings this requirement governs converge alongside every other target's, each in its own declared mode
- **AND** a consumer declaring no mapping of this target reaches the service not at all, not even for the session preflight

### Requirement: The value and the four fields travel standard input, never an argument vector

Every value and every declared field SHALL reach the service on the standard input of its own command, as one structured item payload, and SHALL NOT appear in an argument vector or an environment variable.
This target SHALL declare that it carries all four fields — the username, the URL, the notes, and the tags — each on that same standard input, so no declared field of this target is refused for want of a channel and a field whose source is another safix entry is admissible on every one of the four.
A declared URL SHALL be written where the person's own browser looks for it rather than into a similarly named field that no autofill reads.

#### Scenario: No value and no field is ever an argument

- **WHEN** every command a run issues is recorded with its arguments, its environment and its standard input
- **THEN** no argument and no environment variable carries a value or a field
- **AND** the program the runtime invokes refuses any argument that assigns one, so the rule is enforced by the instrument rather than by review

#### Scenario: The reason the assignment form is refused is recorded

- **WHEN** the refusal of the assignment form is documented
- **THEN** it states the service's own published reason: an assignment statement is recorded in shell history and can be visible to other processes
- **AND** it states that the structured payload on standard input is the one form safix uses, for writing a new item and for editing an existing one alike

#### Scenario: A field sourced from another entry is a secret and still crosses

- **WHEN** a declared field's source is another entry of the mapping's own user
- **THEN** it is resolved at run time and crosses on the same standard input the value does
- **AND** it is refused on no ground of channel, because this target has no argument-bound field

#### Scenario: The capability is declared rather than assumed

- **WHEN** this target's field capability is read
- **THEN** it names all four fields and the single channel that carries them
- **AND** the record states that this is an asymmetry against a target whose command carries some fields only as arguments and one field not at all, which is why a per-target capability exists instead of one shared default

### Requirement: The session is the operator's, and it is checked before anything is read

A run SHALL establish that the service is reachable and the session is authenticated before any mapping's side is read, and SHALL refuse naming the service's own words when it is not.
safix SHALL NOT sign in, SHALL NOT mint, hold, print or store a session token, and SHALL NOT pass one in an argument vector; the session material SHALL be whatever the operator's own environment already carries into the child process.
A declared account SHALL be optional, and its absence SHALL NOT be a refusal, because the program resolves its own default account and an automation token names one implicitly.

#### Scenario: A signed-out run refuses before it decrypts anything

- **WHEN** a run begins with no authenticated session
- **THEN** it refuses, naming the declared account when one is declared and carrying the service's own message
- **AND** safix's own side of no mapping has been decrypted, because the check precedes the first read

#### Scenario: safix is not a keyring

- **WHEN** the commands a run may issue are enumerated
- **THEN** none of them signs in, signs out, creates or revokes a credential of the service
- **AND** the reason is recorded: the store is a store being written, never a keyring being managed

#### Scenario: A session never becomes an argument

- **WHEN** session material reaches a child process
- **THEN** it reaches it in that child's environment, inherited from the operator's own
- **AND** it is never placed in an argument vector, because a process's arguments are readable by every user on the machine while its environment is not

#### Scenario: An undeclared account is a working configuration

- **WHEN** mappings are declared and no account is
- **THEN** the run proceeds, letting the service resolve its own account
- **AND** where the account is the defect, the refusal carries the service's own sentence rather than a safix-invented one about a missing declaration

### Requirement: Two-way remembers the last agreement inside the item

A `two-way` mapping SHALL record the last-synced state as a concealed custom field of the mapped item itself, and SHALL NOT record any value-derived state in the repository.
When both sides have changed since that state, the run SHALL write nothing for the mapping and report a conflict naming both one-way remedies.
An unreadable or unrecognised recorded state SHALL be treated as absent, which converts the mapping to bootstrap semantics rather than to a refusal.
Because the state lives on the item, this target SHALL reserve no item name and SHALL have no refusal keeping a reserved name out of a consumer's reach.

#### Scenario: The tiebreak is the recorded state

- **WHEN** exactly one side differs from the last-synced state
- **THEN** the other side converges to it
- **AND** the recorded state is updated in the same write that carries the value

#### Scenario: Both changed is a conflict, not a guess

- **WHEN** both sides differ from the last-synced state and from each other
- **THEN** nothing is written
- **AND** the finding names the mapping and the two one-way modes that would each resolve it

#### Scenario: No oracle lands in the repository

- **WHEN** the repository is searched for sync state
- **THEN** no digest or derivative of a secret value is committed anywhere

#### Scenario: A memory nobody can read is bootstrap, not breakage

- **WHEN** the recorded state is absent, unreadable, or carries a format this safix does not know
- **THEN** the mapping is judged as though nothing had ever been recorded
- **AND** the run converges or reports a conflict on the two sides alone, without refusing

#### Scenario: No name is reserved, and the record says why

- **WHEN** this target's evaluation-time refusals are read beside another target's
- **THEN** this one has no reserved-name refusal
- **AND** the reason is recorded: the companion object another target needs exists only because that target's command cannot write a custom field, and this one's can

### Requirement: An edit preserves everything the declaration does not name

A write onto an existing item SHALL begin from that item's own current content, replace only the value, the declared fields, and the recorded state, and leave every other member of the item as it was.
safix SHALL NOT construct an item from a blank template when editing.

#### Scenario: What safix does not declare survives a write

- **WHEN** a mapping converges onto an item carrying a passkey, a one-time-password field, and a custom section no declaration names
- **THEN** all three are still present, unchanged, after the write
- **AND** the value and the declared fields are the only members that moved

#### Scenario: The danger is removed rather than refused

- **WHEN** the reason for the round-trip is documented
- **THEN** it states the service's own published danger: an item edited from a template loses the passkeys on it
- **AND** it states that safix cannot reach that outcome because it never sends a template, so no refusal about passkey-bearing items exists and an ordinary login item carrying one stays mappable

### Requirement: The report names mappings and never values, including when a run stops partway

Each run SHALL report per mapping — unchanged, updated, pulled, conflict, or refused with the reason — and no value and no derivative of a value SHALL appear in any output path.
A mapping that could not be judged SHALL be reported, never silently skipped.
A failure against the service on one mapping SHALL refuse that mapping and SHALL NOT end the run, so the report is complete over the declarations whatever happened, and a run carrying any refusal SHALL exit non-zero.
An item in a declared vault that no mapping declares SHALL be reported as information and SHALL NOT be removed.
A field divergence SHALL be named by the field and never by its content.

#### Scenario: The report is complete and value-free

- **WHEN** a run finishes over any mix of outcomes
- **THEN** every declared mapping appears in the report with its outcome
- **AND** no output contains a value or any derivative of one

#### Scenario: A network failure mid-run is a refusal for its own mapping

- **WHEN** the service refuses or is unreachable while one mapping of several is being converged
- **THEN** that mapping is reported refused, naming the item and carrying the service's own message
- **AND** every other declared mapping is still judged and reported, and the run exits non-zero

#### Scenario: A field divergence is named, not printed

- **WHEN** a mapping's value agrees and a declared field does not
- **THEN** the report names the field
- **AND** it prints neither the declared content nor the content found, because a notes body and a field sourced from another entry are themselves secrets

#### Scenario: An item no mapping declares is reported and left alone

- **WHEN** a declared vault holds an item no mapping names
- **THEN** it is reported as information
- **AND** it is not removed, by this run or by any other effect of running it

### Requirement: audit compares the mirror without writing, scoped to the 1password target

`audit`, bare or with `1password` as its target, SHALL read both sides of every declared mapping of this target — or the ones named — compare them, and change nothing on either side.
Each mapping's outcome SHALL be reported as agreeing, diverged in its value, diverged in named fields, or unjudgeable, with a value divergence taking precedence over a field divergence for the same mapping.
The report SHALL include, as information, every item in a declared vault that no currently declared mapping accounts for, and that information SHALL NOT change the exit status.

#### Scenario: A compare-only run writes nothing

- **WHEN** `audit 1password` runs over any mix of agreeing and diverged mappings
- **THEN** no side of any mapping is written, and no item is created
- **AND** each mapping's outcome is reported

#### Scenario: The remedy named depends on the mode

- **WHEN** a diverged mapping is reported
- **THEN** a mapping under a pushing mode is given `safix sync 1password <mapping>` as its remedy
- **AND** a mapping under `1password-to-safix` is told instead that the declaration is the author, because for that mode converging toward the declaration is what the mode already means

#### Scenario: A value divergence is not reported as a field one

- **WHEN** a mapping's value and one of its declared fields both differ
- **THEN** the outcome reported is the value divergence
- **AND** the field divergence does not replace it, so a cosmetic drift is never mistaken for a secret drift and a secret drift is never reported as cosmetic

#### Scenario: audit's exit status answers only whether every compared mapping agreed

- **WHEN** a run finds one or more items no mapping declares and every compared mapping agrees
- **THEN** `audit 1password` still exits reporting agreement
- **AND** the undeclared items are reported alongside it as information

### Requirement: No check of this repository ever runs the real 1Password command, and the absence is stated

The flake SHALL NOT reference the 1Password command-line package from any check, package or development shell, and no check SHALL drive a real one.
The declaration surface, the refusals and the transport's argument, environment and standard-input discipline SHALL be held by checks against a stand-in program that records what it was given and refuses what safix promises never to send.
The absence of a real-binary check SHALL be written down where an implementer and a reader meet it — in the declaring module, in the transport, and here — rather than left to be inferred from a missing file.

#### Scenario: The package stays out of the closure

- **WHEN** the flake's checks, packages and development shells are enumerated
- **THEN** none of them references the 1Password command-line package
- **AND** the reason is recorded: that package is unfree, so a reference to it makes this flake fail to evaluate for every consumer who has not allowed unfree packages

#### Scenario: Three reasons, each sufficient, none expiring

- **WHEN** the absence of a real-binary check is documented
- **THEN** it names all three grounds: the package's licence, the lack of any self-hostable server to point a sandboxed node at, and the network that every authentication path needs and no hermetic build has
- **AND** it states that none of the three is a condition that may later be satisfied, so the absence is permanent rather than deferred

#### Scenario: The stand-in enforces the promise rather than illustrating it

- **WHEN** the stand-in program receives an invocation
- **THEN** it exits non-zero on any argument that assigns a value, and on an item command that names no vault
- **AND** a run that is not pointed at the stand-in is refused by the suite before a process is spawned, because a machine that develops this suite plausibly holds a real, signed-in command and a real vault

#### Scenario: What the model cannot establish is recorded beside it

- **WHEN** the transport's documentation is read
- **THEN** it states which facts about the service's own payload shape and flags are taken from published documentation rather than measured
- **AND** it names what a contributor holding a licensed command should re-measure, so the model's standing is visible rather than assumed
