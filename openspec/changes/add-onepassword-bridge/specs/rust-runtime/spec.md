## MODIFIED Requirements

### Requirement: Every external program the runtime invokes is selectable by a named variable

Each external program the runtime invokes SHALL be selectable through a named environment variable, so a hermetic check can substitute a build of its own, and the set of such programs and the set of such variables SHALL be the same size.
A program no check of this repository may ever run for real SHALL still have its variable, because the variable is the seam a stand-in stands in.

#### Scenario: The set is complete

- **WHEN** the external programs the runtime invokes are enumerated against the environment variables that select them
- **THEN** each program has one
- **AND** the two that previously had none — the age key generator, spawned by a hardcoded program name, and the ssh-key converter the installer's identity assembly needs — now do

#### Scenario: The variable is read, not merely declared

- **WHEN** a check exercises the key generator or the ssh-key converter
- **THEN** it drives the behaviour by pointing that program's variable at a build of its own
- **AND** the check fails if the variable is ignored, which is what makes the override evidence rather than documentation

#### Scenario: A program that is never run for real still has its variable

- **WHEN** the 1Password command is invoked by the sync and audit paths
- **THEN** it is located through its own named variable, defaulting to the program's ordinary name on the operator's path
- **AND** every check of that target drives the variable at a stand-in, which is the only way the target is exercised at all, because the real package is unfree and enters no check, package or development shell of this flake
