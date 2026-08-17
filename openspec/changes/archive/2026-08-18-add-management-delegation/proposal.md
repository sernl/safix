## Why

The scaffolding acts — `adduser`, `enroll`, editing a group — are one operator's acts today, and the custody model now describes fleets where they are not: organizations owning machines and services, people consenting to organizational escrow, managers administering other people across up to a hundred hosts.
Phase D, last of the program by design — delegation without silos and ownership in place is the operator reading everything, the configuration safix itself warns about — gives delegation a record and gives the scaffolding verbs awareness of it, while key generation stays with the person it belongs to, which is the custody principle every phase has preserved.

## What Changes

- Delegation becomes a record: `flake.safix.organizations.<o>.managers` names the people who scaffold for that organization, and a person may declare `managedBy` naming the organization whose managers scaffold for them. Both sides are declarations; neither confers a read.
- The scaffolding verbs become delegation-aware: when the target person declares `managedBy`, `adduser`-adjacent edits and `enroll` refuse an acting identity outside that organization's managers — read from the same git identity the commit will carry — and the scaffold's commit records the organization context. When no `managedBy` is declared, nothing changes.
- Group edits get a verb: `safix group add <group> <subject>` and `safix group remove <group> <subject>` write the declaration the way `adduser` and `enroll` already write records — text edits, parsed before staging, committed — with `remove` printing the same not-retroactive disclosure every narrowing carries, and both delegation-aware over groups an organization's silo declarations cover.
- The boundary is stated, not implied: delegation binds the cooperative path. The verbs refuse out-of-scope scaffolds; they are not authorization — the tree is, evaluation refuses structure, and anyone who can commit can edit declarations directly. One sentence, on the option and in the README.
- **BREAKING** for nothing: with no `managers` and no `managedBy` declared, every verb behaves exactly as today.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `custody-subjects`: the delegation records — managers on the organization, `managedBy` on the person — and what they mean and do not mean.
- `safix-cli`: the `group` verb, and the delegation awareness of the scaffolding verbs.

## Impact

Affected code:

- `modules/flake/safix`: the two record fields and their refusals (an undeclared organization, a manager the fleet does not declare).
- `crates/safix-core`: the git-identity read; the delegation check in `adduser`, `enroll`, and the new group editor; the declaration edits for group membership reusing the enrollment change's editor machinery.
- `crates/safix`: the `group` verb, usage text, refusal prose and snapshots.
- `modules/flake/checks`: fixtures where alice manages for acme and mallory is refused.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Samples use alice as acme's manager, bob as the managed person, mallory as the refused outsider.
Ordering: after `add-organization-custody`, whose records this reads; the two changes archive in that order for the shared capability history.
