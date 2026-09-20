# Spec Delta

## MODIFIED Requirements

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
