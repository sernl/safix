## Why

Phase A made machines and groups declarable subjects; what people actually grant secrets to, day to day, is neither — it is a service: the thing running on one or several machines that needs an API token, a database password, a person's credential rendered to it.
Today that relationship is flattened into a machine grant, which over-states the audience in the declaration (the whole machine, when one unit reads it) and under-states it in placement (nothing narrows who on the machine may open the landed file).
`extend-custody-subjects` shaped this phase and deferred one question to it — whether a service resolves to its machines' keys or carries a minted identity of its own — and this change answers it with the fleet in view.

## What Changes

- Services become declarable subjects: `flake.safix.services.<s>` names the machines it runs on, an owner, and the unix user and group its landed entries belong to.
- Audiences may include services anywhere subjects are named — grants, group membership, and through groups, silos. A service in an audience is its own rendered element, so a change to its machine set is a re-wrap of the same files, exactly as group membership changes are.
- A service resolves to its machines' keys and mints nothing. The honest sentence is stated on the option rather than implied away: a service grant narrows what is declared and placed — the audience names the service, and the landed file belongs to the service's unix user and group — while the host identity remains what decrypts, so the machine is the trust boundary for everything running on it, exactly as the provisioner has it.
- Placement is service-scoped: on a NixOS machine the entry lands owned by the service's declared user and group; on a machine served by a standalone home-manager profile, which has no ownership axis, a service that declares ownership is refused at evaluation rather than its claim being dropped — the same asymmetry the consumption modules already enforce for entry-level ownership.
- Evaluation refuses what it can see: a service naming an undeclared machine, a grant to a service whose machine set is empty, a subject-namespace collision, and the ownership-toward-user-scope case above. A declared service nothing grants to remains inert, byte for byte.
- **BREAKING** for nothing: every phase-A declaration is unchanged, and a tree with no services declared behaves identically.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `custody-subjects`: services join the subject model — declarable, audience-nameable, machine-resolved, ownership-carrying, inert until granted to.
- `secret-consumption`: system-scope resolution gains service-scoped placement; the user-scope ownership refusal extends to service grants.

## Impact

Affected code:

- `modules/flake/safix`: the `services` declaration in `options.nix`/`types.nix`; audience resolution and the new element marker in `resolve.nix` and `policy.nix`; the evaluation refusals beside phase A's.
- `modules/consume`: system-scope placement carrying the service's user and group; the user-scope refusal.
- `crates/safix-core`: the model record for the service element so `check`'s audience readings and the revocation report cover machine-set shrinks.
- `modules/flake/checks`: service fixtures in the subjects and portability suites; the fixture fleet gains one granted service.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Ordering: deltas land on the established `custody-subjects` and `secret-consumption` main specs; no other change holds deltas on either, so the history stays single-writer.
Phases C and D are unchanged by this and remain shaped, not built.
