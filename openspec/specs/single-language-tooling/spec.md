# single-language-tooling Specification

## Purpose

What language safix's own tooling is written in, what the exemption for scripting means precisely enough to be decidable, and what the tree is permitted to contain once the shell and python runtimes are gone.

## Requirements

### Requirement: safix's tooling is rust

Every part of safix that stands between an operator's intent and a secret's ciphertext, or between a ciphertext and a claim about it, SHALL be written in rust.

#### Scenario: The tree holds no second runtime

- **WHEN** the repository is searched for an executable implementation of any subcommand
- **THEN** exactly one is found
- **AND** it is the rust binary the flake builds as `packages.safix`

#### Scenario: No package builds a non-rust tool

- **WHEN** the flake's package attributes are enumerated
- **THEN** none of them builds a shell script or a python program as a safix tool
- **AND** `packages.safix-sh` is absent

#### Scenario: The reader closure carries no python

- **WHEN** the runtime closure of `packages.safix` and of every safix check is enumerated
- **THEN** no python interpreter and no python library is present in it

### Requirement: The scripting exemption is decided by a stated test, not by file extension

Code claimed as scripting SHALL be judged by whether an operator's secret depends on it being right, and each survivor SHALL be recorded individually with the reason it passes that test.

#### Scenario: The test is applied to a candidate

- **WHEN** a shell or yaml fragment is proposed for retention
- **THEN** it is asked whether it reads plaintext, decides whether a write is refused, or states a claim a check asserts
- **AND** it is retained only if the answer to all three is no

#### Scenario: Every survivor is named

- **WHEN** the retained non-rust fragments are enumerated
- **THEN** each appears in this change's design with its own justification
- **AND** no survivor is covered only by a category-level exemption

#### Scenario: A fixture builder is scripting

- **WHEN** inline shell inside a check derivation assembles a fixture repository
- **THEN** it is scripting
- **AND** the reason is that it constructs the subject while the rust suite makes every claim about it, so a mistake in it fails the build loudly rather than weakening an assertion

#### Scenario: An operator's generator fragment is not safix's tooling

- **WHEN** the `script` and `validation` fragments a consumer declares on a generator are considered
- **THEN** they are outside this requirement
- **AND** the reason is that they are data safix executes on the operator's behalf, and rewriting them in rust would make safix a compiler for them

### Requirement: Migrated tooling is deleted rather than deprecated

Once a non-rust tool's behaviour exists in rust and the parity obligation for it is discharged, the non-rust tool SHALL be removed from the tree rather than retained under a deprecated name.

#### Scenario: Nothing is kept as an oracle

- **WHEN** the tree is searched after the retirement
- **THEN** no file exists whose stated purpose is to be compared against the rust runtime

#### Scenario: The evidence the retired gate produced is recorded where it belongs

- **WHEN** the differential gate is retired
- **THEN** the changelog states that it was green across every subcommand, names the commit at which that held, and names the modes it covered
- **AND** the decisions it recorded as known differences are rewritten as statements about the rust runtime with the retired behaviour named as history
- **AND** the harness itself remains reachable through version control rather than through the working tree
