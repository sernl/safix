## MODIFIED Requirements

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
