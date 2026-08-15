//! The refusal type.
//!
//! Every refusal is a variant carrying the values its message interpolates,
//! rather than a variant carrying a finished sentence. That is what lets a
//! program embedding this crate branch on a refusal instead of matching on its
//! prose, and it is why rendering lives in the command rather than here.

use std::io;

/// A refusal from the safix runtime.
///
/// The variant list grows as the runtime is ported; it is marked non-exhaustive
/// so that adding one is not a breaking change for a matching embedder.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A value could not be read from the stream it was being read from.
    ///
    /// Whatever had been read when the failure happened was zeroed before this
    /// was returned; no partial value survives the error.
    #[error("could not read the value")]
    SecretRead {
        /// The underlying failure from the stream.
        #[source]
        cause: io::Error,
    },
}

/// The result type this crate returns.
pub type Result<T> = std::result::Result<T, Error>;
