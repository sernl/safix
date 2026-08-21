## Purpose

A host may already run a secret store this package did not write, and safix has to run after it without discovering it.
Ordering against such a store joins host attachment as something the consumer names, for the reason this capability already gives about user registries.

## ADDED Requirements

### Requirement: Ordering against a foreign secret store is a consumer-named dependency

Where another component of the host installs secrets, the step or unit this package's installer must follow SHALL be named by the consumer.
The package SHALL NOT read any option belonging to that component to discover it, and naming nothing SHALL be a supported configuration.

#### Scenario: The package does not look for it

- **WHEN** the package decides what its installer runs after
- **THEN** it reads only its own namespace and the host's own activation and unit options
- **AND** no option path belonging to a particular secret-management framework is consulted

#### Scenario: The reason it is not built in

- **WHEN** the ordering is documented
- **THEN** it records that discovering one framework's installer would make that framework's option tree part of this package's interface, and would answer for one foreign store out of the many a host may run
- **AND** the same reason is the one already given for a consumer's user registry

#### Scenario: What is given up by not discovering it

- **WHEN** a consumer names no ordering on a host that has a foreign store
- **THEN** the package cannot guarantee the ordering and does not claim to
- **AND** the installer's own pre-decryption refusal names these options as the remedy, so the failure is attributable rather than silent
