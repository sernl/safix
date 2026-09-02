## MODIFIED Requirements

### Requirement: Absent verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a verb of the same name exists with different semantics, the help SHALL state what the verb is not, so the absence of the other tool's semantics stays on the record.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no verb exists for ongoing secret delivery, because activation already delivers what it would

#### Scenario: The same names, different semantics

- **WHEN** the help for `import` or `export` is read
- **THEN** it states that each moves declared mappings across the clan boundary, one mapping at a time
- **AND** states that neither is a plaintext dump or restore, because a plaintext export tree outlives the migration that justified it

#### Scenario: `upload` exists under a name clan also uses, with narrower semantics

- **WHEN** the help for `upload` is read
- **THEN** it states that it moves only a machine's own host identity, once, before that machine's first activation
- **AND** states that it is not clan's ongoing vars-delivery verb of the same name, because activation already delivers what this package resolves for a machine that already has its identity
