## Context

See proposal.md — Why.
What the approach builds on:

- Phase A's subject machinery, all landed: the registry-shaped resolver over users, machines, groups and silos; audience elements as rendered references with `AUDIENCE_MARKERS` and the injectivity property test; the inertness discipline held byte-for-byte; the `safix-portability` check over the three consumption shapes; the revocation report extended for narrowed audiences.
- The consumption modules' recorded ownership asymmetry: owner and group are carried at system scope and refused rather than dropped at user scope.
- The provisioner's reality: on any machine, sops-nix decrypts with the host identity, and everything a unit reads on that host was decrypted by that one key. What separates services on a host is the host's own access control over the landed files, not cryptography.
- The fleet's services (nixflix, buildbot, stalwart, cognee, ntfy and the rest in dotfiles) each run on one or two machines under their own unix users; none has, or wants, an identity separate from its host's.

## Goals / Non-Goals

Goals: the declaration names the thing the secret is for; placement enforces the narrowing the host can actually enforce; machine-set changes are re-wraps with the same growth and shrink semantics every other audience has; inert until granted to.

Non-goals:
- Per-service identities. Rejected in D1, with the honest boundary stated instead of engineered around.
- Runtime provisioning of service credentials (systemd credentials, keyrings, sockets): the entry lands as a file the service's user owns, which is the provisioner's model; anything past that is a different provisioning system, not a custody model.
- Service discovery or unit wiring: safix records which machines a service runs on because audiences need it; it does not derive it from, or push it into, the machine's service configuration.

## Decisions

### D1. A service resolves to its machines' keys, and the boundary is stated rather than dressed up

Phase A deferred this with both options named; the fleet decides it.
A per-service identity would be a second key held by the same host — the host must read it at activation to place the service's files, so the machine's compromise story is unchanged — and it would need minting, custody, enrollment into every audience file, and rotation on service moves: ceremony without a boundary.
What a service grant genuinely narrows is the declaration (the audience names the service, and review reads who a secret is for) and the placement (the landed file belongs to the service's unix user and group, which the host enforces).
What it does not narrow is what the host identity could decrypt, and the option's documentation says so in one sentence, the same honesty `plaintext-staging` applies to its own bounds.
A machine rebuild rotates the host key and re-wraps machine-granted and service-granted files in the same `fix`, so the rebuild story phase A established does not change shape.

### D2. The audience element is the service's own mark

Phase A made audiences lists of rendered references so membership changes re-wrap rather than migrate; services take the same treatment with their own marker beside `@group` and `@~machine`, chosen from outside the name alphabet with the same injectivity argument, and the property test extended over the fourth element kind.
Resolution expands a service element to its machines' recipients at generation time, exactly as a group expands to its members'.

### D3. Ownership fields ride the service, with a per-grant override deliberately omitted

The unix user and group live on the service declaration, once, because they are properties of what the service is on its machines, not of any single secret it reads.
A per-grant override was considered and left out: nothing in the fleet needs one entry of a service owned differently from the rest, and every axis added to grants is an axis the refusals and the report must speak about.
If a real need appears it is an additive option later, not a redesign.

### D4. The user-scope refusal extends the recorded asymmetry rather than inventing a rule

A user-scope profile has no ownership axis, and the consumption modules already refuse an entry that claims one there rather than dropping the claim.
A service declaring `user` or `group`, granted an entry resolving on a machine served by a user-scope profile, takes the same refusal at evaluation, naming the missing axis; a service declaring neither resolves there with ordinary placement.
This keeps the portability requirement honest: the standalone shape refuses what it cannot honour instead of pretending, which is what "identical behaviour" has to mean when a capability is scope-specific.

### D5. Two services, one machine, one entry: the collision is a path collision

Each service's entries land under the service's own path prefix, so two services granted the same entry coexist; what is refused is two resolutions onto one literal path, through the existing collision refusal rather than a service-specific rule.
The prefix is the resolved key: an entry reached through a service resolves in its machines' sets under `<service>/<name>`, so the provisioner's own default path — already a function of the name and so unable to collide — *is* the prefix, nothing authors a path, and the only remaining way onto one literal path is a declared `path`, which the existing refusal owns.
The composed name is admissible where a declared name carrying `/` is refused because both halves are drawn from the alphabet `wellFormedName` admits: neither can be `..`, so the file lands one level inside the directory the provisioner manages rather than walking out of it.

## Risks / Trade-offs

- [The declaration can over-promise isolation to a reader who skips the option doc] → the one-sentence boundary statement lives on the option itself, and the README's service section leads with it; nothing else in the tree calls a service grant an isolation mechanism.
- [A service's machine set drifting from where the unit actually runs] → safix cannot see unit placement (a non-goal); what it guarantees is that the declared set and the audience agree, and the shrink report catches the retirement half of the drift. The enrollment of the other half — a unit moved without the declaration moving — is the consumer's configuration discipline, stated in the option doc.
- [Marker growth] → the fourth marker lands in the same `AUDIENCE_MARKERS` set with the same property test; a fifth subject kind would rightly prompt the question whether marks scale, but four is not the moment.

## Migration Plan

Additive and inert until declared; first use is declaring one fleet service over its machine and granting it the entry it reads today via a machine grant, then watching `fix` re-wrap nothing but that file's stanzas.
Rollback is removing declarations nothing else references.
Deltas land on established main specs with no concurrent writer; archive is unordered with respect to anything open.

## Open Questions

None; phase A's deferred question is answered in D1, and phases C and D stay shaped as recorded.
