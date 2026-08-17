## Purpose

A minted value gains a committed record of the definition that minted it, so that a later edit to the declaration is detectable instead of silent.

## ADDED Requirements

### Requirement: Minting records the definition it minted under

A generated value SHALL be accompanied by a committed, plaintext record carrying a digest of the generator definition that produced it, written in the same commit as the value and refreshed whenever the value is regenerated.
The digest SHALL be computed over the definition alone; no value and no derivative of a value appears in the record.

#### Scenario: The record rides the mint's own commit

- **WHEN** a generator mints or regenerates a value
- **THEN** the definition record is written in the same commit as the value
- **AND** a mint interrupted before the commit leaves neither

#### Scenario: The record is about the definition, never the value

- **WHEN** the record's content is inspected
- **THEN** it derives from the generator's declaration alone
- **AND** two mints of different values under one definition produce the same record

#### Scenario: The record does not live where its meaning would lie

- **WHEN** the record's path is inspected
- **THEN** it is not under a path that means everything below it is encrypted
- **AND** not under a path that means declared public outputs
