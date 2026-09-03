# public-outputs Specification

## Purpose

Outputs a generator declares as not secret: where their plaintext lives in the repository, how a nix module reads one at evaluation, and the checked guarantee that the recipient policy never reaches them.

## Requirements

### Requirement: An output declared as not secret is stored as plaintext in the repository

A generator output whose declaration marks it as not secret SHALL be written to the repository in the clear, and SHALL NOT be encrypted, staged as ciphertext, or given a creation rule.

#### Scenario: The bytes are readable without a key

- **WHEN** a public output has been generated
- **THEN** its file holds exactly the bytes the generator produced
- **AND** reading it requires no identity

#### Scenario: A public output is not routed to the encrypting backend

- **WHEN** the write path for a public output is followed
- **THEN** the encrypting backend is not invoked for it

#### Scenario: The declaration is per output

- **WHEN** a generator declares several outputs
- **THEN** each carries its own secrecy
- **AND** a generator may write a secret output and a public one in the same run

### Requirement: The plaintext store is separable from the ciphertext tree by path prefix

Public outputs SHALL be stored under a top-level prefix distinct from the one holding encrypted material, and SHALL NOT be nested inside it.

#### Scenario: The two trees do not overlap

- **WHEN** the public store's prefix and the encrypted tree's prefix are compared
- **THEN** neither is a prefix of the other

#### Scenario: A prefix-scoped rule can address exactly one of them

- **WHEN** an exclusion, a backup policy or a search is scoped to the encrypted tree's prefix
- **THEN** no plaintext output is inside its scope

#### Scenario: The reason is recorded

- **WHEN** the location decision is documented
- **THEN** it states that a path named for secrets must mean that everything under it is encrypted, without qualification
- **AND** it records the alternative that was refused and why

#### Scenario: The layout distinguishes shared from per-user

- **WHEN** a public output's path is computed
- **THEN** a shared entry's path is keyed by its audience and a per-user entry's by its carrier
- **AND** the leaf carries the output's name

#### Scenario: A vault-mode leaf is opaque, not keyed by audience or carrier

- **WHEN** a public output's path is computed and a vault is declared
- **THEN** the leaf under the public prefix is a hash of the naming key and the output's readable identity, held as a single file rather than a `<name>/value` directory
- **AND** prefix separation from the encrypted tree still holds: neither prefix is a prefix of the other, opaque or not

### Requirement: No generated creation rule matches any public path, and this is checked

For every creation rule the policy renderer produces and every path the public store holds, a check SHALL assert that the rule does not match the path.

#### Scenario: The non-interaction is asserted by matching

- **WHEN** the check runs
- **THEN** it matches each generated rule against each public path
- **AND** it fails while any match exists

#### Scenario: The assertion is behavioural rather than textual

- **WHEN** the check's method is read
- **THEN** it matches paths against patterns
- **AND** it does not inspect a pattern as a string, because a pattern read as text says what it looks like while a match says what the backend will do with it

#### Scenario: The public store also fails the existing catch-all check

- **WHEN** a rule is written that would reach the public store
- **THEN** it fails both this check and the catch-all check
- **AND** the reason for two checks rather than one is recorded: one asks whether a rule reaches the public store and the other whether a rule reaches anywhere nothing is placed

### Requirement: A public output is readable at evaluation and a secret one is not

The declaration surface SHALL expose a path accessor for every output and a value accessor only for public ones, and reaching for a value on a secret output SHALL fail with a sentence naming the entry and the accessor to use instead.

#### Scenario: The path accessor is available for both

- **WHEN** the path accessor is read for a secret output and for a public one
- **THEN** both yield a path
- **AND** neither yields a value

#### Scenario: The value accessor reads the file

- **WHEN** the value accessor is read for a public output that has been generated
- **THEN** it yields the file's contents at evaluation time

#### Scenario: An ungenerated public output names the command to run

- **WHEN** the value accessor is read for a public output that has not been generated
- **THEN** evaluation fails
- **AND** the failure names the command that would produce it

#### Scenario: A secret output's value accessor refuses by name

- **WHEN** the value accessor is read for a secret output
- **THEN** evaluation fails with a sentence naming the entry, stating that it is secret, and pointing at the path accessor
- **AND** the reason for a stated refusal rather than an absent option is recorded: the likeliest authoring mistake in this surface is reaching for a value on a secret because the sibling public output has one
