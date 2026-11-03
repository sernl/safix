## MODIFIED Requirements

### Requirement: A transfer compares before it writes, and an agreeing mapping is left alone

`sync`, for the clan target, SHALL read both sides of a mapping and compare them before writing either, and SHALL write nothing and commit nothing when the two agree.

#### Scenario: An agreeing mapping writes nothing

- **WHEN** both sides of a mapping hold the same bytes
- **THEN** nothing is written and nothing is committed

#### Scenario: The comparison precedes the export write for a stated reason

- **WHEN** the safix-to-clan write path is documented
- **THEN** it records that clan's write command writes unconditionally and that a backend re-encrypting an unchanged value produces fresh ciphertext
- **AND** it states that without the comparison every run would commit in clan's store repository for every mapping, that repository being the clan flake or the external age store clan is configured to use

#### Scenario: A second run changes nothing

- **WHEN** `sync clan` runs twice with nothing else intervening
- **THEN** the second run writes nothing and commits nothing

#### Scenario: A value the operator cannot read is refused rather than transferred

- **WHEN** a mapping's safix side cannot be decrypted by the operator running the command
- **THEN** the mapping is refused
- **AND** the reason given is that a value that cannot be read cannot be verified

### Requirement: Each transferred mapping is its own commit

A transfer SHALL commit each mapping separately, naming the mapping, and SHALL NOT combine several mappings into one commit.

#### Scenario: One mapping, one commit

- **WHEN** a run transfers several mappings into safix
- **THEN** each produces its own commit

#### Scenario: The export direction commits where the value landed

- **WHEN** a run transfers a mapping into clan
- **THEN** it makes no commit in the consumer's own repository, where nothing changed
- **AND** each mapping is one invocation of clan's write, which commits what it wrote in clan's store repository, or nowhere when clan's external age store is not a git repository

#### Scenario: The commit names the mapping and not the value

- **WHEN** a transfer's commit message is read
- **THEN** it names the mapping and the direction
- **AND** it names no value

#### Scenario: The reason for single-intent commits is recorded

- **WHEN** the commit discipline is documented
- **THEN** it states that a combined commit's message cannot say what it did without naming values
- **AND** it states that reverting one mapping should not revert the others
