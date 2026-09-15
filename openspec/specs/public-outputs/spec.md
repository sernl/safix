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

Public outputs SHALL be stored under a root distinct from the one holding encrypted material, SHALL NOT be nested inside it, and SHALL NOT contain it.
The non-overlap of the three storage roots SHALL be refused at evaluation rather than assumed of their default values: no two roots may be equal, and no root may be a prefix of another at a path-component boundary.

#### Scenario: The two trees do not overlap

- **WHEN** the public store's root and the encrypted tree's root are compared
- **THEN** neither is a prefix of the other

#### Scenario: An overlapping configuration is refused at evaluation

- **WHEN** two storage roots are set equal, or one is set to a path-component prefix of another
- **THEN** evaluation fails
- **AND** the failure names both options and both values

#### Scenario: Overlap is judged on path components, not on raw string prefixes

- **WHEN** one root's string is a prefix of another's but does not end at a path-component boundary
- **THEN** the two are accepted, because neither directory contains the other

#### Scenario: A prefix-scoped rule can address exactly one of them

- **WHEN** an exclusion, a backup policy or a search is scoped to the encrypted tree's root
- **THEN** no plaintext output is inside its scope

#### Scenario: The reason is recorded

- **WHEN** the location decision is documented
- **THEN** it states that the tree holding ciphertext must be named for that, so that everything under it is encrypted without qualification, and that the naming is the consumer's once the root is theirs to set
- **AND** it records that what remains mechanically enforced, rather than carried by a name, is that the trees do not overlap
- **AND** it records the alternatives that were refused and why

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
The paths a check probes SHALL be derived from the configured storage roots, so that renaming a root moves the probes with the trees rather than leaving the check probing locations nothing uses.

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

#### Scenario: A renamed root moves the probes

- **WHEN** the storage roots are renamed and a rule is written that would reach a renamed tree
- **THEN** both checks fail
- **AND** a check whose probes were left at the default spellings would not, which is why the probes are derived

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

### Requirement: The three storage roots are declared, defaulted, and documented

The declaration surface SHALL carry one option per storage tree — the encrypted tree, the plaintext-output tree, and the generator-record tree — each a repository-relative path whose default is the spelling that tree had before the option existed.
Each option's description SHALL state what its tree holds and what a reader may assume about it, without requiring the reader to consult any other document.

#### Scenario: Three independent options, one per tree

- **WHEN** the declaration surface is read
- **THEN** it carries one option for the encrypted tree, one for the plaintext-output tree, and one for the generator-record tree
- **AND** each may be set independently of the other two

#### Scenario: The defaults preserve the previous layout exactly

- **WHEN** no storage option is set
- **THEN** every ciphertext path, plaintext-output path, generator-record path, generated creation rule and resolved `sopsFile` is byte-identical to the one the same declarations produced before the options existed

#### Scenario: A consumer collects all three under one parent

- **WHEN** a consumer sets all three roots to siblings under one directory
- **THEN** every path the resolver computes falls under that directory
- **AND** one ignore entry, one backup rule and one directory move cover everything this package writes

#### Scenario: A tree is placed where the others are not

- **WHEN** a consumer sets the plaintext-output root to a location outside the parent holding the other two
- **THEN** plaintext outputs resolve there
- **AND** neither of the other two trees moves

#### Scenario: Each description stands on its own

- **WHEN** an option's description is read
- **THEN** it states what the tree holds and why it is separate from the other two
- **AND** the encrypted tree's description states that everything under it is ciphertext without qualification, because that is the sentence a backup policy is written against
- **AND** the documentation records that a root beginning with a dot is skipped by common search tools' defaults, so an audit of such a tree must ask for hidden files

### Requirement: A malformed storage root is refused at evaluation

A storage root that is empty, absolute, contains a parent-directory component, or ends in a path separator SHALL be refused at evaluation, naming the option and the value.

#### Scenario: An absolute root is refused

- **WHEN** a storage root is set to an absolute path
- **THEN** evaluation fails naming that option and its value

#### Scenario: A root escaping the repository is refused

- **WHEN** a storage root contains a parent-directory component
- **THEN** evaluation fails naming that option and its value

#### Scenario: An empty or trailing-separator root is refused rather than normalised

- **WHEN** a storage root is empty, or ends in a path separator
- **THEN** evaluation fails naming that option and its value
- **AND** the value is not silently normalised, because a normalised value would make the overlap comparison depend on a rewrite the operator cannot see
