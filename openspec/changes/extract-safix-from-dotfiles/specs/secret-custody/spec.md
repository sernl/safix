## Purpose

Who can read a secret: the user record under `flake.secrets.users`, the audience algebra over carrying, private declaration and outbound sharing, the placement scopes that adjust where a secret lands without changing who owns it, the refusals that make an incoherent custody claim an evaluation error, and the limits of revocation.

## ADDED Requirements

### Requirement: The user record carries custody and nothing else

A safix user record SHALL carry only `recipient`, `recoveryRecipients`, `carries`, `private`, `sharedWith`, `perHost`, and `perTag`.
It SHALL NOT carry account, profile, or host-membership information of any kind.

#### Scenario: The declarable fields

- **WHEN** a user is declared
- **THEN** the record accepts their recipient, further recipients of their own custody, the catalogue entries they carry, the secrets they declare alone, the secrets they grant outward, and their per-host and per-tag adjustments
- **AND** it accepts no uid, shell, home directory, group, or module-selection field

#### Scenario: A recipient is a public key and never an identity

- **WHEN** a recipient is recorded
- **THEN** the value is a public recipient
- **AND** no field in the record names, escrows, or deploys a private key

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

### Requirement: Carrying is not sharing

Two users carrying one catalogue entry SHALL by default hold independent values in separate files.
An entry marked shared SHALL resolve to one value in one file whose audience is every user who carries it.

#### Scenario: The default is independent values

- **WHEN** two users carry the same catalogue entry and it is not marked shared
- **THEN** each resolves it to their own audience's file
- **AND** setting one value leaves the other untouched

#### Scenario: A shared entry is one value

- **WHEN** two users carry an entry marked shared
- **THEN** both resolve the same file and the same bytes
- **AND** a user joining the carriers can read the existing value

#### Scenario: A shared carrier leaving requires rotation, not a re-wrap

- **WHEN** a user stops carrying a shared entry
- **THEN** the arrangement records that the value needs minting anew rather than merely re-wrapping
- **AND** that finding is derived from the file's own recipient stanzas rather than from any stored record of the former audience

### Requirement: Placement scopes adjust where a secret lands, never who owns it

Per-host and per-tag scopes SHALL adjust which secrets resolve on a given host, and SHALL NOT change any secret's audience.

#### Scenario: Omitting on one host preserves custody

- **WHEN** a user omits one of their secrets on a particular host
- **THEN** they remain in that secret's audience
- **AND** the secret simply does not land on that host

#### Scenario: A shared entry reached only through a host-scoped selection is refused

- **WHEN** a user resolves a shared entry solely through a per-host or per-tag selection
- **THEN** evaluation fails naming the user, the entry, and the host
- **AND** the message states that a host-scoped selection puts nobody in the audience, and directs the declaration to the carrying option instead

#### Scenario: Deny wins within one resolution

- **WHEN** a scope both omits and re-adds a secret
- **THEN** the re-adding slot beats the omitting slot within that resolution
- **AND** the rule is the same one the consumer's other scope resolutions use

### Requirement: A custody claim that cannot be satisfied fails at evaluation

Every declaration that names an audience which cannot be encrypted to, or that states one audience twice in ways that can disagree, SHALL fail evaluation with a message naming the offending declaration.

#### Scenario: A keyless owner

- **WHEN** a user declares or is granted a secret while recording no recipient
- **THEN** evaluation fails naming that user and that secret
- **AND** the message states that there is no key to wrap the file's data key for

#### Scenario: A keyless carrier of a shared entry

- **WHEN** a user carries a shared entry while recording no recipient
- **THEN** evaluation fails naming that user and that entry
- **AND** the message states that no copy can be encrypted to them

#### Scenario: Two mechanisms naming one audience

- **WHEN** an entry is marked shared and is also granted by an owner to a named recipient
- **THEN** evaluation fails naming the entry, the owner, and the recipient
- **AND** the message directs the declaration to one mechanism, since both name an audience and one audience picks one file

#### Scenario: A shared private entry

- **WHEN** a secret declared under a user's private declarations is marked shared
- **THEN** evaluation fails naming that user and that secret
- **AND** the message states that a private entry has no carriers but its holder

#### Scenario: A name declared twice by one user

- **WHEN** a user both carries a catalogue entry and declares a private entry of the same name
- **THEN** evaluation fails naming the user and the name

#### Scenario: A grant naming something that does not exist

- **WHEN** a grant names a user who is not declared, or names a secret the owner declares in neither carried nor private form
- **THEN** evaluation fails naming the grant and the missing element

#### Scenario: A grant colliding with the recipient's own declaration

- **WHEN** a grant hands a name to a user who already declares that name themselves
- **THEN** evaluation fails naming both declarations
- **AND** the message states which of the two must be dropped

### Requirement: Revocation is not retroactive and rotation is never automatic

Narrowing an audience SHALL stop future encryptions from reaching the removed party and SHALL NOT be presented as taking back what they have already read.
No evaluation or rebuild SHALL claim to detect a removal or to rotate on one.

#### Scenario: What narrowing an audience does

- **WHEN** a grant is dropped or a recipient is cleared
- **THEN** the recipient policy and the re-wrap of governed files stop reaching that party
- **AND** the arrangement states that the value they already read stays read, and that only minting a new value revokes it

#### Scenario: Rotation on revoke cannot be automatic

- **WHEN** the removal of a recipient is considered as a trigger
- **THEN** the arrangement states that an evaluation sees only the audience that is declared, never the audience that used to be
- **AND** no rebuild is presented as able to detect the removal

#### Scenario: Re-wrapping is aligned policy, not revocation

- **WHEN** governed files are re-wrapped to the audiences now declared
- **THEN** the operation is described as aligning ciphertext with policy
- **AND** it is explicitly distinguished from revocation in the command's own output

#### Scenario: The statement appears where the choice is made

- **WHEN** a person reads the option that grants a secret outward, the option that records a recipient, or the generated policy file
- **THEN** the non-retroactivity of revocation is stated in each of those places
- **AND** the named alternative — minting a new value — is stated with it

### Requirement: Independent custody is expressible and its cost is disclosed

A person SHALL be able to hold secrets that no other party, the operator included, can decrypt.
Where that is chosen, the absence of any recovery path SHALL be stated where the choice is recorded.

#### Scenario: A person's rule names only their own recipients

- **WHEN** a person records a recipient and no recipients belonging to another party
- **THEN** their rule's recipient list contains only recipients they hold
- **AND** no operator-held, master, or shared identity appears in it

#### Scenario: The disclosure is present at the point of choice

- **WHEN** a person is offered custody of their own secrets
- **THEN** the disclosure states that no party recovers their secrets if their key is lost
- **AND** states that listing a further recipient held by another party is the escrowed alternative, and that it means that party can read everything they hold

#### Scenario: A second recipient of the person's own choosing

- **WHEN** a person chooses independent custody
- **THEN** they are directed to record a second recipient they themselves hold
- **AND** the direction states that this is the mitigation which removes the single point of loss without surrendering independence

#### Scenario: Recovery cannot be retrofitted after loss

- **WHEN** a person has lost their only key
- **THEN** no change to the recipient policy recovers the affected files
- **AND** the arrangement does not present after-the-fact recovery as available
