## MODIFIED Requirements

### Requirement: The database is a store being written, never a keyring being managed

Sync SHALL NOT create databases, change database keys, or program, reprogram, or delete a hardware slot; it SHALL reach the database through the store's own command with a single password prompt per run, and a secret value SHALL travel standard input or pipes, never an argument vector or an environment variable.
A database MAY additionally require a YubiKey challenge-response slot, a key file, or both to open, and reading that slot or that file to unlock the database is not touching it in the sense this requirement forbids: the slot number, its optional serial, and the key file's own path are public identifiers of how the database opens rather than secrets it holds, and MAY travel the store's own command line the way the database's own path already does.
Without a terminal to ask that password on, sync SHALL refuse rather than prompt into the void.

Amended during apply. The requirement had sync reach the database "through the session's secret service when the database is unlocked, else through the store's own command"; the service is not a transport it can use.
The Secret Service collection KeePassXC publishes is its exposed group, so an item found or created through it belongs to whatever group the operator's exposure setting names and not to the group a mapping declares.
Two transports addressing different entries would make a mapping's convergence depend on which one ran, and a service read of an entry in an unexposed group is indistinguishable from the database holding no value — which would let a `backup` mapping write a secret into a group no declaration named, an outcome the report has no way to state.
Everything else in this requirement is unchanged, including the refusal arriving before any secret is read.

#### Scenario: Headless refuses

- **WHEN** sync runs with no terminal to ask the database's password on
- **THEN** it refuses before reading any secret, naming the declared database and the option that is unset when that is the defect

#### Scenario: A value the store cannot carry whole is refused rather than trimmed

- **WHEN** a mapping would write a value carrying a newline into the database
- **THEN** the mapping is refused with the reason and the remedy named
- **AND** nothing is written, because a mirror that silently drops a byte lies about what it holds

#### Scenario: A declared slot is read, never programmed

- **WHEN** a database declares a YubiKey challenge-response slot
- **THEN** every command sync issues against that database reads the slot to answer the store's own unlock challenge
- **AND** no command sync issues anywhere creates, reprograms, or deletes that slot or any other

## ADDED Requirements

### Requirement: A declared composite key travels alongside the password, never in place of it

A database MAY declare a YubiKey challenge-response slot, a key file, or both, as additional factors the store's own command needs to open it.
Every command safix issues against that database — sync's read, write, group-creation, and listing, and the store mirror `safix enroll --mirror-to-store` writes through — SHALL carry the declared factors, and the single password prompt this capability already requires SHALL still be asked once per run alongside them.
A key file SHALL be declared as a string naming an absolute path on the machine the verb runs on, never as a nix path, for the same reason the database's own path already is one: a nix path interpolated into the declaration is copied into the world-readable store on every evaluation, and here that copy would be the very secret the factor exists to be.
A database that will not open on its declared factors SHALL be refused the same way any other unreadable database is, naming the database; safix SHALL NOT attempt to determine which declared factor, if any, was at fault.

#### Scenario: The composite key opens the database everywhere safix reaches it

- **WHEN** a database declares a YubiKey slot, a key file, or both
- **THEN** sync's read, write, group-creation, and listing commands, and the enrollment mirror's write, all carry the declared factors
- **AND** the password is still asked once per run, alongside them rather than instead of them

#### Scenario: A wrong or absent factor refuses like any other unreadable database

- **WHEN** a declared YubiKey slot does not answer, or a declared key file will not load
- **THEN** the database is reported unreadable, naming the database
- **AND** the refusal is the same one a wrong password already produces, carrying whatever the store's own command said rather than a safix-invented diagnosis
