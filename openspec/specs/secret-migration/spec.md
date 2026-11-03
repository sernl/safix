# secret-migration Specification

## Purpose

Move secrets between supported encrypted containers and recipient kinds without losing bytes, deployment metadata or the original recovery path.

## Requirements

### Requirement: Migration verifies every destination before publication

Migration SHALL encrypt to explicitly named recipients, decrypt through explicitly named target identities and compare the recovered bytes with the source. Ambient identities SHALL NOT satisfy independent verification. Existing destinations and aliases of sources SHALL be refused, except a destination the migration's own journal proves it published in an earlier interrupted run of the same plan. Sources SHALL always be retained. A verification failure SHALL publish no new ciphertext or deployment artifacts. Recoverable publication errors, including an interrupting signal delivered between steps, SHALL roll back outputs created by that run. Publication across multiple filesystem paths is not atomic: an interruption during a step MAY leave a subset of already verified outputs, and the journal SHALL make that subset recoverable on the next run of the same plan.

#### Scenario: Binary migration and reverse migration
- **WHEN** a value containing NUL, invalid UTF-8 and a trailing newline migrates between raw age and SOPS binary, then back
- **THEN** each verified destination contains exactly the source bytes
- **AND** every source remains available

#### Scenario: The target cannot decrypt
- **WHEN** target verification uses an identity outside the destination audience
- **THEN** migration refuses before publication even if an ambient source identity could decrypt it

#### Scenario: A signal between steps rolls back
- **WHEN** the process receives an interrupting signal after some outputs are published and before the run completes
- **THEN** the outputs that run published are removed, the staging directories are removed, the journal is removed
- **AND** the run exits with the interruption's own status rather than reporting success or a verification failure

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

### Requirement: An interrupted migration is journaled and resumable

Before publishing its first output, a migration SHALL write a journal beside its receipt naming the plan by a digest of the plan's content and recording each output as it is published: the output's path, its file identity and a digest of its bytes. Rerunning the same plan while its journal exists SHALL resume: every recorded output that still matches its record SHALL be re-verified against the source through the target identities and kept; every output not yet published SHALL be published; the journal SHALL be removed only after the receipt is published. A journal naming a different plan SHALL refuse, naming both plan paths. A recorded output that no longer matches its record SHALL refuse, naming the path, and SHALL NOT be removed or overwritten. A record whose path holds nothing SHALL be published rather than refused: the record is written before the output's link, so a run killed between the two names an output that was never created. The journal SHALL carry no plaintext and no digest of plaintext.

#### Scenario: A rerun finishes what the crash left
- **WHEN** a migration was killed after publishing some ciphertext outputs and the same plan is run again
- **THEN** the kept outputs are re-verified, the remaining outputs, declarations and receipt are published
- **AND** no source is modified and the journal no longer exists afterwards

#### Scenario: A rerun refuses an output someone replaced
- **WHEN** a recorded output was replaced by a file with a different identity or different bytes before the rerun
- **THEN** the rerun refuses before publishing anything, naming that path
- **AND** the replaced file, the other recorded outputs and the journal are left as they were

#### Scenario: A journal from another plan is not resumed
- **WHEN** the journal beside the receipt path was written by a plan with a different digest
- **THEN** the run refuses, naming the journal's plan and the requested plan
- **AND** nothing is published or removed

#### Scenario: A completed migration leaves no journal
- **WHEN** a migration completes without interruption
- **THEN** no journal exists beside the receipt
- **AND** a rerun of the same plan refuses the existing outputs exactly as before

### Requirement: An interrupted migration can be abandoned

`safix migrate --abandon <plan.json>` SHALL remove every output the journal records that still matches its record, remove the recorded staging directories and remove the journal. It SHALL refuse, removing nothing, when no journal exists, when the journal names a different plan, or when any recorded output no longer matches its record; a record whose path holds nothing is already gone and SHALL NOT refuse. Sources SHALL never be touched.

#### Scenario: Abandoning removes only what the journal proves
- **WHEN** an interrupted migration is abandoned
- **THEN** its recorded outputs and staging directories are gone
- **AND** its sources, and any file the journal does not record, are unchanged

#### Scenario: Abandoning refuses a replaced output
- **WHEN** one recorded output no longer matches its record
- **THEN** abandonment refuses naming that path
- **AND** every recorded output and the journal remain

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
