## Purpose

The shape of the rust runtime: what is a library and what is a command, what the runtime is allowed to reimplement and what it must delegate, how a plaintext value is represented so that the bash hazards it replaces are absent constructions rather than avoided ones, how a refusal travels from the library to a terminal, and where concurrency is permitted.

## ADDED Requirements

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

Each crate in the workspace SHALL declare `#![forbid(unsafe_code)]`.

#### Scenario: The attribute is present and is not a lint

- **WHEN** each crate's root module is read
- **THEN** it declares the forbidding attribute
- **AND** the attribute is one no inner scope can relax, so a call site cannot opt out of it

### Requirement: The evaluation seam is preserved

The command SHALL obtain placements, audiences, governed files and policy text by evaluating the nix half, and SHALL NOT reimplement resolution, the type vocabulary, or the recipient policy renderer.

#### Scenario: What the runtime asks nix for

- **WHEN** the runtime needs a resolved placement, an audience, the governed file set, or the policy text
- **THEN** it obtains it by evaluating the declarations the consumer's flake carries
- **AND** the request is the same one the shell runtime makes

#### Scenario: What the runtime does not compute

- **WHEN** the runtime's own code is searched for resolution, audience derivation or policy rendering
- **THEN** none is found
- **AND** the reason is recorded: the nix half is the consumer-facing option surface and is checked by evaluation

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

#### Scenario: The value's path to the backend

- **WHEN** a value is written or read through the backend
- **THEN** it travels on a piped standard input or standard output
- **AND** the child's argument vector and environment contain no plaintext

#### Scenario: The descriptor is explicit at the call site

- **WHEN** any child process is spawned
- **THEN** each of its three standard descriptors is set explicitly
- **AND** none is inherited by omission

### Requirement: Refusals are library data rendered at the command edge

Every refusal SHALL be a variant of the library's error type carrying the data its message needs, and the command SHALL be the only place a refusal is turned into text for a terminal.

#### Scenario: A refusal carries data, not prose

- **WHEN** a refusal variant is inspected
- **THEN** it carries the values its message interpolates — the file and both recipient sets for drift, the name and the declared users for an unknown user, the path for a missing creation rule, the participating nodes for a cycle, the identity paths for a machine that cannot decrypt
- **AND** an embedder can act on those values without parsing a message

#### Scenario: Rendering is pinned

- **WHEN** the command renders a refusal
- **THEN** the rendering is held by a snapshot for that variant
- **AND** a change to the wording changes the snapshot rather than passing silently

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

The runtime SHALL perform every encryption, decryption, re-wrap and metadata read by invoking the backend as a subprocess, and SHALL NOT reimplement its file format, its message authentication, its initialization-vector reuse rule, or its key wrapping.

#### Scenario: What the runtime executes

- **WHEN** any ciphertext is produced or consumed
- **THEN** the backend binary does it
- **AND** the binary is pinned into the package's closure rather than taken from the caller's path

#### Scenario: What the runtime parses

- **WHEN** the runtime reads a ciphertext file directly
- **THEN** it reads only the metadata fields the existing readers read
- **AND** it derives nothing cryptographic from them
