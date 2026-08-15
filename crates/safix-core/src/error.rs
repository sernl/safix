//! The refusal type.
//!
//! Every refusal is a variant carrying the values its message interpolates,
//! rather than a variant carrying a finished sentence. That is what lets a
//! program embedding this crate branch on a refusal instead of matching on its
//! prose, and it is why rendering lives in the command rather than here.
//!
//! The wording is not ours to choose freely. Each message below is the message
//! `modules/flake/safix/safix.sh` prints for the same refusal, because the
//! shell runtime is the oracle the differential harness compares against and
//! its prose is tested. A [`Display`](std::fmt::Display) rendering here is
//! everything the shell prints after `safix: `, including the blank lines and
//! the indented continuations, and it carries no trailing newline — the
//! command's reporter adds that.

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

    /// Nothing here is a repository, so no path can be resolved against one.
    #[error("not inside a git repository")]
    NotInsideRepository,

    /// The nix half could not be evaluated.
    #[error("could not evaluate {attribute} in {root}")]
    NixEvalFailed {
        /// The attribute as a consumer declares it.
        attribute: &'static str,
        /// The repository the evaluation was rooted at.
        root: String,
        /// The failure to run nix at all, when that is what happened. A nix
        /// that ran and refused has already said why on its own stderr.
        #[source]
        cause: Option<io::Error>,
    },

    /// The nix half evaluated to a shape this runtime does not read.
    ///
    /// Every schema this crate reads denies unknown fields, so a field added on
    /// the nix side reaches here rather than being dropped. It has no
    /// counterpart in the shell runtime, which reads the same JSON through `jq`
    /// expressions that select the fields they know and ignore the rest.
    #[error("{attribute} evaluated to a shape this runtime does not read: {cause}")]
    NixSchemaMismatch {
        /// The attribute as a consumer declares it.
        attribute: &'static str,
        /// What the deserializer objected to.
        cause: String,
    },

    /// The declarations name no such user.
    #[error(
        "'{user}' is not a declared user of flake.safix.users.\n\nDeclared users:{}",
        bulleted(.declared)
    )]
    UnknownUser {
        /// The name that was asked for.
        user: String,
        /// Every user the declarations do name, in name order.
        declared: Vec<String>,
    },

    /// This user holds no secret by that name.
    #[error(
        "'{name}' is not a secret flake.safix.users.{user} holds.\n\
        \n\
        A secret is declared in exactly one of three places, and this resolves\n\
        the owning file from all three:\n\
        \n\
        \x20 1. flake.safix.catalogue.{name} — the shared catalogue — selected by\n\
        \x20    flake.safix.users.{user}.carries.{name}\n\
        \x20 2. flake.safix.users.{user}.private.{name} — this user's own entry\n\
        \x20 3. flake.safix.users.<owner>.sharedWith.{user}.{name} — granted from outside\n\
        \n\
        Declare it in one of them, then re-run. A name reaching a set only\n\
        through a perHost.<host> or perTag.<tag> add or force is refused here\n\
        too: placement is derived from those three sources alone, and the\n\
        per-host and per-tag scopes sit outside it deliberately, because they\n\
        adjust a secret for one machine rather than declare who holds it. One\n\
        file serves every host that resolves the secret, so a value set through\n\
        a single host's adjustment would apply everywhere that secret\n\
        resolves rather than only where the adjustment does.\n\
        \n\
        Names flake.safix.users.{user} holds:{}",
        bulleted(.held)
    )]
    UnknownName {
        /// The user whose custody was asked about.
        user: String,
        /// The name that was asked for.
        name: String,
        /// Every name that user does hold, in name order.
        held: Vec<String>,
    },

    /// The placement record carries no file, so there is nowhere to read or
    /// write.
    #[error("the declarations resolved no file for '{name}'")]
    NoFileForName {
        /// The name whose placement is incomplete.
        name: String,
    },

    /// The placement is outside the suffix every creation rule ends in.
    ///
    /// Every generated `path_regex` ends in a literal `\.yaml$`, so that a
    /// recipient sweep can never reach encrypted material safix did not place —
    /// whose original identities are gone and whose recipients are therefore
    /// unrecoverable once rewritten.
    #[error(
        "the declarations place '{name}' at {file}, which is not a *.yaml path; \
        every .sops.yaml creation rule ends in \\.yaml$, so no rule covers it"
    )]
    NotAYamlPath {
        /// The name whose placement is outside the suffix.
        name: String,
        /// Where the declarations placed it.
        file: String,
    },

    /// No user can be assumed, so one has to be named.
    #[error(
        "no default user: '{login}' is not declared in flake.safix.users, and the declarations \
        name {holders} holders. Name one: safix <subcommand> <user> <name>"
    )]
    NoDefaultUser {
        /// The login name this process is running under.
        login: String,
        /// How many declared users hold at least one secret.
        holders: usize,
    },

    /// The file the name resolves to does not exist, so the name has no value.
    #[error(
        "{file} does not exist yet, so '{name}' has no value. Set one with: safix set {user} {name}"
    )]
    NoValueYet {
        /// The file the declarations place the name in.
        file: String,
        /// The name that was asked for.
        name: String,
        /// The user whose custody it is in.
        user: String,
    },

    /// A governed file's recipients could not be read.
    #[error("could not read the recipients of {file}")]
    RecipientsUnreadable {
        /// The file that was being inspected.
        file: String,
        /// What the reader objected to.
        #[source]
        cause: Box<Error>,
    },

    /// The bytes are not a YAML document.
    #[error("the document is not readable as YAML: {cause}")]
    SopsDocumentUnreadable {
        /// What the parser objected to.
        cause: String,
    },

    /// A `sops.age` entry is not a mapping carrying a string `recipient`.
    #[error("a sops age stanza names no recipient")]
    SopsStanzaUnreadable,

    /// The sops binary could not be run.
    #[error("could not run {program}")]
    SopsUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// sops was started with a pipe on its standard output and the pipe was
    /// not there to read.
    #[error("sops produced no readable output")]
    SopsPipeMissing,

    /// A key name could not be rendered as the JSON index sops extracts by.
    #[error("could not build the extraction index for '{key}': {cause}")]
    SopsKeyIndex {
        /// The key that was being indexed.
        key: String,
        /// What the serializer objected to.
        cause: String,
    },

    /// A file could not be read.
    #[error("could not read {path}")]
    FileUnreadable {
        /// The path that was being read.
        path: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// The git binary could not be run.
    #[error("could not run {program}")]
    GitUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// git ran and refused.
    #[error("git {arguments} failed")]
    GitCommandFailed {
        /// The arguments git was given.
        arguments: String,
    },

    /// git printed something that is not text.
    #[error("git printed output that is not text: {cause}")]
    GitOutputNotText {
        /// What the decoder objected to.
        cause: String,
    },

    /// The repository is part-way through an operation a commit would disturb.
    #[error(
        "the repository is mid-{state} ({marker}). Finish or abort it before setting a secret."
    )]
    MidOperation {
        /// The operation's name.
        state: &'static str,
        /// The path whose existence is the evidence.
        marker: String,
    },

    /// The target file has unmerged conflict entries.
    #[error("{file} has unmerged conflict entries. Resolve them before setting a secret.")]
    ConflictEntries {
        /// The repository-relative path.
        file: String,
    },

    /// The target file has changes a commit here would sweep up.
    #[error(
        "{file} already has uncommitted changes:\n\
        \n\
        {status}\n\
        \n\
        This commits the file it writes, so committing it now would carry that\n\
        change under a message naming only one secret. Commit or discard it\n\
        first, then re-run."
    )]
    UncommittedChanges {
        /// The repository-relative path.
        file: String,
        /// git's porcelain status for that path, as git printed it.
        status: String,
    },
}

/// The one-per-line bulleted continuation the shell writes with `sed 's/^/  - /'`.
///
/// Empty for an empty list, and leading rather than trailing with its newline,
/// so that a message ending in a list and a message ending in a heading with no
/// list under it both end without one.
fn bulleted(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("\n  - {item}"))
        .collect::<Vec<_>>()
        .concat()
}

/// The result type this crate returns.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_naming_a_list_ends_without_a_newline() {
        let refusal = Error::UnknownUser {
            user: "dee".into(),
            declared: vec!["ana".into(), "bo".into(), "cy".into()],
        };
        assert_eq!(
            refusal.to_string(),
            "'dee' is not a declared user of flake.safix.users.\n\nDeclared users:\n  - ana\n  - bo\n  - cy"
        );
    }

    #[test]
    fn a_refusal_naming_an_empty_list_ends_at_its_heading() {
        let refusal = Error::UnknownUser {
            user: "dee".into(),
            declared: Vec::new(),
        };
        assert!(refusal.to_string().ends_with("Declared users:"));
    }

    #[test]
    fn the_unknown_name_refusal_keeps_the_shell_s_literal_owner_placeholder() {
        let refusal = Error::UnknownName {
            user: "cy".into(),
            name: "api-token".into(),
            held: Vec::new(),
        };
        let rendered = refusal.to_string();
        assert!(rendered.contains(
            "  3. flake.safix.users.<owner>.sharedWith.cy.api-token — granted from outside\n"
        ));
        assert!(rendered.ends_with("Names flake.safix.users.cy holds:"));
    }
}
