## MODIFIED Requirements

### Requirement: Retired and reserved verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a word this package's own vocabulary once used is retired outright, the help SHALL say so; where one is reserved for a feature not yet built, the help SHALL say that instead, so a reservation is never mistaken for an oversight.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no verb exists for ongoing secret delivery, because activation already delivers what it would
- **AND** it states that no export verb exists, naming clan's own `export` as the bulk plaintext dump safix's design refuses to build on either side of the boundary

#### Scenario: A reserved absence is told apart from a retired one

- **WHEN** the help for `import` is read
- **THEN** it states that `import` is reserved for a future, unbuilt feature — ingesting a value from an external plaintext source one entry at a time — rather than retired outright
- **AND** it states that this is distinct from `export`'s retirement, which is permanent

#### Scenario: `upload` exists under a name clan also uses, with narrower semantics

- **WHEN** the help for `upload` is read
- **THEN** it states that it moves only a machine's own host identity, once, before that machine's first activation
- **AND** states that it is not clan's ongoing vars-delivery verb of the same name, because activation already delivers what this package resolves for a machine that already has its identity
