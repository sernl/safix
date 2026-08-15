#![forbid(unsafe_code)]

//! The safix runtime as a library.
//!
//! safix is two halves. The nix half — declarations, resolution, audiences and
//! the recipient policy — is an algebra with a consumer-facing option surface,
//! and it stays in nix. This crate is the other half: the runtime that talks to
//! `sops`, to `git`, and to the operator, and it is the half that touches
//! plaintext.
//!
//! Nothing here requires a terminal. Argument parsing, prompting and the
//! rendering of refusals belong to the `safix` command; a refusal from this
//! crate is a value of [`Error`] carrying the data its message needs, so an
//! embedder can act on it without parsing prose.
//!
//! # What is implemented today
//!
//! [`Secret`] and [`Error`]. The command's subcommands are not ported yet, and
//! the shell runtime remains what the flake's `safix` package builds. The
//! migration is gated on a differential harness rather than scheduled; see
//! `openspec/changes/rewrite-runtime-in-rust/`.

mod error;
mod probe;
mod secret;

pub use error::{Error, Result};
pub use secret::Secret;
