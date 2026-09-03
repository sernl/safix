## Purpose

Where the secrets vault lives when it is a separate repository from the one declaring users and the catalogue: the two-root model the command-line runtime carries, what lands at which root, how a write spanning both roots lands safely, and how every vault-rooted name is made opaque so a vault host or a vault-only reader learns none of it.

## ADDED Requirements

### Requirement: The vault is declared as an optional, naming-keyed root

`flake.safix.vault` SHALL be typed `nullOr (submodule { root; namingKey; })` and SHALL default to `null`.
`root` SHALL be a path; every audience file's `sopsFile` SHALL resolve rooted at it when the vault is set, and rooted at the declaring flake's own source when it is not.
`namingKey` SHALL be a string of at least 64 lowercase hexadecimal characters, minted by the operator once per vault.

#### Scenario: No vault declared

- **WHEN** a consumer sets no value for `flake.safix.vault`
- **THEN** every audience file resolves exactly where it resolves today, rooted at the declaring flake's own source
- **AND** every physical name is exactly today's readable name

#### Scenario: A vault declared

- **WHEN** a consumer sets `flake.safix.vault` to a `root` and a well-formed `namingKey`
- **THEN** every audience file resolves rooted at that `root` instead
- **AND** the physical name under that root is opaque rather than the readable relative name the audience would derive without a vault, per "Every vault-rooted name is opaque" below

### Requirement: A vault without a well-formed naming key is refused

Evaluation SHALL refuse a declared vault whose `namingKey` is absent, shorter than 64 characters, or contains a character outside `[0-9a-f]`, before resolving any path.
The refusal SHALL name the option and state the requirement the supplied value fails.

#### Scenario: No naming key

- **WHEN** `flake.safix.vault.root` is set and `namingKey` is not
- **THEN** evaluation refuses, naming `flake.safix.vault.namingKey` and stating that a vault requires one

#### Scenario: A naming key shorter than 64 characters

- **WHEN** `namingKey` is a hexadecimal string of fewer than 64 characters
- **THEN** evaluation refuses, naming the option and stating the minimum length

#### Scenario: A naming key containing a non-hexadecimal character

- **WHEN** `namingKey` contains a character outside `[0-9a-f]`
- **THEN** evaluation refuses, naming the option and stating the required alphabet

#### Scenario: A well-formed naming key is accepted

- **WHEN** `namingKey` is at least 64 characters, every one of them in `[0-9a-f]`
- **THEN** evaluation proceeds and every vault-rooted name is derived from it

### Requirement: The runtime resolves two independent repository roots

The command-line runtime SHALL resolve a declaration root, from the repository it runs inside, and a vault root, named by the operator, independently of each other.
Where no vault is declared, the vault root SHALL equal the declaration root.

#### Scenario: Single-repo default

- **WHEN** `flake.safix.vault` is unset and the operator names no vault root
- **THEN** the runtime resolves one root for both declarations and ciphertext, exactly as today

#### Scenario: A vault declared and a root named

- **WHEN** `flake.safix.vault` is set and the operator names a vault root matching its `root`
- **THEN** the runtime resolves two roots, and every operation reads and writes each artifact at the root that governs it

#### Scenario: Declared in nix, unnamed at the command line

- **WHEN** `flake.safix.vault` is set and the operator names no vault root
- **THEN** the runtime refuses before evaluating or writing anything, naming the option and the environment variable that supplies the root

#### Scenario: Named at the command line, undeclared in nix

- **WHEN** the operator names a vault root and `flake.safix.vault` is unset
- **THEN** the runtime refuses, naming the option that must be declared or the environment variable that must be unset

### Requirement: The vault root is a git repository

The vault root SHALL be the top level of a git repository.
The command SHALL refuse before writing anything when it is not.

#### Scenario: A vault that is a repository

- **WHEN** the named vault root is the top level of a git repository
- **THEN** every operation proceeds exactly as it does against the declaration root today

#### Scenario: A vault directory that is not a repository

- **WHEN** the named vault root holds no git repository
- **THEN** the runtime refuses before writing anything, naming the path and stating that it must be a git repository's top level

#### Scenario: A vault root that is a subdirectory of some other repository

- **WHEN** the named vault root is inside a git repository but is not that repository's top level
- **THEN** the runtime refuses, naming the path found by git and the path named by the operator, and stating that the two must agree

### Requirement: Every vault-rooted name is opaque

When a vault is declared, every ciphertext document, public output, generator definition record, and in-document key name the vault holds SHALL be derived from the naming key, a use-specific tag, and the corresponding logical name, rather than from the logical name alone.
No vault-rooted physical name or directory listing SHALL disclose an audience's membership, a secret's name, or a definition record's owner.

#### Scenario: A ciphertext document's name is opaque

- **WHEN** a secret's audience resolves to a file in vault mode
- **THEN** the file's name is a hash of the naming key and the audience's readable identity, not the audience's members or their count
- **AND** the file sits one level under the vault's ciphertext prefix, never nested in an audience-named directory

#### Scenario: A public output's path is opaque

- **WHEN** a public output resolves to a path in vault mode
- **THEN** the path is a hash of the naming key and the output's readable identity, held as a single file rather than a `<name>/value` directory

#### Scenario: A definition record's path is opaque

- **WHEN** a generator's definition record resolves to a path in vault mode
- **THEN** the path is a hash of the naming key and the record's readable owner-or-audience identity, one level under the vault's state prefix

#### Scenario: A document's key name is opaque

- **WHEN** a secret's value is written into a vault-mode document
- **THEN** the top-level key it is stored under is a hash of the naming key, the document's readable identity, and the secret's readable name
- **AND** this holds whether or not the entry declares a custom key name

#### Scenario: No vault, no opacity

- **WHEN** no vault is declared
- **THEN** every ciphertext document, public output, definition record, and in-document key keeps exactly today's readable name

### Requirement: What lands at the vault root and what stays at the declaration root

The runtime SHALL write catalogue, user, and group declarations, and every scaffold it generates for them, at the declaration root.
It SHALL write ciphertext, generated public values, and generator definition records at the vault root, under the opaque names the previous requirement derives.
The recipient policy SHALL NOT be written at the vault root; see "The recipient policy stays at the declaration root" below.
Git authorship for every commit the runtime makes SHALL be read from the declaration root, regardless of which root the commit's content lands in.

#### Scenario: A new user's scaffold lands at the declaration root

- **WHEN** an operator scaffolds a new user
- **THEN** the generated declaration file is written, staged, and committed at the declaration root

#### Scenario: A newly set secret's ciphertext lands at the vault root

- **WHEN** an operator sets a secret's value
- **THEN** the encrypted document is written, staged, and committed at the vault root, under its opaque name
- **AND** nothing is written at the declaration root for that operation

#### Scenario: Authorship is always the declaration root's

- **WHEN** a commit lands at the vault root
- **THEN** the identity that authors it is resolved from the declaration root's git configuration, not the vault's

### Requirement: The recipient policy stays at the declaration root

`.sops.yaml` SHALL be committed at the declaration root whether or not a vault is declared.
For a command that needs creation rules to reach a vault-rooted document, the runtime SHALL render a disposable copy of the rules into the vault working tree, pass it to the encrypting backend explicitly, and never commit it.

#### Scenario: The committed policy is unmoved by a vault

- **WHEN** a vault is declared
- **THEN** `.sops.yaml` is written, read, and drift-checked at the declaration root exactly as it is with no vault declared

#### Scenario: Creation rules for a vault-rooted document are rendered to a scratch file

- **WHEN** a value is set for the first time or a file's recipients are re-wrapped, against a vault-rooted document
- **THEN** the runtime renders that document's creation rules into a file inside the vault working tree and passes it to the encrypting backend explicitly, naming no anchor and no audience in prose

#### Scenario: The scratch rules file is never committed

- **WHEN** the scratch rules file exists at any point during a run
- **THEN** the vault's own `.gitignore` covers its name, so it cannot be staged even if a run ends between writing it and sweeping it

#### Scenario: The scratch rules file is swept on every exit path

- **WHEN** a run that rendered the scratch rules file ends, by return, by error, by panic, or by signal
- **THEN** the file is removed

#### Scenario: Reading and setting a value need no rules

- **WHEN** a value is decrypted or set into an already-encrypted vault-rooted document
- **THEN** the operation proceeds with no scratch rules file rendered

### Requirement: A write spanning both roots commits the vault first

Where one operation writes to both roots, the vault root's commit SHALL land before the declaration root's commit.
The declaration root's commit SHALL name the vault commit it follows.

#### Scenario: A single-root operation commits once

- **WHEN** an operation writes to only one root
- **THEN** exactly one commit lands, at that root

#### Scenario: A two-root operation commits vault, then declaration

- **WHEN** an operation writes to both roots
- **THEN** the vault root's commit lands first
- **AND** the declaration root's commit lands second and names the vault commit's identifier

#### Scenario: The order is stated as a safety property, not a convention

- **WHEN** the reason for vault-first ordering is documented
- **THEN** it states that the declaring flake must never claim a custody grant the vault's committed policy has not yet been re-wrapped for, and that the reverse — a vault ahead of a declaration that has not landed — is inert rather than a custody gap

### Requirement: Both roots are checked clean before either is written

Before any file is written, the runtime SHALL confirm that neither root is mid a git operation, holds unresolved conflict entries on a path the operation will touch, or already has uncommitted changes on that path.
It SHALL refuse, naming which root and why, before writing to either.

#### Scenario: The vault is dirty

- **WHEN** the vault root fails the cleanliness check and the declaration root would pass it
- **THEN** the runtime refuses naming the vault root and the reason
- **AND** nothing is written at either root

#### Scenario: The declaration root is dirty

- **WHEN** the declaration root fails the cleanliness check and the vault root would pass it
- **THEN** the runtime refuses naming the declaration root and the reason
- **AND** nothing is written at either root

#### Scenario: Both are clean

- **WHEN** both roots pass the cleanliness check
- **THEN** writing proceeds in the order this capability states

### Requirement: A vault commit that lands without its declaration commit is safe to re-run

When the vault root's commit has landed and the declaration root's commit has not, the runtime SHALL report which root is ahead, name the vault commit, and state that re-running the same command completes the operation without repeating the vault write.

#### Scenario: The declaration commit fails after the vault commit lands

- **WHEN** an operation's vault-root commit succeeds and its declaration-root commit then fails
- **THEN** the runtime reports the vault commit's identifier and the files still pending at the declaration root
- **AND** the report states that re-running the command is the remedy

#### Scenario: Re-running completes the operation without repeating the vault write

- **WHEN** the same command is run again after the state in the prior scenario
- **THEN** the vault-root write reproduces identical content and stages nothing new, so no second vault commit is made
- **AND** the declaration-root commit proceeds and completes the operation

### Requirement: A vault commit discloses the lock bump it requires

After a command commits to the vault root, the runtime SHALL state that the change is not visible to any consuming build until the declaring flake's lock entry for the vault is updated.
It SHALL name the update command when it can determine which flake input the vault root corresponds to.

#### Scenario: The disclosure follows every vault-root commit

- **WHEN** a command commits to the vault root
- **THEN** its output states that a lock update is required before a consuming build sees the change

#### Scenario: The input name is determined

- **WHEN** the declaring flake's lock file names exactly one input whose locked source matches the vault root
- **THEN** the disclosure names that input and the exact command that updates it

#### Scenario: The input name cannot be determined

- **WHEN** the declaring flake's lock file names no input matching the vault root, or names more than one
- **THEN** the disclosure still states that a lock update is required, in terms general enough to remain true, without naming an input it cannot identify

### Requirement: What an opaque vault still reveals

The documentation of this capability SHALL state, rather than omit, what an opaque vault continues to disclose to its host and to a reader holding only the vault.

#### Scenario: Counts and sizes are unchanged

- **WHEN** the residuals are read
- **THEN** they state that the number of documents, the number of keys per document, and each leaf's ciphertext length are visible exactly as they are without a vault

#### Scenario: Recipient public keys are visible

- **WHEN** the residuals are read
- **THEN** they state that each document's age recipient public keys are visible in the clear, because hiding them is a stated non-goal

#### Scenario: The naming key is visible to anyone who can evaluate the declarations

- **WHEN** the residuals are read
- **THEN** they state that the naming key is an evaluation-time value in the declaring flake, so it — and therefore every name it derives — is visible to every local user of a machine that has the declaring flake in its nix store
- **AND** they state that opacity holds only against the vault host and a reader holding only the vault
