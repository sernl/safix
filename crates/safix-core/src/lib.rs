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
//! evaluates the attributes of `flake.safix.lib` the runtime reads, and
//! resolves a name to the file and key holding it. [`check`] is the drift
//! report over that view, and [`audit`] the one over the clan bridge — two
//! reports rather than one because the second needs the decryption and the clan
//! the first is defined by refusing. [`sops`] and [`git`] are the two subprocess
//! drivers, and [`Secret`] is what a decrypted value comes back as.
//!
//! # What is here
//!
//! One module per subcommand that does more than read: [`set`] writes one typed
//! value, [`fix`] converges the policy and the ciphertext onto the
//! declarations, [`generate`] walks the generator graph, [`keygen`] mints an
//! identity, and [`adduser`] declares a person. [`inputs`] is how a generator's
//! values reach its script, and [`scratch`] is what an aborted write must not
//! leave behind. [`definition`] is the record a mint leaves of the declaration it
//! minted under, which is what lets [`check`] report a value whose generator has
//! changed since.
//!
//! Every one of them was compared against a shell runtime by the differential
//! harness before it shipped — standard output, standard error, exit code and
//! effect on the repository, over one fixture fleet. Both that runtime and that
//! harness were retired once the port completed and the claims they carried were
//! written as assertions against literals; see
//! `openspec/changes/rewrite-runtime-in-rust/` for the port and
//! `openspec/changes/rust-only-runtime/` for the retirement.

pub mod adduser;
pub mod audit;
pub mod bridge;
pub mod check;
pub mod clan;
pub mod definition;
mod digest;
pub mod edit;
mod error;
pub mod fix;
pub mod generate;
pub mod git;
pub mod inputs;
pub mod keygen;
pub mod model;
pub mod nix;
mod probe;
pub mod progress;
pub mod public;
pub mod scratch;
mod secret;
pub mod set;
pub mod sops;
pub mod staging;
mod workspace;

pub use error::{Code, Error, Result};
pub use progress::Progress;
pub use secret::Secret;
pub use workspace::Workspace;
