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
//! # The shape of a run
//!
//! [`Workspace`] is one run's view of one repository: it finds the repository,
//! evaluates the four attributes of `flake.safix.lib` the runtime reads, and
//! resolves a name to the file and key holding it. [`check`] is the drift
//! report over that view. [`sops`] and [`git`] are the two subprocess drivers,
//! and [`Secret`] is what a decrypted value comes back as.
//!
//! # What is implemented today
//!
//! The read paths — the placement resolution behind `list`, the decryption
//! behind `get`, and the four-part report behind `check`. The write paths and
//! the generator graph are not ported, and the shell runtime remains what the
//! flake's `safix` package builds. The migration is gated on a differential
//! harness rather than scheduled; see
//! `openspec/changes/rewrite-runtime-in-rust/`.

pub mod adduser;
pub mod check;
mod error;
pub mod fix;
pub mod git;
pub mod inputs;
pub mod keygen;
pub mod model;
pub mod nix;
mod probe;
pub mod progress;
pub mod scratch;
mod secret;
pub mod set;
pub mod sops;
mod workspace;

pub use error::{Error, Result};
pub use progress::Progress;
pub use secret::Secret;
pub use workspace::Workspace;
