# safix-cli Specification

## Purpose

The command that is the whole lifecycle of a secret: its subcommand contract, the fact that a value is always addressed by name and never by file, the pipe-only path a value travels in and out, the separation between the half that writes content and the half that writes policy, and the absences that are recorded rather than left mysterious.

## Requirements

### Requirement: One command covers the lifecycle, by name and never by file

The package SHALL provide a single command named `safix` with the subcommands `set`, `get`, `list`, `generate`, `check`, `fix`, `keygen`, and `adduser`.
Every subcommand that addresses a secret SHALL address it by name, and SHALL NOT require the operator to name a file.

#### Scenario: Addressing a secret

- **WHEN** an operator sets, reads, or generates a value
- **THEN** they name the secret
- **AND** the file and the key within it are resolved from the declarations

#### Scenario: The subcommand set is closed

- **WHEN** an unrecognised subcommand is given
- **THEN** the command fails naming the subcommands it accepts

#### Scenario: Runtime dependencies are pinned into the command

- **WHEN** the command runs
- **THEN** the tools it invokes come from its own closure
- **AND** none of them is inherited from the caller's environment

### Requirement: Values move through pipes wherever a pipe remains possible

Every leg of a value's journey SHALL be a pipe except those inside the private staging root, and no value SHALL reach an argument vector or an environment variable on any leg.

#### Scenario: The stream-writing and reading verbs are unchanged

- **WHEN** a value is written from standard input or read to standard output
- **THEN** it travels a pipe end to end

#### Scenario: The encrypting backend is still driven by pipes

- **WHEN** the encrypting backend is invoked for any operation
- **THEN** the value reaches it on a pipe
- **AND** no invocation names a value in its arguments or environment

#### Scenario: The exception is bounded and named

- **WHEN** the exception to this requirement is read
- **THEN** it names the generator staging root and the editor buffer
- **AND** it names no other location

#### Scenario: The change from the earlier absolute is stated

- **WHEN** this requirement is compared against the one it replaces
- **THEN** the difference is stated rather than presented as a clarification
- **AND** the reason is recorded: the interoperable generator contract is a filesystem contract, and emulating it with pipes would break tools that seek or reopen their inputs, with a truncated secret as the failure mode

### Requirement: The content half cannot alter the policy

Subcommands that write values SHALL be structurally unable to grant anyone the ability to read one.

#### Scenario: Setting a value grants nothing

- **WHEN** a value is written
- **THEN** the recipients used are those the file's own metadata or the committed policy already declares
- **AND** no run of a value-writing subcommand adds a recipient

#### Scenario: Policy changes go through one subcommand

- **WHEN** the recipient policy must change
- **THEN** the change comes from the declarations, applied by the reconciling subcommand
- **AND** that subcommand regenerates the policy before re-wrapping, never the other way round

### Requirement: `check` diffs intent against reality and `fix` reconciles what is reconcilable

`check` SHALL be read-only and SHALL report each finding with the command that resolves it.
`fix` SHALL regenerate the policy and re-wrap the governed files to the audiences declared, and SHALL name what needs a human rather than attempting it.

#### Scenario: A finding carries its remedy

- **WHEN** `check` reports a finding
- **THEN** the report includes the command that fixes it
- **AND** the report distinguishes what `fix` can reconcile from what requires a keyholder

#### Scenario: `check` needs no identity it does not have

- **WHEN** `check` examines a file the operator holds no identity for
- **THEN** it answers from the document's structure rather than its contents
- **AND** it does not attempt decryption

#### Scenario: `fix` is not revocation

- **WHEN** `fix` re-wraps files after an audience narrows
- **THEN** its output states that this aligns ciphertext with policy
- **AND** states that revoking means minting a new value, naming the command that does so

#### Scenario: `list` shows what a person holds

- **WHEN** `list` runs
- **THEN** it reports each secret's origin, the file it resolves to, whether it has a generator, and whether it is shared

### Requirement: `keygen` belongs to the person and `adduser` to the operator

`keygen` SHALL mint an identity on the machine of the person who will hold it.
`adduser` SHALL scaffold a declaration for a person from a public recipient they supply, and SHALL mint nothing.

#### Scenario: Onboarding requires a public key only

- **WHEN** a person is onboarded
- **THEN** the only material they provide is a public recipient, or the public key it derives from
- **AND** the derivation is reproducible by anyone holding that public key alone

#### Scenario: `adduser` mints nothing

- **WHEN** `adduser` runs
- **THEN** it creates no key, no password material, and no secret value
- **AND** its help text states this

#### Scenario: Only the recipient's shape is checked

- **WHEN** a recipient is supplied
- **THEN** its shape is validated
- **AND** the help text states that whether anyone holds the private half is not knowable from the operator's machine

#### Scenario: A recipient requiring interaction is refused for the primary field

- **WHEN** a recipient that requires a physical interaction to decrypt is supplied as the primary recipient
- **THEN** it is refused
- **AND** the message directs it to the further-recipients field, because activation decrypts without interaction

#### Scenario: A newly declared person holds nothing

- **WHEN** `adduser` completes
- **THEN** the person's declarations are empty, so no audience includes them and no rule is emitted for them yet
- **AND** the output names the sequence that gives them their first secret

#### Scenario: --show prints the operator's own public recipient and mints nothing

- **WHEN** `keygen --show` runs
- **THEN** it prints the public recipient derived from the identity already minted on this machine
- **AND** it mints no identity, appends nothing to the keys file, and writes nothing
- **AND** when no identity exists yet on this machine, it refuses, naming plain `keygen` as the remedy

### Requirement: check reports a value minted under a definition that has changed

`safix check` SHALL report a finding for every generated value whose recorded definition digest no longer matches the current declaration, naming the entry and both remedies: regenerate the value, or revert the declaration edit.
A value with no record predates the record and SHALL NOT be a finding.

#### Scenario: An edited definition is reported

- **WHEN** a generator's declaration changes after its value was minted
- **THEN** `check` reports the entry as minted under a definition that no longer exists
- **AND** the finding names regeneration and reverting the edit as the remedies
- **AND** the finding carries no value

#### Scenario: Regeneration clears the finding

- **WHEN** the value is regenerated under the current declaration
- **THEN** the finding is gone on the next `check`

#### Scenario: A capability grant is a definition change

- **WHEN** a generator's network grant flips after its value was minted
- **THEN** `check` reports the entry as minted under a definition that no longer exists
- **AND** the record stays put until a regeneration adopts the grant or the flip is reverted

#### Scenario: What is out of scope stays quiet

- **WHEN** an entry has no generator, or a generated value has no record
- **THEN** no drift finding exists for it

### Requirement: set reads a stream when one is offered

`safix set` SHALL read the value from standard input when standard input is not a terminal, storing the bytes exactly as received, with no prompt and no confirmation.
When standard input is a terminal, the hidden double prompt SHALL remain the behaviour.

#### Scenario: A piped value is stored as its own bytes

- **WHEN** a value is piped into `safix set <name>`
- **THEN** the stored bytes are exactly the piped bytes
- **AND** nothing prompts and nothing asks for confirmation
- **AND** the value reaches no argument vector and no environment variable

#### Scenario: Empty input keeps its refusal

- **WHEN** the pipe yields no bytes
- **THEN** the write is refused as an empty value, exactly as an empty prompt is

#### Scenario: The terminal path is unchanged

- **WHEN** standard input is a terminal
- **THEN** the hidden prompt, the confirmation, and the single-line contract hold as they do today

### Requirement: Scaffolding verbs honour the delegation records

When the target person declares `managedBy`, `enroll` and the record-editing half of onboarding SHALL refuse an acting identity that is not among that organization's managers, reading the acting identity from the same git identity the resulting commit carries, and a permitted scaffold's commit SHALL record the organization context.
When the target declares no `managedBy`, the verbs SHALL behave exactly as before.

#### Scenario: A manager scaffolds and the record says so

- **WHEN** alice, a manager of acme, enrolls a card for bob, who declares `managedBy` acme
- **THEN** the scaffold proceeds and its commit records the acme context

#### Scenario: An outsider is refused before anything is edited

- **WHEN** mallory, no manager of acme, attempts the same scaffold
- **THEN** the verb refuses before editing any file, naming the organization and where its managers are declared

#### Scenario: Unmanaged people are untouched by the feature

- **WHEN** the target person declares no `managedBy`
- **THEN** the verb neither reads nor mentions delegation

### Requirement: Group membership is a verb with the narrowing disclosure

`safix group add <group> <subject>` and `safix group remove <group> <subject>` SHALL edit the group's declaration as text, parsed by the real parser before staging and committed, with `remove` printing the not-retroactive disclosure and naming the revocation report that will carry the shrink.
Both SHALL honour the delegation records over groups the organization's silo declarations cover, and both SHALL refuse a subject or group the fleet does not declare.

#### Scenario: An addition is one inserted line

- **WHEN** alice runs `safix group add oncall bob`
- **THEN** the group's declaration gains bob as one inserted line, parsed before staging, and the commit names the act

#### Scenario: A removal says what it does not undo

- **WHEN** alice runs `safix group remove oncall bob`
- **THEN** the edit lands and the verb prints that bob has seen what the group could read and rotation is the remedy
- **AND** the next `check` reports the shrink as the revocation it is

### Requirement: Retired and reserved verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a word this package's own vocabulary once used is retired outright, the help SHALL say so; where one is reserved for a feature not yet built, the help SHALL say that instead, so a reservation is never mistaken for an oversight.

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
