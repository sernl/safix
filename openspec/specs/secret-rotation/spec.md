# secret-rotation Specification

## Purpose

How long a value may live: named rotation policies, the deadline every opted-in entry carries, the finding that reports a value past it, the verb that re-mints a generator-backed value, and the workstation timer that runs that verb unattended.

## Requirements

### Requirement: A rotation policy is declared once and referenced by name

The declarations SHALL carry a set of named rotation policies, each with an interval written as a whole number of days, weeks or hours. An entry SHALL opt in by naming one policy. Naming a policy the declarations do not define, or an interval that is not a positive whole number of one of those units, SHALL fail evaluation naming the entry or the policy. The policy SHALL be carried on every resolved placement so the runtime never derives a deadline from anything but the declarations and the value's own timestamp.

#### Scenario: One policy governs many entries
- **WHEN** three entries name the policy `quarterly` and its interval changes from ninety days to sixty
- **THEN** all three deadlines move on the next evaluation without any entry being edited

#### Scenario: An undefined policy is refused
- **WHEN** an entry names a policy the declarations do not define
- **THEN** evaluation fails naming the entry and the policy

#### Scenario: A malformed interval is refused
- **WHEN** a policy's interval is `90` or `1.5d` or `0d`
- **THEN** evaluation fails naming the policy and the accepted forms

### Requirement: Every opted-in entry has a deadline, shown wherever the entry is listed

An entry's deadline SHALL be the timestamp of its last write plus its policy's interval. An opted-in entry with no timestamp record SHALL be due. `safix list` SHALL show the remaining time or `due` in a column of its own; the picker SHALL show the same column, SHALL recompute it every frame so the countdown ticks while the picker is open, and SHALL tint a due row distinctly. An entry with no policy SHALL show the absent marker.

#### Scenario: The list shows time remaining
- **WHEN** an entry rotated ten days ago names a thirty-day policy
- **THEN** `list` shows twenty days remaining for it
- **AND** an entry with no policy shows the absent marker

#### Scenario: The picker counts down
- **WHEN** the picker stays open across a second boundary
- **THEN** the remaining-time cell of an opted-in entry changes without any key being pressed

#### Scenario: A due row is marked
- **WHEN** an entry's deadline has passed
- **THEN** its cell reads `due` and its row carries the due tint in the picker

#### Scenario: A value with no record is due
- **WHEN** an opted-in entry has never been written by a verb that records a timestamp
- **THEN** it is due, so a policy applied to an old value cannot wait out an interval that never started

### Requirement: `check` reports every entry past its deadline with the remedy that fits it

`safix check` SHALL report a finding for every opted-in entry whose deadline has passed, naming the entry, the policy, how long ago the deadline passed, and the remedy: `safix rotate <user> <name>` when the entry has a generator, `safix set <user> <name>` when it does not. The finding SHALL carry no value.

#### Scenario: A due generated value names rotate
- **WHEN** a generator-backed entry is past its deadline
- **THEN** `check` reports it with `safix rotate` as the remedy and exits non-zero

#### Scenario: A due typed value names set
- **WHEN** an entry without a generator is past its deadline
- **THEN** `check` reports it with `safix set` as the remedy

#### Scenario: Rotation clears the finding
- **WHEN** the entry is rotated or set
- **THEN** the next `check` carries no finding for it

### Requirement: `rotate` re-mints what a generator can re-mint

`safix rotate <user> <name>` SHALL regenerate one generator-backed entry now, with the cascade its dependents require, confirmed once unless `--yes` is given, and SHALL record the new timestamp. `safix rotate --due [<user>]` SHALL regenerate every due generator-backed entry the user holds, in dependency order with cascades merged, SHALL list every due typed entry it cannot mint with `safix set` as the remedy, and SHALL exit zero when nothing is due. `rotate` on an entry with no generator SHALL refuse naming `safix set`. The value SHALL travel exactly the path `generate` uses.

#### Scenario: One entry is rotated now
- **WHEN** `safix rotate alice api-token` runs against a generator-backed entry
- **THEN** the value is re-minted and committed, the timestamp moves, and dependents are re-minted in order

#### Scenario: Everything due is rotated in one run
- **WHEN** `safix rotate --due --yes alice` runs with two due generated entries and one due typed entry
- **THEN** the two are re-minted and committed
- **AND** the typed entry is listed as needing `safix set` and the exit status is zero

#### Scenario: Nothing due is a quiet success
- **WHEN** `safix rotate --due` runs and no entry is past its deadline
- **THEN** it reports that nothing is due and exits zero

#### Scenario: A typed value cannot be rotated by the verb
- **WHEN** `safix rotate alice mail-password` names an entry with no generator
- **THEN** the verb refuses naming `safix set alice mail-password`

### Requirement: `rotation set` and `rotation unset` edit the declaration as text

`safix rotation set <user> <name> <policy>` SHALL add or replace the entry's policy in its declaration, parsed by the real parser before staging and committed; `safix rotation unset <user> <name>` SHALL remove it. Both SHALL refuse an entry or a policy the declarations do not define, and SHALL honour the delegation records the other scaffolding verbs honour.

#### Scenario: A policy is assigned from the command line
- **WHEN** `safix rotation set alice api-token quarterly` runs
- **THEN** the entry's declaration gains `rotation = "quarterly";`, the file parses, and the commit names the act

#### Scenario: An unknown policy is refused before editing
- **WHEN** the named policy is not declared
- **THEN** the verb refuses naming the declared policies and edits nothing

### Requirement: A workstation timer runs the due rotations unattended

The home-manager consumption module SHALL offer rotation timer options: whether the timer is enabled, the repository the verb runs in, the calendar expression, and environment for the identity. When enabled on Linux it SHALL install a systemd user service running `safix rotate --due --yes` in that repository and a timer firing on that calendar. It SHALL be off by default. The option documentation SHALL state that the identity must decrypt without a prompt or a card, that the timer commits locally and pushes nothing, and that machines receive rotated values on their next rebuild.

#### Scenario: The timer is installed when enabled
- **WHEN** a home profile enables the timer naming a repository and `daily`
- **THEN** a user service and a timer exist, the service runs the due rotation in that repository, and the timer fires daily

#### Scenario: Off by default
- **WHEN** a home profile sets no rotation option
- **THEN** no rotation unit exists

#### Scenario: A rotation that needs a prompt fails the unit rather than hanging
- **WHEN** the timer fires and the identity requires a pinentry or a card
- **THEN** the service fails with the underlying refusal and the repository is left uncommitted
