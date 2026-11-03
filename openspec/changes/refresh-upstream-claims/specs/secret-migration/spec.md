## MODIFIED Requirements

### Requirement: Migration preserves deployment declarations

A successful migration SHALL publish usable declarations for the chosen safix, sops-nix or agenix consumer and a value-free receipt preserving source and destination mappings, paths, modes, owners, groups and requested lifecycle hooks. A declaration is usable when it evaluates under every consumer release at or above the minimum the migration documentation states for that target. Generated declarations SHALL copy ciphertext into the Nix store when their consumer requires store paths, without copying plaintext or identities. A target that cannot express a requested semantic, including unsupported hooks, templates or early-user permissions, SHALL be refused before publication rather than silently dropping that semantic.

#### Scenario: Custom installation contract survives conversion
- **WHEN** a secret with a custom path, mode and supported service hook is migrated
- **THEN** the generated consumer declaration retains those settings
- **AND** the receipt contains no plaintext or low-entropy plaintext digest

#### Scenario: A later candidate fails verification
- **WHEN** an earlier candidate verifies but a later candidate cannot be decrypted by its target identity
- **THEN** no candidate, declaration or receipt is published
- **AND** all source ciphertext remains unchanged

#### Scenario: The minimum consumer version is stated where the targets are documented
- **WHEN** the documentation of the migration's deployment targets is read
- **THEN** it states the minimum agenix release the agenix target's declarations are written for
- **AND** it states the minimum sops-nix revision a JSON destination with a nested key needs

## ADDED Requirements

### Requirement: The agenix target carries a root-owned early-user entry as an ordinary entry

Migration to the agenix target SHALL accept an entry whose deployment asks for installation before user creation when that entry is root-owned, and SHALL publish it as an ordinary agenix entry that does not name the request, because agenix installs every secret before users are created.
The receipt SHALL still record the request.

#### Scenario: A root-owned early-user entry migrates to agenix
- **WHEN** a plan targeting agenix carries a whole raw-age entry with `neededForUsers: true` and no non-root owner or group
- **THEN** the migration succeeds
- **AND** the generated declaration holds the entry without a `neededForUsers` field
- **AND** the receipt records `neededForUsers: true` for that entry

#### Scenario: A non-root early-user entry is still refused
- **WHEN** a plan targeting agenix carries an entry with `neededForUsers: true` and a non-root owner
- **THEN** the migration refuses before publication, stating that early-user secrets must be root-owned

#### Scenario: Hooks and templates are still refused for agenix
- **WHEN** a plan targeting agenix carries a restart or reload unit, or a template
- **THEN** the migration refuses before publication, naming the semantic agenix cannot carry

### Requirement: The agenix target names a group for every entry

Every entry the agenix target declares SHALL name its group.
Where the plan gives a group or a gid, that is the group; otherwise the group SHALL be the one agenix 0.18.0 defaults to for the entry's owner, so an owner without a group evaluates under agenix releases whose own default fails for a user that has no group attribute.

#### Scenario: An owner without a group evaluates on nix-darwin
- **WHEN** a plan targeting agenix names an owner and no group or gid
- **THEN** the generated declaration names a group that resolves to the owner's primary group where the owner's user declares one, and to `0` where it does not

#### Scenario: A plan's own group is kept
- **WHEN** a plan targeting agenix names a group or a gid
- **THEN** the generated declaration names that group, exactly as before this requirement

#### Scenario: An entry with no owner names the root group
- **WHEN** a plan targeting agenix names no owner, uid, group or gid
- **THEN** the generated declaration names group `0`
