## MODIFIED Requirements

### Requirement: What a far side is, and what stays outside that contract

A far side of a mapping SHALL be reached through one stated contract: what it needs before the first mapping is touched, the reading of one address as a value and the metadata declared beside it, the writing of both together, the enumeration of what it holds, and what it declares itself able to carry.
A refusal that only one far side can raise SHALL stay outside that contract, be raised by that far side alone, and be documented as belonging to it.
Those refusals SHALL include, and are not widened by this requirement to more than: clan's stale-generator refusal, clan's shared-placement address discovery, clan's committing in its own store repository rather than in this one, clan's ungenerated var being an ordinary absence, the keepassxc value channel's single-line limit, and the keepassxc write burst.
Clan's store repository SHALL be named as the clan flake, or the directory `CLAN_AGE_STORE_DIR` names when the clan sets `vars.settings.age.externalStore`, in which case clan commits nothing when that directory is not a git repository.

#### Scenario: Every far side answers the same five questions

- **WHEN** a far side is reached
- **THEN** it is asked what it needs to unlock, what it holds at an address, what to write there, what it holds in total, and what it can carry
- **AND** nothing else about a far side is asked through that contract

#### Scenario: A per-target refusal is not promoted into the contract

- **WHEN** a refusal only one far side can raise is located
- **THEN** it is raised by that far side, and the contract says nothing about it
- **AND** the reason it cannot be shared is recorded beside it, so a later target does not inherit a rule that was never about it

#### Scenario: What a far side can carry is something it declares

- **WHEN** a far side is asked what it can carry
- **THEN** it answers per field, naming whether it can carry that field at all and over which channel, and whether its value channel accepts more than one line
- **AND** every refusal about a field or a value's shape is derived from that answer rather than written as a special case

#### Scenario: Where clan commits is stated for both store placements

- **WHEN** the clan target's commit behaviour is documented, in the runtime or in the operator guide
- **THEN** it names the clan flake as the store repository for a clan without an external store
- **AND** it names the directory `CLAN_AGE_STORE_DIR` names for a clan with `vars.settings.age.externalStore` set, which `safix sync clan` reaches only when that variable is exported in its environment
