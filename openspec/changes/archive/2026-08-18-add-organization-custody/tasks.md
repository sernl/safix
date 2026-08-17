## 1. The principal

- [x] 1.1 Add `flake.safix.organizations.<o>`: custody (anchored, noted escrow identities, the `recoveryRecipient` shape), with descriptions in the types' voice and `acme` as the example
- [x] 1.2 Extend the subject-namespace collision refusal over organizations, and add the empty-custody refusal covering escrow, grants, and ownership resolution, each listing every violation at once
- [x] 1.3 Verify inertness: declared, unreferenced organizations leave the four projections byte-identical, extending the existing fixture

## 2. Consent

- [x] 2.1 Add `escrowedTo` on the person, refusing undeclared organizations, with the trade-off sentence in the person's view on the option
- [x] 2.2 Expansion adds the organization's custody keys to every file the person's audience covers, beside `recoveryRecipients` and never through it (design D3)
- [x] 2.3 `fix` re-wraps on custody rotation with no person's declaration changing; withdrawal reports as a revocation with the disclosure carried
- [x] 2.4 Verify: consent widens alice's files to acme's keys; rotation re-wraps in one place; withdrawal produces the revocation finding

## 3. Ownership and audiences

- [x] 3.1 `owner` on machines and services accepts an organization; `ownerOf` resolves through it to custody keys (design D4)
- [x] 3.2 The organization audience element: fifth marker with the injectivity argument beside the others; grants may name organizations; groups refuse them as members
- [x] 3.3 Verify: the acme-owned fixture machine's `ownerOf` grant carries acme's keys; an owner change re-wraps; the group-membership refusal fires naming both

## 4. The runtime's reading

- [x] 4.1 Carry the organization element in the audience alphabet (`AUDIENCE_MARKERS` plus the mapped property strategy) and in the revocation report, naming a withdrawn consent and a shrunk custody through the organization
- [x] 4.2 Verify: fixtures for both narrowings produce findings naming the organization; the fixture fleet gains `acme`, one consenting person, one owned machine, exercised by the existing custody checks

## 5. The record

- [x] 5.1 README: the organization section — the principal, the consent, the rotation property — concise, led by the person's-view trade-off sentence; samples are acme, alice, bob
- [x] 5.2 CHANGELOG under Unreleased
- [x] 5.3 Verify: `openspec validate add-organization-custody --strict` passes
