# rust-runtime Specification

## Purpose

The shape of the rust runtime: what is a library and what is a command, what the runtime is allowed to reimplement and what it must delegate, how a plaintext value is represented so that the bash hazards it replaces are absent constructions rather than avoided ones, how a refusal travels from the library to a terminal, and where concurrency is permitted.

## Requirements

### Requirement: The runtime is an embeddable library with a thin command over it

The runtime SHALL be published as a library crate carrying the domain types, the drivers, the drift logic, generator execution and placement consumption, and a command crate carrying argument parsing, terminal interaction and rendering, and nothing else.

#### Scenario: The library runs without a terminal

- **WHEN** any runtime behaviour is exercised by a test
- **THEN** it is reachable through the library crate alone
- **AND** no terminal, no argument vector and no rendering is required to reach it

#### Scenario: The command adds no behaviour

- **WHEN** the command crate's contents are enumerated
- **THEN** each item parses arguments, interacts with the operator, or renders a result or a refusal
- **AND** no decision about custody, drift, ordering or writing is made there

#### Scenario: An embedder takes no rendering dependency

- **WHEN** the library crate's dependency graph is enumerated
- **THEN** the command's diagnostic renderer is absent from it

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

### Requirement: The evaluation seam is preserved

The command SHALL obtain placements, audiences, governed files and policy text by evaluating the nix half, and SHALL NOT reimplement resolution, the type vocabulary, or the recipient policy renderer.

#### Scenario: What the runtime asks nix for

- **WHEN** the runtime needs a resolved placement, an audience, the governed file set, or the policy text
- **THEN** it obtains it by evaluating the declarations the consumer's flake carries, or, when an entry file was named through `--entry` or `SAFIX_ENTRY`, the declarations that file carries
- **AND** the request is the same one the shell runtime makes, using the same attribute path either way

#### Scenario: What the runtime does not compute

- **WHEN** the runtime's own code is searched for resolution, audience derivation or policy rendering
- **THEN** none is found
- **AND** the reason is recorded: the nix half is the consumer-facing option surface and is checked by evaluation, regardless of whether that evaluation targets a flake or a named file

### Requirement: A plaintext value is a type that cannot be rendered, serialized or logged

A plaintext secret SHALL be represented by a dedicated type that zeroes its contents when dropped and that implements none of the debug, display or serialization traits.

#### Scenario: The traits are absent

- **WHEN** the type is probed at compile time for the debug, display and serialization traits
- **THEN** each probe reports the trait absent
- **AND** the probe is compiled rather than described, so adding any of those traits later fails the build

#### Scenario: The value is gone when the binding is

- **WHEN** a value of the type is dropped
- **THEN** its buffer is zeroed

#### Scenario: What the absence replaces

- **WHEN** the reason for the absent traits is recorded
- **THEN** it names the shell hazard it removes: in the shell a value is a string and can be spelled into a format, a log, an argument or a temporary file with no diagnostic
- **AND** it states that the corresponding mistakes here fail to compile rather than being avoided by convention

### Requirement: A plaintext value is constructible only by reading a stream

The type SHALL provide construction from a readable stream only, and SHALL NOT provide construction from a string, a string slice, an argument, or an environment variable.

#### Scenario: The only door

- **WHEN** the type's constructors are enumerated
- **THEN** each takes a stream to read from
- **AND** no constructor takes an owned or borrowed string

#### Scenario: A read that fails carries no partial value

- **WHEN** reading the stream fails partway
- **THEN** construction fails
- **AND** whatever had been read is zeroed rather than returned

### Requirement: No plaintext value reaches a child process except through a pipe

Every invocation of the cryptographic backend that carries a plaintext value SHALL pass it on a pipe, and no plaintext value SHALL be placed in the argument vector or the environment of any child process.
A declared field resolved out of another secret entry is a plaintext value in the sense this requirement governs, and a declared field written as a literal in an evaluated declaration is not, because that declaration is already world-readable.
A target whose only channel for a field is an argument vector SHALL therefore refuse a field resolved out of an entry rather than place it there, and the refusal SHALL be a variant of the library's error type naming the target and the field.

#### Scenario: The value's path to the backend

- **WHEN** a value is written or read through the backend
- **THEN** it travels on a piped standard input or standard output
- **AND** the child's argument vector and environment contain no plaintext

#### Scenario: The descriptor is explicit at the call site

- **WHEN** any child process is spawned
- **THEN** each of its three standard descriptors is set explicitly
- **AND** none is inherited by omission

#### Scenario: A field resolved out of an entry never reaches an argument vector

- **WHEN** a declared field is resolved out of another entry and the target's only channel for that field is an argument vector
- **THEN** the run refuses, naming the target and the field
- **AND** the refusal is reachable before any child process is spawned for that mapping

#### Scenario: The two provenances are distinguishable by type rather than by care

- **WHEN** a resolved field is inspected
- **THEN** a field resolved out of an entry is held by the type that cannot be rendered, serialized or turned into a string, and a literal field is held as an ordinary string
- **AND** placing a resolved field in an argument vector is therefore not expressible, rather than being permitted and avoided

### Requirement: Refusals are library data rendered at the command edge

Every refusal SHALL be a variant of the library's error type carrying the data its message needs, and the command SHALL be the only place a refusal is turned into text for a terminal.

#### Scenario: A refusal carries data, not prose

- **WHEN** a refusal variant is inspected
- **THEN** it carries the values its message interpolates — the file and both recipient sets for drift, the name and the declared users for an unknown user, the path for a missing creation rule, the participating nodes for a cycle, the identity paths for a machine that cannot decrypt
- **AND** an embedder can act on those values without parsing a message

#### Scenario: Refusal identity is stable without freezing prose

- **WHEN** the command renders a refusal
- **THEN** its diagnostic includes the stable refusal code and the public context needed to act on it
- **AND** diagnostic wording may improve without changing the code or exposing secret payloads

#### Scenario: One refusal, one message

- **WHEN** the same refusal is reached through the library and through the command
- **THEN** the message is the same string
- **AND** it is produced once rather than written twice

### Requirement: Concurrency is bounded and confined to the fan-outs the shell already has

Asynchronous execution SHALL appear only where work the shell runtime already fanned out still fans out in this runtime, bounded by a limit on concurrent work; every other path SHALL be sequential.

#### Scenario: Where concurrency is permitted

- **WHEN** the runtime's concurrent regions are enumerated
- **THEN** each is bounded rather than spawning one task per item
- **AND** a fan-out that did not survive the port is withdrawn in the design, with the reason stated where a reader meets the sequential path

#### Scenario: Why writing stays sequential

- **WHEN** the sequencing of a write is documented
- **THEN** it records that a flake evaluation reads the files version control knows about, so staging must precede regeneration and regeneration must precede committing
- **AND** it states that interleaving two writers through that sequence produces a policy matching neither

### Requirement: The cryptographic backend stays the authority

The runtime SHALL perform encryption, decryption and rewrapping through upstream age, SOPS or GnuPG subprocesses. It MAY inspect public metadata and ciphertext structure, but SHALL NOT reimplement cryptographic file encoding, message authentication, initialization-vector reuse or key wrapping.
This SHALL hold for the installer as well as for the operator-facing verbs.

#### Scenario: What the runtime executes

- **WHEN** any ciphertext is produced or consumed
- **THEN** the backend binary does it
- **AND** the binary is pinned into the package's closure rather than taken from the caller's path

#### Scenario: What the runtime parses

- **WHEN** the runtime reads a ciphertext file directly
- **THEN** it reads only public recipient metadata, encryption policy and encrypted value shapes
- **AND** it derives nothing cryptographic from them

#### Scenario: The installer decrypts per document, not per entry

- **WHEN** the installer decrypts the documents a manifest names
- **THEN** it shares decryption across entries requesting the same document and representation
- **AND** whole-document byte output remains separate from the parsed representation used for keyed extraction

#### Scenario: A decrypted value never leaves the value type

- **WHEN** a decrypted document is held, keys are extracted from it, and each value is written into the store
- **THEN** every value stays inside the runtime's non-renderable value type until the write
- **AND** none of them reaches an argument vector or an environment variable at any point, which is the same discipline the operator-facing verbs are already held to

#### Scenario: The identity the backend uses is assembled, not reimplemented

- **WHEN** the installer prepares the identity the backend will decrypt with
- **THEN** configured age identities, SSH private keys and GnuPG homes are passed through the upstream tool's identity mechanisms
- **AND** any assembled private key file has owner-only permissions
- **AND** SSH-to-age conversion, where applicable, invokes the upstream converter rather than reimplementing key derivation

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
A program no check of this repository may ever run for real SHALL still have its variable, because the variable is the seam a stand-in stands in.

#### Scenario: The set is complete

- **WHEN** the external programs the runtime invokes are enumerated against the environment variables that select them
- **THEN** each program has one
- **AND** the two that previously had none — the age key generator, spawned by a hardcoded program name, and the ssh-key converter the installer's identity assembly needs — now do

#### Scenario: The variable is read, not merely declared

- **WHEN** a check exercises the key generator or the ssh-key converter
- **THEN** it drives the behaviour by pointing that program's variable at a build of its own
- **AND** the check fails if the variable is ignored, which is what makes the override evidence rather than documentation

#### Scenario: A program that is never run for real still has its variable

- **WHEN** the 1Password command is invoked by the sync and audit paths
- **THEN** it is located through its own named variable, defaulting to the program's ordinary name on the operator's path
- **AND** every check of that target drives the variable at a stand-in, which is the only way the target is exercised at all, because the real package is unfree and enters no check, package or development shell of this flake
