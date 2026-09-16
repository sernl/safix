## ADDED Requirements

### Requirement: The declaration surface is exercised end to end by worked consumers, at both scopes

The repository SHALL carry worked consumers that declare every feature of the declaration namespace and consume the result at both the system and the user scope, and each SHALL be evaluated by a check rather than merely committed.
A feature the declaration surface carries and no worked consumer declares SHALL be a gap that is recorded and closed, not a documentation choice.

#### Scenario: Every declarable feature appears in a worked consumer

- **WHEN** the declaration namespace's features are enumerated against the worked consumers
- **THEN** each of them is declared by a consumer that a check evaluates
- **AND** the list of features includes the ones reachable only through another subject — a grant to the owner of a machine, a grant to an organization, a placement addition, a nested group member, a delegation with both consenting sides, and a recovery recipient

#### Scenario: A consumption profile exists at each scope

- **WHEN** the worked consumers are enumerated
- **THEN** one of them is a system-scope profile and one is a user-scope profile, both consuming the same declarations
- **AND** both name the person or machine they serve, the host they resolve on, and the projection they are bound to, so the whole binding a consumer must supply is visible in one file per scope

#### Scenario: Coverage is asserted rather than claimed

- **WHEN** a worked consumer is said to cover a feature
- **THEN** an assertion over the resolved projection names the evidence that feature produces
- **AND** the reason is recorded: a comparison between two consumers holds that they agree, so removing one feature from both of them leaves that comparison green while the coverage claim becomes false

#### Scenario: The profile surface is evaluated by a check of its own

- **WHEN** the check that evaluates the consumption profiles is enumerated
- **THEN** it is separate from the one comparing the two declaration-side consumers
- **AND** the reason is recorded: what a profile resolves is a materialization under a scope, which the fields the declaration-side comparison is over do not carry, and a profile evaluation needs a host module system the declaration-side check's sandbox deliberately does not hold

#### Scenario: A worked consumer declares nothing that does not exist

- **WHEN** a worked consumer names an option
- **THEN** that option is declared by the package at the revision the consumer is committed against
- **AND** a feature whose option has not landed yet is absent from the consumers rather than present as commented-out text, because nothing evaluates a comment

## MODIFIED Requirements

### Requirement: One declaration serves both system and user scope

A resolved entry SHALL be materializable into either the system-scope or the user-scope form this package's own installer accepts, from the same declaration.
A worked profile SHALL exist for each scope, consuming one set of declarations, and both SHALL be evaluated by a check.

#### Scenario: The same entry on either side

- **WHEN** the same entry is materialized for a system configuration and for a user profile
- **THEN** its mode, its path, and the key it reads are the same declaration in both
- **AND** nothing about the declaration states which scope it is for

#### Scenario: A scope-specific field is refused where it is meaningless

- **WHEN** an ownership field is set on an entry materialized into a scope that has no ownership axis
- **THEN** evaluation fails naming the entry and the field
- **AND** the axis is read off the entry type this package declares, so the refusal does not depend on any other framework's option declaration being present

#### Scenario: A path is a function of the consuming configuration

- **WHEN** an entry's path needs a value the consuming configuration computes
- **THEN** the path is declared as a function of that configuration
- **AND** the same declaration resolves correctly under either scope

#### Scenario: A worked profile exists at each scope

- **WHEN** the worked profiles are read
- **THEN** one is a system-scope profile and one is a user-scope profile, over the same declarations
- **AND** each resolves the set its subject holds on its host, so the claim that one declaration serves both scopes is demonstrated rather than asserted

#### Scenario: The scope-asymmetric refusal is demonstrated, not only stated

- **WHEN** the worked profiles are read for the ownership axis
- **THEN** the system-scope profile carries an entry whose ownership resolves
- **AND** the user-scope profile does not declare an ownership field, and the documentation states the refusal that doing so produces, so the asymmetry is visible in the examples rather than only in prose

### Requirement: A consumer bridges its own vocabulary with an adapter it owns

Bridging a consumer's existing user vocabulary to safix's SHALL be a projection written in the consumer's repository.
The package SHALL ship no adapter for any particular consumer.
The worked example the documentation carries in place of an adapter SHALL be one a check evaluates.

#### Scenario: What an adapter is

- **WHEN** a consumer with its own user registry adopts the package
- **THEN** the bridge is a mapping from their records into safix's user records
- **AND** it lives in their repository

#### Scenario: The package ships none

- **WHEN** the package's modules are enumerated
- **THEN** none of them names a particular consumer's option path
- **AND** the documentation carries a worked example rather than an importable adapter

#### Scenario: An adapter is sufficient on its own

- **WHEN** a consumer supplies only an adapter
- **THEN** every capability of the package is available to them
- **AND** no further integration point is required

#### Scenario: The worked example is evaluated

- **WHEN** the worked example the documentation carries is read
- **THEN** a check evaluates it
- **AND** the reason is recorded: an example nobody evaluates is documentation that rots, and the examples' own document had already come to describe a mechanism its example no longer used
