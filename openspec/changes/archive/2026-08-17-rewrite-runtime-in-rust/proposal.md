# The runtime in rust: an embeddable library and a thin command

## Why

safix is two halves that do not resemble each other.
The nix half is an algebra — declarations, resolution, audiences, policy — and it is checked by evaluating it.
The runtime half is `modules/flake/safix/safix.sh` at 2149 lines, plus `sops_recipients.py`, `sops_keys.py` and `sops-recipients-check.py`, and it is the half that touches plaintext.

Its failure modes are not hypothetical.
They are this project's documented bug history, recorded in the script's own comments as the shapes it deliberately avoids, each one a hazard the script survives by convention rather than by construction:

- A herestring or a command substitution materializes its operand in `$TMPDIR`, so the value path has to avoid two of bash's three most natural spellings and read stdin through `read -r -d ''` instead, "where a shredder cannot reach what it never registered" (`safix.sh:402`).
- A function-scoped `trap ... RETURN` does not run when the process dies between the write and the return, "which is the abort a value actually leaks through", so cleanup has to be one process-wide `EXIT` trap with `INT` and `TERM` routed through exit (`safix.sh:31`, `safix.sh:111`).
- A process substitution inherits `errexit`, so a bare `$?` on the next line is a line the subshell never reaches on exactly the run whose status matters, and a failed generator would be reported as one that printed nothing (`safix.sh:563`).
- A generator that reads standard input eats the answers to every prompt after it, silently, "since a prompt that reads end-of-input looks exactly like one nobody answered", so descriptors are part of the generator interface (`safix.sh:557`).

Every one of those is a comment holding a convention in place.
Delete the comment and the next edit reintroduces the bug; the language offers no help, because in bash a secret is a string and a string can be spelled anywhere a string goes.

A rust runtime makes the same four unrepresentable rather than avoided.
A value that exists only behind a `Secret` newtype over `secrecy`/`zeroize` has no `Display`, no `Debug` and no `Serialize`, so it cannot reach a format string, a log line, an argument vector or a JSON document — not by discipline, but because the call does not compile.
A scratch file owned by an RAII guard is shredded on every unwind, including the ones a `RETURN` trap misses.
A child process's exit status is a value the type system will not let the caller ignore, so there is no spelling of the generator call that loses it.
Descriptors are `Stdio` values passed explicitly at the call site, so a generator inheriting the operator's stdin is a thing someone has to write on purpose.

The nix half is not in scope and does not move.
It is the consumer-facing option surface, and rewriting it would trade a checked algebra for an unchecked one.

## What Changes

- A cargo workspace of two crates.
  `safix-core` is an embeddable library: the domain types, the sops and git drivers, the drift logic, generator DAG execution, and consumption of the placements the nix half evaluates.
  `safix` is a thin command over it.
  Every crate carries `#![forbid(unsafe_code)]`.
- The scope boundary is the evaluation seam and does not move.
  The command keeps consuming `nix eval --json` for placements, audiences, governed files and policy text, exactly as the script does today.
  `resolve.nix`, `types.nix` and `policy.nix` stay nix.
- Secret values become a type.
  `Secret` is a newtype over `secrecy`/`zeroize` that zeroes on drop, implements none of `Debug`, `Display` or `Serialize`, and is constructible only by reading a stream.
  No value reaches the argv or the environment of any child process; sops is driven through `Stdio::piped()` throughout.
- Refusals become data.
  Every refusal the script spells as a `die` becomes a `thiserror` variant in the library carrying what its message needs, rendered by `miette` at the command edge.
  The refusal prose is tested prose, so the variants are held to the script's wording.
- Concurrency is bounded and local to three places: the `fix` re-wrap, the `check` probes, and independent branches of the generator DAG.
  Everything else stays sequential, because the staging discipline the script depends on is sequential.
- sops stays the cryptographic authority, invoked as a subprocess.
  Nothing here reimplements the sops file format.
- Retirement of the shell runtime is gated, not scheduled.
  A differential harness runs the shell oracle and the rust binary over one fixture fleet and compares stdout, stderr semantics, exit codes and repository effects per subcommand.
  Until that gate is green for every subcommand, `packages.safix` remains the shell script and the rust binary ships beside it as `packages.safix-rs`.

Not in scope: any change to the nix algebra, the option surface, the consumption modules, or the recipient policy renderer; any reimplementation of sops; any change to what `packages.safix` builds.

## Capabilities

### New Capabilities

- `rust-runtime`: the workspace and crate boundary, the secret custody type and its construction rule, the refusal model and its rendering, the concurrency policy, and the subprocess contract that keeps sops the authority.
- `runtime-equivalence`: the differential harness — what is compared per subcommand, what "byte-identical" means for each channel, and the gate that permits retiring the shell runtime.
- `rust-supply-chain`: the MSRV and its enforcement, the lint posture, dependency review, locked builds, and the flake checks that carry all of it.

### Modified Capabilities

None.
`safix-cli` states the command's contract in terms of behaviour and refusals, and that contract is what `runtime-equivalence` holds the rust binary to; it is not restated or altered here.

## Impact

Affected code:

- New: `Cargo.toml`, `Cargo.lock`, `crates/safix-core/`, `crates/safix/`, `rustfmt.toml`, `clippy.toml`, `deny.toml`.
- New: `modules/flake/rust.nix` — the crane wiring, `packages.safix-rs`, and the build, test, clippy, format, dependency and advisory checks.
- Modified: `flake.nix` — `crane` and a pinned advisory database join the inputs, and the new module is imported.
- Modified: `.gitignore` — the cargo build directory.
- New: `CHANGELOG.md`, `CONTRIBUTING.md`.
- Unmodified for the whole of this change: `modules/flake/safix/safix.sh` and the three python helpers, which remain what `packages.safix` builds until the equivalence gate is green.

Affected checks: `safix-rs-build`, `safix-rs-test`, `safix-rs-clippy`, `safix-rs-fmt`, `safix-rs-deny` and `safix-rs-audit` are new, and the differential harness joins them as `safix-rs-differential` when the first subcommand lands.
