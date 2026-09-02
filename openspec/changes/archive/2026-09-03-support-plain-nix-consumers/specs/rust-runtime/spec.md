## MODIFIED Requirements

### Requirement: The evaluation seam is preserved

The command SHALL obtain placements, audiences, governed files and policy text by evaluating the nix half, and SHALL NOT reimplement resolution, the type vocabulary, or the recipient policy renderer.

#### Scenario: What the runtime asks nix for

- **WHEN** the runtime needs a resolved placement, an audience, the governed file set, or the policy text
- **THEN** it obtains it by evaluating the declarations the consumer's flake carries, or, when an entry file was named through `--entry` or `SAFIX_ENTRY`, the declarations that file carries
- **AND** the request is the same one the shell runtime makes, using the same attribute path either way

#### Scenario: What the runtime does not compute

- **WHEN** the runtime's own code is searched for resolution, audience derivation or policy rendering
- **THEN** none is found
- **AND** the reason is recorded: the nix half is the consumer-facing option surface and is checked by evaluation, regardless of whether that evaluation targets a flake or a named file
