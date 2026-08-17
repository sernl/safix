## Context

See proposal.md — Why.
What the approach builds on: phase C's organization records; the enrollment change's declaration editor (line-insertion edits parsed by the real parser before staging, the `Edit` outcome vocabulary, the commit discipline) as the machinery the group verb reuses; the revocation report over shrunk groups, already landed in phase A; the repository's standing non-goal of runtime authorization, which this change's boundary sentence restates rather than erodes.

## Goals / Non-Goals

Goals: delegation legible on both sides; out-of-scope scaffolds refused on the cooperative path with the acting identity the commit would carry; group membership editable by verb with the same disclosures hand edits owe; unmanaged fleets untouched.

Non-goals:
- Authorization. The refusals bind the verbs, not the repository; a hostile committer edits nix directly and evaluation, not delegation, judges the result. Stated on the option, in the README, and in the refusal prose itself.
- Bulk onboarding. A hundred hosts are groups' and tags' problem, solved; a hundred people arrive one custody record at a time by design, because each brings a key only they should mint. If genuine bulk ergonomics are ever wanted they are a future change with their own consent story.
- Manager hierarchies, roles, or per-verb grants. One flat list per organization; the first fleet that needs more will know what it needs.

## Decisions

### D1. The acting identity is the git identity, because the commit is the act

The verbs already commit every scaffold; whoever the commit will name is who is acting, so the delegation check reads exactly that (`user.name`/`user.email` as the repository resolves them) and refuses before any file is edited.
An identity flag (`--as alice`) was rejected: it would let the check and the commit disagree, and the entire value of the record is that the act and its attribution cannot separate.
The check is by declared person name matched against the committer identity through the person's declaration; where no correspondence is declarable the refusal says so rather than guessing.

### D2. `managedBy` is the person's consent, mirroring escrow's shape

Phase C put escrow consent on the person; delegation takes the identical shape for the identical reason — nothing an organization declares can subject anyone to management, and a review of bob's file shows everything that binds bob.
The organization's `managers` list is the other half, and both must agree for a scaffold to be judged at all.

### D3. The group verb reuses the declaration editor, and silo coverage decides its delegation scope

`safix group` edits declarations the way `enroll` edits custody records — the same insertion machinery, the same parsed-before-staged discipline, the same additive-first posture with `remove` carrying the disclosure `sharedWith` revocations already print.
Its delegation scope: a group covered by an organization's silo declarations is that organization's to manage, which reuses the one organizational-boundary record the model already has instead of inventing a per-group owner field; a group no silo covers is unmanaged and any committer may edit it, exactly as today.
Amended during apply with the semantics the sentence above left open, confirmed at integration: a silo set is an organization's when any of its groups' expanded membership reaches a person whose `managedBy` names it, and every group in that set is then that organization's to manage — including a group holding none of its people, because who sits on the far side of the boundary is exactly what the boundary-erecting organization must control; a per-group reading would leave the contractors list editable by anyone.
Where several organizations cover one set, a manager of any of them is in scope, since demanding all would refuse a manager acting inside their own remit.

### D4. Refusal placement is the verb layer, deliberately

Evaluation cannot refuse these — the tree after a hostile edit is structurally valid, which is the honest limit — so the delegation refusals live where the cooperative path runs: in the verbs, before any edit, with prose naming the organization, its managers' declaration site, and the boundary sentence.
This is the one refusal family in safix that guards process rather than structure, and its documentation says exactly that so nobody mistakes it for the other kind.

## Risks / Trade-offs

- [Process guards invite overtrust] → the boundary sentence appears at every surface the feature has (option, README, refusal prose), and the spec's no-read scenario pins that delegation never touches an audience.
- [Git identity spoofing] → anyone who would spoof `user.name` could edit the tree directly; the guard's threat model is mistake-prevention and attribution, stated, not spoof-resistance.
- [The silo-coverage scope for groups may surprise] → it is one sentence in the verb's usage text, and the unmanaged default means surprise takes the safe direction.

## Migration Plan

Additive and inert: no `managers`, no `managedBy`, no behaviour change anywhere.
Fixture fleet models alice managing for acme over bob, with mallory refused.
Archive after `add-organization-custody` for the shared capability history.

## Open Questions

None.
