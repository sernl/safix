# Spec Delta

## MODIFIED Requirements

### Requirement: One command covers the lifecycle, by name and never by file

The safix command SHALL retain its existing lifecycle verbs and add migrate and identity custody operations. Ordinary secret operations SHALL continue to address declaration names. Installation, explicit migration plans and identity backup/restore SHALL address their input artifacts rather than inventing declaration names for them.

#### Scenario: The explicit migration boundary
- **WHEN** an operator invokes migration with a versioned plan
- **THEN** that plan names the source, destination, recipient and deployment contracts to verify
- **AND** ordinary set and get operations continue resolving files from names

#### Scenario: Addressing a secret

- **WHEN** an operator sets, reads, or generates a value
- **THEN** they name the secret
- **AND** the file and the key within it are resolved from the declarations

#### Scenario: Choosing is a way of naming

- **WHEN** an operator chooses an entry from those offered rather than typing its name
- **THEN** the run proceeds exactly as though the chosen name had been given as an argument
- **AND** no file is named at any point

#### Scenario: The subcommand set is closed

- **WHEN** an unrecognised subcommand is given
- **THEN** the command fails naming the subcommands it accepts
- **AND** that list includes every supported verb, including identity and migrate

#### Scenario: Runtime dependencies are pinned into the command

- **WHEN** the command runs
- **THEN** its default tool paths come from its own closure
- **AND** explicit upstream identity and GnuPG executable settings retain their documented precedence

#### Scenario: Installation names its manifest rather than a secret

- **WHEN** the help for `install` is read
- **THEN** it states that the manifest path is its argument because a manifest is a build product, not a declaration, and nothing in the declarations names it
- **AND** it states that the verb resolves no secret by name, so the by-name rule has nothing to apply to

### Requirement: Values move through pipes wherever a pipe remains possible

Secret and private-key payloads SHALL cross cryptographic subprocess boundaries through pipes, never command arguments or environment variables. Plaintext files SHALL be confined to protected generator/editor staging, private runtime installation state and explicitly selected owner-protected identity custody locations outside repositories and the Nix store.

#### Scenario: The stream-writing and reading verbs are unchanged

- **WHEN** a value is written from standard input or read to standard output
- **THEN** it travels a pipe end to end

#### Scenario: The encrypting backend is still driven by pipes

- **WHEN** the encrypting backend is invoked for any operation
- **THEN** the value reaches it on a pipe
- **AND** no invocation names a value in its arguments or environment

#### Scenario: The exception is bounded and named

- **WHEN** the exception to this requirement is read
- **THEN** it names generator and editor staging, runtime installation and protected local identity custody
- **AND** migration staging and published recovery artifacts contain ciphertext, not plaintext

#### Scenario: The change from the earlier absolute is stated

- **WHEN** this requirement is compared against the one it replaces
- **THEN** the difference is stated rather than presented as a clarification
- **AND** the reason is recorded: generators and editors need seekable files, installed services consume runtime paths, and cryptographic tools retain identities in protected local custody

### Requirement: The content half cannot alter the policy

Ordinary value-writing commands SHALL use only the declared audience and SHALL refuse inspectable SOPS recipient drift before mutation. They SHALL NOT add recipients to declarations. Raw age writes SHALL use the declared audience without claiming to verify the inaccessible previous recipient roster; explicit migration MAY name a different target audience and SHALL independently verify it.

#### Scenario: Setting a value grants nothing

- **WHEN** a value is written
- **THEN** the recipients used are those the file's own metadata or the committed policy already declares
- **AND** no run adds a recipient to the declared audience

#### Scenario: Policy changes go through one subcommand

- **WHEN** the recipient policy must change
- **THEN** the change comes from the declarations, applied by the reconciling subcommand
- **AND** that subcommand regenerates the policy before re-wrapping, never the other way round

## ADDED Requirements

### Requirement: Every value path preserves bytes or refuses before mutation

Safix SHALL preserve arbitrary bytes on its own secret storage, generator and installation paths. Intentional empty values SHALL remain distinguishable from missing values and placeholders. Text-only external destinations SHALL refuse values they cannot represent before changing the destination.

#### Scenario: A non-UTF-8 piped value
- **WHEN** set receives bytes 00 ff fe 41 0a
- **THEN** get and installation return exactly those bytes

#### Scenario: A text-only destination
- **WHEN** synchronization sends a non-UTF-8 value to a text-only API
- **THEN** it refuses before modifying the destination
