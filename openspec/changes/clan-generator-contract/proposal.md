# The generator interface clan already has: `$out`, `$in`, `$prompts`, and what that costs

## Why

The operator's requirement is that safix and clan vars interoperate "top to bottom, bottom to top" — that safix can take secrets from clan vars, and that clan vars can consume secrets safix generates.
A bridge that moves *values* is Change C.
This change is the prerequisite: the two systems must first agree on what a generator *is*, because a value produced under one contract is not portable to a system whose generators are shaped differently.

Today they are shaped differently in every part.

| | safix 0.1 | clan vars |
|---|---|---|
| output | script's standard output is the value; a JSON object keyed by name when there are several | one file per declared name at `$out/<name>` |
| dependency input | `$in_<name>`, a read-once file descriptor path | `$in/<dependency>/<filename>`, a real file |
| prompt input | `$in_<name>`, the same descriptor | `$prompts/<key>`, a real file |
| share | `shared` on the entry | `share` on the generator |
| public outputs | none; every output is encrypted | `files.<name>.secret = false` writes plaintext in-repo, readable at eval as `.value` |
| validation | a shell fragment judging the candidate before the write | `validationHash`, an invalidation trigger |

A generator written for one runs under the other only by accident.
More to the point, the clan-shaped contract is the one an operator writing for a fleet that has both will write, because it is the one clan-core's own samples are written in and the one clan's documentation teaches.

The contract also carries something safix has no equivalent for and the operator asked for by name: `files.secret = false` as public outputs stored in plaintext in the repository, readable at evaluation, which is what makes `.value` possible.
That accessor is not a convenience.
It is how a public key, a fingerprint or a derived identifier reaches a nix module without a deployment-time indirection, and clan's own service modules depend on it.

And the operator asked for editor input — "usage of the default editor to input secrets" — which safix has no path for at all.

## What this costs, stated first

The v2 contract materializes plaintext as files.
safix 0.1's most load-bearing promise is that it does not: `secret-generators` requires that a generated value "travels a pipe and never argv, the environment, or a file", and `safix-cli` requires that "values move through pipes only".
Those requirements are not decoration; the rust rewrite exists because the shell runtime could only hold them by convention.

`$out/<name>` is a file.
`$in/<dep>/<file>` is a file.
`$prompts/<key>` is a file.
An editor edits a file.

This change therefore weakens safix's central invariant, deliberately, and the weakening is the change's principal risk rather than a side effect of it.
What replaces the pipe is not another absolute but a containment discipline, and it is materially weaker: the plaintext exists, in a directory, for the duration of a run.

The containment is the operator's own rule, adopted verbatim as a spec requirement: plaintext staged during generation or editing lives in a mode-700 private directory on tmpfs; on linux the runtime verifies at execution time that the chosen mount is actually tmpfs; if no tmpfs is available it refuses unless the operator passes a flag acknowledging disk-backed staging; and it shreds on every exit path regardless.
This fleet's `/tmp` is ext4, which is why that is a rule rather than a preference.

Two honest limits on that containment are written into the spec rather than left to be discovered.
Shredding a tmpfs file overwrites pages in memory, and a page that was swapped before the overwrite is not reached — tmpfs bounds the exposure to memory and swap, not to memory alone.
And what a generator script or an editor does with a value it has been handed is outside safix's reach: a script that redirects `$in/dep/file` somewhere else, or an editor whose undo history is configured to a global directory, has put plaintext where safix cannot shred it.

## What Changes

- The generator executor becomes clan's. `$out` is a directory with one file per declared name; `$in/<dependency>/<filename>` carries a dependency's outputs; `$prompts/<key>` carries one answered prompt each; the process's working directory is the staging root; `runtimeInputs` is on PATH; the script runs under `bash` as it does today.
- `share` is added to the generator and *derived* rather than authored: a generator is shared exactly when all of its outputs are shared entries, and a generator whose outputs disagree is refused at evaluation. This keeps `shared` on the entry, where safix's audience model needs it, while giving the bridge the clan-shaped field it must compare against — and it has a second effect worth having on its own, in that a generator's outputs then always land in one audience and so one file, which is what makes a multi-output write one rename.
- Multi-output atomicity is preserved and stated: every declared output must be present in `$out` before anything is written, a missing one refuses the whole run naming what *was* produced, and the writes go through the existing stage-beside-and-rename path.
- `files.<name>.secret = false` becomes a public output, stored as plaintext in the repository under a store designed here, with `.path` and `.value` accessors on the nix side and a checked guarantee that no generated `.sops.yaml` rule matches any public path.
- An `edit` subcommand opens the operator's editor on a tmpfs-staged plaintext file and writes the result through the same path `set` writes through.
- The 0.1 descriptor interface is removed rather than kept alongside, with an evaluation-time refusal that names this change and gives the mechanical rewrite.
- `validation` stays what it is — a check on the value before the write — and clan's `validationHash` is explicitly *not* adopted here. The two solve different problems and the bridge, not this change, is where the second one is needed.

Not in scope: moving any value between the two systems, which is `clan-bridge`; any change to audiences, policy rendering or the recipient model.

## Capabilities

### New Capabilities

- `plaintext-staging`: where plaintext may exist while a run is in progress, how the runtime establishes that the location is what it claims, what happens when it is not, and what the shred does and does not achieve.
- `public-outputs`: `files.<name>.secret = false`, the in-repo plaintext store's layout, the `.path` and `.value` accessors and their eval-time behaviour, and the checked non-interaction between that store and every generated creation rule.
- `editor-input`: the `edit` subcommand, editor selection, and what a run does when the editor fails, changes nothing, or empties the value.

### Modified Capabilities

- `secret-generators`: the executor interface moves from descriptors to directories, multi-output moves from a JSON object to `$out/<name>`, and `share` joins the generator as a derived field. The pipes-only requirement is modified rather than deleted: it still governs every path where a pipe remains possible, and now names the exception explicitly.
- `safix-cli`: `edit` joins the verb list, and the "values move through pipes only" requirement is modified to state where that no longer holds and what stands in its place.

## Impact

Affected code:

- Modified: `crates/safix-core/src/generate.rs` — the executor.
- Modified: `crates/safix-core/src/inputs.rs` — descriptor construction is replaced by staging-directory construction.
- Modified: `crates/safix-core/src/scratch.rs` — the guard registry gains the private staging directory, and the tmpfs probe lands beside it.
- New: `crates/safix-core/src/edit.rs`, and an `edit` arm in `crates/safix/src/main.rs` and `usage.rs`.
- New: `crates/safix-core/src/public.rs` — the plaintext store's reads and writes.
- Modified: `modules/flake/safix/types.nix` — `files` becomes an attribute set carrying `secret`, `share` joins the generator as a derived read-only field, and the descriptor prose is replaced.
- Modified: `modules/flake/safix/resolve.nix` — the share-agreement refusal and the v1-script refusal.
- Modified: `modules/flake/safix/checks.nix` — a public-path non-interaction check joining the existing rule-shape family.
- Modified: `modules/flake/safix/policy.nix` — unchanged in behaviour, and re-read to confirm no generated rule can reach the public store.

Affected checks: `safix-public-no-rule` is new; the generator checks in `modules/flake/checks/generators.nix` are rewritten against the new interface; the integration suite's `generate`, `generate-refusals` and `generate-isolation` tests change subject.

Affected consumers: every generator declared in dotfiles. That is the whole of the migration surface today, and it is why the break is affordable.
