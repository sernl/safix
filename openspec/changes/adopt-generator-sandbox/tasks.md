## 1. The envelope command

- [x] 1.1 Add the sandbox module to `safix-core`: a pure construction that takes the staging root, the store path set, and the network grant and returns the bubblewrap argv — clan's argv at the pinned rev, minus the uid pair per design D3
- [x] 1.2 Add the darwin half: the `sandbox-exec` profile construction from the same inputs, adopting clan's deny-default profile with the staging paths granted and outbound network only under the grant
- [x] 1.3 Add the availability probe, run once per generation run before the first fragment: bubblewrap answers on linux, `/usr/bin/sandbox-exec` existence answers on darwin, any other platform is refused as having no envelope
- [x] 1.4 Unit-test the constructions unconditionally: default argv and profile, the network variant of each, and the probe's three answers

## 2. The wiring

- [x] 2.1 Resolve bubblewrap through the same mechanism `runtimeInputs` resolve, so no preinstalled tool is assumed (design D2)
- [x] 2.2 Wrap the mint spawn in `generate.rs` with the envelope command
- [x] 2.3 Wrap the validation spawn with the same envelope under the same generator's grant, and confirm the candidate still arrives on standard input through it
- [x] 2.4 Add the no-backend refusal to `error.rs`, naming the backend looked for and what supplies it, raised before any fragment runs
- [x] 2.5 Verify: the existing generator and validation suites pass unchanged inside the envelope, which is the claim that the envelope changed what a fragment may reach and nothing about what it receives

## 3. The declaration

- [x] 3.1 Add `network` to the generator submodule in `modules/flake/safix/types.nix`, default false, with a description stating what the grant re-shares and that the filesystem confinement stays
- [x] 3.2 Carry the grant through the generator record to the runtime beside the fields that already travel
- [x] 3.3 Rewrite the containment paragraph in the `script` option's description: the "caller's filesystem and network" sentence is withdrawn, the envelope is stated, the grant is stated beside it, and what remains the fragment author's is what moves over a granted connection
- [x] 3.4 Add a module-evaluation test that the grant is readable at evaluation, which is the audit surface the spec promises

## 4. The proof

- [x] 4.1 Add the hostile-fragment fixture: a write outside the staging root fails inside the envelope and the run refuses with that fragment's own failure, storing nothing
- [x] 4.2 Add the network-absence case: a fragment without the grant attempting a connection fails; platform-conditional presence per design D7
- [x] 4.3 Add the declared-escape case: with the grant, the connection succeeds against a local listener and a write outside the staging root still fails
- [x] 4.4 Extend the strace reading in `syscall_proof.rs` to observe the envelope from outside the runtime, linux-only, with the non-linux half saying what it did not do
- [x] 4.5 Test the no-backend refusal by hiding the backend from the resolved toolset, and assert it precedes the first fragment
- [x] 4.6 Test that `--no-sandbox` and any equivalent is refused as an unknown flag, which is the spec's no-bypass scenario

## 5. The record

- [x] 5.1 Write the breaking entry in `CHANGELOG.md` for the next minor, leading with what the change costs — the withdrawn open-executor contract — before what it buys
- [x] 5.2 Update `README.md`'s generator section: the envelope, the grant, and the refusal
- [x] 5.3 Record in design.md any deviation from clan's envelope discovered during implementation, beside D3's uid deviation
- [x] 5.4 Verify: `openspec validate adopt-generator-sandbox --strict` passes
- [x] 5.5 Verify before this change archives that `clan-generator-contract` has archived, so `secret-generators` keeps a single-writer history (design's Migration ordering)
