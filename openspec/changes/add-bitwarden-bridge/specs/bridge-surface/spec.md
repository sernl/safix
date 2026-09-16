## MODIFIED Requirements

### Requirement: A bridge relationship is declared rather than passed as arguments

Each relationship between a clan var and a safix entry SHALL be declared in the consumer's nix, naming both endpoints and a direction, and no verb SHALL accept an endpoint as a command-line argument.
Where a target's far side is a network service rather than a path, the service SHALL be declared once alongside that target's mappings, and neither it nor any credential or session token it needs SHALL be a run argument.

#### Scenario: A mapping names both sides

- **WHEN** a mapping is declared
- **THEN** it names a clan generator and file, a placement, and a machine exactly as that placement requires
- **AND** it names a safix user and entry name

#### Scenario: A run takes no endpoint arguments

- **WHEN** `sync`'s arguments are enumerated
- **THEN** none of them names a machine, a generator, a file, a user or an entry
- **AND** the mappings a run acts on come from the declarations

#### Scenario: The mapping carries its own identifier

- **WHEN** a mapping is reported on, committed, or refused
- **THEN** the mapping's own declared name appears
- **AND** it is not derived from either endpoint

#### Scenario: The clan flake is declared once

- **WHEN** more than one clan flake is declared
- **THEN** evaluation refuses
- **AND** the refusal states that one consumer bridges one clan

#### Scenario: A network-backed target's endpoint is declared once, beside its mappings

- **WHEN** a target whose far side is a network service is declared
- **THEN** the service is named once for that target rather than per mapping, and MAY be left undeclared where the target's own client already holds that configuration
- **AND** no verb accepts it as an argument, for the reason this requirement gives for every other endpoint: a standing relationship is reviewable where a run's arguments are not

#### Scenario: A session token is not an endpoint a run may be given

- **WHEN** the arguments of every verb are enumerated
- **THEN** none of them accepts a session token, an access token, or a password for a far side
- **AND** a token a run mints for itself is confined to the environment of the children that need it, which each target's own capability states and bounds
