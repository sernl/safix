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
  What exists today is the `Secret` type and the refusal type; no subcommand is ported.
- `Secret`, a plaintext value that zeroes on drop, is constructible only by reading a stream, and implements none of `Debug`, `Display`, `serde::Serialize`, `From<String>` or `From<&str>`.
  Those five absences are `const` assertions over a compile-time probe rather than sentences, so adding any of them fails the build.
- `packages.safix-rs`, the rust binary, and the checks `safix-rs-build`, `safix-rs-test`, `safix-rs-clippy`, `safix-rs-fmt`, `safix-rs-deny` and `safix-rs-audit`.

### Unchanged, deliberately

- `packages.safix` still builds `modules/flake/safix/safix.sh`.
  The rust binary is not what ships and does not implement any subcommand: run it with anything but `--version` and it refuses, saying so.
  It takes over one subcommand at a time, and only after a differential harness has compared that subcommand against the shell runtime on standard output, standard error, exit code and effect on the repository.
