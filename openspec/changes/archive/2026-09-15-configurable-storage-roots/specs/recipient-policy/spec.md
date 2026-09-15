## MODIFIED Requirements

### Requirement: The policy file is generated and never hand-edited

The recipient policy SHALL be rendered from the same declarations the resolver reads, SHALL be committed to the declaring repository because the encryption tool reads the committed file from there — whether or not a vault is declared — and SHALL carry a header stating that it is generated and naming the command that regenerates it.
Every path the header's worked examples spell SHALL be derived from the configured storage roots, because a committed header documenting a layout the repository does not have is a false statement in the one file a reviewer reads to learn who can open what.
For a command that needs creation rules to reach a vault-rooted document, the runtime SHALL additionally render a disposable, uncommitted copy of the rules into the vault working tree, scoped to that command alone.

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

#### Scenario: A renamed encrypted root moves the rules and the header together

- **WHEN** the encrypted root is renamed and the policy is regenerated
- **THEN** every rule's pattern is anchored under the new root
- **AND** every path the header's worked examples spell names the new root
- **AND** the drift check reports the committed file until it is regenerated

#### Scenario: A vault does not move the committed file

- **WHEN** a consumer declares a vault
- **THEN** the committed policy file still lives in the declaring repository, exactly where it lives with no vault declared
- **AND** the vault repository never carries a committed policy file of its own

#### Scenario: A vault-rooted command reads a disposable rendering, not the committed file

- **WHEN** a command needs creation rules to encrypt or re-key a vault-rooted document
- **THEN** it reads a rendering produced for that run alone, inside the vault working tree, never the committed file at the declaring root
- **AND** that rendering is never committed
