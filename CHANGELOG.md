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

## [0.2.0] — unreleased

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
