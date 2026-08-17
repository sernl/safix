## Purpose

The bridge changed which verbs exist, so the help's record of absences moves with it: `import` and `export` now name the mapping-scoped transfer verbs, and what stays absent — any upload, and the plaintext dump and restore — stays recorded.
This delta modifies the requirement `extract-safix-from-dotfiles` established, whose "no export or import verb exists" sentence predates the bridge.

## MODIFIED Requirements

### Requirement: Absent verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a verb of the same name exists with different semantics, the help SHALL state what the verb is not, so the absence of the other tool's semantics stays on the record.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no upload verb exists because activation already delivers what it would

#### Scenario: The same names, different semantics

- **WHEN** the help for `import` or `export` is read
- **THEN** it states that each moves declared mappings across the clan boundary, one mapping at a time
- **AND** states that neither is a plaintext dump or restore, because a plaintext export tree outlives the migration that justified it
