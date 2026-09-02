## MODIFIED Requirements

### Requirement: Declarations are mergeable and may be scattered

The catalogue SHALL be an attribute set option merged by the nix module system, so that declarations made in separate files merge into one record, whether that merge happens through a flake-parts module a consumer's flake imports or through `lib.mkVault`'s `modules` argument.
No file layout, naming scheme, or import order SHALL be required of a consumer.

#### Scenario: One secret per file

- **WHEN** a consumer declares each secret in its own module file, anywhere in its tree
- **THEN** the resolver sees the same record it would see from a single file declaring all of them
- **AND** no import order changes the result

#### Scenario: The package prescribes no layout

- **WHEN** a consumer arranges its declaration files
- **THEN** nothing in the package reads a path, a filename, or a directory structure to find them
- **AND** the only requirement is that the modules reach the mechanism that merges them: a consumer's flake imports, or the `modules` argument to `lib.mkVault`

#### Scenario: Two files declaring one name

- **WHEN** two modules declare the same secret name with different fields
- **THEN** the module system's own merge rules apply and a genuine conflict is an evaluation error naming the option
- **AND** the package adds no silent last-writer-wins behaviour of its own
