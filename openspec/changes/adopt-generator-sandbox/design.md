## Context

See proposal.md — Why.
What matters for the approach:

- `crates/safix-core/src/generate.rs` walks generators sequentially and spawns each script and validation fragment through `generator_shell` (`crates/safix-core/src/nix.rs`), which puts the resolved `runtimeInputs` on `PATH` and runs the fragment under bash. The envelope wraps this one spawn site's command; nothing else in the walk changes.
- The staging root is a mode-0700, memory-backed directory created per generator and shredded however the run ends (`plaintext-staging`). `$out`, `$in`, and `$prompts` all live under it, and prompts are materialized by the runtime before the fragment starts, so the fragment needs no terminal and no caller filesystem.
- clan's envelope, read at the rev this fleet pins (`56e35624`, `pkgs/clan-cli/clan_lib/sandbox_exec/__init__.py`): on linux, bubblewrap with `--unshare-all`, a tmpfs root, `/nix/store` bound read-only, `/dev` and `/proc` provided, a tmpfs `/tmp`, and the generator's directories bound read-write; on darwin, a `sandbox-exec` profile that denies by default, reads the store, writes the granted paths, and allows localhost networking only; `sandbox_works()` probes bubblewrap on linux and `/usr/bin/sandbox-exec` on darwin, and anything else raises.
- clan offers `--no-sandbox` because its generators can come from third-party clan modules — its own help text warns "potentially executing untrusted code from external clan modules" — and because it chose degradation over refusal on backendless platforms. Neither reason transfers: a safix generator is the operator's own declaration, and safix already prefers a named refusal to a silent weakening.

## Goals / Non-Goals

Goals:

- The default envelope: staging root writable, store readable, nothing else reachable, no network.
- Interop preserved: a fragment runs identically under safix's executor and clan's default executor.
- The network as a declared, evaluation-visible capability on the generator, applying to its script and its validation fragments alike.
- A named refusal, never a silent unsandboxed run, when no backend exists.

Non-goals:

- Purity or determinism: `/dev/urandom` stays, and must — keygen is the point.
- Sandboxing anything besides generator fragments: editors, sops, git, and clan subprocesses keep their existing contracts.
- Resource limits, seccomp filtering, or process accounting beyond what the backend itself does.
- A filesystem escape. The network is the one capability with a named legitimate need (an ACME or API-token generator); a fragment that needs the caller's filesystem is the defect the envelope exists to catch, and no declaration reopens it in this change.

## Decisions

### D1. Adopt clan's envelope rather than invent one

Bubblewrap on linux, `sandbox-exec` on darwin, with the same mount and deny surface clan constructs.
The alternatives — landlock, raw namespace calls, seccomp — are all linux-shaped, all in-process, and none is what the fragment will meet under clan's executor, so any of them buys divergence with the added cost of maintaining our own confinement design.
Interop is the reason this change exists; the envelope is adopted, not approximated.

### D2. Bubblewrap arrives the way the runtime's tools already arrive

`generator_shell` already turns resolved nixpkgs attributes into a `PATH`; bubblewrap joins that resolution rather than becoming a new acquisition mechanism or a preinstallation requirement.
Requiring a system-wide `bwrap` was rejected because it diverges from clan (which resolves it through nix at spawn time) and breaks the ephemeral-shell usage the rest of safix supports.
On darwin nothing is acquired: `/usr/bin/sandbox-exec` is the system's, as it is for clan and for nix itself.

### D3. One deviation from clan's argv: the caller's uid stays

clan passes `--uid 1000 --gid 1000`; safix omits the pair and keeps the caller's uid mapped.
The staging root is created mode 0700 and owned by the caller before the fragment starts, so a synthetic uid inside the namespace could not write `$out` without loosening the root's mode or adding ownership games, and loosening the staging root to serve the sandbox would invert the point.
This is recorded as the one deliberate deviation; everything else in the argv is clan's.

### D4. `network = true` re-shares the network and changes nothing else

On linux the declaration adds `--share-net` beside `--unshare-all`; on darwin the profile adds outbound allowance.
The declaration lives on the generator submodule, travels to the runtime inside the same generator record the rest of the declaration travels in, and governs the script and the validation fragments alike — a validation that verifies a minted token against the API that issued it has the same need its script had.
A per-fragment split was rejected as a second axis nothing yet needs.

### D5. No invocation-level bypass

There is no `--no-sandbox` and no equivalent.
The precedents are already recorded: `clan-generator-contract` D6 (a flag that suspends a requirement loses to a surface that carries a different one) and `clan-bridge`'s export-drift decision (an override flag with nowhere to record the intent is a switch that turns a refusal into a silent loss).
The cost is stated plainly: on a machine with no backend, generation refuses and there is no way to proceed unsandboxed.
Whether that refusal ever needs a pressure valve is a question for evidence from use, the same posture the export-drift flag question took.

### D6. The probe runs once, before the first fragment

Availability is checked once per generation run, before any fragment starts, mirroring `sandbox_works()`: bubblewrap answers on linux, `/usr/bin/sandbox-exec` existence answers on darwin, and every other platform refuses as having no envelope.
A per-fragment probe was rejected: availability does not change mid-run, and a refusal after generator three has committed is worse than the same refusal before generator one.

### D7. The proof follows the existing conditional pattern

The envelope's enforcement claims are proved at three strengths.
The argv and profile construction are pure functions and are unit-tested unconditionally, network variant included.
The behavioural claims — a write outside staging fails, a connection without the declaration fails, the declared escape opens the network and nothing else — run as integration tests with a hostile fragment, present where the backend can actually run and absent rather than trivially green where it cannot, because bubblewrap needs user namespaces the nix build sandbox does not grant; this is the same shape `syscall_proof.rs` and the platform-conditional checks already use.
The strace reading in `syscall_proof.rs` extends to observe the envelope from outside the runtime, linux-only, with the non-linux half saying what it did not do.
The no-backend refusal is tested by hiding the backend from the resolved toolset.

## Risks / Trade-offs

- [Bubblewrap cannot nest inside the nix build sandbox, so CI's build-sandboxed checks cannot run the real envelope] → the pure construction tests always run; the behavioural suite is platform-conditional and additionally runnable as a check outside the build sandbox, the shape `clan-bridge` 5.2 already names for the same problem.
- [`sandbox-exec` is undocumented and disfavoured by Apple] → it is what nix itself and clan run on darwin today; adopting their profile means migrating when they migrate rather than alone.
- [A fragment somewhere silently depended on reading the caller's filesystem] → the fleet declares no such generator today; the first offender fails loudly at its own `generate` with the fragment's error inside the envelope, and the answer is fixing the fragment, not a filesystem escape (see Non-goals).
- [The uid deviation (D3) could surface a fragment that assumes uid 1000] → no fragment in the fleet reads its uid; a fragment that does gets the caller's, which is what it got before this change.

## Migration Plan

- Lands as a breaking entry in `CHANGELOG.md` for the next minor (0.3.0): the documented "caller's filesystem and network" contract is withdrawn.
- The fleet requires no declaration changes; no existing generator reaches the network.
- A future generator that needs the network adds `network = true` at its declaration, which is also the audit trail.
- Rollback is a revert: the envelope wraps one spawn site and adds one option, and nothing else moved.
- Ordering: `clan-generator-contract` holds the current `secret-generators` delta and must archive before this change archives, so the capability's history stays single-writer.

## Open Questions

- Whether a read-only extra-paths declaration is ever warranted (a fragment consulting a caller-side corpus, say) is deferred until a real fragment asks; it would be a new declared capability with its own change, not a loosening of this one.
