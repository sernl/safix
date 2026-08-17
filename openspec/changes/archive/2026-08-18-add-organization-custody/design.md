## Context

See proposal.md — Why.
What the approach builds on: phase A's registry resolver and rendered-reference audience elements with their marker alphabet and injectivity property; phase B's fourth element and its expansion discipline; the revocation report over narrowed audiences; `recoveryRecipients`' existing semantics — additive recipients over everything a person holds — whose documentation carries the escrow warning this change converts into a declaration.

## Goals / Non-Goals

Goals: escrow as legible consent in the person's record; organizational custody rotated in one declaration; ownership resolving to a principal rather than dead-ending at a name; inert until referenced.

Non-goals:
- Organization membership. People relate to an organization here in exactly one way — consenting to its escrow — and groups already express every people-set an audience needs. A `members` list would be a second groups mechanism.
- Scoped escrow (only work entries, only some files). `recoveryRecipients` semantics are whole-custody and honest about it; per-entry escrow classes are a real feature with real complexity, and nothing requested it. If it ever arrives it is entry-level vocabulary, not a weakening of this declaration's stated breadth.
- Any runtime power. Owning a machine or holding escrow confers reads through cryptography and nothing through the verbs; management of people and subjects is phase D's, built on these records.

## Decisions

### D1. Escrow consent lives on the person, structurally

The alternative — `organizations.<o>.covers = [ alice ]` — puts the widening of alice's audience in a file alice may never review.
The declaration therefore sits on the person (`escrowedTo`), the organization's side holds only its own keys, and the refusal families make the asymmetry structural: nothing an organization declares can widen anyone's audience.
This is the same shape consent takes everywhere else in safix — `sharedWith` is the sharer's declaration, `recoveryRecipients` the holder's — extended to the one relationship that was previously assembled from raw keys with a warning attached.

### D2. The organization is an audience element like every subject before it

Fifth marked kind, same rendered-reference treatment, same injectivity argument, expansion to the organization's custody keys at generation time — so custody rotation re-wraps the same files, exactly as group membership and service machine-sets already behave.
The marker joins `AUDIENCE_MARKERS` and the property test covers it by construction, since phase B made the strategy map the constant.

### D3. `escrowedTo` expands beside `recoveryRecipients`, not through it

Expansion adds the organization's keys to the person's every-file recipient set at resolution time; it does not rewrite the person's `recoveryRecipients` record.
Writing keys into the person's record would break rotation-in-one-place (the property this change exists to add) and would blur whose declaration holds what: the person holds the consent, the organization holds the keys.

### D4. Ownership referents stay strings, validated against the widened subject space

Phase A typed `owner` as a subject name; organizations join the space that name is checked against, and `ownerOf` resolution branches on what the name denotes — a person's recipients or an organization's custody keys.
A typed union was considered and declined: the name space is already collision-refused, so the string plus the check is the same guarantee with none of the module-type ceremony.

## Risks / Trade-offs

- [Whole-custody escrow can surprise a person who consents casually] → the trade-off sentence lives in the option the person edits, in their own view ("acme's custody can open everything you hold"), and the withdrawal scenario carries the not-retroactive disclosure. The declaration being in their file is the mechanism that makes the surprise reviewable.
- [An organization's custody keys are high-value] → they are exactly as protectable as any recipient key safix already handles, and rotating them is now one declaration plus one `fix` — cheaper than the raw-key alternative this replaces, which is the mitigation.
- [Marker growth again] → fifth kind on the mapped constant; the question "do marks scale" is now real and answered for this program: phase D adds no subject kind, so the alphabet is complete at five.

## Migration Plan

Additive and inert.
First fleet use: declare `acme`... there is no acme; first real use is the operator's own organization if one ever exists, and the fixture fleet carries the model meanwhile — which is the honest statement of who this phase is for: safix as a tool other developers adopt, per the operator's portability directive.
Rollback is removing declarations nothing references.
Archive order: before `add-management-delegation`, which reads these records.

## Open Questions

None.
