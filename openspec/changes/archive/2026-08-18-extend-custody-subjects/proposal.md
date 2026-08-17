## Why

safix's audience model has one kind of subject: a person.
The operator's direction on 2026-08-17 names what the fleet — and safix as a tool for other developers — actually contains: machines and services beside people; NixOS hosts and home-manager profiles inside them or standalone on other distros; machines and services owned by people, and sometimes by corporations; people working on other people's machines and needing to share secrets with the host's owner; organizations where individuals, contractors, and groups siloed from each other must each read some things and must not read others; and managers administering other people across fleets of up to a hundred hosts.
Today every one of those relationships is either flattened into a person-to-person grant or lives outside safix entirely.

The extension is one model, not a feature list: audiences stay derived, placement stays derived from audience, custody stays with whoever holds the key — and the set of things that can hold a key and appear in an audience grows from "a person" to "a subject": a person, a machine, a service, a group of subjects.
Ownership and siloing become declarations the existing refusal machinery can see.

## What Changes

This is larger than one coherent unit and is proposed as a program with a concrete first phase; the artifacts describe the whole shape so the operator can see it before committing to the later cuts.

Phase A, this change:

- Machines become declarable subjects: `flake.safix.machines.<m>` with a recipient (the ssh-to-age of its host identity, exactly the key its system scope already decrypts with), an owner (a person, or an organization once phase C exists), and tags.
- Audiences may include machines: a person can grant an entry to a machine — the "share with the host's owner's infrastructure" case — and the file's stanzas then carry the machine's key, with placement resolving at that machine's system scope.
- Groups: `flake.safix.groups.<g>` naming a set of subjects; an audience naming a group encrypts to its members' keys, membership change re-wraps on `fix`, and a shrunk group is reported as the revocation it is.
- Silos: a declaration that named groups are mutually exclusive audiences, and an evaluation refusal for any file whose audience would span two silos — the corporate case where contractors and teams must be provably unable to read each other's material.
- Ownership as a record with consequences: `sharedWith` gains the ability to name "the owner of machine m", resolving through the declaration, so the grant survives the host changing hands by re-wrapping rather than by silently pointing at the old owner.
- The portability guarantee stated as a requirement rather than an accident: every subject-model feature works identically for a standalone home-manager profile on a non-NixOS distro.

Phases B-D, each its own future change, proposed here and not built here.
None of the three is implemented by this change, and no code in the tree anticipates one; what this section is, is the record that they exist as proposals and the order they are proposed in, which is design D5's: each later cut changes who may act rather than only what an audience can name, and each needs the vocabulary the one before it adds.

- B, `add-service-subjects`: services as subjects — a service names the machines it runs on and resolves to their keys with service-scoped placement, so "people use services and services are rendered to them" is declarable without granting the whole machine.
  After A because a service resolves to other subjects' keys, which is the algebra A establishes; its own change because service-scoped placement touches the consumption modules' surface.
- C, `add-organization-custody`: organizations as owning principals with their own recovery custody, where the escrow trade-off safix's prose already states becomes a consent-visible declaration instead of a warning.
  After A because consent needs the silo and ownership vocabulary to be sayable at all.
- D, `add-management-delegation`: managers who scaffold, never mint — `adduser`, `enroll`, and group edits performed for others across many hosts, with key generation staying with the person it belongs to.
  Last, because delegation without silos and ownership in place is the operator reading everything, which is the configuration safix warns about today.

**BREAKING** for nothing in phase A: every existing declaration is a valid declaration of the extended model, and a tree with no machines, groups, or silos declared behaves exactly as today.

## Capabilities

### New Capabilities

- `custody-subjects`: what a subject is, how machines and groups join audiences, what silos refuse, and what ownership resolves.

### Modified Capabilities

None in phase A; phases B-D will carry their own deltas.

## Impact

Affected code:

- `modules/flake/safix`: the `machines`, `groups`, and `silos` declarations; audience resolution extended over subjects; the silo and ownership refusals; policy generation over machine and group recipients.
- `modules/consume`: system scope gains machine-entry resolution beside the person-entry resolution it has.
- `crates/safix-core`: the model records for the new subjects; `check` and `fix` over group and machine audiences; the revocation report over shrunk groups.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Ordering: independent of the four proposals already open; `enroll-hardware-custody` and `add-keepassxc-sync` operate on persons and are untouched by phase A, and the audit's mapping model gains machine-held entries only when phase B gives services a reason to bridge.
Scale: groups and tags are the answer to a-hundred-hosts; nothing in phase A introduces a per-host enumeration a fleet that size would have to write by hand.
