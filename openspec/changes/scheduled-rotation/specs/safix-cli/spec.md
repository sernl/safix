# Spec Delta

## ADDED Requirements

### Requirement: `rotate` and `rotation` join the closed subcommand set

The command SHALL accept `rotate` and `rotation` as subcommands, listed in its help with one line each, and `list` SHALL carry the rotation deadline as a column beside the timestamps.

#### Scenario: The verbs are discoverable
- **WHEN** the help enumerates the verbs
- **THEN** `rotate` and `rotation` appear with their forms and one-line meanings

#### Scenario: `list` carries the deadline
- **WHEN** `list` runs
- **THEN** each row shows the remaining time, `due`, or the absent marker in a column of its own
