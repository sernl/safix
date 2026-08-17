## Why

`types.nix` documents the gap in its own words: the staging root is bounded containment and not a sandbox, and a fragment that writes outside `$out` has put plaintext somewhere safix does not look and cannot shred.
`clan-generator-contract` adopted clan's executor interface so that a fragment written for either system runs under the other, but clan runs that interface inside a sandbox by default and safix runs it with the caller's filesystem and network, so the interop is incomplete exactly where it is most material — what a fragment may do while it holds plaintext.
The operator answered the question that change's design recorded: generators behave securely by default, and the change opens now rather than later because the fleet declares no network-reaching generator today, so the default changes while breaking nothing that exists.

## What Changes

- **BREAKING**: a generator's script and its validation fragments run inside a sandbox by default — the staging root is the only writable path, `/nix/store` is read-only, and there is no network. The documented contract "the fragment runs with the caller's filesystem and network" is withdrawn.
- The envelope is clan's own, adopted rather than invented — bubblewrap on linux, `sandbox-exec` on darwin — so a fragment keeps running under both systems' default executors, which is the interop `clan-generator-contract` established.
- A generator that needs the network declares it on the generator itself (`network = true`). The declaration re-shares the network and nothing else; the filesystem confinement stays.
- There is no invocation-level bypass. Where clan offers `--no-sandbox`, safix refuses when no sandbox backend is available, naming the backend it looked for and what would supply it.
- The syscall proof extends to the envelope: a hostile fragment that writes outside the staging root or opens a network connection is held to fail, observed from outside the runtime with the same strace reading the existing proof uses.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-generators`: the execution envelope becomes part of the executor contract — sandboxed by default, network by declaration, refusal without a backend — and the requirement that documented uncontained copying as solely the fragment author's responsibility is narrowed to the declared-escape path.

## Impact

Affected code:

- `crates/safix-core`: a sandbox module — the bubblewrap command construction, the darwin profile, the availability probe — wired into the mint and validation spawns in `generate.rs`, with the no-backend refusal in `error.rs`.
- `modules/flake/safix/types.nix`: the `network` option on the generator submodule, and the rewrite of the containment paragraph that currently disclaims a sandbox.
- `crates/safix/tests`: hostile-fragment fixtures, the envelope extension to `syscall_proof.rs`, the no-backend refusal, and the declared-escape path.
- `README.md` and `CHANGELOG.md`: the envelope, the declaration, and the breaking entry.

Dependencies: bubblewrap on linux, reaching the runtime by a mechanism design settles; darwin uses the system `sandbox-exec` clan already relies on.
Fleet: no declared generator reaches the network today, so adoption requires no declaration change anywhere.
Ordering: independent of `clan-bridge`; builds on the executor `clan-generator-contract` finished.
