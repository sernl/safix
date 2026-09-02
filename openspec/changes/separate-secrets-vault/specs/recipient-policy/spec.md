## MODIFIED Requirements

### Requirement: The policy file is generated and never hand-edited

The recipient policy SHALL be rendered from the same declarations the resolver reads, SHALL be committed to the repository the encrypting backend reads it from — the vault root when the consumer declares one, the consumer's own repository otherwise — because the encryption tool reads it from the filesystem, and SHALL carry a header stating that it is generated and naming the command that regenerates it.

#### Scenario: One declaration, two projections

- **WHEN** the resolver computes a secret's audience and the renderer emits that audience's rule
- **THEN** both are derived from the same declarations
- **AND** no arrangement of declarations produces a file whose rule and whose resolved audience disagree

#### Scenario: Drift from the committed file is a finding

- **WHEN** the committed policy file differs from the one the declarations imply
- **THEN** a check fails
- **AND** the failure names the command that regenerates the file

#### Scenario: The generator does not overwrite the people

- **WHEN** the policy is regenerated
- **THEN** every declared person's rule is present in the output, because the people are the generator's input
- **AND** regeneration cannot drop a rule that no declaration removed
