## Purpose

What a secret is as a declaration: the entry vocabulary under `flake.safix.catalogue`, the alphabet a name may be drawn from, the property that makes declarations scatterable across a consumer's tree, and the placement questions an entry is deliberately unable to answer.

## ADDED Requirements

### Requirement: Declarations are mergeable and may be scattered

The catalogue SHALL be an attribute set option on a flake-parts module, so that declarations made in separate files merge into one record.
No file layout, naming scheme, or import order SHALL be required of a consumer.

#### Scenario: One secret per file

- **WHEN** a consumer declares each secret in its own module file, anywhere in its tree
- **THEN** the resolver sees the same record it would see from a single file declaring all of them
- **AND** no import order changes the result

#### Scenario: The package prescribes no layout

- **WHEN** a consumer arranges its declaration files
- **THEN** nothing in the package reads a path, a filename, or a directory structure to find them
- **AND** the only requirement is that the modules are imported by the consumer's flake

#### Scenario: Two files declaring one name

- **WHEN** two modules declare the same secret name with different fields
- **THEN** the module system's own merge rules apply and a genuine conflict is an evaluation error naming the option
- **AND** the package adds no silent last-writer-wins behaviour of its own

### Requirement: An entry declares what a secret is, never where its ciphertext lives

A catalogue entry SHALL carry `mode`, `path`, `shared`, `generator`, and `sopsKey`, and SHALL carry no field naming the encrypted file that holds the value.

#### Scenario: The declarable fields

- **WHEN** an entry is declared
- **THEN** it may set the on-disk mode, the on-disk path as a function of the consuming configuration, whether its carriers hold one value between them, how the value is minted, and which key inside the encrypted file holds it
- **AND** each unset field takes a documented default

#### Scenario: Naming the encrypted file is refused

- **WHEN** an entry attempts to name the encrypted file its value lives in
- **THEN** evaluation fails naming that entry
- **AND** the message states that the file is derived from the audience and names the option that widens an audience

#### Scenario: The key inside the file defaults to the entry's own name

- **WHEN** an entry sets no explicit key
- **THEN** the value is read under a key equal to the entry's name
- **AND** setting the key explicitly changes only which key is read, never which file is read

### Requirement: The catalogue and a user's private declarations share one vocabulary

An entry declared in the catalogue and an entry declared under a user's own private declarations SHALL be the same type with the same defaults.

#### Scenario: A private entry is a catalogue entry with a different audience

- **WHEN** the same fields are declared in the catalogue and in a user's private declarations
- **THEN** both produce the same resolved entry apart from who can read it
- **AND** no field is available in one place and absent in the other

#### Scenario: Declaring privately is itself selecting

- **WHEN** a user declares a secret in their own private declarations
- **THEN** that secret resolves for them with no second selection step
- **AND** no catalogue entry of that name is required to exist

### Requirement: Names are drawn from a restricted alphabet

Every secret, user, and recovery-recipient anchor name SHALL match `[a-z0-9][a-z0-9_-]*`, and a name outside it SHALL fail evaluation naming the declaration.

#### Scenario: A malformed secret name

- **WHEN** a secret is declared with a name outside the alphabet
- **THEN** evaluation fails naming the declaration site and the offending name
- **AND** the message states that the name becomes the last component of an on-disk path

#### Scenario: A malformed user name

- **WHEN** a user is declared with a name outside the alphabet
- **THEN** evaluation fails naming that user
- **AND** the message states that the name is interpolated into a secrets path and into a recipient rule's path pattern

#### Scenario: The audience separator lies outside the alphabet

- **WHEN** a shared audience's members are joined into one directory name
- **THEN** the separator used is a character the name alphabet excludes
- **AND** a separator drawn from inside the alphabet fails a check, because two distinct audiences could otherwise join to one directory and so to one rule

### Requirement: Two entries may not claim one on-disk path

Two resolved entries that would be written to the same on-disk path SHALL fail evaluation naming both.

#### Scenario: A collision between two declarations

- **WHEN** two entries resolve to the same path for one consuming configuration
- **THEN** evaluation fails naming both entries and the shared path
- **AND** the message states that the secret provisioner unlinks whatever occupies a path it manages, so the two would delete each other's output

#### Scenario: The guard reads the built configuration

- **WHEN** a declaration reaches a configuration without passing through the catalogue
- **THEN** the path-collision guard still covers it, because it reads the paths claimed by the built configuration
- **AND** a path claimed twice fails regardless of where either declaration came from
