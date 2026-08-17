## Purpose

Enrolling a hardware key becomes one verb with a proof at the end, instead of a hand ceremony with nothing at the end.

## ADDED Requirements

### Requirement: Enrollment provisions a fresh card without asking a human to invent anything

On a card whose PIV access is factory-default, enrollment SHALL generate the PIN and the PUK, set both, and set the management key to a random value stored on the card protected by the PIN.
The management key SHALL be stored nowhere else; the PIN and PUK SHALL reach the operator only through the custody paths below.

#### Scenario: A factory-fresh card is provisioned

- **WHEN** enrollment runs against a card with the factory PIN
- **THEN** the PIN and PUK are safix-generated, distinct, and set without prompting anyone to choose them
- **AND** the management key is random and lives only on the card, PIN-protected

#### Scenario: A provisioned card is not re-provisioned

- **WHEN** enrollment runs against a card whose PIN is no longer the factory default
- **THEN** access provisioning is skipped and the PIN is obtained from custody or one hidden prompt
- **AND** nothing attempts to change the card's existing access

### Requirement: One identity per card, generated where a terminal is required and supplied

Enrollment SHALL generate one age identity in the card's first empty retired slot, driving the generator under a pseudo-terminal that supplies the PIN, with the identity named for the person and the serial.
A touch during generation SHALL be surfaced as an instruction, and is the only act the operator performs.

#### Scenario: The one interactive step is the touch

- **WHEN** identity generation runs
- **THEN** the PIN is supplied by enrollment, not typed by the operator
- **AND** the operator is told when to touch the card

#### Scenario: More than one connected card is refused

- **WHEN** two cards are connected and no serial is named
- **THEN** enrollment refuses, naming both serials and the flag that selects one

### Requirement: Enrollment is additive custody, wired end to end

Enrollment SHALL append the identity block to the same identity file `keygen` appends to, add the card's recipient to the person's `recoveryRecipients`, regenerate the policy, re-wrap the governed files, and commit — and SHALL never remove or replace any recipient.
The primary `recipient` SHALL remain software-only; the existing hardware-recipient refusal stands.

#### Scenario: The card joins every file the person's audience covers

- **WHEN** enrollment completes for a person
- **THEN** the card's recipient appears in `recoveryRecipients` and every re-wrapped file carries its stanza
- **AND** every file that opened before still opens with what opened it

#### Scenario: A second card is a second enrollment

- **WHEN** a backup card is enrolled
- **THEN** it gets its own identity and its own recipient beside the first
- **AND** neither enrollment knows about or modifies the other

### Requirement: An enrollment ends with a decrypt proof or it has not ended

Enrollment SHALL prove the card alone opens a governed file in the person's audience — using an identity source holding only the card's stub, exercising the PIN and the touch — and SHALL report the enrollment incomplete when the proof has not passed.

#### Scenario: The proof is the real store, not a canary

- **WHEN** the proof runs
- **THEN** the file it opens is one the person's audience actually governs
- **AND** no software identity is reachable by the decryption that proves it

### Requirement: The generated PIN and PUK land in the root of trust

Enrollment SHALL write the PIN and PUK to the operator's password store — the session's secret service when the database is unlocked, else the store's own command with a single password prompt — and MAY additionally write them as a safix secret readable by the person's other recipients.
Neither SHALL appear on standard output unbidden, in an argument vector, or in an environment variable.

#### Scenario: The store receives the PIN without a human copying it

- **WHEN** enrollment stores the credentials with the database unlocked
- **THEN** they arrive through the secret service with no prompt at all
- **AND** the values travel no argument vector and no environment variable

### Requirement: No OTP slot is ever written

Enrollment SHALL NOT write, reprogram, or delete any OTP slot under any flag.
The refusal SHALL name why: a programmed challenge-response slot guards a database whose loss is permanent.

#### Scenario: The forbidden surface stays forbidden

- **WHEN** any enrollment path is exercised
- **THEN** no OTP configuration command is issued
- **AND** asking for one is refused with the hazard named

### Requirement: Enrollment is an operator's act, and delegation stays delegation

Enrollment SHALL refuse without a controlling terminal.
When a clan is declared, the recipient SHALL be registered through clan's own command; consumer-specific wiring SHALL go through `flake.safix.enrollHook`, which receives the person, the serial, and the recipient — and running without a hook succeeds, having done less, and says so.

#### Scenario: Headless invocation is refused

- **WHEN** enrollment runs with no terminal
- **THEN** it refuses before touching the card, naming why a terminal is required

#### Scenario: clan learns about the recipient from its own command

- **WHEN** a clan is declared and enrollment completes
- **THEN** the registration ran through clan's command, and safix wrote nothing into clan's store
