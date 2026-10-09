# Spec Delta

## ADDED Requirements

### Requirement: The drift report judges a public output by its own file

The drift report SHALL answer "does this entry hold a value" for an output declared not secret from the plaintext file the resolver computes for it, never from the encrypted document its placement also names.
That file SHALL be read from the path the resolver emits for the entry, which is the opaque destination when a vault is declared and the readable one otherwise, so that the reading is correct in both naming modes without the runtime computing a path.
A public output holding a non-empty file SHALL produce no finding.
A public output holding nothing SHALL produce a finding naming that file and the entry the generator writing it is declared on, whose remedy is the generator run, and the report SHALL NOT offer to have the value typed.

#### Scenario: A minted public output is not a finding

- **WHEN** the report runs over a public output whose file holds bytes
- **THEN** it reports nothing for that entry
- **AND** it reports nothing about the encrypted document the entry's placement also names, which was never written for it

#### Scenario: An unminted public output names the run that writes it

- **WHEN** the report runs over a public output whose file is absent or empty
- **THEN** the finding names that file rather than the encrypted document
- **AND** the remedy is the generator run, named for the entry the generator is declared on
- **AND** no remedy offers to have the value typed, because a public output is only ever written by a generator

#### Scenario: The generator is declared on another entry

- **WHEN** a public output's own entry carries no generator, the generator being declared on the sibling entry that also writes it
- **THEN** the finding still names that sibling
- **AND** the absence of a generator on the entry itself is not read as "nothing mints this"
