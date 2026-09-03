## MODIFIED Requirements

### Requirement: The policy file is generated and never hand-edited

The recipient policy SHALL be rendered from the same declarations the resolver reads, SHALL be committed to the declaring repository because the encryption tool reads the committed file from there — whether or not a vault is declared — and SHALL carry a header stating that it is generated and naming the command that regenerates it.
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

#### Scenario: A vault does not move the committed file

- **WHEN** a consumer declares a vault
- **THEN** the committed policy file still lives in the declaring repository, exactly where it lives with no vault declared
- **AND** the vault repository never carries a committed policy file of its own

#### Scenario: A vault-rooted command reads a disposable rendering, not the committed file

- **WHEN** a command needs creation rules to encrypt or re-key a vault-rooted document
- **THEN** it reads a rendering produced for that run alone, inside the vault working tree, never the committed file at the declaring root
- **AND** that rendering is never committed

### Requirement: Every rule is anchored, extension-terminated, and scoped to one directory level

Every path pattern SHALL begin with a start-of-string anchor, SHALL end with the literal file extension anchored at end of string, and SHALL match exactly one directory level.
A pattern violating any of the three SHALL fail a check that names the offending rule.

#### Scenario: A well-formed rule

- **WHEN** a rule is emitted
- **THEN** its pattern begins with a start-of-string anchor
- **AND** ends with the literal file extension anchored at end of string
- **AND** matches a single directory level, admitting no nested path and no prefixed path

#### Scenario: Why the start anchor is load-bearing

- **WHEN** a pattern omits the start anchor
- **THEN** it also matches the same suffix under any prefix, because the encryption tool matches patterns unanchored against the path relative to the policy file
- **AND** the check fails naming the rule

#### Scenario: Why the extension anchor is load-bearing

- **WHEN** a pattern omits the extension anchor
- **THEN** it can match encrypted material this package did not place, whose recipients a sweep would silently rewrite
- **AND** the check fails naming the rule

#### Scenario: Why one directory level is load-bearing

- **WHEN** a pattern admits nested paths through an unrestricted wildcard
- **THEN** a file dropped in a subdirectory would inherit a person's recipients rather than failing closed
- **AND** the check fails naming the rule

#### Scenario: A file placed beside a person's secrets rides their custody

- **WHEN** a file is added inside a directory an existing rule already covers
- **THEN** it is governed by that rule with no new rule required
- **AND** no review round is needed for it

#### Scenario: Anchoring makes rule order immaterial

- **WHEN** two people's rules coexist
- **THEN** their patterns match disjoint directories
- **AND** reordering the rules changes no file's recipients

#### Scenario: A vault-mode rule matches one opaque file, not a directory wildcard

- **WHEN** a rule is emitted for a vault-rooted, opaquely named document
- **THEN** its pattern is the literal opaque filename, anchored at both ends
- **AND** it still satisfies every clause of this requirement, because a single-file match is the limiting case of one directory level rather than an exception to it
