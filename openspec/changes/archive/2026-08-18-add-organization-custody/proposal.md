## Why

The custody model knows people, machines, services and groups; it does not know the thing that owns fleets and employs people.
Phase A gave ownership a record and deliberately no powers; safix's own prose has always named the escrow trade-off as a warning — listing an operator-held identity in a person's `recoveryRecipients` buys recoverability at the price of that operator reading everything — and a warning is where the corporate case currently ends.
Phase C, shaped by `extend-custody-subjects` and directed by the operator, turns both into declarations: organizations as principals with recovery custody of their own, and escrow as something a person consents to by name, in their own record, rather than something assembled out of raw keys.

## What Changes

- Organizations become declarable principals: `flake.safix.organizations.<o>` — `acme`, canonically — carrying the organization's own recovery custody: the escrow identities it holds, anchored and noted like a person's `recoveryRecipients`.
- Escrow becomes consent written where it acts: a person declares `escrowedTo` naming the organization, and every file their audience covers gains the organization's custody keys at the next re-wrap. The organization cannot add itself to anyone; the declaration lives in the person's record, reads as consent, and carries the trade-off sentence safix used to print as a warning.
- Rotation moves to one place: the organization rotates its custody keys in its own declaration, and every consenting person's files re-wrap in one `fix` — the property raw-key escrow never had.
- Ownership grows its intended referent: a machine's or service's `owner` may name an organization, and `ownerOf` grants resolve through it to the organization's custody keys.
- Organizations may be named in grants directly, as their own audience element expanding to their custody keys; groups may not contain organizations — a principal is not a member.
- Evaluation refuses what it can see, each listing every violation at once: `escrowedTo` naming an undeclared organization; escrow toward, a grant to, or ownership resolving through an organization whose custody is empty; a subject-namespace collision extended over organizations.
- **BREAKING** for nothing: declared, unreferenced organizations are byte-inert.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `custody-subjects`: organizations join the model — declarable principals with custody, consent-visible escrow, ownership referents, grantable audience elements, inert until referenced.

## Impact

Affected code:

- `modules/flake/safix`: the `organizations` declaration; `escrowedTo` on the person; audience resolution over the organization element and through `ownerOf`; the evaluation refusals beside the existing families.
- `crates/safix-core`: the organization element in the audience alphabet; the revocation report covering a person withdrawing consent and an organization shrinking its custody.
- `modules/flake/checks`: organization fixtures in the subjects, portability and byte-inertness suites; the fixture fleet gains `acme` with one consenting person and one owned machine.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Samples throughout use the community's names: `acme` for the organization, `alice` and `bob` for people.
Ordering: lands after the convention-name sweep and before `add-management-delegation`, which reads the records this change creates.
