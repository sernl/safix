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
        Error::NoValueRead => "safix::no_value_read",
        Error::NoConfirmationRead => "safix::no_confirmation_read",
        Error::EmptyValue => "safix::empty_value",
        Error::EntriesDiffer => "safix::entries_differ",
        Error::FileUnwritable { .. } => "safix::file_unwritable",
        Error::NoAudienceForFile { .. } => "safix::no_audience_for_file",
        Error::CandidateRecipientsUnreadable { .. } => "safix::candidate_recipients_unreadable",
        Error::RecipientDrift { .. } => "safix::recipient_drift",
        Error::RewrapUnschedulable { .. } => "safix::rewrap_unschedulable",
        Error::NoCreationRule { .. } => "safix::no_creation_rule",
        Error::SopsCreateFailed { .. } => "safix::sops_create_failed",
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
/// the structure does not; and this is the one channel the differential harness
/// cannot compare against the shell runtime, so being able to pin it against
/// itself is what keeps it from being unchecked.
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
pub fn report(refusal: &Refusal) {
    if plain_selected() {
        report_plain(refusal);
    } else {
        report_graphical(refusal);
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use safix_core::Error;

    use super::*;

    /// One value of every refusal the read paths can produce, so that adding a
    /// variant without a snapshot is a test that does not exist rather than one
    /// that silently passes.
    fn every_refusal() -> Vec<(&'static str, Refusal)> {
        let mut all = runtime_refusals();
        all.extend(command_refusals());
        all
    }

    /// The library's refusals, which an embedder also receives.
    fn runtime_refusals() -> Vec<(&'static str, Refusal)> {
        let mut all = read_path_refusals();
        all.extend(write_path_refusals());
        all
    }

    /// The refusals reachable from `list`, `get` and `check`.
    fn read_path_refusals() -> Vec<(&'static str, Refusal)> {
        vec![
            (
                "not_a_repository",
                Refusal::Runtime(Error::NotInsideRepository),
            ),
            (
                "nix_eval_failed",
                Refusal::Runtime(Error::NixEvalFailed {
                    attribute: "flake.safix.lib.placements",
                    root: "/srv/fleet".into(),
                    cause: None,
                }),
            ),
            (
                "nix_schema_mismatch",
                Refusal::Runtime(Error::NixSchemaMismatch {
                    attribute: "flake.safix.lib.placements",
                    cause: "unknown field `mode`".into(),
                }),
            ),
            (
                "unknown_user",
                Refusal::Runtime(Error::UnknownUser {
                    user: "dee".into(),
                    declared: vec!["ana".into(), "bo".into(), "cy".into()],
                }),
            ),
            (
                "unknown_name",
                Refusal::Runtime(Error::UnknownName {
                    user: "ana".into(),
                    name: "no-such-secret".into(),
                    held: vec!["ana-alone".into(), "team-vault".into()],
                }),
            ),
            (
                "no_file_for_name",
                Refusal::Runtime(Error::NoFileForName {
                    name: "ana-alone".into(),
                }),
            ),
            (
                "not_a_yaml_path",
                Refusal::Runtime(Error::NotAYamlPath {
                    name: "bad-path".into(),
                    file: "secrets/safix/users/bo/notes.txt".into(),
                }),
            ),
            (
                "no_default_user",
                Refusal::Runtime(Error::NoDefaultUser {
                    login: "builder".into(),
                    holders: 2,
                }),
            ),
            (
                "no_value_yet",
                Refusal::Runtime(Error::NoValueYet {
                    file: "secrets/safix/users/bo/secrets.yaml".into(),
                    name: "bo-service".into(),
                    user: "bo".into(),
                }),
            ),
            (
                "recipients_unreadable",
                Refusal::Runtime(Error::RecipientsUnreadable {
                    file: "secrets/safix/users/ana/secrets.yaml".into(),
                    cause: Box::new(Error::SopsStanzaUnreadable),
                }),
            ),
            (
                "mid_operation",
                Refusal::Runtime(Error::MidOperation {
                    state: "rebase-merge",
                    marker: "/srv/fleet/.git/rebase-merge".into(),
                }),
            ),
            (
                "conflict_entries",
                Refusal::Runtime(Error::ConflictEntries {
                    file: "secrets/safix/users/ana/secrets.yaml".into(),
                }),
            ),
            (
                "uncommitted_changes",
                Refusal::Runtime(Error::UncommittedChanges {
                    file: "secrets/safix/users/ana/secrets.yaml".into(),
                    status: " M secrets/safix/users/ana/secrets.yaml".into(),
                }),
            ),
            (
                "secret_unreadable",
                Refusal::Runtime(Error::SecretRead {
                    cause: io::Error::from(io::ErrorKind::UnexpectedEof),
                }),
            ),
        ]
    }

    /// The refusals reachable from `set` and `fix`.
    fn write_path_refusals() -> Vec<(&'static str, Refusal)> {
        vec![
            ("no_value_read", Refusal::Runtime(Error::NoValueRead)),
            (
                "no_confirmation_read",
                Refusal::Runtime(Error::NoConfirmationRead),
            ),
            ("empty_value", Refusal::Runtime(Error::EmptyValue)),
            ("entries_differ", Refusal::Runtime(Error::EntriesDiffer)),
            (
                "file_unwritable",
                Refusal::Runtime(Error::FileUnwritable {
                    path: "secrets/safix/users/ana/secrets.yaml".into(),
                    cause: io::Error::from(io::ErrorKind::PermissionDenied),
                }),
            ),
            (
                "no_audience_for_file",
                Refusal::Runtime(Error::NoAudienceForFile {
                    file: "secrets/elsewhere/notes.yaml".into(),
                }),
            ),
            (
                "candidate_recipients_unreadable",
                Refusal::Runtime(Error::CandidateRecipientsUnreadable {
                    file: "secrets/safix/users/ana/secrets.yaml".into(),
                    cause: Box::new(Error::SopsStanzaUnreadable),
                }),
            ),
            (
                "recipient_drift",
                Refusal::Runtime(Error::RecipientDrift {
                    file: "secrets/safix/users/ana/secrets.yaml".into(),
                    extra: vec!["age1cy".into()],
                    missing: vec!["age1escrow".into()],
                }),
            ),
            (
                "recipient_drift_one_sided",
                Refusal::Runtime(Error::RecipientDrift {
                    file: "secrets/safix/users/bo/secrets.yaml".into(),
                    extra: Vec::new(),
                    missing: vec!["age1bo".into()],
                }),
            ),
            (
                "rewrap_unschedulable",
                Refusal::Runtime(Error::RewrapUnschedulable {
                    cause: "task panicked".into(),
                }),
            ),
            (
                "no_creation_rule",
                Refusal::Runtime(Error::NoCreationRule {
                    file: "secrets/safix/shared/ana,bo/secrets.yaml".into(),
                }),
            ),
            (
                "sops_create_failed",
                Refusal::Runtime(Error::SopsCreateFailed {
                    file: "secrets/safix/users/ana/secrets.yaml".into(),
                    output: "Failed to get the data key: no key could be obtained".into(),
                }),
            ),
        ]
    }

    /// The command's own, about how it was invoked.
    fn command_refusals() -> Vec<(&'static str, Refusal)> {
        vec![
            (
                "usage",
                Refusal::Usage {
                    form: "list [<user>]",
                },
            ),
            (
                "unknown_subcommand",
                Refusal::UnknownSubcommand {
                    subcommand: "rotate".into(),
                },
            ),
            (
                "not_ported",
                Refusal::NotPorted {
                    subcommand: "set".into(),
                },
            ),
        ]
    }

    /// The graphical rendering is the one channel the differential harness does
    /// not compare against the shell runtime, so it is pinned against itself.
    ///
    /// Both renderings are the functions the command prints through, so what is
    /// held here is what is written, not a third rendering made for the test.
    #[test]
    fn every_refusal_renders_the_same_under_both_reporters() {
        for (name, refusal) in every_refusal() {
            insta::assert_snapshot!(format!("plain-{name}"), render_plain(&refusal));
            insta::assert_snapshot!(format!("graphical-{name}"), render_graphical(&refusal));
        }
    }

    #[test]
    fn the_plain_reporter_is_selected_only_by_its_own_value() {
        assert_eq!(FORMAT_VARIABLE, "SAFIX_ERROR_FORMAT");
        assert_eq!(PLAIN, "plain");
    }
}
