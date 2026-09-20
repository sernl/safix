# Spec Delta

## ADDED Requirements

### Requirement: The user-scope namespace may schedule the operator's rotations

The user-scope consumption module SHALL carry a rotation timer namespace that selects a repository, a calendar and an environment and installs a systemd user service and timer running the due-rotation verb. It SHALL declare no secret, no recipient, no grant and no audience, and SHALL define nothing when disabled.

#### Scenario: The timer namespace is selection only
- **WHEN** the rotation options are read
- **THEN** none of them names a secret, a recipient, a grant or an audience

#### Scenario: Disabled defines nothing
- **WHEN** the timer is not enabled
- **THEN** no unit, timer or environment file is defined for it
