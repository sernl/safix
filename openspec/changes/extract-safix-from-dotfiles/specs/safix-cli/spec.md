## Purpose

The command that is the whole lifecycle of a secret: its subcommand contract, the fact that a value is always addressed by name and never by file, the pipe-only path a value travels in and out, the separation between the half that writes content and the half that writes policy, and the absences that are recorded rather than left mysterious.

## ADDED Requirements

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

### Requirement: Values move through pipes only

A secret value SHALL reach the encryption tool as a stream and SHALL leave it as a stream.
No subcommand SHALL place a value in an argument vector, an environment variable, or a file it creates.

#### Scenario: Writing a value

- **WHEN** a value is set
- **THEN** the input is read without echoing, confirmed, and written as a stream
- **AND** it appears in no argument vector, environment variable, or temporary file

#### Scenario: Reading a value

- **WHEN** a value is read
- **THEN** it is written to standard output for piping
- **AND** nothing else the command prints is mixed into that stream

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

### Requirement: Absent verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no upload verb exists because activation already delivers what it would
- **AND** states that no export or import verb exists because they serve a backend migration this package does not have, and a plaintext export tree outlives the migration that justified it
