# Changelog

All notable changes to this project are recorded here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning policy

Two surfaces are versioned, and they are not versioned by the same thing.

The `safix-core` library's public interface is what [semantic versioning](https://semver.org/spec/v2.0.0.html) governs.
While the major version is `0`, a breaking change to that interface moves the minor version.

The `safix` command's behaviour — its subcommands, its exit codes, and the wording of its refusals — is governed by `crates/safix/tests/` and the refusal snapshots rather than by the version number.
It was governed by the differential harness while a second runtime existed; that harness is described in `openspec/changes/rewrite-runtime-in-rust/design.md` and its retirement in `openspec/changes/rust-only-runtime/`.
A refusal's prose is a tested string, so it changes when a test changes, and the changelog records it either way.

The nix half — `flake.safix.*`, the flake module, and the consumption modules — is the option surface consumers write against.
A change to it is a breaking change whether or not any rust changed.

## [Unreleased]

### The generator envelope is clan's, and what that costs

This is a breaking change to the generator interface and to what a fragment written for safix 0.2 may assume.
The cost is stated first because it is the change.

0.2 documented, in the `script` option's own description, that "the fragment runs with the caller's filesystem and network".
That sentence is withdrawn.
A generator's script and its validation fragments now run inside a sandbox: the staging root is the only writable path, the nix store is read-only, and there is no network.
A fragment that read a file of the operator's, wrote a note beside its output, curled an endpoint, or reached a tool it did not declare in `runtimeInputs` worked before this change and fails after it, at the read or write itself, with the fragment's own error.

Three consequences are worth naming rather than discovering:

- `runtimeInputs` is now the whole of what a fragment can run.
  The tools are still *prepended* to the caller's `PATH`, but the paths that `PATH` otherwise names — `/usr/bin`, a NixOS profile's symlink tree — do not exist inside the envelope, so a fragment that reached an undeclared tool and worked for whoever wrote it now fails for them too.
- A validation fragment has no writable path at all.
  By the time a candidate is judged the staging root has been shredded, so that fragment gets the envelope's own temporary filesystem and nothing on the host.
  The candidate still arrives on standard input, which is a pipe and crosses the envelope as one.
- Generation refuses where no backend runs, and there is no way to proceed unsandboxed.
  Where clan offers `--no-sandbox`, safix offers nothing: on a machine whose kernel refuses the namespaces bubblewrap is made of, `safix generate` refuses and names what it looked for.

What is bought is the gap `plaintext-staging` states it cannot close on its own.
The staging root was bounded containment, and its documented limit was that a fragment which copied the plaintext it held somewhere else had put it where safix does not look and cannot shred.
That copy now fails.

The envelope is adopted rather than invented — bubblewrap on linux, `sandbox-exec` on darwin, from clan's `clan_lib/sandbox_exec` at the revision this fleet pins — because interop is the point: a fragment meets the same confinement under either system's default executor.
Three deviations from clan's argument vector are recorded in `crates/safix-core/src/sandbox.rs` and in `openspec/changes/adopt-generator-sandbox/design.md`.

The fleet needs no declaration change: no generator declared today reaches the network or writes outside its staging root.

### Added

- A third top-level tree, `state/safix/definitions/`, holding one plaintext line per generated value: a digest of the generator declaration that minted it, written by `safix generate` in the same commit as the value and refreshed by every regeneration.
  Neither existing tree could hold it. A path named for secrets has to mean that everything under it is encrypted without qualification, and `public/` means declared public outputs a nix module reads at evaluation; a bookkeeping file there would dilute that into "plaintext things safix wrote".
  The digest covers everything that decides what a mint produces — the script, `runtimeInputs`, the `network` grant, prompts, dependencies, the outputs with their secrecy, and the validation fragment — and neither of the two fields that cannot: `description` is a label and `share` is derived by the resolver from the entries.
  No value and no derivative of a value is in the record, which is what lets it be committed in the clear. It carries a leading format tag, so changing the canonical form reads as unknown-version rather than as universal drift — and that tag is at `safix-definition-v2`, because covering the grant is the first thing to have moved it.
  `safix-core` gains `definition` for the record and computes the digest itself rather than taking a dependency for it; `openspec/changes/settle-clan-vars-parity/design.md` records the two rejected locations, a reserved key inside the sops document and a derivation from git history.
- `safix check` gains a fifth finding class over that record: a generated value whose recorded definition is not the one the declarations carry now.
  It names the entry, the generator, the record, and both remedies — regenerate to adopt the declaration, revert the edit to adopt the value — and recommends neither, because the tree holds two things that disagree and nothing here knows which was meant.
  It carries no value and needs no identity: the question is answered from a digest of a declaration, and `check` still decrypts nothing.
  A value with no record predates the record and is not a finding, which is how everything minted before this change is grandfathered; a record whose format tag this version does not write is not a finding either.
  The producing generator is read off the placements through `Placements::producer_of` rather than off `flake.safix.lib.generatorPlan`, because the plan is guarded — a cycle or two producers for one output throws instead of returning an order — and a report that evaluated it would fall silent on exactly the trees whose declarations are wrong. A test binds the two readings to the one relation `resolve.nix` projects.
  This is what clan reports as `invalid_generators`; measured against the real clan, safix's record is the broader of the two, because clan's `validationHash` is null unless the generator declares `validation`.
- `safix set` reads the value from standard input when standard input is not a terminal, which makes it scriptable — the contract safix's own bridge already relies on when it writes into clan, and a dependency of the planned hardware-custody and KeePassXC work.
  The bytes are stored exactly as sent: `echo` pipes a trailing newline and `printf` does not, and nothing removes one. An empty pipe takes the empty-value refusal, because it is what a failed upstream command leaves behind.
  The terminal path is unchanged. What the piped form drops is the confirmation, and that is the point rather than a concession: the second prompt exists to catch a value mistyped invisibly, and a piped value has no typist.
  The fork is the terminal test on standard input and nothing else, which is `clan vars set`'s own branch, so one piece of calling code scripts both commands.
- `safix-generate-definition-drift` and `safix-value-source`, two more checks over the integration suite.
  The first drives the record and the four states the finding tells apart; the second drives both sides of `set`'s source fork, the piped one over a pipe and the typed one over a pseudoterminal the suite allocates — which is the only way left to reach the prompt path, since a pipe now selects the stream source.
  Four drills were observed red: never reporting drift fails the drift assertion, always reporting it fails the four silences, forcing the prompt path fails the two piped tests, and forcing the stream path fails the terminal one.
  `crates/safix/tests/abort_residue.rs` gains a fifth thing every interrupted run is held to — no definition record for a value it did not commit — and the window where that claim is not vacuous: a mint interrupted while sops holds its candidate open. Writing the record in place instead reddens exactly that test.
- `safix audit`: the report over the clan bridge.
  It compares both sides of every declared mapping, or the one named in either direction, and changes nothing on either side of the boundary.
  A mapping agrees when both sides hold the same bytes and when neither side holds a value yet; it is a finding when the two hold different values, when one side holds a value the other does not, or when the comparison could not be made at all.
  Each finding names the mapping, its two endpoints and the command that converges it, and never a value or a digest of one.
  Its exit codes are `check`'s: zero when every mapping agrees, one when any does not.
- A verb rather than rows in `safix check`, which is a decision about `check`'s contract rather than about the comparison — the comparison is the transfer's own and was already tested.
  `check` decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not have, and it needs no clan; this needs both, so the verb that needs them carries them and both of `check`'s properties stay unconditionally true.
  A mapping this operator cannot decrypt is reported as one that could not be judged rather than skipped, because a report that dropped those would be a report about who ran it.
  `openspec/changes/clan-bridge/design.md` records the decision under "The audit's shape, and its name", together with the two shapes it was taken over.
- `safix-bridge-audit`, one more single-runtime check, running `crates/safix/tests/audit.rs`.
  Its two drills were observed red: making two present values compare equal fails the diverged-mapping test and the one that transfers between two audits, and skipping the mapping that will not decrypt fails only the test written for it and leaves the rest of the target green.
- `safix generate` refuses a run order carrying a cycle of generators, naming the ones participating in it, before the first generator runs.
  Nothing a consumer's flake evaluates reaches it: `flake.safix.lib.generatorPlan` refuses a cycle at evaluation and leaves the generators inside one out of the order, so an order carrying one came from a stand-in for nix or from a program embedding `safix-core`, for which the plan is a value with public fields rather than that refusal's postcondition.
  Refused before the walk rather than at the generator whose input never arrives, because a run commits as it goes: the alternative mints and commits every generator ahead of the cycle and then reports the first missing input as an empty output.
  `Error::GeneratorCycle` carries the participating generators, so an embedder branches on them rather than on the message.
- `safix-generate-cycle` and `safix-identity-recipiency`, two more checks over one test each.
  The first holds the cycle refusal to when it arrives; the second holds the sentence the consumption module's identity preflight makes about what it did not check — that an identity present and readable and not a recipient of these files does not open them — against fixture ciphertext, which is the one claim on that path an evaluation cannot hold.
  Both drills were observed red, each on the assertion written for it.
- `safix-bridge-real-clan`, the one check that drives the real clan command rather than the stub, over a throwaway clan built inside the check: one machine, three `age`-backed generators, an identity minted per run and a recipient derived from it.
  Every other bridge check drives a stub, which can be asked what it was handed but would go on answering safix's argument vectors after clan changed its command line; this one establishes that those arguments mean to clan what safix thinks they mean.
  It asserts eleven claims through `crates/safix/tests/real_clan.rs`: the raw bytes off a real `clan vars get`, a real `clan vars set` fed on standard input and committing in clan's own repository, a second run of either verb leaving both histories where they were, the two absent-var states told apart by clan's own words, the drift refusal against a real `clan vars check`, `audit` finding a real divergence and finding none once a transfer resolves it, and clan's repository unchanged across every read safix makes.
  Absent rather than trivially green off linux, and its two drills were observed red: withholding the throwaway clan makes all eleven tests report an absence and libtest call them passed, which the result-line guard catches, and putting the stub in the real command's place fails ten of the eleven.
- `network` on the generator submodule, default `false`: the one capability a fragment can be granted, and the only thing the grant does.
  `true` re-shares the network and leaves the filesystem confinement in force, and it governs `script` and `validation` alike, because a validation that verifies a minted token against the API that issued it has the same need its script had.
  It is a declaration rather than an invocation flag so that which generators may reach the network is a question the tree answers at evaluation, with no runtime consulted and no record to keep of who passed what when.
  `safix-generators` reads both answers out of the resolved placement record, which is the audit surface itself.
- `Error::SandboxUnavailable` and `Error::SandboxUnsupported`, raised by an availability probe that runs once before the first fragment: the first names the backend that did not run, the second says the platform has no envelope.
  Once, because availability does not change mid-run and a refusal after generator three has committed is worse than the same refusal before generator one.
  Neither is convertible into an unsandboxed run, and `generate` now refuses any long flag it does not take, so `--no-sandbox` gets the usage line rather than a refusal about a secret nobody declared.
- `safix-generate-envelope` and `safix-generate-no-bypass`, over `crates/safix/tests/sandbox.rs`.
  The first is linux-only and holds the confinement behaviourally: a fragment writing into the repository fails and the run refuses with that fragment's own failure having stored nothing; a fragment with no grant cannot reach a listener the test holds on loopback, and that listener accepts nothing; with the grant the connection reaches it carrying what the fragment sent while the same write into the repository still fails; a withheld backend refuses before the first fragment, which the refusal's code is what establishes.
  Each escape fixture is drilled against an unconfined run of the same fragment, so an absent file is the envelope's doing rather than the fragment's, and the check reads the suite's absence sentence out of the output so a kernel that refuses the namespaces reports that rather than going green.
  The second is every platform's, because no fragment runs for the argument reader to refuse a flag.
- The envelope's other half in `safix-syscall-proof`: a fragment's open of a file in the repository, refused by the kernel and observed from outside the runtime, with an open inside the staging root succeeding in the same trace so the refusal is the envelope's rather than a fragment that never tried.
- `safix-generate-network-drift`, over the coupling between the envelope and the definition record: a generator that gains `network` reads as definition drift with nothing else about it changed.
  Every other field the digest covers changes what a mint does; this one changes what it *may* do, and the ciphertext of a value minted without the grant is indistinguishable from one minted with it. A digest that left the grant out would report nothing over the flip, which is what makes the test a drill on the coverage rather than on the report.
  This is why the format tag moved: covering a new field moves every digest, so every record written before it now reads as unknown-version and produces no finding, which is the grandfathering the tag exists for rather than a concession made for this change.
- `clan-core` as a flake input, read by that check and by nothing else.
  clan-cli is not packaged in nixpkgs, so there is no attribute to reach for, and the input also supplies `packages.clan-core-flake` — a clan-core whose lock names store paths rather than URLs — which is what lets the throwaway clan lock in a sandbox with no network.
  Pinned to a revision rather than a branch, because the subject of the check is a specific clan's behaviour and an input that moved on every `nix flake update` would redden it for reasons unrelated to safix.

### Changed

- `README.md` opens by stating who the project is for: its operator's own fleet and use case.
  The status section's subcommand count now includes the bridge pair and `audit`, the stale no-remote sentence is gone, and the port history is condensed to what the changelog and the openspec records do not already hold.
- Piping a value into `safix set` is no longer the prompt path reading two lines from a pipe.
  A caller that wrote the value twice into standard input now stores both lines and the newline between them, because that is exactly what it sent. The scripted form is one value: `printf '%s' "$TOKEN" | safix set ana grafana-token`.
  This is the one consumer-visible behaviour change in this release, and it is the deliberate half of the feature rather than a side effect of it.
- The catch-all policy check's probes gain the definition-record tree's two shapes, beside the public store's, so a generated creation rule that reached either fails a check rather than encrypting a plaintext record the next time anyone ran sops against that path.
- The recorded absences stand, re-examined rather than restated. No `upload`, because activation already delivers what it would; no plaintext dump and restore, because such a tree outlives the migration that justified it and the backend count is still one.
  Both were re-examined against the custody-subjects extension directed on the same day: machines and services joining the audience model changes who may be in an audience, not how a value reaches a machine.
  The absence of clan's flake-level per-export generator placement is not recorded as a non-goal either, because that question dissolves into that extension rather than standing as an absence.

### Fixed

- The design record's account of what invalidates a clan generator's recorded validation, corrected against the real clan.
  `validationHash` is null unless the generator declares `validation`, and a null-in-nix, null-on-disk pair counts as valid, so a change to a generator's `script` alone does not make clan call it stale — only a change to its declared `validation` does.
  A generator that declares `validation` and has never run *is* reported stale, so `safix export` refuses the first export into one, which is correct and was not a state any stub fixture would have produced.
  No runtime behaviour changed; what changed is that the claim is now asserted by a check rather than recorded from a reading.
  `openspec/changes/clan-bridge/design.md` holds both findings under "Landing the real clan as a check".

## [0.2.0] — 2026-08-16

`Cargo.toml` still reads `0.1.0`.
Cutting the version is a release decision and is not made by this section.

### The generator contract is clan's, and what that costs

This is a breaking change to the nix option surface, to the generator interface, and to safix's most load-bearing promise.
The cost is stated first because it is the change, not a side effect of it.

safix 0.1 required that a generated value "travels a pipe and never argv, the environment, or a file", and that values "move through pipes only".
The interoperable generator contract is a filesystem contract: `$out/<name>`, `$in/<generator>/<file>` and `$prompts/<key>` are paths, and an editor edits a file.
There is no version of clan compatibility that keeps the pipe.
Emulating one with FIFOs was considered and refused: a FIFO is not seekable and not re-readable, so `head -c 32 "$in/dep/key"` and any tool that opens its input twice would break with a truncated secret as the failure mode, and a directory of FIFOs cannot answer `ls "$out"` or `[ -f "$out/x" ]`, which scripts written against this contract legitimately do.

So the absolute is replaced by a bounded containment, and the two are not equivalent:

| | pipe (0.1) | tmpfs staging (0.2) |
|---|---|---|
| plaintext at rest on a block device | never | never, unless `--allow-disk-staging` is passed |
| plaintext in memory | for the transfer | for the run |
| plaintext in swap | possible | possible |
| reachable by another process of the same user | no | yes, for the run's duration |
| reachable by root | yes | yes |
| survives a crash | no | no, if the sweep runs; yes, in the window before it |

The fourth row is the one that is genuinely worse.
A pipe between two processes safix spawned is reachable by neither a third process nor a shell; a mode-`0700` directory on `/dev/shm` is reachable by anything running as that user, which on a workstation includes the operator's own shell, editor and agent processes.

What is retained: the pipe requirement is modified rather than deleted.
`set` from standard input, `get` to standard output and every sops invocation still travel pipes end to end, and no value reaches an argument vector or an environment variable on any leg.
`crates/safix/tests/syscall_proof.rs` holds that at the system call, admitting exactly two destinations — a pipe, and a file inside the run's staging root — and sweeping every staging root afterwards, so admitting the second is not a weakening of the reading.

### Added

- `plaintext-staging`: a mode-`0700` directory on a filesystem verified with `statfs` rather than inferred from its name, one per run, every file `0600`, registered for removal before it is created and swept on return, on error, on panic and from both signal handlers.
  There is no fallback to `/tmp` — this fleet's is ext4, so a silent fallback would be the exact failure the rule prevents under a code path that looks like it succeeded.
  `--allow-disk-staging` accepts a disk-backed directory; `SAFIX_STAGING_DIR` names the mount to use, replacing the conventional candidates rather than preceding them, and is verified like any other.
  Replacing is deliberate twice over: an operator who sets it has said where plaintext goes, and a runtime that tried it and quietly staged elsewhere would look like it honoured the setting; and with a fallback behind it no drill on a host that has a tmpfs could ever reach the refusal, so the rule would be asserted only by reading the code.
  Two residual exposures are documented rather than smoothed over: a page swapped before the overwrite is not reached, and the directory is readable by every process running as its owner for the run's duration.
- `files.<name>.secret = false`: a public output, stored in the clear at `public/safix/users/<user>/<name>/value` or `public/safix/shared/<audience>/<name>/value`, given no creation rule, and readable at evaluation through `flake.safix.lib.publicValue`.
  `flake.safix.lib.outputPath` answers for every output and is a path, never a value.
  The store sits under a top-level prefix rather than inside `secrets/`, because a path named for secrets has to mean everything under it is encrypted without qualification.
  The default is `secret = true` rather than clan's `false`: a mistyped field that leaves a value encrypted is recoverable by fixing the typo, and one that publishes a value is not.
- `safix edit <name>`: the operator's own editor on one value, `$VISUAL` then `$EDITOR`, refusing when neither is set and adding no fallback program.
  A non-zero exit writes nothing, an unchanged buffer commits nothing, an emptied buffer takes the existing empty-value refusal, and a changed buffer goes through the same write path `set` uses.
  A verb rather than an option on `set`, because the two have different custody profiles and an option would make custody a function of a flag.
- `share` on a generator: derived from its outputs rather than authored, true exactly when every entry it writes is `shared`, and refused when the outputs disagree.
  That constrains a generator's outputs to one audience, so one file, so one rename — which closes the crash window a keypair's two renames used to open.
  It does not close in general: a `--regenerate` cascade still commits per generator.
- `checks.safix-public-no-rule`, matching every generated creation rule against every public path.
  The public store's shape also joins `catchAllProbes`, so a rule reaching it fails two checks that ask different questions.
- `flake.safix.bridge`: the declared relationship between a clan var and a safix entry.
  One `clanFlake` per consumer, and `mappings.<id>` naming a clan machine, generator and file, a safix user and name, and a direction.
  Direction is written as its endpoints — `clan-to-safix` or `safix-to-clan` — rather than as `import` or `export`, because `clan vars export` moves values out of clan and `safix export` moves them in, so a direction spelled with either word means opposite things depending on which tool the reader has in mind.
  Evaluation refuses five mistakes that are local to the consumer and claims nothing about the clan half, which lives in another flake.
- `safix import` and `safix export`, one per direction, acting on every mapping of theirs or on the one named.
  Each mapping is reported as unchanged, updated, absent at source, or refused with its reason; a refused mapping does not stop the others and the run exits non-zero.
  An imported value is written by the same path a hand-supplied value takes, so it acquires the recipient-drift refusal, the staged write and rename, and a commit naming the mapping and the direction.
  An exported value is written by `clan vars set` with the value on standard input, and clan commits it in clan's own repository.
- Both verbs read both sides before writing either, and an agreeing mapping is not written and not committed.
  For export the comparison is load-bearing rather than an optimisation: clan's write is unconditional and its `age` backend re-encrypts an unchanged value into fresh ciphertext, so without it every run would commit in the clan repository for every mapping, forever, each diff decrypting to what it decrypted to before.
- Refusal codes `safix::mapping_wrong_direction`, `safix::source_has_no_value`, `safix::source_unreadable` and `safix::generator_definition_drifted`, alongside `safix::clan_unavailable`, `safix::clan_pipe_missing`, `safix::clan_var_unknown`, `safix::clan_command_failed`, `safix::unknown_mapping` and `safix::no_clan_flake`.
- Refusal codes `safix::staging_not_memory_backed`, `safix::staging_unusable`, `safix::generator_output_missing`, `safix::no_editor`, `safix::public_not_editable` and `safix::editor_failed`.

### Changed — breaking

- A generator script writes `$out/<name>` per declared output instead of printing its value.
  Standard output is no longer a value; it reaches the operator like standard error.
- A prompt is the file `$prompts/<name>`, and `$prompts` is created only when prompts are declared — clan's behaviour, matched so no script comes to rely on the difference.
  Unlike clan, an ambient `$prompts` is removed from the environment rather than inherited.
- A dependency is the file `$in/<generator>/<name>`, keyed by the entry the producing generator is declared on.
  A dependency nothing generates — a hand-set value, which safix has and clan does not — is keyed by its own name.
  Only declared dependencies are placed under `$in`, where clan places every file of the dependency generator: safix's edge names an entry, and materializing its siblings would hand a script plaintext it never declared.
- Output bytes are stored exactly as written.
  0.1 stripped one trailing newline from a single-output value; under this contract the file *is* the value, and removing a byte would corrupt every key whose last byte is a newline.
  A generator that wants no trailing newline writes with `printf`.
- `generator.files` is an attribute set carrying `secret` rather than a list of names.
- The JSON multi-output form is gone: several outputs are several files, so `safix::generator_not_an_object` and `safix::generator_keys_differ` are retired and replaced by `safix::generator_output_missing`, which names the absent output and lists what `$out` did contain.
- The `$in_<name>` descriptor interface is removed with no compatibility mode, and evaluation refuses three shapes by name: a script referencing `$in_`, a script referencing `$out_name` where it names nothing, and a script that never references `$out`.
  Each names this change and gives the rewrite.
  A compatibility mode was refused because the two interfaces differ in custody rather than in spelling: a run containing both would stage plaintext on a filesystem while the per-generator documentation claimed a descriptor-only guarantee.
- The hyphen-to-underscore identifier mapping and the input-collision refusal that rested on it are gone.
  Prompts and dependencies no longer share a name space, because one lives under `$prompts` and the other under `$in`.
- `safix::dependency_has_no_value` names the dependency and the path it would have been written to.
- A public output is not selected for the secret provisioner.
  It has no ciphertext, no key and no creation rule, so an entry for one would be an activation that fails to extract a key which will never exist.
  It stays in `flake.safix.lib.placements`, where `generate`, `list` and `check` read it.

### Fixed

- A signal arriving while a generator's script or its validation fragment is running now ends the run with 130 or 143 rather than reporting the generator as having failed.
  A script the operator interrupted is a child that died on a signal, so what `wait` reports carries no exit code; read as an ordinary result that is a failure, and the run ended with 1 and a sentence blaming the script for the operator's Ctrl-C.
  The validation case was worse: it said the candidate had been judged and rejected when nothing judged it.
  Both readings are now taken before the status is interpreted, and the child's own termination signal is read alongside the handler's flag — a keyboard interrupt reaches the whole process group, so the child dies at the same instant the runtime is signalled and the flag may not be set yet.
  Only `SIGINT` and `SIGTERM` are read that way; a child killed by `SIGSEGV` failed.
- The quiescence lock now covers the generator's script and its validation, as it already covered sops.
  Without it the signal handler's sweep could remove the staging root while the script was still writing into it.

### Not adopted

- clan's `validationHash`, as a thing safix computes, records or writes.
  It answers "has the definition changed such that this value is stale"; safix's `validation` answers "is this candidate acceptable before it is written".
  Neither subsumes the other, and safix writes into git, so the failure to prevent is a bad value reaching a committed file rather than a stale one persisting.
  `validation` is unchanged: the candidate arrives on standard input and `$out_name` names the output under judgement.
  What safix does do with clan's is *ask about it*: `safix export` refuses a mapping whose clan-side generator clan already considers stale, and it establishes that by running `clan vars check --generator` rather than by reading the hash clan recorded, which is a file in clan's store.
  Nothing in the runtime writes that record.
  See "The bridge" below for why the refusal exists at all.
- clan's generator sandbox.
  clan runs a script inside `sandbox_cmd` with the staging root as the only writable path and refuses when sandboxing is unavailable.
  Adopting it is a second material change to what a generator may do, would break a generator that reaches the network, and is separable from the interface change.
  Whether it becomes its own 0.2 change or is deliberately out of scope is an open question for the operator.

### The bridge, and the two decisions it turned on

The operator's requirement was bidirectional and explicit: values move from clan into safix and from safix into clan, top to bottom and bottom to top.
Two questions gated it, and both are answered here rather than left to be inferred from the code.

*clan is reached only through clan's own command, in both directions.*
The brief specified that import decrypt clan material with the operator's admin identity, and that is not what shipped.
This fleet's clan sets `secretStore = "age"`, so "decrypt clan material" would have meant implementing clan's age backend — its directory scheme, its recipient sidecars, its stanza type — inside safix: a second decryption path for a store safix does not own, whose layout is versioned by someone else, and which would silently support that one backend and no other.
Symmetric delegation costs one thing, which is that a consumer without clan-cli cannot import either, and that is arguably correct because a consumer with no clan has nothing to import from.
The runtime reads, writes, encrypts, decrypts and parses none of clan's stored files, and `safix-bridge-transfer` drives a clan that records what it was handed rather than asserting this from the shape of the code.

*An export into a generator clan already considers stale is refused, not written.*
clan records a validation per generator and regenerates when the recorded one no longer matches the definition.
`clan vars set` does not update that record, so changing a clan-side generator's definition and then running a routine `clan vars generate` silently replaces whatever was exported.
The hazard is that it is triggered by editing a nix file rather than by running a command, and that `clan vars get` keeps returning the old value in the meantime — both confirmed against a real clan rather than inferred.
So `safix export` asks clan whether the generator is stale and refuses when it is, naming both remedies: bring clan's side back into agreement, or declare the mapping `clan-to-safix` instead, which is the right shape when clan's generator is the producer.
There is no flag that exports anyway, because safix has nowhere to record that a var is externally supplied and a flag would turn a refusal into a silent loss.

One requirement was deleted rather than implemented.
The bridge surface once required evaluation to refuse a `safix-to-clan` mapping whose source entry had "neither a generator nor a declared value", and that has no referent: an entry declares where a value lives rather than that one is there, so at evaluation a hand-set entry before its first write and one after it are the same declaration, and refusing on it would refuse the ordinary export.
It is replaced by a run-time refusal — export refuses when the source key is absent from the source file, naming `safix set` and `safix generate` — and the evaluation-time silence is still asserted, beside it, as its sibling.

### Removed

- The shell runtime, `modules/flake/safix/safix.sh`, 2149 lines, and `packages.safix-sh` with it.
  The package set is now `[ "safix" ]` alone.
- The behavioural suite `safix-selftest.sh`, 1741 lines, and the comparative harness `safix-differential.sh`, 2153 lines.
- The python ciphertext readers `sops_recipients.py` and `sops_keys.py`, 81 lines each, and `readers.nix` which packaged them.
  python3 and pyyaml leave this repository's closures entirely.
- `package.nix`, which built the shell runtime, and `checks/differential.nix`, which drove the comparison.
- The nineteen `checks.safix-differential-*` attributes.
  Four survive under new names; see Changed.
- bash, util-linux, diffutils, findutils, gnugrep, gnused and jq leave the check harnesses with the scripts that needed them.

The oracle's service, recorded because retiring it retires nothing else.
At commit `8409f15` the differential gate was green across every subcommand the shell runtime had, over nineteen modes — `clean`, `missing`, `drift`, `orphan`, `unknown`, `norule`, `write`, `refuse`, `guard`, `converge`, `abort`, `pipes`, `generate`, `regenerate`, `genrefuse`, `keygen`, `adduser`, `drills` and `strace` — comparing standard output byte for byte, standard error byte for byte under the plain reporter, exit codes as numbers, and the repository through one projection applied to both sides.
That is a fact about a state of the tree, and facts about past states are what version control holds: the harness, the runtime and the readers are all reachable at `8409f15`.
Keeping the oracle alive would not have preserved that fact; it would have produced a new one on each run, about a pair of runtimes only one of which anyone runs.

### Added

- `crates/safix/tests/`, the integration suite: 66 tests across thirteen targets, driving the built binary against throwaway repositories with real sops, real age, real git and a real `nix-instantiate --parse`.
  Only `nix` is stubbed, by a binary the suite builds itself which asserts the attribute path it was asked for.
  Each of the eighteen retired behavioural modes is one test asserting against a literal — the value that should be at that key, the paths that should be in that commit, the files that should not exist after that abort — rather than against a second implementation.
- `checks.safix-integration`, which compiles the suite once, runs it whole in the sandbox, and leaves the test binaries and the three programs they drive in its output.
  Every check naming one mode runs one test of that build; the runner reads the result line rather than the exit status, because libtest exits zero having run nothing when a filter names no test.
- The suite stages plaintext in a mode-700 directory on tmpfs, verified as tmpfs at runtime rather than assumed, and removed on every exit path including a panicking one.
  A platform without one refuses unless `SAFIX_TEST_DISK_STAGING` says the caller accepts disk-backed staging.

### Changed

- The eighteen `checks.safix-*` behavioural attributes keep their names and change their subject: from a shell script judged against a fixture to the shipped binary judged against a literal.
  A consumer's CI keeps running the check it configured.
- Four differential modes were never comparisons and are re-expressed as single-runtime checks: `safix-differential-abort` becomes `safix-abort-residue`, `-pipes` becomes `safix-value-pipe`, `-strace` becomes `safix-syscall-proof`, and `-drills` becomes `safix-channel-drills`.
  `safix-channel-drills` gains the exit-status channel, which a comparison got for nothing and a single runtime must assert deliberately, and now requires each mutation to be caught by its own channel and by no other.
  Two more single-runtime checks sit beside them with no differential ancestor: `safix-bridge-transfer`, which drives both bridge directions against a clan that records what it was handed, and `safix-memory-backing`, which holds the tmpfs rule against the kernel's own mount table rather than against the probe that enforces it.
- `checks.safix-rs-test` runs `--lib --bins`.
  It had been running every target since the integration suite landed, without the backends those tests need.

## [0.1.0] — unreleased

### Added

- `safix-core`, the runtime as an embeddable library, and `safix`, a thin command over it.
  Both forbid unsafe code.
  Every subcommand is ported: the read paths `list`, `get` and `check`, the write paths `set` and `fix`, the generator graph behind `generate`, and the two that touch custody itself, `keygen` and `adduser`.
- `Secret`, a plaintext value that zeroes on drop, is constructible only by reading a stream, and implements none of `Debug`, `Display`, `serde::Serialize`, `From<String>` or `From<&str>`.
  Those five absences are `const` assertions over a compile-time probe rather than sentences, so adding any of them fails the build.
- The checks `safix-rs-build`, `safix-rs-test`, `safix-rs-clippy`, `safix-rs-fmt`, `safix-rs-deny` and `safix-rs-audit`.
- The nix half read as types: placements, audiences, governed files and recipients, each denying unknown fields, so a field added on the nix side reaches a refusal rather than a reader that keeps working while answering an older question.
- The two ciphertext readers in rust, answering which recipients a document names and which keys it holds without decrypting it.
  The python helpers stay where they are: they are the oracle the rust ones are judged against.
- `SAFIX_ERROR_FORMAT=plain`, which renders a refusal in the shell runtime's shape — `safix: <message>` with two-space-indented continuations, no colour, no diagnostic code, no span.
  It changes the bytes on standard error and nothing else, and the differential harness asserts that.
- `set`, which prompts twice without echoing, writes through `sops set --value-stdin --idempotent`, and commits the one file it wrote.
  The value is JSON-encoded in the process rather than through a `jq` subprocess, and reaches `sops` only down a pipe.
  A write is refused before the operator types anything when the repository is in a state a commit would misrepresent, and refused after encryption and before the rename when the document `sops` produced names recipients the declarations do not — `sops set` takes an existing file's recipients from that file, so a value minted into a drifted file would be wrapped for the audience that used to be, and this commits what it writes.
- A candidate document is prepared beside its target and renamed into place, and is shredded on every path out including a caught signal.
  `SIGINT` and `SIGTERM` exit 130 and 143 after sweeping, and a signal arriving while `sops` holds the candidate open is acted on once `sops` has been waited on and before the rename, so the target file is as it was and nothing reached the history.
- `fix`, which regenerates `.sops.yaml` from the declarations and then re-wraps each governed file to it, in that order because re-wrapping first re-wraps to a policy about to change.
  Without `--yes` it runs one file at a time with `sops` holding the operator's own streams; with `--yes` it re-wraps several at once under a semaphore, bounded by `SAFIX_FIX_CONCURRENCY` (default 4), replaying each file's output in the order the declarations name the files.
  Setting that bound to `1` returns the `--yes` path to inheriting the streams.
- `generate`, which walks the topological order `flake.safix.lib.generatorPlan` computes, one generator at a time.
  Each prompt and each dependency reaches the script as `$in_<name>`, holding the path of an inherited read-only descriptor: a prompt travels down a pipe a thread feeds and a dependency down the one `sops` writes into, so neither value is ever a file.
  The close-on-exec flag comes off the read end alone, immediately before the spawn, and the parent drops its own copy immediately after — which is what keeps a generator that ignores a dependency from blocking the `sops` feeding it.
  The walk is sequential rather than fanned out over independent branches: a prompt is read from one standard input, the commits are the plan's order rather than the scheduler's, and a process spawned between the handover and the exec would inherit what the generator was given.
  One output takes the script's standard output with one echo-shaped trailing newline removed from a single-line value; several take a JSON object keyed by output name, and all of a generator's outputs land in one commit.
  `--regenerate` carries the whole downstream set, lists it, and asks before the first commit rather than after the last.
- `keygen`, which appends an age identity to the file sops reads identities from and never truncates it, prints the public half alone, and refuses to mint for anybody but the caller without `--for-someone-else`.
- `adduser`, which writes one custody record, regenerates the policy that declaration implies, and commits the two — staging the scaffold before the regeneration, because a flake evaluation sees the files git knows about and would otherwise write the policy of a tree without the person just declared.
  The recipient's shape is checked and nothing else; a recipient needing a card, a PIN and a touch is refused for this field because activation decrypts non-interactively.
  Everything past the declaration reaches `flake.safix.onboardingHook`, and no hook configured is a supported configuration.
- `safix --version`, which the shell runtime has no answer for; see "Known differences".
- The differential harness, and the checks `safix-differential-clean`, `-missing`, `-drift`, `-orphan`, `-unknown`, `-norule`, `-write`, `-refuse`, `-guard`, `-converge`, `-abort`, `-pipes`, `-generate`, `-regenerate`, `-genrefuse`, `-keygen`, `-adduser` and `-drills`.
  Each drives the shell runtime and the rust runtime over one fixture fleet and compares standard output byte for byte, standard error byte for byte under the plain reporter, exit codes as numbers, and the repository through one projection applied to both sides.
  `-drills` is what keeps the rest honest: it mutates the rust side once per channel and fails unless each mutation is caught by the channel that exists to catch it.
  `-abort` and `-pipes` are not comparisons and say so: the first holds an interrupted write to leaving nothing behind, the second reads the `sops` process' own command line and environment and holds the value to travelling down a pipe and no other way.
  The write-path comparisons add three assertions to the four channels — no candidate document left beside a target, no key disturbed that `set` was not asked to write, and two substitutions each carrying its own proof, for a side's own commits and its own repository root.
  The commit substitution is positional over that side's own history, because `generate` commits once per generator and a single marker would let a runtime name the wrong one of its own commits and still compare equal.
  `-keygen` is not a byte comparison either: two correct runs mint two different identities, so each side is held to the property — one identity appended, the file readable by its owner alone, its public half printed and its private half not, the repository untouched — and only the rendering is compared with the recipient normalized away.
- `safix-differential-strace`, linux only, which runs one `set` and one `generate` under `strace -f -y` and holds every `write` carrying a fixture value to a descriptor `strace` resolves as a pipe.
  Where `-pipes` shows the two routes the value did not take, this shows the one it did, for both runtimes.
  It carries its own drill: a runtime that writes a value to a regular file has to be caught, and caught by the pipe assertion rather than incidentally by the residue sweep.
  It is linux only because it needs `ptrace`; on other platforms the attribute is a derivation that says it observed nothing.

### Changed

- `packages.safix` is the rust binary.
  The shell runtime becomes `packages.safix-sh`, installed under that name so that holding both in one profile is not a collision over one path.
  It is kept in the tree, built and linted, because it is the oracle every `safix-differential-*` mode compares against: retiring it would retire the evidence that the two agree.

### Unchanged, deliberately

- The nix half.
  `flake.safix.*`, the resolution algebra, the recipient policy renderer and the consumption modules are the consumer-facing option surface and were never in scope; what was replaced is the runtime.
- `modules/flake/safix/sops_recipients.py` and `sops_keys.py`, for the same reason `safix-sh` is kept: they are what the rust readers were judged against.

### Known differences

These were the places the two runtimes were deliberately pinned apart rather than held to agreeing.
They are decisions, not observations, so they outlive the comparison that recorded them.
Each is stated as what this runtime does, with the behaviour it diverged from named as history and the check that holds it today named where one does.

- `list` renders sorted, as everything else in this runtime does.
  The shell runtime rendered in the placement document's own key order; the two coincided over `nix eval --json`, which emits every attribute set sorted.
  `safix-get-list` asserts the rows and the order.
- The `list` table is aligned by character count.
  The shell runtime piped through `column -t`, which aligns by display width; every field but a generator's description is drawn from the resolver's alphabet, so the two parted company only over a non-ASCII description.
  `safix-get-list` asserts the column offsets.
- A governed path holding something that is not a YAML document is not reported by the key reader; the recipient half of the report does speak about it.
  This was true of both runtimes and is a property of reading a document's shape without decrypting it.
- A nix half declaring a field this runtime does not read is refused.
  Every schema it reads denies unknown fields, where the shell runtime's `jq` expressions selected the fields they knew and ignored the rest — so a field added on the nix side reaches a refusal here rather than a reader that keeps working while answering an older question.
  The refusal's rendering is held by the `nix_schema_mismatch` snapshot.
- `safix --version` prints the package name and version on standard output and exits zero.
  The shell runtime reached its unknown-subcommand refusal and exited 1.
  A strictly wider surface rather than a different answer to a question both were asked, and the convention for a compiled binary.
  `safix-integration` asserts it; `safix-differential-unknown`, which pinned it before, went with the oracle.
- `fix` without `--yes` hands `sops updatekeys` the run's own standard input, so its confirmation is answerable.
  The shell runtime drove its re-wrap loop with `done < <(jq -r '.managed[]' ...)`, so `sops` inherited the pipe carrying the governed file names and read its confirmation from there: the answer to one file's prompt was the next file's name, which is never `y`.
  What is asserted today is the convergence rather than the interactive confirmation — `safix-governed-extras` runs `fix --yes` and holds `check` to having nothing left to report.
  The interactive path is exercised by no check; that is a gap this change did not close.
- `SIGINT` and `SIGTERM` during `set` exit 130 and 143, having swept the candidate document and written nothing.
  The shell runtime responded to neither: at a prompt, `bash` restarted the `read` the interrupt returned from and deferred its `trap 'exit 130' INT` while the stream stayed open, so the run kept waiting; during encryption, a non-interactive `bash` waiting for a foreground command ignored `SIGINT` outright, so the run wrote, committed and exited zero.
  `safix-abort-residue` holds all four windows, and `safix-abort` holds the two the behavioural suite covered.
- A git that exits non-zero is a refusal like any other: `safix: git <arguments> failed`, and exit 1 whatever git exited with.
  The shell runtime ran under `set -e` and exited with git's own status, saying nothing of its own.
  The extra line names the subcommand that stopped the run, which git's own message — about a lock file — does not.
  The refusal's rendering is held by the `git_command_failed` snapshot; `safix-differential-write`, which drove it under two git statuses end to end, went with the oracle, and no check drives it end to end today.
- The two entries are read sequentially from standard input, whether it is a pipe or a regular file.
  The shell runtime re-opened `/dev/stdin` for each read, which yields another handle on a pipe but a fresh description at offset zero on a regular file — so over a file its confirmation was the first line read a second time, and the double entry stopped checking anything.
  The checks feed a pipe, so the seekable case is exercised by no check; it was a property of the retired runtime's re-opening and this runtime has no such branch.
