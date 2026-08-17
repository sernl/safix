## Purpose

A service-granted entry lands as the service's own: owned by its declared unix user and group at system scope, and refused rather than silently unowned where no ownership axis exists.

## ADDED Requirements

### Requirement: Service-scoped placement carries the service's ownership

An entry granted to a service SHALL resolve at each of the service's machines, landing with the unix user and group the service declares, so that the narrowing the declaration states is enforced by the host's own access control.
Two services on one machine SHALL NOT resolve one entry onto one path, refused exactly as any other path collision is.

#### Scenario: The landed file belongs to the service

- **WHEN** a service-granted entry resolves at a machine's system scope
- **THEN** the resolved entry carries the service's declared user and group
- **AND** the path is the service's own rather than shared with another service's entries

#### Scenario: Ownership without an axis is refused, not dropped

- **WHEN** a service declaring a user or group is granted an entry that would resolve on a machine served by a user-scope profile
- **THEN** evaluation refuses, naming the service, the machine, and the missing axis
- **AND** a service declaring no ownership resolves there with the scope's ordinary placement
