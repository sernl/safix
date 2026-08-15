# Contributing to safix

## The rule that has no exceptions

No real recipient, no real hostname and no real user name enters this repository.

Fixture people are `ana`, `bo` and `cy`.
Fixture recipients are strings shaped like an `age1` public key and are not keys; `modules/flake/checks/fixture-fleet.nix` holds them, and the note at the top of that file explains what each fixture person exists to exercise.
Every key that a test actually decrypts with is minted inside that test's own scratch directory and dies with it.

A real recipient has been carried into this repository twice and removed twice — see `adecf4d` and `e4a8ad1`.
Both arrived as an example beside a working change, which is the shape to watch for.

Nothing in this repository deploys, switches or activates anything.
Every check builds or evaluates.

## Getting a shell

```
nix develop
```

That gives you `sops`, `age`, `jq`, the `safix` command this repository builds, and the rust toolchain from the same pinned nixpkgs the flake's cargo checks compile with — so a local `cargo clippy` and the check named `safix-rs-clippy` are the same compiler on the same sources.

## Running the checks

```
nix flake check
```

Individual checks build on their own, which is what you want while iterating:

```
nix build .#checks.x86_64-linux.safix-rs-clippy
nix build .#checks.x86_64-linux.safix-set-new
```

`nix flake check` evaluates the current system only.
`nix flake show` currently fails for a reason that predates the rust work: the flake declares a darwin platform the pinned nixpkgs has dropped.

## The fixture fleet

The checks drive the real `sops`, the real `age` and the real `git` against a throwaway repository built from scratch each run.
`crates/safix/tests/harness/mod.rs` is that recipe, and it is the one to copy from rather than a second one to write.

Every test builds a `Fixture`, which:

- Makes a scratch directory and removes it on every exit path, including a panicking one.
  It is mode 700 and on tmpfs, verified as tmpfs at runtime rather than assumed, because a value staged on a disk-backed `/tmp` outlives the directory being removed.
  A platform without one refuses unless `SAFIX_TEST_DISK_STAGING=1` says you accept disk-backed staging.
- Mints an age identity into that directory with `age-keygen`, and a second recipient for the shared audience so that "the audience is two people" is a claim a file encrypted to one key would fail.
- Writes a fixture `.sops.yaml` reproducing the two properties the command depends on: rules anchored with `^` and ending in `\.yaml$`, and no catch-all, so an unruled path fails closed exactly as it does in a real tree.
- Stubs `nix` only, because a flake evaluation is what a sandbox cannot do, and the stub asserts the attribute path it was asked for.
  `sops` is never stubbed: a check that stands a stub in for the backend stays green over a command calling something the tree no longer contains.

## Running one test locally

The suite is ordinary `cargo test`, so one test is one filter:

```
nix develop -c cargo test --test write_path set_new_creates_the_file_through_the_creation_rules
```

The target names are the files under `crates/safix/tests/`, and each named check runs exactly one of their tests — `modules/flake/checks/cli.nix` maps mode to target and test name, and `modules/flake/checks/single-runtime.nix` maps the four claims that were never comparisons to whole targets.
To run what a check runs, from the check's name, read the mapping there.

Two things about running it outside a build sandbox are worth knowing.
A terminal is one: the command reads a value from `/dev/tty` when it can open one, so the harness detaches runs whose value arrives on standard input into their own session with `setsid`, and a run that found your terminal would wait at a prompt the test never answers.
`strace` is the other: `crates/safix/tests/syscall_proof.rs` needs it, and it is in the devshell for that reason.

## Working on the rust half

The workspace is `crates/safix-core` (the runtime as a library, no terminal in it) and `crates/safix` (argument parsing, prompting, rendering, and nothing else).

Three things about it are not negotiable, and each is enforced rather than requested:

- Every crate declares `#![forbid(unsafe_code)]`.
- A plaintext value is a `Secret`. It has no `Debug`, no `Display`, no `serde::Serialize`, and no conversion from a `String` or a `&str`; those five absences are `const` assertions over a compile-time probe, so adding any of them fails the build with the message saying which. Do not work around the probe — if you need something the type does not offer, the type is what should change, along with the sentence at it.
- The library may not panic. `unwrap`, `expect`, `panic!` and slice indexing are denied in `crates/safix-core`; tests are exempt via `clippy.toml`. Every relaxation of the pedantic group is listed in the workspace manifest rather than as an attribute at a call site.

The minimum supported rust version is the toolchain the flake pins, and cargo enforces it.
The reasoning, and what it would take to lower it, is in the workspace manifest beside the field.

Before pushing:

```
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- --deny warnings
cargo test --locked --workspace
cargo-deny --offline --all-features check bans licenses sources
```

## Commits and changes

Commits are atomic and use conventional-commit subjects; the existing log is the reference.

Substantive work is planned as an OpenSpec change under `openspec/changes/`, with a proposal, a design recording the decisions and their reasons, delta specs, and tasks.
`openspec validate --all --strict` must pass.

One discipline runs through all of it: no sentence describing a guarantee is written before the code enforcing it exists in the same commit.
If a claim is worth documenting it is worth a check, and if it cannot have one yet then say what was actually checked instead.
Where a check exists, the perturbation that makes it fail belongs beside it — a check nobody has seen go red is a check nobody has reason to trust.
