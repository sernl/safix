## ADDED Requirements

### Requirement: A hybrid post-quantum age recipient is an age recipient

Wherever a recipient field accepts a native age recipient, it SHALL also accept a hybrid X25519 + ML-KEM-768 recipient, written `age1pq1` followed by 1952 bech32 characters.
Such a recipient SHALL be rendered into the recipient policy and wrapped into sops files like any other age recipient.
A recipient beginning `age1yubikey1` SHALL remain a hardware recipient and SHALL NOT be taken for a hybrid one.

#### Scenario: A person's primary recipient is hybrid

- **WHEN** a person's `recipient` is a well-formed hybrid recipient
- **THEN** evaluation accepts it and the person's rule in the generated policy names it
- **AND** a file in that person's audience is wrapped to it by sops

#### Scenario: A further recipient is hybrid

- **WHEN** a hybrid recipient is listed under a person's `recoveryRecipients`
- **THEN** it is accepted and every file in that person's audience is wrapped to it as well as to their primary recipient

#### Scenario: A card stays a card

- **WHEN** a recipient beginning `age1yubikey1` is offered where a hybrid recipient would be accepted
- **THEN** it is judged as a hardware recipient, never as a hybrid or native one

### Requirement: A native-age file does not mix hybrid and classic recipients

A write to a native-age (`format = "age"`) destination whose recipient set holds both a hybrid recipient and a classic one (X25519 or SSH) SHALL be refused before any ciphertext is produced, naming the mix as the cause.
A sops-format file MAY mix them, and the documentation SHALL state that such a file is only as resistant to a quantum adversary as its weakest recipient.

#### Scenario: A mixed native-age write is refused

- **WHEN** a native-age destination is written to one hybrid and one X25519 recipient
- **THEN** the write is refused before `age` runs
- **AND** the destination is left as it was

#### Scenario: A migration plan that mixes them is refused when read

- **WHEN** a migration plan names an `age`-format destination with one hybrid and one classic recipient
- **THEN** the plan is refused before any source is decrypted or any destination is written

#### Scenario: An all-hybrid native-age write succeeds

- **WHEN** a native-age destination is written only to hybrid recipients
- **THEN** the write succeeds and each recipient's identity decrypts it

#### Scenario: A sops file mixes them

- **WHEN** a sops-format file's audience holds a hybrid recipient and an X25519 recipient
- **THEN** the file is wrapped to both
- **AND** either identity decrypts it
