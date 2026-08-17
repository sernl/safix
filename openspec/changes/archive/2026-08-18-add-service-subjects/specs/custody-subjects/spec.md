## Purpose

Services join the subject model: the thing a secret is actually for becomes the thing the declaration names, without inventing a second identity mechanism beside the machines that run it.

## ADDED Requirements

### Requirement: A service is a declarable subject that resolves to its machines

A service SHALL be declarable with the machines it runs on, an owner, and the unix user and group its landed entries belong to.
A service SHALL resolve to its declared machines' recipients and SHALL NOT carry an identity of its own; the documentation SHALL state, where the option is declared, that the audience names the service while the host identity remains what decrypts, so the machine is the trust boundary for everything running on it.

#### Scenario: The service names machines the fleet declares

- **WHEN** a service names a machine no declaration covers
- **THEN** evaluation refuses, naming the service and the machine

#### Scenario: Declaration alone is inert

- **WHEN** services are declared and no grant names one
- **THEN** every generated rule and every governed file is byte-identical to the tree without them

### Requirement: An audience may include services, as elements that follow the declaration

A grant SHALL be able to name a service anywhere subjects are named, and a file whose audience includes one SHALL carry the recipients of the service's machines at generation time.
The service SHALL appear in the audience as its own rendered element, so a change to its machine set re-wraps the same files rather than moving them, with growth a re-wrap and shrink reported as the revocation it is.

#### Scenario: A machine joins the service and the files follow

- **WHEN** a machine is added to a granted service's set and `fix` runs
- **THEN** the service's audience files re-wrap to include the new machine's recipient
- **AND** no file moves

#### Scenario: A machine leaving is a revocation

- **WHEN** a machine is removed from a granted service's set
- **THEN** `check` reports the shrink as a revocation with rotation named as the remedy
- **AND** the not-retroactive disclosure is carried as it is for every other narrowing

#### Scenario: An empty service cannot be granted to

- **WHEN** a grant names a service whose machine set is empty
- **THEN** evaluation refuses, naming the grant and the empty set, because the file would be encrypted to nobody

### Requirement: Services share the one subject name space

A service's name SHALL live in the same name space as users, machines, and groups, with collisions refused at evaluation, and groups MAY include services as members.

#### Scenario: A collision is refused where it is declared

- **WHEN** a service is declared with a name a user, machine, or group already holds
- **THEN** evaluation refuses, naming both declarations
