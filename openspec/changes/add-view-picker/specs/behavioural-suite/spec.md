## ADDED Requirements

### Requirement: An interactive surface is driven through a real pseudo-terminal, never through a seam in the binary

Where a subcommand reads keystrokes and draws, the suite SHALL drive it through a pseudo-terminal attached to both the input it reads and the output it draws on, and the shipped binary SHALL carry no option, environment variable or branch whose only caller is a test wanting to bypass that surface.

#### Scenario: The pseudo-terminal carries both directions

- **WHEN** a test exercises a subcommand that reads keystrokes and draws
- **THEN** the pseudo-terminal's slave is attached to the stream the run reads and to the stream it draws on
- **AND** standard error stays a pipe, so a refusal is observed separately from anything drawn

#### Scenario: A run with no terminal is asserted rather than avoided

- **WHEN** the same subcommand is run with pipes on every stream, as the hermetic check sandbox provides
- **THEN** the test asserts the refusal that state produces, naming its code
- **AND** it does not substitute a terminal to make the run proceed

#### Scenario: No selection override ships

- **WHEN** the binary's options and the environment variables it reads are enumerated
- **THEN** none of them pre-selects, pre-fills or skips an interactive choice
- **AND** the reason is recorded: the non-interactive path already exists as the form that names the entry, so an override would be a second code path shipping to operators whose only caller is a test, and the test it enables would assert that path rather than the surface

#### Scenario: The environment variables the harness does set name programs, not branches

- **WHEN** the harness's environment variables are read
- **THEN** each names a program to locate or substitute
- **AND** none of them selects a branch inside the binary's own logic
