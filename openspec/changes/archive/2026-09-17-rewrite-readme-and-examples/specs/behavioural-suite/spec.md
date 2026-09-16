## ADDED Requirements

### Requirement: The two worked consumers are compared to each other on every field the runtime reads

The check comparing the two declaration-side consumers SHALL compare every projection field the runtime reads, SHALL compare each side's own declaration of a field rather than one side's against itself, and SHALL elide only the part of a value that is a function of where the consumer's own root happens to be.

#### Scenario: Every field the runtime reads is compared

- **WHEN** the compared fields are enumerated against the fields the runtime reads from a projection
- **THEN** every field the runtime reads and that serializes is compared
- **AND** a field excluded from the comparison is excluded because it is a function, with the exclusion recorded beside the list

#### Scenario: A value is never compared with itself

- **WHEN** the expected and the actual side of each compared value are traced to their sources
- **THEN** the expected value comes from one consumer's own declarations and the actual value from the other's
- **AND** no compared value is taken from the same file for both sides, because such a row is green under every possible divergence

#### Scenario: A hook is a compared value like any other

- **WHEN** the hooks the command invokes are compared
- **THEN** each consumer declares them itself and the two declarations are compared
- **AND** one literal assertion holds that the hook is present and carries its fixture's text, so both consumers losing the hook fails rather than comparing empty to empty

#### Scenario: Root-dependent absolute strings are elided, not dropped

- **WHEN** a compared field carries a value derived from a path declared relative to each consumer's own root
- **THEN** the leading root is replaced by one stable marker on both sides and the remainder of the value is compared
- **AND** the field stays in the comparison, because a field dropped from it is a declaration compared nowhere

#### Scenario: The elision is as narrow as the divergence

- **WHEN** the elision is applied
- **THEN** it replaces only the consumer's own root prefix
- **AND** it does not elide store paths generally, because a store path appearing where one is not expected is itself a finding

#### Scenario: The severity drill for the comparison

- **WHEN** one field of one consumer's fleet is changed and the other's is not
- **THEN** the check fails on exactly that field
- **AND** this is the evidence that the two are compared rather than merely both evaluated

### Requirement: A hand-maintained list of the example's own files is refused

Where a check discovers a worked example's files by reading the directory, the example SHALL NOT also enumerate them by hand, and the check SHALL assert that no such enumeration exists.

#### Scenario: The example's own entry point discovers its files the same way

- **WHEN** the worked example that is a flake is read
- **THEN** it obtains its declaration files by reading its own module directory
- **AND** it names no individual file under that directory

#### Scenario: The absence of a hand list is asserted

- **WHEN** the check runs
- **THEN** it asserts that the example's entry point contains no per-file path under its module directory
- **AND** the reason is recorded: a hand list beside a discovered one drifts on the next added file, and the drift is silent because nothing evaluates the entry point

#### Scenario: Comparing two lists is refused as the remedy

- **WHEN** asserting equality between a hand list and the discovered set is considered
- **THEN** it is refused
- **AND** the reason is recorded: extracting an import list from nix source is either a parser with one caller or a text match that breaks when the formatter moves a path to another line

#### Scenario: The severity drill for the list

- **WHEN** a per-file import path is put back into the example's entry point
- **THEN** the check fails naming it
- **AND** when a new declaration file is added to the module directory with no other edit, the check stays green and the new file is in both the example's own evaluation and the check's
