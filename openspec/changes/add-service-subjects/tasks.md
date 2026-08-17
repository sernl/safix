## 1. The declaration

- [ ] 1.1 Add `flake.safix.services.<s>` to the option surface: machines, owner, unix user and group (both nullable), tags — with the boundary sentence in the option's own description (design D1) and descriptions in the types' voice
- [ ] 1.2 Evaluation refusals beside phase A's, each listing every violation at once: a service naming an undeclared machine; a grant to a service with an empty machine set; a subject-namespace collision extended over services; ownership declared toward a machine served by a user-scope profile (design D4)
- [ ] 1.3 Verify inertness: declared, ungranted services leave `policyText`, `audiences`, `placements` and `publicPaths` byte-identical, extending the phase-A fixture
- [ ] 1.4 Module-evaluation tests for each refusal and for a well-formed service's projection

## 2. The audience element

- [ ] 2.1 Add the service marker to `AUDIENCE_MARKERS` with the injectivity argument written beside the existing ones, and extend the property test over the fourth element kind (design D2)
- [ ] 2.2 Resolution expands a service element to its machines' recipients at generation time; groups may include services
- [ ] 2.3 `fix` re-wraps on machine-set growth; `check` reports machine-set shrink as a revocation with rotation named and the not-retroactive disclosure carried
- [ ] 2.4 Verify: expansion, growth re-wrap, shrink revocation, and the same-files-no-move property over a fixture whose service gains and loses a machine

## 3. Service-scoped placement

- [ ] 3.1 System-scope resolution lands service-granted entries under the service's own path prefix with its declared user and group
- [ ] 3.2 The path-collision refusal covers two services resolving one entry onto one literal path (design D5), and a service with no ownership resolves at user scope with ordinary placement
- [ ] 3.3 Verify placement across the three consumption shapes in the portability check: system carries ownership; user-scope refuses the ownership claim and accepts the ownerless service; answers otherwise identical

## 4. The runtime's reading

- [ ] 4.1 Carry the service element in the model record so audience readings parse it, and the revocation report names a departed machine through the service it left
- [ ] 4.2 Verify: a fixture with a shrunk service produces the revocation finding naming service and machine, and the fixture fleet gains one granted service exercised by the existing custody checks

## 5. The record

- [ ] 5.1 README: one service section, led by the boundary sentence, concise; the subjects narrative gains the fourth kind
- [ ] 5.2 CHANGELOG under Unreleased: the declaration, the element, the placement, the refusals
- [ ] 5.3 Verify: `openspec validate add-service-subjects --strict` passes
