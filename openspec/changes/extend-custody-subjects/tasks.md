## 1. Machines as subjects

- [x] 1.1 Add `flake.safix.machines.<m>`: recipient (the age form of the host identity), owner, tags, with descriptions in the types' voice and the inertness property stated on the option
- [x] 1.2 Verify inertness: a fixture tree with machines declared and no grant naming them generates byte-identical rules and files to the tree without them
- [x] 1.3 Extend audience resolution so a grant may name a machine; the derived file carries the machine's recipient; the hardware-recipient refusal is documented as not transferring, with the reason
- [x] 1.4 System-scope consumption resolves machine-granted entries with the host identity it already uses; module-evaluation test over a granted fixture

## 2. Groups

- [x] 2.1 Add `flake.safix.groups.<g>`: members are subjects (people, machines, groups); cycles among groups refused at evaluation naming the participants
- [x] 2.2 Audience expansion at evaluation: a group-named audience encrypts to the expanded membership; the derived directory is the group-marked form (design D2), and ad-hoc guest-list naming is untouched
- [x] 2.3 `fix` re-wraps on membership growth; `check` reports membership shrink as a revocation with rotation as the remedy and the not-retroactive disclosure
- [x] 2.4 Verify: expansion, growth re-wrap, shrink revocation report, and the group cycle refusal, each over fixtures

## 3. Silos

- [x] 3.1 Add the silo declaration over named groups: silo sets, with a group in two sets refused (design D3's linearity)
- [x] 3.2 The cross-silo audience refusal at evaluation, listing every violating grant at once
- [x] 3.3 Verify: a would-be cross-silo file never generates a rule; the refusal names both silos and the grant; a person owning machines in two silos is not itself refused

## 4. Ownership

- [x] 4.1 Grants may name `ownerOf.<machine>`, resolving through the machine's declared owner at evaluation
- [x] 4.2 Owner change plus `fix` re-wraps toward the new owner, reported as a narrowing with the not-retroactive disclosure toward the old
- [x] 4.3 Verify both, over a fixture whose ownership changes between two runs

## 5. Portability

- [x] 5.1 Run every subject-model refusal and resolution check over all three consumption shapes — NixOS system, home-manager in NixOS, standalone home-manager — and fail on divergence (design D6)
- [x] 5.2 Verify explicitly on the standalone shape that nothing evaluated requires NixOS anywhere in the fleet

## 6. The record and the program

- [x] 6.1 README: the subject model in the narrative's own progression — machines, groups, silos, ownership — concise; CHANGELOG under Unreleased
- [x] 6.2 Name the follow-up changes in this change's record as proposed and not built: `add-service-subjects`, `add-organization-custody`, `add-management-delegation`, in that order, with design D5's reason for the order
- [x] 6.3 Verify: `openspec validate extend-custody-subjects --strict` passes
