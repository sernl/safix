//! How a refusal reaches the operator.
//!
//! Two renderings of the same value. The graphical one is `miette`'s, with a
//! diagnostic code and the help text that names the way out; it is what an
//! operator sees. The plain one is the shell runtime's shape exactly —
//! `safix: <message>`, no colour, no code, no span — and it exists so that the
//! differential harness can compare standard error byte for byte instead of
//! matching a pattern over a graphical rendering, which would be a comparison
//! whose strictness nobody could state.
//!
//! Selecting a reporter alters the bytes on standard error and nothing else.
//! It does not touch standard output, the exit status, or anything the run does
//! to the repository, and the harness asserts that by running the same
//! invocation twice.

use std::fmt::Display;

use miette::Diagnostic;
use safix_core::Error;
use thiserror::Error as ThisError;

/// The environment variable that chooses the rendering.
pub const FORMAT_VARIABLE: &str = "SAFIX_ERROR_FORMAT";

/// The value of [`FORMAT_VARIABLE`] that selects the shell runtime's shape.
pub const PLAIN: &str = "plain";

/// The program name every refusal is prefixed with.
///
/// A literal rather than `argv[0]`, because the shell runtime uses a literal:
/// the binary is installed as `safix-rs` for as long as the port is in
/// progress, and a refusal that named itself after the file would differ from
/// the oracle on every line it prints.
pub const PROGRAM: &str = "safix";

/// A refusal on its way to a terminal.
///
/// The runtime's refusals arrive wrapped rather than rendered, so that
/// `miette`'s trait is implemented at the edge and not on the library's own
/// type: an embedder takes [`Error`] and no renderer with it. The other
/// variants are the command's own — how it was invoked is the command's
/// business and the library has no opinion about it.
#[derive(Debug, ThisError)]
pub enum Refusal {
    /// The runtime refused.
    #[error(transparent)]
    Runtime(#[from] Error),

    /// The subcommand was given arguments it does not take.
    #[error("usage: {PROGRAM} {form}")]
    Usage {
        /// The one-line form, as the shell runtime spells it.
        form: &'static str,
    },

    /// No such subcommand.
    #[error(
        "unknown subcommand '{subcommand}' \
        (expected set, get, list, generate, check, fix, keygen or adduser)"
    )]
    UnknownSubcommand {
        /// What was asked for.
        subcommand: String,
    },

    /// A subcommand the shell runtime has and this binary has not reached yet.
    ///
    /// Not a divergence to be fixed by widening this binary: the shell runtime
    /// is what ships, and a subcommand appears here only once the differential
    /// harness has compared it. It refuses rather than approximating, because
    /// an approximation of `set` writes ciphertext.
    #[error("`{PROGRAM} {subcommand}` is not ported to the rust runtime yet")]
    NotPorted {
        /// The subcommand that was asked for.
        subcommand: String,
    },
}

impl Diagnostic for Refusal {
    fn code(&self) -> Option<Box<dyn Display + '_>> {
        Some(Box::new(match self {
            Self::Runtime(error) => code_of(error),
            Self::Usage { .. } => "safix::usage",
            Self::UnknownSubcommand { .. } => "safix::unknown_subcommand",
            Self::NotPorted { .. } => "safix::not_ported",
        }))
    }

    fn help(&self) -> Option<Box<dyn Display + '_>> {
        let help = match self {
            Self::Runtime(error) => help_of(error)?,
            Self::Usage { .. } | Self::UnknownSubcommand { .. } => {
                "`safix <subcommand> -h` explains one of them."
            }
            Self::NotPorted { .. } => {
                "the shell runtime is the one that ships: run the flake's `safix` package. This binary is `safix-rs`, and it takes over one subcommand at a time as each passes the differential harness."
            }
        };
        Some(Box::new(help))
    }
}

/// The stable name of a refusal, which is what a script greps for and what a
/// snapshot is keyed by.
fn code_of(error: &Error) -> &'static str {
    match error {
        Error::SecretRead { .. } => "safix::secret_unreadable",
        Error::NotInsideRepository => "safix::not_a_repository",
        Error::NixEvalFailed { .. } => "safix::nix_eval_failed",
        Error::NixSchemaMismatch { .. } => "safix::nix_schema_mismatch",
        Error::UnknownUser { .. } => "safix::unknown_user",
        Error::UnknownName { .. } => "safix::unknown_name",
        Error::NoFileForName { .. } => "safix::no_file_for_name",
        Error::NotAYamlPath { .. } => "safix::not_a_yaml_path",
        Error::NoDefaultUser { .. } => "safix::no_default_user",
        Error::NoValueYet { .. } => "safix::no_value_yet",
        Error::RecipientsUnreadable { .. } => "safix::recipients_unreadable",
        Error::SopsDocumentUnreadable { .. } => "safix::document_unreadable",
        Error::SopsStanzaUnreadable => "safix::stanza_unreadable",
        Error::SopsUnavailable { .. } => "safix::sops_unavailable",
        Error::SopsPipeMissing => "safix::sops_pipe_missing",
        Error::SopsKeyIndex { .. } => "safix::sops_key_index",
        Error::FileUnreadable { .. } => "safix::file_unreadable",
        Error::GitUnavailable { .. } => "safix::git_unavailable",
        Error::GitCommandFailed { .. } => "safix::git_command_failed",
        Error::GitOutputNotText { .. } => "safix::git_output_not_text",
        Error::MidOperation { .. } => "safix::mid_operation",
        Error::ConflictEntries { .. } => "safix::conflict_entries",
        Error::UncommittedChanges { .. } => "safix::uncommitted_changes",
        _ => "safix::refusal",
    }
}

/// The way out, for the refusals that have one the message does not already
/// carry.
///
/// Absent where the message is itself the instruction: the shell runtime writes
/// one paragraph per refusal, and repeating half of it under a `help:` heading
/// would be two statements of one remedy that can drift apart.
fn help_of(error: &Error) -> Option<&'static str> {
    match error {
        Error::NixEvalFailed { .. } => {
            Some("nix has already said why on its own standard error, above this.")
        }
        Error::NixSchemaMismatch { .. } => Some(
            "the nix half declares a field this runtime does not read. Both halves ship from one repository, so this is a version skew between them rather than a configuration error.",
        ),
        Error::NotInsideRepository => {
            Some("run this inside the repository holding the declarations, or set SAFIX_REPO_ROOT.")
        }
        _ => None,
    }
}

/// Whether the environment asks for the shell runtime's shape.
#[must_use]
pub fn plain_selected() -> bool {
    std::env::var(FORMAT_VARIABLE).is_ok_and(|value| value == PLAIN)
}

/// Write a refusal in the shell runtime's shape.
///
/// The message's own newlines are its continuation lines, indented as the shell
/// indents them, so one `safix: ` prefix covers the whole paragraph.
pub fn report_plain(message: &dyn Display) {
    eprintln!("{PROGRAM}: {message}");
}

/// Write a refusal graphically.
pub fn report_graphical(refusal: Refusal) {
    eprint!("{:?}", miette::Report::new(refusal));
}

/// Write a refusal in whichever shape the environment selected.
pub fn report(refusal: Refusal) {
    if plain_selected() {
        report_plain(&refusal);
    } else {
        report_graphical(refusal);
    }
}
