## Purpose

The system module's activation contribution stops being empty, so the requirement recording its asymmetry with the user scope has to say what it now contributes and what it still does not.
The namespace gains the options that name the installer's ordering and govern its identity derivation, which are selection and decryption and so belong here rather than in custody.

## MODIFIED Requirements

### Requirement: The system scope claims no activation guard

The system-scope module SHALL contribute the installer described by `secret-installation` and SHALL NOT install an activation guard beside it, and the documentation SHALL record that no atomic refusal point at system activation has been demonstrated.
The installer's own pre-decryption identity check SHALL NOT be described as such a guard: it refuses before this package writes anything, and it makes no claim about what the rest of the activation has already done.

#### Scenario: What the system module installs

- **WHEN** the system module's activation contribution is enumerated
- **THEN** it contains this package's installer and no identity check that claims atomicity
- **AND** the documentation states why rather than leaving the asymmetry with the user scope unexplained

#### Scenario: The installer's own check is not the guard the user scope has

- **WHEN** the installer's pre-decryption identity refusal is documented
- **THEN** it states that it refuses before this package writes, links, or restarts anything of its own
- **AND** it does not state that the system generation is intact, because system activation reached this step by running earlier ones

## ADDED Requirements

### Requirement: The namespace names the installer's ordering and its identity derivation

The consumption namespace SHALL carry the activation steps and the units this package's installer runs after, and a switch governing whether the system-scope identity is derived from the host's ssh host keys.
These SHALL be selection and decryption only: none of them declares a secret, a recipient, a grant, or an audience.

#### Scenario: The ordering is named here and nowhere else

- **WHEN** a consumer orders this package's installer after another component's
- **THEN** they name that component's activation step or unit through options of this namespace
- **AND** no option of this namespace names a particular component

#### Scenario: Naming nothing stays supported

- **WHEN** neither ordering option is set
- **THEN** the installer is registered with no dependency on any other component
- **AND** the profile evaluates, because a host with no foreign secret store needs no ordering

#### Scenario: The derivation switch is decryption, not custody

- **WHEN** the derivation switch is turned off
- **THEN** the identity is exactly what the consumer named
- **AND** nothing about which entries resolve, or who may read them, changes
