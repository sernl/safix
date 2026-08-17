## 1. The records

- [x] 1.1 Add `organizations.<o>.managers` and `users.<u>.managedBy`, refusing dangling references on either side, with the boundary sentence on both options and descriptions in the types' voice
- [x] 1.2 Verify: the records place no key in any audience (byte-inertness over a managed fixture), and both dangling refusals fire listing every violation

## 2. The acting identity

- [x] 2.1 Read the acting identity from the repository's resolved git identity, matched to a declared person; a commit identity no person declares is its own named refusal when a delegation check is reached (design D1)
- [x] 2.2 `enroll` and the onboarding record edits refuse an out-of-scope actor before any file is edited, naming the organization and its managers' declaration site; permitted scaffolds record the organization context in the commit
- [x] 2.3 Verify: alice-for-bob proceeds with the context recorded; mallory-for-bob refuses before editing; an unmanaged target never consults delegation — three fixtures, refusal snapshots paired

## 3. The group verb

- [ ] 3.1 `safix group add|remove <group> <subject>` over the declaration editor: one inserted or removed line, parsed before staging, committed naming the act; undeclared groups and subjects refused
- [ ] 3.2 `remove` prints the not-retroactive disclosure naming the revocation report; delegation scope over silo-covered groups per design D3, unmanaged groups editable as today
- [ ] 3.3 Verify: the addition lands as one line and the policy re-derives; the removal's next `check` carries the revocation finding; the silo-covered refusal fires for mallory and not for alice; usage scaffold and verb table extended with the tests that hold them

## 4. The record

- [ ] 4.1 README: the delegation section led by the boundary sentence; samples alice, bob, acme, mallory
- [ ] 4.2 CHANGELOG under Unreleased
- [ ] 4.3 Verify: `openspec validate add-management-delegation --strict` passes, and the archive-order note against `add-organization-custody` is recorded
