## Purpose

The runtime gains an installer: a manifest schema, an identity-assembly step, a decryption step, and a filesystem sequence that mounts, writes, chowns and symlinks.
Three of this capability's requirements move as a result.
The cryptographic-authority requirement gains the installer's decrypt path, which is where the largest new volume of decryption lives.
The unsafe-code requirement gains the mount syscall, which is the first privileged syscall the runtime makes and the first opportunity to weaken that rule.
And the completeness of the external-binary overrides becomes a requirement, because the installer needs the two tools that did not have one.

## MODIFIED Requirements

### Requirement: The cryptographic backend stays the authority

The runtime SHALL perform every encryption, decryption, re-wrap and metadata read by invoking the backend as a subprocess, and SHALL NOT reimplement its file format, its message authentication, its initialization-vector reuse rule, or its key wrapping.
This SHALL hold for the installer as well as for the operator-facing verbs.

#### Scenario: What the runtime executes

- **WHEN** any ciphertext is produced or consumed
- **THEN** the backend binary does it
- **AND** the binary is pinned into the package's closure rather than taken from the caller's path

#### Scenario: What the runtime parses

- **WHEN** the runtime reads a ciphertext file directly
- **THEN** it reads only the metadata fields the existing readers read
- **AND** it derives nothing cryptographic from them

#### Scenario: The installer decrypts per document, not per entry

- **WHEN** the installer decrypts the documents a manifest names
- **THEN** it invokes the backend once per distinct document and extracts each declared key from the result in memory
- **AND** the reason is recorded: a manifest's entries share documents by construction, since one audience gets one file, so per-entry invocation would multiply subprocesses by entry count for no additional isolation

#### Scenario: A decrypted value never leaves the value type

- **WHEN** a decrypted document is held, keys are extracted from it, and each value is written into the store
- **THEN** every value stays inside the runtime's non-renderable value type until the write
- **AND** none of them reaches an argument vector or an environment variable at any point, which is the same discipline the operator-facing verbs are already held to

#### Scenario: The identity the backend uses is assembled, not reimplemented

- **WHEN** the installer prepares the identity the backend will decrypt with
- **THEN** it writes a key file at owner-only permissions and names it to the backend through the backend's own environment variable
- **AND** the ssh-key-to-age conversion it needs is a subprocess of the upstream tool rather than an in-process implementation, so no key-derivation arithmetic enters this workspace

### Requirement: Every crate forbids unsafe code

Each crate in the workspace SHALL declare `#![forbid(unsafe_code)]`, including the crate that performs the installer's privileged filesystem work.

#### Scenario: The attribute is present and is not a lint

- **WHEN** each crate's root module is read
- **THEN** it declares the forbidding attribute
- **AND** the attribute is one no inner scope can relax, so a call site cannot opt out of it

#### Scenario: A syscall the standard library does not expose goes through a safe wrapper

- **WHEN** the installer mounts its store's filesystem, or reads a mounted filesystem's type
- **THEN** it does so through a crate exposing a safe interface to that syscall
- **AND** the forbid declaration stands unweakened, so a syscall's arrival is not an occasion to make an exception

## ADDED Requirements

### Requirement: The installer manifest is one schema definition in the library

The manifest the installer reads SHALL be defined once, as a deserializable type in the library crate, rejecting unknown fields and carrying a schema version.
No second definition of its shape SHALL exist in the runtime.

#### Scenario: The command adds no schema

- **WHEN** the command crate is searched for a manifest definition
- **THEN** it holds none: it parses the manifest path and the flags and hands both to the library
- **AND** the installer's whole decision surface is therefore reachable by a library-level test with no terminal and no argument vector

#### Scenario: An unknown field is a refusal

- **WHEN** the library deserializes a manifest carrying a field the type does not declare
- **THEN** it refuses naming the field
- **AND** the reason is recorded: silently ignoring an unknown field is how a schema change becomes a wrong installation instead of a message

### Requirement: Every external program the runtime invokes is selectable by a named variable

Each external program the runtime invokes SHALL be selectable through a named environment variable, so a hermetic check can substitute a build of its own, and the set of such programs and the set of such variables SHALL be the same size.

#### Scenario: The set is complete

- **WHEN** the external programs the runtime invokes are enumerated against the environment variables that select them
- **THEN** each program has one
- **AND** the two that previously had none — the age key generator, spawned by a hardcoded program name, and the ssh-key converter the installer's identity assembly needs — now do

#### Scenario: The variable is read, not merely declared

- **WHEN** a check exercises the key generator or the ssh-key converter
- **THEN** it drives the behaviour by pointing that program's variable at a build of its own
- **AND** the check fails if the variable is ignored, which is what makes the override evidence rather than documentation
