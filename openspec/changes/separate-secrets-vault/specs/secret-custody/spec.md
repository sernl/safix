## MODIFIED Requirements

### Requirement: The audience picks the file, and one audience gets one file

A secret's audience SHALL be its owner together with every user the owner grants it to, or for a shared entry every user who carries it.
Each distinct audience SHALL resolve to exactly one encrypted file, and every secret with that audience SHALL live in it.

#### Scenario: A sole-owner audience

- **WHEN** a secret's audience is one person
- **THEN** it resolves to that person's own file
- **AND** no other person's recipient appears on it

#### Scenario: A multi-member audience

- **WHEN** a secret's audience is several people
- **THEN** it resolves to a file belonging to exactly that audience, in a directory named for its members in sorted order
- **AND** the path states who can open the file without opening it

#### Scenario: Sharing moves the value rather than widening a personal file

- **WHEN** an owner grants one of several secrets to another person
- **THEN** the granted secret moves to the audience's own file
- **AND** the owner's remaining secrets stay in the owner's file, unreadable by the grantee

#### Scenario: A multi-member audience in vault mode

- **WHEN** a secret's audience is several people and a vault is declared
- **THEN** it still resolves to a file belonging to exactly that audience
- **AND** the file's name is opaque rather than a directory named for its members in sorted order, so the path no longer states who can open it — that guarantee is scoped to the case with no vault declared
