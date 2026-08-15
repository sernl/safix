# Changelog

All notable changes to this project are recorded here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning policy

Two surfaces are versioned, and they are not versioned by the same thing.

The `safix-core` library's public interface is what [semantic versioning](https://semver.org/spec/v2.0.0.html) governs.
While the major version is `0`, a breaking change to that interface moves the minor version.

The `safix` command's behaviour — its subcommands, its exit codes, and the wording of its refusals — is governed by the differential harness described in `openspec/changes/rewrite-runtime-in-rust/design.md` rather than by the version number.
A refusal's prose is a tested string, so it changes when a test changes, and the changelog records it either way.

The nix half — `flake.safix.*`, the flake module, and the consumption modules — is the option surface consumers write against.
A change to it is a breaking change whether or not any rust changed.

## [Unreleased]

### Added

- `safix-core`, the runtime as an embeddable library, and `safix`, a thin command over it.
  Both forbid unsafe code.
  The read paths are ported: `list`, `get` and `check`.
  So are the two write paths that change no declaration: `set` and `fix`.
- `Secret`, a plaintext value that zeroes on drop, is constructible only by reading a stream, and implements none of `Debug`, `Display`, `serde::Serialize`, `From<String>` or `From<&str>`.
  Those five absences are `const` assertions over a compile-time probe rather than sentences, so adding any of them fails the build.
- `packages.safix-rs`, the rust binary, and the checks `safix-rs-build`, `safix-rs-test`, `safix-rs-clippy`, `safix-rs-fmt`, `safix-rs-deny` and `safix-rs-audit`.
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
- The differential harness, and the checks `safix-differential-clean`, `-missing`, `-drift`, `-orphan`, `-unknown`, `-norule`, `-write`, `-refuse`, `-guard`, `-converge`, `-abort`, `-pipes` and `-drills`.
  Each drives the shell runtime and the rust runtime over one fixture fleet and compares standard output byte for byte, standard error byte for byte under the plain reporter, exit codes as numbers, and the repository through one projection applied to both sides.
  `-drills` is what keeps the rest honest: it mutates the rust side once per channel and fails unless each mutation is caught by the channel that exists to catch it.
  `-abort` and `-pipes` are not comparisons and say so: the first holds an interrupted write to leaving nothing behind, the second reads the `sops` process' own command line and environment and holds the value to travelling down a pipe and no other way.
  The write-path comparisons add three assertions to the four channels — no candidate document left beside a target, no key disturbed that `set` was not asked to write, and two substitutions each carrying its own proof, for a side's own commit hash and its own repository root.

### Unchanged, deliberately

- `packages.safix` still builds `modules/flake/safix/safix.sh`.
  The rust binary is not what ships.
  `generate`, `keygen` and `adduser` are not ported and refuse rather than approximating; the general usage text is the shell's and is not reproduced by a binary implementing five of eight subcommands.
  Each takes over only after the differential harness has compared it against the shell runtime on standard output, standard error, exit code and effect on the repository.

### Known differences

- The shell runtime's `list` renders in the placement document's own key order, while everything else in either runtime renders sorted.
  The two coincide over `nix eval --json`, which emits every attribute set sorted, and the harness asserts its own fixture is in that order.
- The `list` table is aligned by character count where `column -t` aligns by display width.
  Every field but a generator's description is drawn from the resolver's alphabet, so the two part company only over a non-ASCII description.
- A governed path holding something that is not a YAML document is reported by neither runtime's key reader.
  The recipient half of the report does speak about it.
- The rust runtime has one refusal the shell has no counterpart for: a nix half declaring a field this runtime does not read.
- `fix` without `--yes` cannot be confirmed in the shell runtime.
  It drives its re-wrap loop with `done < <(jq -r '.managed[]' ...)`, so `sops updatekeys` inherits the pipe carrying the governed file names and reads its confirmation from there: the answer to one file's prompt is the next file's name, which is never `y`.
  The rust runtime hands `sops` the run's own standard input.
  `safix-differential-converge` pins the difference and asserts that neither re-wraps anything when no answer is available.
- The shell runtime has no response to `SIGINT` during `set`.
  At a prompt, `bash` restarts a `read` the interrupt returned from and defers its `trap 'exit 130' INT` while the stream stays open, so the run keeps waiting; during encryption, a non-interactive `bash` waiting for a foreground command ignores `SIGINT` outright, so the run writes, commits and exits zero.
  The rust runtime exits 130 in both, having swept the candidate document and written nothing.
  `safix-differential-abort` asserts both oracle behaviours, so an oracle that later acquires a response fails the check rather than quietly making the drills comparable.
- Reading the two entries from a *seekable* standard input differs between the runtimes, and the harness feeds a pipe for that reason.
  The shell runtime re-opens `/dev/stdin` for each read, which yields another handle on a pipe and a fresh description at offset zero on a regular file — so over a file its confirmation is the first line read a second time, and the double entry stops checking anything.
  The rust runtime reads the two entries sequentially in both cases.
