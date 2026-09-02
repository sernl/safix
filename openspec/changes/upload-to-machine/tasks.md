Revisions are as named in `proposal.md`, and every line anchor below was read at one of them.

No real recipient, no real hostname, and no real fleet identifier from any machine enters this repository; fixtures use the `alice`, `bob` and `carol` names the existing consumption and custody fixtures already use, together with synthetic `age1` recipients and generated-for-the-fixture ed25519 keys.
Nothing here deploys, switches, or activates a real machine, per this change's own boundary requirement; the ssh transport is driven against a fixture sshd inside the test harness, mirroring how `clan-stub.rs` and `nix-stub.rs` already stand in for subprocess boundaries elsewhere in this crate.

## 1. The verb, its parsing, and the machine-targeting refusals

- [x] 1.1 Add `crates/safix-core/src/upload.rs` with the machine-resolution entry point: look up the positional name in `Workspace`'s declared machines, distinguishing "not declared at all" from "declared but no recipient" from "declared as a person"
- [x] 1.2 Add `upload_command` to `crates/safix/src/main.rs` beside the other custody-touching verbs, parsing `<machine>`, `--directory DIR`, `--identity PATH`, `--to ADDRESS`, and `--force` in any order around the one positional, matching `enroll_command`'s stated convention
- [x] 1.3 Add `upload` to `crates/safix/src/usage.rs`'s `SCAFFOLD` table and to `expected_verbs()`'s derived list, in the operator-facing row order the table carries after `rename-transfer-verbs` task 5.3 rewrites it — grouped with the other custody/identity verbs (`keygen`, `adduser`, `enroll`) rather than with the target-scoped bridge verbs `sync` and `audit` — confirmed by reading `usage.rs` rather than assumed from today's row positions
- [x] 1.4 Write the three machine-targeting refusals: undeclared name, declared machine with null recipient, and a person's name — the third reusing the undeclared-machine message rather than a distinct one
- [x] 1.5 Verify: `cargo test -p safix upload::` covers all three refusals with a fixture workspace declaring one machine with a recipient, one without, and one person; each refusal is asserted by matching its message rather than only its exit code

## 2. `--directory` mode

- [x] 2.1 Read the private key at `--identity PATH`, derive its public half and its age recipient by shelling out to `ssh-to-age`, mirroring the existing external-tool convention `keygen.rs` documents for the same conversion
- [x] 2.2 Refuse before creating `DIR` when the derived recipient does not equal the machine's declared one, naming both
- [x] 2.3 Write `DIR/etc/ssh/ssh_host_ed25519_key` at mode `0600` and `DIR/etc/ssh/ssh_host_ed25519_key.pub` at mode `0644`, creating parent directories as needed, and create no other path under `DIR`
- [x] 2.4 Refuse `--directory` given without `--identity`, before touching the filesystem
- [x] 2.5 Assert no network code path is reachable from this mode: no `Host`, no ssh connection type, is constructed when `--directory` is given
- [x] 2.6 Severity drill: a fixture whose supplied key derives to a recipient one character different from the declared one still refuses; a fixture whose declared recipient is null still refuses even with a matching key humanly indistinguishable from valid, because the null-recipient refusal in 1.4 fires first
- [x] 2.7 Verify: `cargo test -p safix upload::directory` green, file modes asserted with `std::fs::metadata`, and both drills in 2.6 observed

## 3. Remote mode: the probe and the three-way branch

- [x] 3.1 Implement the unauthenticated host-key probe: connect to the operator-named address and read the offered ed25519 host key without completing authentication, using the same `Host` abstraction the transport in group 4 will reuse
- [x] 3.2 Convert the probed key to its age form with `ssh-to-age`, the same conversion 2.1 uses, so one function serves both directions
- [x] 3.3 Implement the three-way branch: probed recipient equals declared — report and return without opening a writing session; no key presented — require `--identity` and proceed to group 4's transport; probed recipient present and unequal — refuse naming both, unless `--force` and `--identity` are both given
- [x] 3.4 Make `--force` inert on the match branch: assert that a match with `--force` and `--identity` both given still reports and still writes nothing
- [x] 3.5 Add a harness fixture sshd (or a stub presenting a scripted host key, mirroring `support/clan-stub.rs`'s pattern) with three configurations: presenting the declared recipient's key, presenting no key, and presenting an unrelated key
- [x] 3.6 Severity drill, and this is the property this whole change exists for: against the "presents the declared recipient's key" fixture, assert that no write-capable ssh session is ever opened — not merely that no file changed, since a bug that opens a session and writes nothing by coincidence would pass a weaker assertion and hide the next regression that makes it write something
- [x] 3.7 Second severity drill: flipping one byte of the declared recipient in the fixture (so probed and declared differ where they matched a moment ago) turns 3.6's assertion around — the same fixture now takes the mismatch branch and requires `--force`, proving the branch selection is driven by the comparison and not by a fixture-specific shortcut
- [x] 3.8 Verify: `cargo test -p safix upload::remote` green, both drills in 3.6 and 3.7 observed, and the match-branch assertion in 3.6 is against session construction rather than against file-write side effects alone

## 4. The transport, mirroring `clan_lib/ssh/upload.py`

- [x] 4.1 Build the tarball inside `crates/safix-core/src/staging.rs`'s `Staging`, not in an arbitrary temporary directory, carrying the same two files 2.3 writes locally
- [x] 4.2 Pack directories at mode `0700` and files at mode `0400` inside the tarball, matching `clan_lib/ssh/upload.py:56-93`'s tarinfo overrides
- [x] 4.3 Run the wipe-then-extract sequence at the fixed destination `/mnt/etc/ssh`, as `root`, matching `clan_lib/ssh/upload.py:95-121`'s `install -d && find -mindepth 1 -delete && tar -xzf -` shape
- [x] 4.4 Add the path-depth assertion from `clan_lib/ssh/upload.py:9-11,34-53` against the fixed destination, so a future change to the destination cannot silently drop below three components without this check going red
- [x] 4.5 Assert the `Staging` root is created before the tarball is written into it and is removed after the transfer, on both the success path and a simulated transport failure
- [x] 4.6 Severity drill: pointing the destination constant at a two-component path turns 4.4 red, demonstrating the check is live rather than trivially satisfied by the current constant
- [x] 4.7 Verify: `cargo test -p safix upload::transport` green, the tarball's mode bits asserted by reading the archive rather than by trusting the writer, and the drill in 4.6 observed

## 5. The command boundary: no deploy, no rebuild

- [x] 5.1 Assert that `upload_command`'s dependency graph contains no rebuild, switch, or activation invocation — grep the module for any subprocess spawn and enumerate them in a test, asserting the enumerated set contains only `ssh-to-age` and the ssh transport itself
- [x] 5.2 State in the command's success output that the machine's own next rebuild is what activates what was written
- [x] 5.3 Verify: the assertion in 5.1 is a positive enumeration rather than an absence check on one forbidden name, so a new subprocess call added later must be added to the enumeration or the test fails

## 6. Help text and the recorded absences

- [x] 6.1 Write `upload`'s help text stating the three absences the `machine-provisioning` delta's last requirement names: this command provisions machines and not people, no systemd-credentials mode exists yet, and no deploy is triggered
- [x] 6.2 Update `crates/safix/src/usage.rs`'s retired-and-reserved-verbs block — the text `rename-transfer-verbs` task 5.3 writes in place of today's "one verb that does not exist here" section — narrowing its statement that no upload verb exists to state instead that no verb exists for ongoing secret delivery, and adding the sentence distinguishing this `upload` from clan's verb of the same name, matching the `safix-cli` delta
- [x] 6.3 Verify: `safix -h` and `safix upload -h` both render the updated text, asserted by a snapshot test in the style `crates/safix/tests/` already uses for other help text

## 7. README documentation

- [x] 7.1 Add a subsection documenting `safix upload`: the two write modes, the honest no-op, the transport it mirrors, and the three named absences
- [x] 7.2 State plainly, beside the machine subject section (`README.md:177-196`), that a machine's recipient is declared before the machine can boot, and that `upload` is the step that makes the declaration true on disk
- [x] 7.3 Verify: every guarantee stated in the new README section names a check from groups 1 through 6 that holds it
- [x] 7.4 Add a `CHANGELOG.md` entry under `## [Unreleased]` naming the new `safix upload` verb, its two write modes, the honest no-op, and the `safix-cli` retired-and-reserved-verbs narrowing; verify by reading the entry against `proposal.md`'s **BREAKING** marker for completeness

## 8. Verification

- [x] 8.1 `openspec validate upload-to-machine --strict`
- [x] 8.2 `openspec validate --all --strict`, compared against the baseline reported before this change was proposed
- [x] 8.3 `cargo test -p safix-core upload::` and `cargo test -p safix upload::` both green
- [x] 8.4 `rg` the whole tree for any real fleet identifier introduced by this change's fixtures and confirm none
- [x] 8.5 Re-read `openspec/specs/safix-cli/spec.md` and `openspec/specs/plaintext-staging/spec.md` after archive-time merge (not performed by this change) would apply, confirming the two delta edits above are the only changes each requirement needs
- [x] 8.6 Wire `crates/safix/tests/upload.rs`'s integration tests into `modules/flake/checks/cli.nix` through its `mode` helper, one check per claim named in groups 1 through 5 (the three machine-targeting refusals, the directory-mode drills, the remote-mode severity drills, the transport-depth drill), each with its own perturbation drill recorded in that file's ledger comment; verify with `nix build .#checks.x86_64-linux.<check-name>` for each new check
