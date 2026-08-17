# Design: the runtime in rust

## Context

`modules/flake/safix/safix.sh` is 2149 lines and carries the whole of safix's contact with plaintext.
Reading its comments is the brief: each of the four hazards named in the proposal is documented at the point where the script sidesteps it, and each sidestep is a convention with nothing but a comment holding it in place.
The three python helpers — `sops_recipients.py`, `sops_keys.py`, `sops-recipients-check.py` — are the ciphertext readers the script shells out to, and they are in scope for the same reason: they are runtime, not algebra.

The nix half is not runtime.
`resolve.nix`, `types.nix` and `policy.nix` are a checked algebra with a consumer-facing option surface, and the checks that hold them evaluate nix.
Nothing in this change touches them, and the seam between the halves — `nix eval --json` producing placements, audiences, governed files and policy text — is preserved exactly as the script uses it.

## Goals / Non-Goals

Goals.
Make each of the four documented bash hazards unrepresentable rather than avoided.
Publish the runtime as a library another program can embed, with the command a thin shell over it.
Keep sops the cryptographic authority.
Earn the retirement of the shell runtime by demonstrated equivalence rather than by assertion.

Non-Goals.
Any change to the nix algebra, the option surface, the consumption modules, or the policy renderer.
Any reimplementation of the sops file format, age, or the recipient wrapping.
Any change to what `packages.safix` builds, for the whole of this change.
Any performance claim: the concurrency introduced here is bounded and exists where the script already fans out, and no speedup is documented before a measurement exists.

## Decisions

### D1. Two crates: an embeddable library and a thin command

The workspace is `safix-core` and `safix`.
`safix-core` holds the domain types, the sops and git drivers, the drift logic, generator DAG execution, and consumption of the placements the nix half evaluates.
`safix` holds argument parsing, the terminal interaction, and the rendering of refusals, and nothing else.

The split is what makes the runtime embeddable, and the test for whether a thing is in the right crate is whether it can be exercised without a terminal.
Every crate declares `#![forbid(unsafe_code)]`, which is a compiler-enforced attribute rather than a lint, so it cannot be silenced at a call site.

### D2. The scope boundary is the evaluation seam, and it does not move

The command keeps consuming `nix eval --json` for placements, audiences, governed files and policy text, exactly as the script does.
This is not a transitional arrangement.
The nix algebra is the option surface a consumer writes against, it is checked by evaluation, and it is the half of safix that has no plaintext in it.
Rewriting it in rust would trade a checked algebra for an unchecked one and would move safix's public interface out of the module system its consumers live in.

### D3. Secret values are a type whose misuse does not compile

`Secret` is a newtype over the `secrecy` and `zeroize` crates.
It zeroes its buffer on drop.
It implements none of `Debug`, `Display` or `Serialize`, so it cannot reach a format string, a panic message, a log line or a JSON document.
It is constructible only by reading a stream — a pipe, or the process's own standard input — and there is no constructor taking a `String`, a `&str` or an argument.

That last rule is what closes the bash hazard rather than restating it.
In the script, the value is a shell variable, and a shell variable can be spelled into a herestring, a command substitution, an argument or a log with no diagnostic; four comments exist to say do not.
Here the corresponding mistakes are absent constructors and absent trait implementations, and the absence is asserted by a compile-time probe rather than described, because a trait that someone later adds would silently satisfy every prose claim about it.

Values reach sops through `Stdio::piped()`.
No value is ever placed in the argv or the environment of a child process.

### D4. Refusals are library data, rendered at the edge

The script's refusal channel is one function: `die` prints `safix: <message>` to stderr and exits 1, and `note` prints two-space-indented continuation lines to stderr.
There are 55 such refusals, and their wording is tested prose — the CLI contract states them, and the shell self-test reads them.

In the library each refusal becomes a `thiserror` variant carrying the data its message needs: the file and the two recipient sets for drift, the name and the declared users for an unknown user, the path for a missing creation rule, the participating nodes for a generator cycle, the identity paths for a keyless machine.
A variant carrying formatted prose instead of data would make the message the only thing an embedder can act on, which defeats the point of publishing a library.

`miette` renders at the command edge, and every variant's rendering is held by an `insta` snapshot.
Message parity with the shell runtime is not a rendering coincidence; it is the subject of D7.

### D5. Concurrency is bounded, and only where a fan-out survives the port

`tokio` appears in one place: re-wrapping files in `fix --yes`, bounded by a semaphore rather than spawning per item, with the output replayed in declaration order so that which re-wrap finished first is not observable.

Three sites were planned, one per fan-out in the script, and two did not survive contact with the rust implementation of the same work.
`check`'s probes are subprocesses in the script and in-process metadata reads here, so there is no fan-out left to bound; D11 records that where it was found.
The generator walk is sequential for three reasons the module states where a reader meets them — one standard input for prompts, a commit order that is the plan's rather than the scheduler's, and one staging root holding plaintext at a time.
Neither withdrawal is an omission and neither retracts a measured speedup, because none was ever measured: this document promises no performance claim, so a bound over work that no longer fans out is not added on the chance that it would help.

Everywhere else is sequential, and that is a correctness requirement rather than a simplification.
The script's write discipline is sequential — stage before regenerate, regenerate before commit — and the reason is recorded at `safix.sh:1663`: a flake evaluation reads the files git knows about, so regenerating a policy before staging a new scaffold renders the policy of a fleet that does not include it.
Two writers interleaving through that sequence would produce a `.sops.yaml` that matches neither.

### D6. sops stays the cryptographic authority

Every encrypt, decrypt, re-wrap and metadata read goes through the sops binary as a subprocess, pinned into the package closure the way the shell package pins it today.
The sops file format, its MAC, its IV reuse rule and its key wrapping are not reimplemented, not parsed for anything but the fields the readers already read, and not depended on beyond what the current python readers depend on.
This bounds the blast radius of a bug in this change to orchestration, and it keeps safix's cryptographic surface a reviewed upstream one.

### D7. The differential harness, and what "byte-identical" means per channel

The gate that permits retiring the shell runtime is a harness that drives both runtimes over one fixture fleet — throwaway age keys minted at test time, users `ana`, `bo` and `cy`, synthetic `age1` recipients — and compares four channels per subcommand invocation.
Each channel gets its own definition because they are not equally comparable, and calling all four "byte-identical" without saying so would be the kind of claim this repository does not make.

Standard output is compared byte for byte, with no normalization at all.
This is the machine-readable channel — a value from `get`, a table from `list` — and it is where a difference is a defect with no argument available.

Standard error is compared byte for byte under a plain reporter, and the plain reporter is code rather than a comparison rule.
`miette`'s graphical rendering adds diagnostic codes, source spans, colour and wrapping, so a normalizing regular expression over its output would be a comparison whose strictness nobody could state.
Instead `safix` selects its reporter from `SAFIX_ERROR_FORMAT`, and the plain reporter emits exactly the shell's shape: `safix: <message>` and two-space-indented notes, no colour, no code, no span.
The harness sets that variable on the rust side only, so both sides receive an identical argv, and stderr is then compared with no normalization either.
Two further checks keep that from being a loophole: the variable is asserted to affect rendering only, by running the same invocation with and without it and requiring identical stdout, exit code and repository effects; and the graphical rendering of every variant is held by its own `insta` snapshot, so the channel that is not compared against the shell is still pinned against itself.

Exit codes are compared exactly.
The script exits 0 or 1, plus 130 and 143 for `INT` and `TERM`, and the rust binary's mapping to those numbers is part of the contract, not an implementation detail.

Repository effects are compared exactly, over a canonical projection rather than over the bytes on disk, and the reason is a property of sops rather than a concession.
A newly set value gets a fresh IV, and the MAC and `lastmodified` change with it, so two correct runs produce different ciphertext files; comparing file bytes would compare sops's random number generator.
The projection is therefore: the ordered list of commits by subject and by `--name-status`; the full `git status --porcelain=v2`; the working tree's paths and modes; the decrypted plaintext of every governed file; and the recipient set of every governed file, read through the same reader both sides use.
One script computes the projection and both sides are passed through it, so the two cannot disagree about what was compared.
A fifth assertion sits beside the four: after both runs, no plaintext residue exists in either run's temporary directory.

The harness is not trusted until it has been shown to fail.
Each channel gets a severity drill in which the rust side is deliberately mutated — one refusal's wording, one exit code, one staged path, one written value — and each mutation must be caught, and caught by the channel that exists to catch it rather than incidentally by another.

The gate is per subcommand.
`packages.safix` remains the shell script until every subcommand passes, and the rust binary ships beside it as `packages.safix-rs` in the meantime, so nothing a consumer runs changes underneath them on the strength of a partially-migrated runtime.

### D8. MSRV is the pinned toolchain, and the pin is what is tested

The locked nixpkgs provides rustc 1.97.1, and the workspace states `rust-version = "1.97"`.

The rule that produced that number is that this repository states the newest version it requires rather than the oldest it might tolerate, because the oldest is a guarantee with no check behind it.
`nix flake check` builds with exactly one toolchain, so a lower MSRV would be a compatibility claim this repository has never compiled.
Cargo enforces `rust-version` itself — a build on an older toolchain fails naming the required version — so the claim and the code enforcing it land in the same commit, which is the standing rule here.
Edition 2024 does not constrain the choice, since it requires only 1.85; the pin does.

Lowering the MSRV later is available and cheap: lower the field and add a check that builds the workspace against the older toolchain.
The number moves when the check does, and not before.

### D9. crane, and a package name that does not shadow the shipping one

The flake gains `crane` for cargo builds and a pinned `advisory-db` for the advisory check.
`packages.safix-rs` is the new output; `packages.safix` is untouched for the whole of this change, per D7.

The checks are `safix-rs-build`, `safix-rs-test`, `safix-rs-clippy`, `safix-rs-fmt`, `safix-rs-deny` and `safix-rs-audit`.
Advisories are split from `cargo-deny` into their own check because the advisory database is a network resource and the build sandbox has none: `cargo-deny` runs `bans`, `licenses` and `sources` offline over the vendored graph, and the advisory scan runs against the database pinned in `flake.lock`.
Splitting them also means a newly published advisory turns exactly one check red, and does so only when the lock is updated, which is a signal with a date on it rather than a build that fails at an unrelated moment.

### D10. Lints deny what the script cannot express

`clippy::pedantic` is on, with the curated subset of allows recorded in the workspace manifest beside it.
`unwrap_used`, `expect_used`, `panic`, `indexing_slicing` and the arithmetic and conversion lints are denied in library code.

The reason is specific to this crate rather than general hygiene: a panic in a runtime holding a decrypted value unwinds through the drop that zeroes it, and its message is a place a value could appear if a type ever grew a `Debug`.
Denying the panicking constructions in the library keeps every failure a `Result` the caller can render, which is what D4 requires to be true for an embedder as well as for the command.
The command crate is permitted `expect` in `main` alone, where the alternative is unreachable code.

### D11. What the read-path gate found, and what it deliberately does not hold

The first stage to run the harness is the read paths — `list`, `get` and `check`.
Seven checks compare them: six fixtures, and the drills that keep those honest.
Every compared invocation agreed on all four channels, so the list below is not a list of failures; it is the list of places where the two runtimes are known to be able to differ and the harness is the reason we know.

The shell's `list` renders in the placement document's own key order, and every other ordering in either runtime is sorted.
`jq`'s `to_entries` preserves the document's order while `keys` sorts, so the shell's table and the shell's own `refuse_unknown_name` disagree about ordering on any input where the two differ; the rust runtime reads placements into an ordered map and so always sorts.
Over `nix eval --json` they coincide, because nix emits every attribute set with its names sorted — which makes this a property of the producer rather than of either runtime, and one a fixture could break without either being wrong.
The harness therefore asserts its own fixture is in nix's order before comparing anything against it.
This was found by the harness rather than by reading: a perturbation appended a placement, and the comparison failed on standard output.

`check`'s probes are not a subprocess fan-out in the rust runtime, so D5's second concurrency site does not exist on this path.
The shell shells out to `sops-keys-of` and `sops-recipients-of` once per file per question; the rust readers are in-process, and the report's finding order is part of its compared output.
`check` is therefore sequential here, and adding concurrency to it later would be a change to what is compared, not an optimisation under it.
`SAFIX_RECIPIENTS_OF` and `SAFIX_KEYS_OF` accordingly have no rust counterpart, while `SAFIX_GIT`, `SAFIX_SOPS`, `SAFIX_NIX` and `SAFIX_REPO_ROOT` do.

A governed path holding something that is not a YAML document is reported by neither runtime's key reader.
In the shell this is a swallow rather than a decision: the reader runs inside a pipeline whose failure becomes a false answer to "does this hold a value", and inside a process substitution whose failure ends the loop over keys.
The rust runtime matches it deliberately and says so where it does.
The blind spot is bounded: the recipient half of the report reads the same file and does speak about it, because a document with no `sops` block reports the sentinel rather than failing.

The rust runtime has one refusal the shell has no counterpart for.
Every schema read from the nix half denies unknown fields, so a field added there reaches `NixSchemaMismatch`; the shell reads the same JSON through `jq` expressions that select what they know and ignore the rest.
This is the coupling the schema types exist to create, and it is a refusal the harness cannot compare because no fixture can produce it from both sides at once.

The `list` table is aligned by character count, where `column -t` aligns by display width.
Every field but one is a name, a path or a key drawn from the resolver's alphabet; the exception is a generator's description, which is free text, and that is where the two would part company over an east-asian or combining character.
Closing it means a display-width table, and the gap is recorded rather than papered over because the fixture fleet is ASCII and a green harness would otherwise be read as evidence about a case it never ran.

Finally, the general usage text is not ported in this stage.
The shell's `usage` lists eight subcommands and this binary implements three, so reproducing it would advertise five it refuses; a bare invocation of `safix-rs` names the three it has, and `safix-rs set` refuses rather than approximating.
The harness compares the help of the three ported subcommands and does not compare the general usage, and that exclusion ends when the last subcommand lands.

## Risks / Trade-offs

The rewrite runs beside the shell runtime for its whole duration, so the two can drift while both are live.
The mitigation is that the shell remains the only shipping binary and the differential harness compares against it directly, so drift is what the harness reports rather than something discovered later.

The differential harness is itself code that can be wrong, and a harness that passes vacuously is worse than none.
The severity drills in D7 are the answer, and each channel's drill is a task rather than a note.

`miette` is a comparatively large dependency for a tool whose value is a small trusted surface.
It is confined to the command crate; `safix-core` depends on `thiserror` only, so an embedder takes none of it.

Retiring the python readers means reimplementing what they parse.
That is a real risk of behavioural difference in the metadata-reading path, and it is why the recipient set of every governed file is one of the four compared repository effects rather than something the harness takes on trust.

## Migration Plan

Stage 1 is this change's scaffold: the workspace, the toolchain pin, the lint and dependency posture, the crane wiring, and the `Secret` type with its construction rule and its compile-time absence probes.
Stage 2 is the read paths — the placement resolution behind `list`, the decryption behind `get`, and the four-part report behind `check` — together with the schemas that read the nix half, the sops and git drivers, and the first real use of the differential harness.
Its findings are D11.
Later stages port the write paths and then the generator DAG, each behind its own gate, and the shell runtime is retired only when the last gate closes.
Their findings continue the list D11 began, and are recorded in `CHANGELOG.md` under *Known differences* rather than here, because that is the list a consumer of the shipped binary reads.
Some of those entries are pinned by an assertion on each runtime, so an oracle that stops differing fails the check that records the difference rather than quietly making the two comparable; each entry names the check that pins it, where one does.
No stage moves `packages.safix`.

## Open Questions

None blocking Stage 1.
The platform matrix question carried out of `add-consumption-modules` — `nix flake show` failing on a dropped darwin platform — is unrelated and stays where it was recorded.
