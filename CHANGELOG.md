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
- `Secret`, a plaintext value that zeroes on drop, is constructible only by reading a stream, and implements none of `Debug`, `Display`, `serde::Serialize`, `From<String>` or `From<&str>`.
  Those five absences are `const` assertions over a compile-time probe rather than sentences, so adding any of them fails the build.
- `packages.safix-rs`, the rust binary, and the checks `safix-rs-build`, `safix-rs-test`, `safix-rs-clippy`, `safix-rs-fmt`, `safix-rs-deny` and `safix-rs-audit`.
- The nix half read as types: placements, audiences, governed files and recipients, each denying unknown fields, so a field added on the nix side reaches a refusal rather than a reader that keeps working while answering an older question.
- The two ciphertext readers in rust, answering which recipients a document names and which keys it holds without decrypting it.
  The python helpers stay where they are: they are the oracle the rust ones are judged against.
- `SAFIX_ERROR_FORMAT=plain`, which renders a refusal in the shell runtime's shape — `safix: <message>` with two-space-indented continuations, no colour, no diagnostic code, no span.
  It changes the bytes on standard error and nothing else, and the differential harness asserts that.
- The differential harness, and the checks `safix-differential-clean`, `-missing`, `-drift`, `-orphan`, `-unknown`, `-norule` and `-drills`.
  Each drives the shell runtime and the rust runtime over one fixture fleet and compares standard output byte for byte, standard error byte for byte under the plain reporter, exit codes as numbers, and the repository through one projection applied to both sides.
  `-drills` is what keeps the rest honest: it mutates the rust side once per channel and fails unless each mutation is caught by the channel that exists to catch it.

### Unchanged, deliberately

- `packages.safix` still builds `modules/flake/safix/safix.sh`.
  The rust binary is not what ships.
  `set`, `generate`, `fix`, `keygen` and `adduser` are not ported and refuse rather than approximating; the general usage text is the shell's and is not reproduced by a binary implementing three of eight subcommands.
  Each takes over only after the differential harness has compared it against the shell runtime on standard output, standard error, exit code and effect on the repository.

### Known differences

- The shell runtime's `list` renders in the placement document's own key order, while everything else in either runtime renders sorted.
  The two coincide over `nix eval --json`, which emits every attribute set sorted, and the harness asserts its own fixture is in that order.
- The `list` table is aligned by character count where `column -t` aligns by display width.
  Every field but a generator's description is drawn from the resolver's alphabet, so the two part company only over a non-ASCII description.
- A governed path holding something that is not a YAML document is reported by neither runtime's key reader.
  The recipient half of the report does speak about it.
- The rust runtime has one refusal the shell has no counterpart for: a nix half declaring a field this runtime does not read.
