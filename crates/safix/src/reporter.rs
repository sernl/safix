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

    use safix_core::{Code, Error, delegation};

    use super::*;

    /// One value of every refusal the runtime can raise, keyed by its code.
    ///
    /// This match has no wildcard arm, and that is what holds the snapshots to
    /// the type rather than to a list somebody maintains. A variant added to
    /// [`Error`] reaches [`Code`] first — the table assigning codes refuses to
    /// compile without it — and then reaches here, which refuses to compile
    /// without a value to render; `insta` then refuses to pass without a
    /// snapshot of that value. Nothing in that chain is a habit.
    ///
    /// The values are fixtures throughout: the fleet is `alice`, `bob` and
    /// `carol`, the one every check in this repository drives, and every age
    /// string is synthetic — 58 characters of one bech32 letter, minted by
    /// nobody and opening nothing.
    ///
    /// The arms are in the order [`Error`] declares its variants, which is the
    /// order [`Code::ALL`] iterates and the order the refusals were ported in:
    /// the read paths, then the write paths, then the generator graph, then
    /// custody.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per refusal, and the compiler checking that there is one for each is \
                  the point; splitting the table would mean a wildcard arm to split it at"
    )]
    fn sample(code: Code) -> Error {
        match code {
            Code::SecretRead => Error::SecretRead {
                cause: io::Error::from(io::ErrorKind::UnexpectedEof),
            },
            Code::NotInsideRepository => Error::NotInsideRepository,
            Code::NixEvalFailed => Error::NixEvalFailed {
                attribute: "flake.safix.lib.placements",
                root: "/srv/fleet".into(),
                cause: None,
            },
            Code::NixSchemaMismatch => Error::NixSchemaMismatch {
                attribute: "flake.safix.lib.placements",
                cause: "unknown field `mode`".into(),
            },
            Code::UnknownUser => Error::UnknownUser {
                user: "dave".into(),
                declared: vec!["alice".into(), "bob".into(), "carol".into()],
            },
            Code::UnknownName => Error::UnknownName {
                user: "alice".into(),
                name: "no-such-secret".into(),
                held: vec!["alice-alone".into(), "team-vault".into()],
            },
            Code::NoFileForName => Error::NoFileForName {
                name: "alice-alone".into(),
            },
            Code::NotAYamlPath => Error::NotAYamlPath {
                name: "bad-path".into(),
                file: "secrets/safix/users/bob/notes.txt".into(),
            },
            Code::NoDefaultUser => Error::NoDefaultUser {
                login: "builder".into(),
                holders: 2,
            },
            Code::NoValueYet => Error::NoValueYet {
                file: "secrets/safix/users/bob/secrets.yaml".into(),
                name: "bob-service".into(),
                user: "bob".into(),
            },
            Code::RecipientsUnreadable => Error::RecipientsUnreadable {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                cause: Box::new(Error::SopsStanzaUnreadable),
            },
            Code::SopsDocumentUnreadable => Error::SopsDocumentUnreadable {
                cause: "invalid type: string \"a note\", expected a map at line 1 column 1".into(),
            },
            Code::SopsStanzaUnreadable => Error::SopsStanzaUnreadable,
            Code::SopsUnavailable => Error::SopsUnavailable {
                program: "sops".into(),
                cause: io::Error::from(io::ErrorKind::NotFound),
            },
            Code::SopsPipeMissing => Error::SopsPipeMissing,
            Code::SopsKeyIndex => Error::SopsKeyIndex {
                key: "ops_tooling".into(),
                cause: "invalid unicode code point".into(),
            },
            Code::ClanUnavailable => Error::ClanUnavailable {
                program: "clan".into(),
                cause: io::Error::from(io::ErrorKind::NotFound),
            },
            Code::ClanPipeMissing => Error::ClanPipeMissing,
            Code::ClanVarUnknown => Error::ClanVarUnknown {
                mapping: "ntfy-token".into(),
                machine: "alice-workstation".into(),
                generator: "ntfy".into(),
                file: "token".into(),
            },
            Code::ClanCommandFailed => Error::ClanCommandFailed {
                mapping: "ntfy-token".into(),
                machine: "alice-workstation".into(),
                var_id: "ntfy/token".into(),
                output: "Error: Machine alice-workstation does not exist".into(),
            },
            Code::UnknownMapping => Error::UnknownMapping {
                mapping: "ntfy-tokne".into(),
                declared: vec!["ntfy-token".into(), "wg-key".into()],
            },
            Code::MappingWrongDirection => Error::MappingWrongDirection {
                mapping: "ntfy-token".into(),
                direction: "safix-to-clan",
                verb: "export",
                asked: "import",
            },
            Code::SourceHasNoValue => Error::SourceHasNoValue {
                mapping: "ntfy-token".into(),
                user: "alice".into(),
                name: "ntfy-token".into(),
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                generated: false,
            },
            Code::SourceUnreadable => Error::SourceUnreadable {
                mapping: "ntfy-token".into(),
                user: "alice".into(),
                name: "ntfy-token".into(),
                file: "secrets/safix/users/alice/secrets.yaml".into(),
            },
            Code::GeneratorDefinitionDrifted => Error::GeneratorDefinitionDrifted {
                mapping: "ntfy-token".into(),
                machine: "alice-workstation".into(),
                generator: "ntfy".into(),
            },
            Code::NoClanFlake => Error::NoClanFlake,
            Code::FileUnreadable => Error::FileUnreadable {
                path: "secrets/safix/users/alice/secrets.yaml.safix-tmp.4213.yaml".into(),
                cause: io::Error::from(io::ErrorKind::PermissionDenied),
            },
            Code::GitUnavailable => Error::GitUnavailable {
                program: "git".into(),
                cause: io::Error::from(io::ErrorKind::NotFound),
            },
            Code::GitCommandFailed => Error::GitCommandFailed {
                arguments: "commit -q -m chore(safix): set alice-alone for alice \
                    -- secrets/safix/users/alice/secrets.yaml"
                    .into(),
            },
            Code::GitOutputNotText => Error::GitOutputNotText {
                cause: "invalid utf-8 sequence of 1 bytes from index 12".into(),
            },
            Code::MidOperation => Error::MidOperation {
                state: "rebase-merge",
                marker: "/srv/fleet/.git/rebase-merge".into(),
            },
            Code::ConflictEntries => Error::ConflictEntries {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
            },
            Code::UncommittedChanges => Error::UncommittedChanges {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                status: " M secrets/safix/users/alice/secrets.yaml".into(),
            },
            Code::NoValueRead => Error::NoValueRead,
            Code::NoConfirmationRead => Error::NoConfirmationRead,
            Code::EmptyValue => Error::EmptyValue,
            Code::EntriesDiffer => Error::EntriesDiffer,
            Code::FileUnwritable => Error::FileUnwritable {
                path: "secrets/safix/users/alice/secrets.yaml".into(),
                cause: io::Error::from(io::ErrorKind::PermissionDenied),
            },
            Code::NoAudienceForFile => Error::NoAudienceForFile {
                file: "secrets/elsewhere/notes.yaml".into(),
            },
            Code::CandidateRecipientsUnreadable => Error::CandidateRecipientsUnreadable {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                cause: Box::new(Error::SopsStanzaUnreadable),
            },
            Code::RecipientDrift => Error::RecipientDrift {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                extra: vec!["age1carol".into()],
                missing: vec!["age1escrow".into()],
            },
            Code::NoCreationRule => Error::NoCreationRule {
                file: "secrets/safix/shared/alice,bob/secrets.yaml".into(),
            },
            Code::RewrapUnschedulable => Error::RewrapUnschedulable {
                cause: "task panicked".into(),
            },
            Code::SopsCreateFailed => Error::SopsCreateFailed {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                output: "Failed to get the data key: no key could be obtained".into(),
            },
            Code::GeneratorCycle => Error::GeneratorCycle {
                user: "alice".into(),
                cycle: vec!["base".into(), "derived".into(), "base".into()],
            },
            Code::NoGenerator => Error::NoGenerator {
                user: "alice".into(),
                name: "api-token".into(),
            },
            Code::DependencyHasNoValue => Error::DependencyHasNoValue {
                name: "base-pub".into(),
                producer: "base".into(),
                file: "secrets/safix/users/alice/secrets.yaml".into(),
            },
            Code::SandboxUnavailable => Error::SandboxUnavailable {
                backend: safix_core::sandbox::Backend::Bubblewrap.program(),
                supplied_by: safix_core::sandbox::Backend::Bubblewrap.supplied_by(),
            },
            Code::SandboxUnsupported => Error::SandboxUnsupported {
                platform: "freebsd",
            },
            Code::StagingNotMemoryBacked => Error::StagingNotMemoryBacked {
                candidates: vec!["/dev/shm".into(), "/run/user/1000".into()],
                disk_backed: vec!["/dev/shm".into()],
            },
            Code::StagingUnusable => Error::StagingUnusable {
                path: "/dev/shm/safix-stage-4213-0/out".into(),
                cause: io::Error::from(io::ErrorKind::PermissionDenied),
            },
            Code::GeneratorOutputMissing => Error::GeneratorOutputMissing {
                generator: "wg-private".into(),
                output: "wg-public".into(),
                produced: vec!["wg-private".into()],
            },
            Code::NoValueForPrompt => Error::NoValueForPrompt {
                name: "seed".into(),
            },
            Code::PromptUnanswered => Error::PromptUnanswered {
                name: "seed".into(),
            },
            Code::GeneratorFailed => Error::GeneratorFailed {
                generator: "api-token".into(),
                status: 3,
            },
            Code::GeneratorProducedNothing => Error::GeneratorProducedNothing {
                generator: "blank".into(),
                output: "blank".into(),
            },
            Code::ValidationRejected => Error::ValidationRejected {
                generator: "unvalidated".into(),
                output: "unvalidated".into(),
            },
            Code::CascadeDeclined => Error::CascadeDeclined,
            Code::NoEditor => Error::NoEditor,
            Code::PublicNotEditable => Error::PublicNotEditable {
                name: "wg-public".into(),
                path: "public/safix/users/alice/wg-public/value".into(),
            },
            Code::EditorFailed => Error::EditorFailed { status: 1 },
            Code::KeygenForSomeoneElse => Error::KeygenForSomeoneElse { user: "bob".into() },
            Code::KeygenFailed => Error::KeygenFailed,
            Code::KeygenNoPublicKey => Error::KeygenNoPublicKey {
                file: "/home/alice/.config/sops/age/keys.txt".into(),
            },
            Code::BadUserName => Error::BadUserName {
                name: "Alice Smith".into(),
                pattern: "[a-z0-9][a-z0-9_-]*".into(),
            },
            Code::HardwareRecipient => Error::HardwareRecipient {
                recipient: format!("age1yubikey1{}", "q".repeat(58)),
            },
            Code::BadRecipient => Error::BadRecipient {
                recipient: "age1-not-a-key".into(),
            },
            Code::AlreadyDeclared => Error::AlreadyDeclared {
                user: "alice".into(),
            },
            Code::ScaffoldExists => Error::ScaffoldExists {
                file: "safix/users/dave.nix".into(),
            },
            Code::HostWithoutHook => Error::HostWithoutHook,
            Code::Unparsable => Error::Unparsable {
                path: "/srv/fleet/safix/users/dave.nix".into(),
            },
            Code::ScaffoldDeclined => Error::ScaffoldDeclined,
            Code::PolicyEvalAfterScaffold => Error::PolicyEvalAfterScaffold {
                root: "/srv/fleet".into(),
            },
            Code::HookFailed => Error::HookFailed { status: 2 },
            Code::EntropyUnreadable => Error::EntropyUnreadable {
                source: "/dev/urandom",
                cause: io::Error::from(io::ErrorKind::PermissionDenied),
            },
            Code::YkmanUnavailable => Error::YkmanUnavailable {
                program: "ykman".into(),
                cause: io::Error::from(io::ErrorKind::NotFound),
            },
            Code::PcscdUnavailable => Error::PcscdUnavailable,
            Code::NoCardConnected => Error::NoCardConnected,
            Code::CardsAmbiguous => Error::CardsAmbiguous {
                serials: vec!["11111111".into(), "22222222".into()],
            },
            Code::CardCommandFailed => Error::CardCommandFailed {
                arguments: "--device 11111111 piv access change-pin -P <redacted> -n <redacted>"
                    .into(),
                output: "Error: Wrong PIN. 2 tries left.".into(),
            },
            Code::CardPinRejected => Error::CardPinRejected {
                serial: "11111111".into(),
            },
            Code::OtpRefused => Error::OtpRefused,
            Code::TouchPolicyNever => Error::TouchPolicyNever,
            Code::NoTerminal => Error::NoTerminal,
            Code::PtyUnusable => Error::PtyUnusable {
                cause: io::Error::from(io::ErrorKind::PermissionDenied),
            },
            Code::PluginUnavailable => Error::PluginUnavailable {
                program: "age-plugin-yubikey".into(),
                cause: io::Error::from(io::ErrorKind::NotFound),
            },
            Code::PluginFailed => Error::PluginFailed { status: 1 },
            Code::PluginStalled => Error::PluginStalled { seconds: 90 },
            Code::PluginNoIdentity => Error::PluginNoIdentity,
            Code::NoDeclarationFile => Error::NoDeclarationFile {
                user: "carol".into(),
                file: "safix/users/carol.nix".into(),
            },
            Code::RecipientsLost => Error::RecipientsLost {
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                lost: vec!["age1bob".into()],
            },
            Code::NoFileToProveWith => Error::NoFileToProveWith {
                user: "carol".into(),
            },
            Code::StoreUnavailable => Error::StoreUnavailable {
                program: "secret-tool".into(),
                cause: io::Error::from(io::ErrorKind::NotFound),
            },
            Code::StoreMirrorFailed => Error::StoreMirrorFailed {
                transport: "the password store's own command",
                status: 1,
                output: "Invalid credentials.".into(),
            },
            Code::NoStoreDatabase => Error::NoStoreDatabase { mappings: 3 },
            Code::UnknownSyncMapping => Error::UnknownSyncMapping {
                mapping: "grafana-typo".into(),
                declared: vec!["grafana".into(), "router".into()],
            },
            Code::StoreLocked => Error::StoreLocked {
                database: "/home/alice/.keys/master.kdbx".into(),
            },
            Code::DatabaseUnreadable => Error::DatabaseUnreadable {
                database: "/home/alice/.keys/master.kdbx".into(),
                output: "Invalid credentials were provided, please try again.".into(),
            },
            Code::StorePipeMissing => Error::StorePipeMissing,
            Code::StoreCommandFailed => Error::StoreCommandFailed {
                entry: "safix/alice/grafana".into(),
                arguments: "add --quiet --password-prompt /home/alice/.keys/master.kdbx \
                    safix/alice/grafana"
                    .into(),
                output: "Could not create entry with path safix/alice/grafana.".into(),
            },
            Code::ValueSpansLines => Error::ValueSpansLines {
                entry: "safix/alice/grafana".into(),
            },
            Code::SyncSourceEmpty => Error::SyncSourceEmpty {
                mapping: "grafana".into(),
                user: "alice".into(),
                name: "grafana-password".into(),
                file: "secrets/safix/users/alice/secrets.yaml".into(),
                generated: false,
            },
            Code::StoreEntryAbsent => Error::StoreEntryAbsent {
                mapping: "router".into(),
                entry: "safix/bob/router".into(),
                mode: "keepassxc-to-safix",
            },
            Code::ClanUserRegistrationFailed => Error::ClanUserRegistrationFailed {
                user: "alice".into(),
                output: "Error: user alice already exists".into(),
            },
            Code::EnrollHookFailed => Error::EnrollHookFailed { status: 2 },
            Code::ActorUndeclared => Error::ActorUndeclared {
                name: "Mallory Example".into(),
                email: "mallory@example.com".into(),
                delegation: Box::new(delegation::Refused {
                    through: delegation::Through::Consent {
                        person: "bob".into(),
                    },
                    organizations: vec!["acme".into()],
                }),
                declared: vec!["alice".into(), "bob".into(), "carol".into()],
            },
            Code::UnknownGroup => Error::UnknownGroup {
                group: "oncal".into(),
                declared: vec!["infra".into(), "oncall".into()],
            },
            Code::UnknownSubject => Error::UnknownSubject {
                subject: "bo".into(),
                declared: vec!["alice".into(), "bob".into(), "deck".into()],
            },
            Code::NoGroupDeclaration => Error::NoGroupDeclaration {
                group: "oncall".into(),
                file: "safix/groups/oncall.nix".into(),
            },
            Code::ScaffoldOutOfScope => Error::ScaffoldOutOfScope {
                actor: "mallory".into(),
                delegation: Box::new(delegation::Refused {
                    through: delegation::Through::Consent {
                        person: "bob".into(),
                    },
                    organizations: vec!["acme".into()],
                }),
                managers: vec!["alice".into()],
            },
        }
    }

    /// A second shape of a refusal whose message branches on its own data.
    ///
    /// [`sample`] holds one value per variant, which is what the compiler can
    /// check for it. A variant that reads differently over different data is
    /// held here as well, under a name of its own, because the branch is in the
    /// prose rather than in the type and nothing but a second value pins it.
    fn further_shapes() -> Vec<(&'static str, Refusal)> {
        vec![
            (
                "recipient_drift_one_sided",
                Refusal::Runtime(Error::RecipientDrift {
                    file: "secrets/safix/users/bob/secrets.yaml".into(),
                    extra: Vec::new(),
                    missing: vec!["age1bob".into()],
                }),
            ),
            (
                "source_has_no_value_generated",
                Refusal::Runtime(Error::SourceHasNoValue {
                    mapping: "wg-key".into(),
                    user: "alice".into(),
                    name: "wg-private".into(),
                    file: "secrets/safix/users/alice/secrets.yaml".into(),
                    generated: true,
                }),
            ),
            (
                "sync_source_empty_generated",
                Refusal::Runtime(Error::SyncSourceEmpty {
                    mapping: "wg-key".into(),
                    user: "alice".into(),
                    name: "wg-private".into(),
                    file: "secrets/safix/users/alice/secrets.yaml".into(),
                    generated: true,
                }),
            ),
            (
                "unknown_sync_mapping_none_declared",
                Refusal::Runtime(Error::UnknownSyncMapping {
                    mapping: "grafana".into(),
                    declared: Vec::new(),
                }),
            ),
            // The group half of the same refusal: the delegation is silo coverage
            // rather than a person's consent, two organizations cover the group,
            // and neither declares a manager — so the sentence joins two option
            // paths and the heading stands over nothing.
            (
                "scaffold_out_of_scope_group",
                Refusal::Runtime(Error::ScaffoldOutOfScope {
                    actor: "dave".into(),
                    delegation: Box::new(delegation::Refused {
                        through: delegation::Through::Silo {
                            group: "oncall".into(),
                        },
                        organizations: vec!["acme".into(), "globex".into()],
                    }),
                    managers: Vec::new(),
                }),
            ),
        ]
    }

    /// The command's own, about how it was invoked.
    ///
    /// Not driven by [`Code`], because these are not the library's refusals:
    /// they are [`Refusal`]'s own variants, and [`Refusal`] is closed and
    /// declared in this file, so the two matches in its [`Diagnostic`]
    /// implementation already refuse to compile when one arrives without a code
    /// and a help.
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
            ("host_needs_hostname", Refusal::HostNeedsHostname),
            (
                "unknown_option",
                Refusal::UnknownOption {
                    option: "--force".into(),
                },
            ),
            (
                "option_needs_value",
                Refusal::OptionNeedsValue {
                    option: "--serial".into(),
                },
            ),
        ]
    }

    /// Every refusal, keyed by the snapshot holding it.
    ///
    /// One value per [`Code`], which is the part the compiler maintains, then
    /// the further shapes and the command's own. A sample filed under a code
    /// that is not its own is refused here rather than silently snapshotted
    /// under the wrong name.
    fn every_refusal() -> Vec<(&'static str, Refusal)> {
        let mut all: Vec<(&'static str, Refusal)> = Code::ALL
            .iter()
            .map(|&code| {
                let error = sample(code);
                assert_eq!(
                    error.code(),
                    code,
                    "the value sampled for {code} is a different refusal"
                );
                (code.name(), Refusal::Runtime(error))
            })
            .collect();
        all.extend(further_shapes());
        all.extend(command_refusals());
        all
    }

    /// The graphical rendering never had a second runtime to be compared
    /// against, so it is pinned against itself.
    ///
    /// Both renderings are the functions the command prints through, so what is
    /// held here is what is written, not a third rendering made for the test.
    /// Each runtime refusal is filed under its own code, so the file naming the
    /// rendering and the string a script greps for cannot drift apart.
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
