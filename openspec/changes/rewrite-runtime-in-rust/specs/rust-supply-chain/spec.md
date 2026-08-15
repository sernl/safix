## Purpose

What the rust half of the repository promises about the toolchain it compiles with, the dependencies it pulls in, the constructions it refuses to allow in library code, and the documentation a consumer or contributor arrives at — and which of those promises is a check rather than a sentence.

## ADDED Requirements

### Requirement: The stated minimum toolchain version is the one that is built

The workspace SHALL declare a minimum supported rust version equal to the toolchain the flake pins, and that declaration SHALL be enforced by the build tool rather than by documentation.

#### Scenario: The number and its enforcement

- **WHEN** the workspace manifest is read
- **THEN** it declares the minimum version
- **AND** a build on an older toolchain fails naming the required version, without any check of this repository's having to run

#### Scenario: The rule that produced the number

- **WHEN** the choice of version is documented
- **THEN** it records that the repository states the newest version it requires rather than the oldest it might tolerate
- **AND** the reason given is that only one toolchain is ever compiled against, so a lower number would be an untested compatibility claim

#### Scenario: Lowering it later

- **WHEN** a lower minimum is wanted
- **THEN** the field is lowered together with a check that builds the workspace on that older toolchain
- **AND** the stated number does not move ahead of the check

### Requirement: Builds are locked and offline

The dependency graph SHALL be committed as a lock file, and every build the flake performs SHALL use it without resolving or fetching anything at build time.

#### Scenario: The lock is the graph

- **WHEN** any check or package builds the workspace
- **THEN** it builds the exact versions the lock file names
- **AND** the build performs no network access

#### Scenario: A stale lock is visible

- **WHEN** a manifest and the lock file disagree
- **THEN** the build fails rather than silently updating the lock

### Requirement: Library code may not panic by construction

Clippy SHALL run with the pedantic group enabled and with the panicking constructions denied in the library crate, and the exceptions SHALL be enumerated in the workspace manifest rather than scattered as attributes.

#### Scenario: What is denied

- **WHEN** the lint configuration is read
- **THEN** unwrapping, expecting, explicit panicking, slice indexing, and the lossy arithmetic and conversion lints are denied for library code
- **AND** each relaxation of the pedantic group is listed in one place

#### Scenario: Why these lints and not hygiene in general

- **WHEN** the reason for the denial is recorded
- **THEN** it states that a panic in a runtime holding a decrypted value unwinds through the drop that zeroes it, and that a panic message is a place a value could surface if a type ever grew a rendering trait
- **AND** it states that every failure being a returned result is what makes the refusal model true for an embedder and not only for the command

#### Scenario: The command's one exception

- **WHEN** the command crate uses a panicking construction
- **THEN** it is in the program's entry point only
- **AND** the alternative there would be unreachable code

### Requirement: Dependencies are reviewed for licence, provenance and duplication

A dependency check SHALL run offline over the locked graph and SHALL fail on a licence outside the allowed set, a source outside the allowed registries, or a duplicated dependency version.

#### Scenario: What the offline check covers

- **WHEN** the dependency check runs
- **THEN** it evaluates bans, licences and sources
- **AND** it does so without network access, over the locked graph

#### Scenario: The allowed licences are enumerated

- **WHEN** the configuration is read
- **THEN** the permitted licences are listed explicitly
- **AND** the repository's own dual licence is among them

### Requirement: Advisories are checked against a database pinned in the lock

The advisory scan SHALL be a check separate from the offline dependency check, and SHALL read an advisory database pinned as a flake input.

#### Scenario: Why the split

- **WHEN** the separation is documented
- **THEN** it records that the advisory database is a network resource and the build sandbox has none
- **AND** that a newly published advisory therefore turns exactly one check red, and only when the lock is updated

#### Scenario: A vulnerable dependency

- **WHEN** the pinned database names an advisory affecting a locked dependency
- **THEN** the advisory check fails naming the advisory and the dependency
- **AND** the other checks are unaffected

### Requirement: Every promise above is a flake check

Building, testing, linting, formatting, dependency review and the advisory scan SHALL each be a check of this flake, so that the repository's single verification command covers all of them.

#### Scenario: The checks exist and are named

- **WHEN** the flake's checks are enumerated
- **THEN** a build, a test, a lint, a format, a dependency and an advisory check for the rust workspace are present
- **AND** each is named so its subject is evident

#### Scenario: The shipping package is not among the things this change moves

- **WHEN** the flake's packages are enumerated
- **THEN** the rust binary appears under its own name
- **AND** the package the shell runtime builds is unchanged

### Requirement: The adoption surface states what the crates are and how to work on them

The published library SHALL carry documentation on its public interface, and the repository SHALL carry a changelog, a stated versioning policy, and a contributor document containing the fixture-fleet recipe.

#### Scenario: The public interface is documented

- **WHEN** the library's public items are enumerated
- **THEN** each carries documentation
- **AND** the custody type's construction rule and its absent traits are stated where the type is defined

#### Scenario: The changelog records what is true now

- **WHEN** the changelog is read during the migration
- **THEN** it states that the rust runtime is not yet what the shipping package builds
- **AND** it does not describe behaviour that has not landed

#### Scenario: A contributor can reproduce the fixtures

- **WHEN** the contributor document is followed
- **THEN** a fixture fleet with throwaway keys can be built and driven locally
- **AND** the document states that no real recipient, host or user name may enter the repository
