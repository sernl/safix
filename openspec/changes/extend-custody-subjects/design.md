## Context

See proposal.md — Why.
What the current model already holds, which the extension builds on rather than replaces:

- Audiences are sets of persons; files derive from audiences; `.sops.yaml` derives from both. Machines exist today only as placement (`perHost`, `perTag`) and as de-facto identities: the system scope decrypts with the host's ed25519 key through sops-nix's default, so every NixOS host already holds exactly the key a machine subject needs — its recipient is `ssh-to-age` of that key, the same derivation clan uses for its machine recipients.
- `sharedWith` is person-to-person, directory-named by its guest list (`shared/ana,bo/`). Revocation-is-not-retroactive is disclosed at every narrowing point.
- The custody principle: key generation belongs to the person who will hold the key; `adduser` mints nothing. Any organizational feature that would quietly centralize minting is a regression, not an extension.
- The consumption modules already run in three shapes (NixOS system, home-manager in NixOS, standalone home-manager), and the standalone shape is the portability anchor the operator named.

## Goals / Non-Goals

Goals: one audience algebra over subjects; the corporate silo provable at evaluation; ownership a grant can resolve through; a hundred hosts declarable without a hundred hand-written lines; identical behaviour across the three consumption shapes.

Non-goals:
- Runtime authorization. safix stays declarations, derivation, and refusals; nothing here evaluates permissions at run time or adds an ACL engine.
- Services, organizations-as-principals, and management delegation in this change — phases B, C, D, each its own change, shaped below.
- Weakening person custody. A person's `private` drawer stays theirs on corporate hosts exactly as on their own; what an organization owns is what is declared under its ownership, never a side-effect over someone's drawer.

## Decisions

### D1. Machines are subjects in the same algebra, not a parallel mechanism

The alternative — a second, machine-specific grant surface — would double every rule (two audience computations, two policy renderers, two revocation reports).
A machine is a subject with a recipient like any other; what differs is only where its entries land (system scope) and that its recipient decrypts non-interactively by nature, which is why the hardware-recipient refusal does not transfer to it.
The recipient is `ssh-to-age` of the host key the machine already decrypts with: no new identity, no enrollment step, and the same derivation the clan bridge's far side uses.

### D2. Group audiences are named by the group, and the guest-list naming stays for ad-hoc shares

`shared/ana,bo/` is readable precisely because the list is short; a hundred-member group in a directory name is not a name.
A group audience derives its directory from the group (`shared/@<group>/`), the `@` marking that the audience is resolved through a declaration rather than enumerated in place; membership lives in one place and the re-wrap follows it.
Ad-hoc person-to-person shares keep the guest-list form unchanged — the two forms answer different questions and both stay derived.

### D3. Silos are an evaluation constraint over audiences, which is the only place they can be strong

A silo enforced at read time is a policy hoping nobody misconfigured a file; a silo enforced where audiences are computed is a file that cannot exist.
The declaration names groups as mutually exclusive; the refusal fires on any would-be audience spanning two, listing every violation at once in the resolver's existing style.
Deliberately not transitive over ownership: a person may own machines in two silos — the operator administering both sides is the normal case — and what is refused is a single file readable from both, not a person's existence in both worlds.

### D4. Ownership is a resolution record, and "share with the owner" is its first consumer

Ownership's phase-A job is exactly one: letting a grant say `ownerOf.<machine>` and resolve through the declaration, so the grant follows the record.
It deliberately confers no powers in phase A — an owner does not thereby read the machine's entries or manage its users — because powers are phase C and D questions, and a record that silently granted them would be the escrow warning safix already prints, arrived at by accident.

### D5. The phases exist because each later cut changes who can act, not just what can be named

Phase A extends what audiences can name; nothing about who may run which verb changes.
Phase B (services) adds a subject that resolves to other subjects' keys with narrower placement — mechanically like groups, worth its own change because service-scoped placement touches the consumption modules' surface.
Phase C (organizations) makes owners principals with recovery custody of their own, which is where the escrow trade-off becomes a consent-visible declaration; it must not precede A, because consent needs the silo and ownership vocabulary to be sayable.
Phase D (management) delegates scaffolding — `adduser`, `enroll`, group edits — across a fleet, and is last because delegation without silos and ownership in place is the operator reading everything, which is the configuration safix warns about today.

### D6. Portability is a requirement with a check, not a hope

The spec's scope-portability requirement gets held the way consumption claims already are: the module-evaluation checks run every subject-model refusal and resolution over all three consumption shapes, standalone home-manager included, and a divergence is a red check rather than a footnote.

## Risks / Trade-offs

- [Model growth is complexity growth] → phase A adds three declarations and two refusals to one algebra; the parallel-mechanism alternative was rejected for doubling everything, and phases land only when their questions are real.
- [A group re-wrap over a hundred hosts is a large `fix`] → it is one derived re-wrap, batched as `fix` already batches; the alternative — hand-enumerated audiences — is the thing that does not scale.
- [Silo pairs grow quadratically if declared pairwise] → the declaration names silo sets, not pairs; membership in more than one silo set is itself refused for a group, keeping the constraint linear.
- [Ownership changes re-point grants, which can surprise the old owner] → the re-wrap is reported as the narrowing it is, with the same not-retroactive disclosure; silent continuity toward the old owner was the alternative and is worse.

## Migration Plan

Phase A is additive and inert until declared: no machines, groups, or silos declared means byte-identical output, which is itself a check (the spec's inertness scenario).
First real use on this fleet: declare the fleet's hosts as machines with the operator as owner, and one group; nothing re-wraps until a grant names them.
Phases B-D are separate changes proposed when A has landed and its model has survived contact with the fleet.
Rollback of A is removing declarations nobody else references.

## Open Questions

- Whether phase B's service subject should resolve to machine keys (a service reads what its machines can read, placement-narrowed) or carry its own minted identity per service. The first is simpler and keyless; the second survives a machine rebuild without re-wrap. Deferrable: phase B's own design decides with the fleet's services in view.
- Whether the `@` group-audience marker is the right spelling for the derived directory name. Deferrable: it is a rendering choice inside the policy generator, and the checks hold whatever is chosen.
