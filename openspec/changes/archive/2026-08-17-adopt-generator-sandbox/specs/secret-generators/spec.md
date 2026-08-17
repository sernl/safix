## Purpose

The execution envelope joins the executor contract: what a fragment may reach while it holds plaintext becomes part of what `secret-generators` promises, rather than a property of whichever machine runs it.

## ADDED Requirements

### Requirement: A fragment runs inside the envelope by default

A generator's script and its validation fragments SHALL run inside a sandbox in which the staging root is the only writable path, the nix store is readable, and the network is absent.
The envelope SHALL match the one clan's default executor applies, so that a fragment written against the shared interface runs under either system's default executor without modification.

#### Scenario: A write outside the staging root fails

- **WHEN** a fragment writes to a path outside its staging root
- **THEN** the write fails inside the envelope
- **AND** the run refuses with that fragment's own failure rather than storing a partial result

#### Scenario: The network is absent

- **WHEN** a fragment with no declared escape attempts a network connection
- **THEN** the attempt fails
- **AND** no traffic leaves the envelope

#### Scenario: The declared tools still run

- **WHEN** a fragment invokes a tool its generator's `runtimeInputs` names
- **THEN** the tool runs from the read-only store

#### Scenario: The pipe legs are unchanged

- **WHEN** a validation fragment judges a candidate value
- **THEN** the candidate arrives on standard input exactly as it does outside the envelope
- **AND** a prompted answer still arrives as a file under the prompts directory

### Requirement: The network is granted by declaration and by nothing else

A generator SHALL reach the network only when its own declaration says so.
The declaration SHALL re-share the network and nothing else: the filesystem confinement of the envelope stays in force.
An invocation-level switch for suspending the envelope SHALL NOT exist.

#### Scenario: The declared escape opens the network only

- **WHEN** a generator whose declaration grants the network runs
- **THEN** the fragment reaches the network
- **AND** a write outside the staging root still fails

#### Scenario: No flag suspends the envelope

- **WHEN** an invocation passes `--no-sandbox` or any equivalent
- **THEN** the invocation is refused as carrying an unknown flag

#### Scenario: The audit reads the tree

- **WHEN** an operator asks which generators may reach the network
- **THEN** the declarations answer the question at evaluation, with no runtime consulted

### Requirement: Generation refuses without a sandbox backend

When no sandbox backend is available, generation SHALL refuse before any fragment runs, naming the backend it looked for and what supplies it.
The refusal SHALL NOT be convertible into an unsandboxed run.

#### Scenario: The refusal precedes every fragment

- **WHEN** generation starts on a machine where the platform's backend is unavailable
- **THEN** it refuses before the first fragment runs
- **AND** the refusal names the missing backend and its remedy

#### Scenario: An unsupported platform is told so

- **WHEN** generation starts on a platform with no backend at all
- **THEN** the refusal says the platform has no envelope
- **AND** nothing runs unsandboxed in its place

## MODIFIED Requirements

### Requirement: A generated value travels a pipe where a pipe is possible, and a private staging file where it is not

A generated value SHALL NOT reach an argument vector or an environment variable at any point. It SHALL reach a file only inside the private staging root that `plaintext-staging` governs, and SHALL travel a pipe on every other leg of its journey.

#### Scenario: The write leg is still a pipe

- **WHEN** a produced value is handed to the encrypting backend
- **THEN** it travels a pipe
- **AND** it does not appear in that process's argument vector or environment

#### Scenario: The staging root is the only filesystem the value reaches

- **WHEN** the filesystem locations a value occupies during a run are enumerated
- **THEN** each is inside the private staging root
- **AND** none is outside it

#### Scenario: The exception is named rather than implied

- **WHEN** this requirement is read
- **THEN** it states that the generator contract is a filesystem contract and that the earlier absolute no longer holds
- **AND** it names what stands in its place

#### Scenario: What the runtime does not control

- **WHEN** a generator whose declaration grants the network moves a value over the connection the declaration opens
- **THEN** that movement is outside what the runtime shreds or observes
- **AND** the interface documentation states this beside the declaration, rather than disclaiming containment on the default path the envelope now enforces
