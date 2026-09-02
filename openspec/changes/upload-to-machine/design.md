Revisions are as named in `proposal.md`; every anchor below was read at one of them.

## Context

A machine's recipient is declared before the machine exists in any bootable form: `flake.safix.machines.<m>.recipient` is the age form of a host ed25519 key the operator holds (`modules/flake/safix/types.nix:548-576`), and every ciphertext that machine's audience reaches gets wrapped to that recipient as soon as `safix fix` runs, independent of whether the machine has ever booted.
That ordering is safix's own, not clan's — clan's machine recipients are typically read back from a target after `nixos-anywhere` has already generated one, where safix's model needs the recipient wrapped *before* the machine can consume the closure that will decrypt with it.
The gap this change closes is the one boot in between: the operator holds the private half already, and nothing safix ships gets it onto the disk of a machine that has not booted, so its first activation can decrypt what was already wrapped to it.

`secret-installation` and `consumer-integration` were read for this change and neither needs a requirement changed.
`secret-installation` governs what safix's own installer does at system-scope activation — the manifest, the store, the ordering, the identity *derivation* — all of which assume an identity is present or derivable by the time activation runs (`openspec/specs/secret-installation/spec.md:98-139`); this change is entirely about the boot before that assumption is first true, and adds no behavior to activation itself.
`consumer-integration` governs what safix reads from a consumer's own option tree, and this change reads none: the ssh address a target is reached at is supplied on the command line (D4), not derived from any registry, so the requirement that safix "reads no option outside its own namespace" (`openspec/specs/consumer-integration/spec.md:9-30`) is unaffected by a verb that reads no option at all.

Two of clan's files were read as the transport reference: `clan_lib/ssh/upload.py` (the tar-over-ssh mechanism, the depth safety, the file modes) and `clan_cli/vars/upload.py` plus `clan_lib/vars/upload.py` (the `--directory`-versus-remote split and what each calls).
Both are cited by line throughout the decisions below.

## Goals / Non-Goals

Goals.
Get an operator-supplied private host key onto a machine's disk before that machine's first activation, in the shape a fresh install consumes it — locally, as a tree, or remotely, over ssh.
Tell the truth when a machine already has what it needs, rather than performing a transfer that would either overwrite a live host's real identity or silently do nothing while reporting success.
Reuse clan's transport shape rather than inventing one, because it is a proven design with a stated safety property (the path-depth check) and no reason exists to diverge from it.

Non-Goals.
Minting a machine's identity. safix mints no machine identity anywhere in its existing surface — `keygen` mints a *person's* age identity and explicitly stops there (`crates/safix-core/src/keygen.rs:1-8`) — and this change does not add the first case of it; the operator supplies the private key, harvested or minted by their own means, exactly as they already supply the public half that becomes the declared recipient.
Provisioning a person's own first identity. That is one-unlock-bootstrap's territory (an active, unarchived change in this repository's own `openspec/changes/one-unlock-bootstrap/`), and this verb parses a machine name only.
A systemd-credentials delivery path for the same material. Recorded as a named non-goal in Decisions below, with the one-sentence extension point stated and nothing built toward it.
Any verb that deploys, switches, or rebuilds a machine. safix is pull-model: a machine's own next rebuild is what activates the closure carrying its ciphertext, and nothing this change adds triggers one.
Ongoing secret delivery to an already-provisioned machine. That is what the closure and safix's own installer already do, and is the reasoning `safix-cli`'s "Absent verbs" requirement keeps, narrowed rather than dropped.

## Decisions

### D1. Two write modes plus one no-write mode, selected by where the target is and what it already presents

`--directory DIR` and remote (bare `safix upload <machine> --to <address>`) are the two write modes, matching clan's own split between `populate_dir` and the ssh path (`clan_cli/vars/upload.py:14-27`).
`--directory` writes a tree at `DIR` and stops; there is no target to probe, because the operator has named a location on their own disk, for `nixos-anywhere --extra-files` or for hand-copying onto installer media.
Remote mode is where the third behavior lives, because it is the only mode with a live target to ask: before writing anything, it probes what host key the target currently presents (D5) and either reports that nothing is needed, refuses on a mismatch, or writes.

Alternative considered and rejected: one mode that always writes, relying on the operator to have checked first.
This is clan's own posture for vars — reuploading an unchanged value is harmless there — and it is not harmless here, because what would be overwritten is the one piece of state that makes a live machine's decryption work at all.
An operator who has already run this once, or who has typo'd a machine name onto a host that turns out to be a different live machine, is exactly the case the no-write mode exists for.

### D2. The transport mirrors clan's shape rather than inventing one

Root, tar-over-ssh, `0400` on files and `0700` on directories, wipe-then-extract via `install -d && find -delete && tar -xzf -`, all taken from `clan_lib/ssh/upload.py:14-121`, and the path-depth safety at `:9-11` and `:34-53` — a directory destination must be at least three components deep, or two deep under `/tmp/`, `/root/`, or `/etc/`, because the wipe is a `find -mindepth 1 -delete` under the destination and a shallow destination makes that catastrophic.

safix's own destination is fixed rather than operator-named: a fresh install mounts its target root at `/mnt` during provisioning (the nixos-anywhere convention `--extra-files` itself assumes), so the pre-seed tree lands at `/mnt/etc/ssh`, depth three, clearing the safety threshold by construction.
The check travels anyway, as defense in depth rather than as a live constraint: a fixed destination that happens to be safe today is not a proof that it stays fixed, and the cost of keeping the check is one comparison against a constant clan already published the reasoning for.

Alternatives considered.
`rsync` gives incremental transfer this payload never benefits from — a handful of small files, wiped and replaced whole every time — at the cost of a dependency clan's own transport does not carry.
Plain `scp` has neither the wipe-then-extract atomicity (a partial `scp` can leave a mixed tree) nor the depth safety, and reproducing both on top of it is reproducing `clan_lib/ssh/upload.py` under a different name.

### D3. The private key travels through `Staging` only where it is transient; a destination the operator named is not staging

Two different plaintext lifetimes exist in this change and they get different treatment.

`--directory DIR` writes the key straight to its final path under `DIR` — `DIR/etc/ssh/ssh_host_ed25519_key` at mode `0600`, `DIR/etc/ssh/ssh_host_ed25519_key.pub` at mode `0644`, mirroring the modes and paths a fresh NixOS install's own `sshd-keygen` would produce — and `DIR` is the deliverable the operator asked for, exactly as `safix set` writing final ciphertext to the repository is a deliverable and not staging.
`plaintext-staging`'s existing rule is about safix's own transient working directories, not about a location the operator named on the command line, and the delta in this change adds a scenario saying so explicitly (see the `plaintext-staging` delta), because this verb is the first one to introduce the ambiguity: every prior use of `Staging` — generation, editing — produces only ciphertext at rest, and this one, of necessity, produces plaintext at rest by design.

Remote mode is the transient case. `clan_lib/ssh/upload.py:56-93` builds its tarball inside a bare `TemporaryDirectory(prefix="vars-upload-")`, with no filesystem-type verification — reasonable for clan, whose payload is secret values already headed for encrypted storage on the far end, and not the posture safix takes anywhere else in this codebase for plaintext it holds even briefly.
Remote mode's tarball is built inside `Staging` instead, the same memory-backed, per-run, owner-only root generation and editing already use (`crates/safix-core/src/staging.rs`), and the `plaintext-staging` delta in this change widens that requirement's scope to name this third occasion.
The staging root is removed the same way it always is — before the tarball is streamed and again on every exit path — so the plaintext key exists on any disk for exactly as long as the ssh transfer takes, which is what D2's transport already assumes and does not itself provide.

Where the key comes from: the operator, always. `--identity PATH` names a local file holding the private half the operator already generated or harvested by whatever means produced the public half they used to compute the declared recipient — the same division of labor `custody-subjects` states for a machine's recipient field, that declaring a machine "mints no second identity" (`modules/flake/safix/types.nix:557-558`), applied here to the write side rather than only the declaration side.
The command reads the file, never accepts the key on its own argument vector, and refuses before writing anything if the supplied key's derived age form does not equal the machine's declared recipient — the integrity check that keeps a pre-seed run from seeding the wrong key onto the right machine, or the right key onto the wrong one.

### D4. The target address is the operator's to supply, because safix carries none of its own

Clan resolves a machine's network address through its own inventory (`clan_lib/network/network.py:333-334`, `get_best_remote`), because clan owns a machine registry with addresses in it.
safix's machine record carries `recipient`, `owner`, and `tags` and nothing naming a host (`modules/flake/safix/types.nix:546-577`), and `consumer-integration` states the reason no field like that exists: reading a consumer's own host registry "would make every consumer's option tree part of this package's interface" (`openspec/specs/consumer-integration/spec.md:20-24`).
A machine that has never booted has no address any registry could have observed yet in any case, which makes the boundary free here rather than merely consistent: remote mode takes the address on the command line, and the requirement this leans on is unmodified because nothing about it changes — safix still reads no option outside its own namespace, having read none in the first place.

### D5. The no-write branch is the default whenever the target's presented identity is not exactly absent

Remote mode probes the target's host key the way `ssh-keyscan` does — unauthenticated, reading only what the target offers during key exchange — and converts it to age form with the same external `ssh-to-age` this package already directs an operator to for a person's own recipient (`crates/safix-core/src/keygen.rs:231-233`), rather than reimplementing the curve conversion.
Three outcomes, and only one of them writes.

The presented key's age form equals the machine's declared recipient: nothing is needed.
The machine has already booted with the right key, its own activation already decrypted whatever ciphertext the closure carried, and a transfer here would either be a no-op with extra steps or, on a machine whose key rotated for a real reason, an overwrite of a live identity.
Report and exit zero; write nothing.

No host key is presented at all: this is the bootstrap case the whole verb exists for — a target with sshd not yet configured with generated keys, the ordinary state of a freshly booted install environment before its first activation.
Write, given `--identity`; refuse naming the missing flag otherwise.

A host key is presented and its age form is neither the declared recipient nor absent: refused by default, naming the mismatch, exactly the posture ssh itself takes on a changed host key rather than a safix invention — a stray key here could mean a live machine under a different identity, a wrong address, or a previous partial run that seeded something else, and none of those is safe to guess between.
An explicit `--force` proceeds past this branch alone; it has no effect on the first branch, because there is nothing "force" could mean once the target already holds the declared identity.

### D6. Person identities and systemd-credentials are named non-goals, not silent omissions

Person identities: `safix upload` parses a machine name against `flake.safix.machines` and refuses a name it does not find there the same way it refuses any other undeclared machine — it does not special-case a person's name to explain the boundary, because the refusal is already the right shape and a second message saying the same thing under a different condition would be two answers to one question.
one-unlock-bootstrap is where a person's own first-identity bootstrap belongs, tracked in that change's own tasks rather than here.

systemd-credentials: binding the host key to a TPM-backed or otherwise encrypted systemd credential at boot, rather than a plaintext file under `/etc/ssh`, is a real alternative delivery mechanism and the operator has deferred it as a feature entire.
The one sentence worth recording is the extension point rather than a design for it: `--directory DIR`'s output is a plain filesystem tree, and a future systemd-credentials backend would consume the same operator-supplied `--identity` input through a different write path from D3 rather than through a different verb, because what changes is where the bytes land and not what decides which bytes they are.

## Risks / Trade-offs

The mismatch branch in D5 is a judgment call rather than a derived fact: a changed host key is refused by default because ssh's own precedent treats a changed key as suspicious rather than routine, and that precedent is carried here on the strength of the analogy rather than on a safix-specific measurement.
`--force` exists because the analogy is not absolute — key rotation on a live machine is a real, if rare, reason to want this — and it is scoped to the mismatch branch alone precisely because the match branch has no legitimate reason to be forced.

The path-depth check in D2 is defense in depth against a destination this change does not currently make configurable.
If a future change makes `/mnt/etc/ssh` an option, the check becomes load-bearing rather than decorative, and this design does not itself add the option or say what its safe range would be beyond restating clan's own threshold.

Probing a target's presented host key before authenticating is a real network round trip on every remote-mode invocation, including the common case where the operator already knows the answer.
That cost is accepted because the alternative — trusting the operator's belief about the target's state — is exactly the belief a typo or a stale assumption falsifies, and this verb's entire reason to exist over a hand-run `scp` is refusing to trust it.
