//! How a refusal reaches the operator.
//!
//! Two renderings of the same value. The graphical one is `miette`'s, with a
//! diagnostic code and the help text that names the way out; it is what an
//! operator sees. The plain one is the retired shell runtime's shape exactly —
//! `safix: <message>`, no colour, no code, no span — and it exists so that
//! standard error can be asserted byte for byte instead of by a pattern over a
//! graphical rendering, which would be an assertion whose strictness nobody
//! could state. The integration suite drives the plain one for that reason and
//! the graphical one where a refusal's code is the claim.
//!
//! Selecting a reporter alters the bytes on standard error and nothing else.
//! It does not touch standard output, the exit status, or anything the run does
//! to the repository.

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
/// A literal rather than `argv[0]`, because the retired shell runtime used one
/// and every refusal's wording was fixed against it. A refusal that named itself
/// after the file it was invoked as would say something different under every
/// symlink and wrapper, which is not a property a message should have.
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
    ///
    /// The list is built from the table `main` dispatches on rather than
    /// written out beside it. A second list is a list that drifts, and this one
    /// had: `edit` shipped and was never added to the sentence.
    #[error(
        "unknown subcommand '{subcommand}' (expected {})",
        crate::expected_verbs()
    )]
    UnknownSubcommand {
        /// What was asked for.
        subcommand: String,
    },

    /// `--host` was the last argument, so no hostname followed it.
    #[error("--host takes a hostname")]
    HostNeedsHostname,

    /// An option `adduser` does not take.
    #[error("unknown option '{option}' (expected --host or --yes)")]
    UnknownOption {
        /// What was asked for.
        option: String,
    },

    /// An option that takes a value was the last argument.
    #[error("{option} takes a value")]
    OptionNeedsValue {
        /// The option that was left without one.
        option: String,
    },
}

impl Diagnostic for Refusal {
    fn code(&self) -> Option<Box<dyn Display + '_>> {
        Some(match self {
            Self::Runtime(error) => Box::new(error.code()) as Box<dyn Display + '_>,
            Self::Usage { .. } => Box::new("safix::usage"),
            Self::UnknownSubcommand { .. } => Box::new("safix::unknown_subcommand"),
            Self::HostNeedsHostname => Box::new("safix::host_needs_hostname"),
            Self::UnknownOption { .. } => Box::new("safix::unknown_option"),
            Self::OptionNeedsValue { .. } => Box::new("safix::option_needs_value"),
        })
    }

    fn help(&self) -> Option<Box<dyn Display + '_>> {
        let help = match self {
            Self::Runtime(error) => help_of(error)?,
            Self::Usage { .. }
            | Self::UnknownSubcommand { .. }
            | Self::HostNeedsHostname
            | Self::UnknownOption { .. }
            | Self::OptionNeedsValue { .. } => "`safix <subcommand> -h` explains one of them.",
        };
        Some(Box::new(help))
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

/// A refusal in the shell runtime's shape.
///
/// The message's own newlines are its continuation lines, indented as the shell
/// indents them, so one `safix: ` prefix covers the whole paragraph.
#[must_use]
pub fn render_plain(message: &dyn Display) -> String {
    format!("{PROGRAM}: {message}\n")
}

/// Write a refusal in the shell runtime's shape.
pub fn report_plain(message: &dyn Display) {
    eprint!("{}", render_plain(message));
}

/// A refusal rendered graphically: the diagnostic code, the message, and the
/// help that names the way out.
///
/// Rendered through a handler built here rather than through `miette`'s
/// installed hook, and the reason is that the hook is not a function of the
/// refusal. It takes colour from whether standard error is a terminal and from
/// half a dozen environment variables, and width from the terminal, so the same
/// refusal renders differently in a shell, in a build sandbox and in a log —
/// which would make a snapshot of it a statement about the machine that took
/// it.
///
/// The cost is colour, and it is worth paying here. These refusals are
/// paragraphs rather than annotated source spans, so colour carries little that
/// the structure does not; and this is the one channel that never had a second
/// runtime to be compared against, so pinning it against itself is what keeps it
/// from being unchecked.
#[must_use]
pub fn render_graphical(refusal: &Refusal) -> String {
    let mut rendered = String::new();
    let handler =
        miette::GraphicalReportHandler::new_themed(miette::GraphicalTheme::unicode_nocolor())
            .with_width(80);
    if handler.render_report(&mut rendered, refusal).is_err() {
        // Writing into a `String` cannot fail for want of space, so this is
        // reached only if the handler itself errors — and a refusal that cannot
        // be rendered still has to reach the operator.
        return render_plain(refusal);
    }
    rendered
}

/// Write a refusal graphically.
pub fn report_graphical(refusal: &Refusal) {
    eprint!("{}", render_graphical(refusal));
}

/// Write a refusal in whichever shape the environment selected.
///
/// A cancelled selection is the one refusal that is not a diagnosis: the
/// operator pressed Escape and knows it, so the whole report is the code, on
/// one line, in either shape.
pub fn report(refusal: &Refusal) {
    if let Refusal::Runtime(Error::SelectionCancelled) = refusal {
        eprintln!("{}", Error::SelectionCancelled.code());
    } else if plain_selected() {
        report_plain(refusal);
    } else {
        report_graphical(refusal);
    }
}
