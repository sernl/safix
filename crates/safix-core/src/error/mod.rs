//! The refusal type.
//!
//! Every refusal is a variant carrying the values its message interpolates,
//! rather than a variant carrying a finished sentence. That is what lets a
//! program embedding this crate branch on a refusal instead of matching on its
//! prose, and it is why rendering lives in the command rather than here.
//!
//! The wording is not ours to choose freely. Each message below is the message
//! the retired shell runtime printed for the same refusal: its prose was the
//! oracle while both existed, and it is now the tested contract in its own
//! right. A [`Display`](std::fmt::Display) rendering here is everything that
//! runtime printed after `safix: `, including the blank lines and the indented
//! continuations, and it carries no trailing newline — the command's reporter
//! adds that.

mod code;
mod prose;

use std::io;

pub use code::Code;

use prose::{
    HOST_WITHOUT_HOOK, NO_TERMINAL, OTP_REFUSED, PCSCD_UNAVAILABLE, actor_undeclared,
    already_declared, bad_recipient, bad_user_name, bulleted, card_pin_rejected, cards_ambiguous,
    drifted, generator_cycle, hardware_recipient, keygen_for_someone_else, no_declaration_file,
    no_file_to_prove_with, no_generator, no_group_declaration, recipients_lost,
    scaffold_out_of_scope, unknown_subject,
};

/// A refusal from the safix runtime.
///
/// The variant list grows as the runtime is ported; it is marked non-exhaustive
/// so that adding one is not a breaking change for a matching embedder. Every
/// variant carries a [`Code`], and the table assigning them has no wildcard
/// arm, so a variant added here without one does not compile.
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

    /// The clan binary could not be run.
    ///
    /// Raised before any mapping is transferred rather than at the mapping that
    /// needed it, because a partial run reports "unchanged" for every mapping
    /// it never reached.
    #[error("{}", prose::clan_unavailable(program))]
    ClanUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// clan was started with a pipe that was not there to use.
    #[error("clan produced no usable pipe")]
    ClanPipeMissing,

    /// clan has no such var.
    ///
    /// The one thing evaluation could not check: the clan half of a mapping
    /// lives in another flake, so a typo in it is a sentence here rather than a
    /// refusal at build time.
    #[error("{}", prose::clan_var_unknown(mapping, machine, generator, file))]
    ClanVarUnknown {
        /// The mapping whose clan side does not resolve.
        mapping: String,
        /// The machine the mapping names.
        machine: String,
        /// The generator the mapping names.
        generator: String,
        /// The file the mapping names.
        file: String,
    },

    /// clan ran and refused, and this is what it said.
    ///
    /// clan's own message is carried verbatim rather than reworded: it is the
    /// authority on its own store, and a guess at what it meant would be a
    /// second, worse account of a failure it has already described.
    #[error("{}", prose::clan_command_failed(mapping, machine, var_id, output))]
    ClanCommandFailed {
        /// The mapping being transferred.
        mapping: String,
        /// The machine the mapping names.
        machine: String,
        /// The var id as clan's command line spells it.
        var_id: String,
        /// clan's own standard error, verbatim.
        output: String,
    },

    /// A verb was given a mapping name the declarations do not carry.
    #[error("{}", prose::unknown_mapping(mapping, &declared.join(", ")))]
    UnknownMapping {
        /// The name that was asked for.
        mapping: String,
        /// Every mapping that is declared.
        declared: Vec<String>,
    },

    /// A named mapping is declared with a direction the `--direction` filter
    /// given to `sync clan` or `audit clan` does not accept.
    ///
    /// Separate from [`Error::UnknownMapping`] because the mapping is declared
    /// and the operator has named it correctly: what does not match is the
    /// `--direction` filter, and the message can say so and name the mapping's
    /// actual direction.
    #[error("{}", prose::mapping_wrong_direction(mapping, actual, filter))]
    MappingWrongDirection {
        /// The mapping that was named.
        mapping: String,
        /// The direction it is actually declared with.
        actual: &'static str,
        /// The `--direction` value the run was filtered to.
        filter: &'static str,
    },

    /// A word `sync` or `audit` reads as a target keyword was given where a
    /// mapping name belongs.
    ///
    /// Evaluation refuses a declared mapping id spelled one of these three
    /// words, so a name reaching this far that still matches one cannot be a
    /// declared mapping under any target — it is the target-keyword role
    /// showing up a second time, after a target has already been named.
    #[error("{}", prose::reserved_mapping_word(word))]
    ReservedMappingWord {
        /// The word that was given.
        word: String,
    },

    /// A mapping name was given to `sync` or `audit` before any target was
    /// named.
    ///
    /// Refused rather than guessed: a mapping id may be declared under both
    /// `clan`'s and `keepassxc`'s namespaces without conflict, so guessing
    /// which one a bare name belongs to would be ambiguous exactly when it
    /// matters.
    #[error("{}", prose::mapping_name_needs_target(verb, name))]
    MappingNameNeedsTarget {
        /// Which verb was asked, `sync` or `audit`.
        verb: &'static str,
        /// The word that was read as an attempted mapping name.
        name: String,
    },

    /// `--direction` was given to a target other than `clan`.
    ///
    /// A keepassxc mapping declares a mode rather than a direction, and a mode
    /// narrows a run by being named rather than by a run-time flag — the same
    /// reason `bridge-surface` already gives for refusing `pull`/`push` as a
    /// mapping's declared direction applies a second time to the flag that
    /// mirrors it.
    #[error("{}", prose::direction_on_wrong_target(target))]
    DirectionOnWrongTarget {
        /// What `--direction` was given alongside, named the way the refusal
        /// reads.
        target: &'static str,
    },

    /// An export's source entry holds no value.
    ///
    /// The runtime sibling of a refusal evaluation cannot make. An entry
    /// declares where a value lives rather than that one is there, and the two
    /// are indistinguishable until something reads the file.
    #[error("{}", prose::source_has_no_value(mapping, user, name, file, *generated))]
    SourceHasNoValue {
        /// The mapping being exported.
        mapping: String,
        /// The user whose entry it is.
        user: String,
        /// The entry's name.
        name: String,
        /// The file that would have held the value.
        file: String,
        /// Whether a generator mints it, which decides which remedy leads.
        generated: bool,
    },

    /// An export's source could not be decrypted by whoever is running.
    ///
    /// sops has already said why on its own standard error. What this adds is
    /// that the mapping was refused rather than transferred, and why that is
    /// the right answer: a value that cannot be read cannot be verified, and
    /// pushing an unverifiable value into another store is worse than not
    /// pushing it.
    #[error("{}", prose::source_unreadable(mapping, user, name, file))]
    SourceUnreadable {
        /// The mapping being exported.
        mapping: String,
        /// The user whose entry it is.
        user: String,
        /// The entry's name.
        name: String,
        /// The file holding it.
        file: String,
    },

    /// clan already considers the mapping's generator stale.
    ///
    /// Refused before the write rather than reported after it: clan's next
    /// routine generation replaces the value of a generator whose recorded
    /// validation no longer matches its definition, so exporting into one is
    /// writing a value that is already scheduled to be discarded.
    #[error("{}", prose::generator_definition_drifted(mapping, machine, generator))]
    GeneratorDefinitionDrifted {
        /// The mapping being exported.
        mapping: String,
        /// The machine the mapping names.
        machine: String,
        /// The generator whose recorded validation is stale.
        generator: String,
    },

    /// A transfer was asked for and no clan is declared to transfer with.
    #[error("{}", prose::no_clan_flake())]
    NoClanFlake,

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

    /// The stream carrying the value ended before the value did.
    #[error("no value read")]
    NoValueRead,

    /// The stream carrying the confirmation ended before it did.
    #[error("no confirmation read")]
    NoConfirmationRead,

    /// The operator entered nothing, or the stream carried nothing.
    ///
    /// Refused rather than stored, because a key holding the empty string is
    /// indistinguishable from the placeholder `set` creates a file with, and
    /// `check` reads that placeholder as "declared, no value yet". The refusal
    /// covers both sources deliberately: an empty pipe is the state a failed
    /// upstream command leaves behind, which is the mistake a script makes where
    /// an empty entry is the one a person makes.
    #[error("the value is empty; refusing to store it")]
    EmptyValue,

    /// The two entries do not match.
    #[error("the two entries differ; nothing was written")]
    EntriesDiffer,

    /// A file could not be written.
    #[error("could not write {path}")]
    FileUnwritable {
        /// The path that was being written.
        path: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// The declarations place a secret in a file they compute no audience for.
    ///
    /// Nothing to hold the candidate document's recipients to, so the drift gate
    /// cannot run — and a write that skipped the gate would be a write with no
    /// gate on it.
    #[error(
        "flake.safix.lib.audiences declares no audience for {file}, \
        so there is nothing to hold its recipients to"
    )]
    NoAudienceForFile {
        /// The repository-relative path.
        file: String,
    },

    /// The candidate document's recipients could not be read.
    #[error("could not read the recipients of the document prepared for {file}")]
    CandidateRecipientsUnreadable {
        /// The repository-relative path the document was prepared for.
        file: String,
        /// What the reader objected to.
        #[source]
        cause: Box<Error>,
    },

    /// The document about to be written names recipients the declarations do
    /// not.
    ///
    /// The reason this refusal exists at write time rather than being left to
    /// `check` and `fix` is in the message: `sops set` on an existing file takes
    /// the file's recipients from the file's own metadata, so a drifted file
    /// would wrap a value minted now for an audience that has since changed —
    /// and `set` commits what it writes.
    #[error("{}", drifted(.file, .extra, .missing))]
    RecipientDrift {
        /// The repository-relative path.
        file: String,
        /// Can open it and is not in its audience.
        extra: Vec<String>,
        /// Is in its audience and cannot open it.
        missing: Vec<String>,
    },

    /// No creation rule covers the path a new file would occupy.
    #[error(
        ".sops.yaml has no creation rule for {file}\n\
        \n\
        The recipient policy is generated from the declarations, and a file with\n\
        no rule must fail closed rather than acquire a default recipient set:\n\
        there is deliberately no catch-all rule to fall back on.\n\
        \n\
        Regenerate it, review the diff, then re-run:\n\
        \n\
        \x20   safix fix\n\
        \x20   git diff .sops.yaml"
    )]
    NoCreationRule {
        /// The repository-relative path the file would occupy.
        file: String,
    },

    /// A bounded re-wrap could not be scheduled or could not be joined.
    ///
    /// About the executor rather than about sops: no re-wrap the operator asked
    /// for was refused, and a run that reaches this has converged over some
    /// prefix of the governed set and not over the rest.
    #[error("a governed file's re-wrap could not be run: {cause}")]
    RewrapUnschedulable {
        /// What the executor objected to.
        cause: String,
    },

    /// sops refused to create the file, for a reason of its own.
    ///
    /// The reason is sops's own text, carried rather than summarized: this is
    /// the one sops failure the runtime intercepts instead of letting through,
    /// and intercepting it must not cost the operator what sops said.
    #[error("sops could not create {file}:\n{output}")]
    SopsCreateFailed {
        /// The repository-relative path.
        file: String,
        /// What sops wrote to its standard error, less one trailing newline.
        output: String,
    },

    /// The run order handed to this runtime carries a cycle of generators.
    ///
    /// Raised before the first generator runs rather than at the one whose input
    /// is missing, because a run commits as it walks. It has no counterpart in
    /// the retired shell runtime and no fixture can produce it from
    /// `flake.safix.lib.generatorPlan`, which refuses a cycle at evaluation and
    /// leaves the generators inside one out of the order: the callers this
    /// reaches are a stand-in for nix and a program embedding this crate, for
    /// which the plan is a value rather than that refusal's postcondition.
    #[error("{}", generator_cycle(.user, .cycle))]
    GeneratorCycle {
        /// The user whose run order carries it.
        user: String,
        /// The generators participating, in the order they were walked, with the
        /// one the walk re-entered repeated at the end.
        cycle: Vec<String>,
    },

    /// The name resolves to an entry nothing mints.
    #[error("{}", no_generator(.user, .name))]
    NoGenerator {
        /// The user whose custody it is in.
        user: String,
        /// The name that was asked for.
        name: String,
    },

    /// A generator reads a secret whose file does not exist.
    #[error(
        "the dependency '{name}' has no value yet, so $in/{producer}/{name} cannot be \
        written: {file} does not exist"
    )]
    DependencyHasNoValue {
        /// The declared name of the dependency.
        name: String,
        /// The generator whose output it is, which names its input directory.
        producer: String,
        /// The file the declarations place the dependency in.
        file: String,
    },

    /// The generator sandbox's tools resolve through a flake, and this run
    /// named neither one.
    ///
    /// Raised in [`crate::generate::run`], immediately after the empty-order
    /// return and before [`crate::sandbox::Envelope::probe`] — see D5 in
    /// `support-plain-nix-consumers`'s design — so a user with nothing to mint
    /// is unaffected by `--entry` exactly as before this refusal existed.
    #[error(
        "generate needs a flake or a declared nixpkgs reference.\n\
        \n\
        A generator's sandbox resolves its own tools through nix shell, which\n\
        is a flake-only operation. This run named --entry, which points at a\n\
        plain file rather than a flake, and named no --nixpkgs or\n\
        SAFIX_NIXPKGS for the sandbox to resolve its tools against instead.\n\
        \n\
        Drop --entry and run against the declaring flake, or add --nixpkgs\n\
        <flake-ref> (or SAFIX_NIXPKGS)."
    )]
    GenerateNeedsNixpkgs,

    /// The platform's sandbox backend is the one it should have and does not run.
    ///
    /// Raised by the probe, before the first fragment. There is deliberately no
    /// flag that converts this into an unsandboxed run: a generator holding
    /// plaintext with the caller's filesystem and network is the state the
    /// envelope exists to remove, and a switch that restores it is that state
    /// with a record of somebody having asked for it.
    #[error(
        "{backend} is not available, so no generator ran.\n\
        \n\
        A generator's script and its validation fragments run inside a sandbox:\n\
        the staging root is the only writable path, the nix store is read-only,\n\
        and there is no network.\n\
        \n\
        {supplied_by}\n\
        \n\
        There is no flag that runs a generator outside the envelope."
    )]
    SandboxUnavailable {
        /// The backend that was looked for, as the machine names it.
        backend: &'static str,
        /// Where that backend comes from, which is what makes the remedy
        /// different on the two platforms — see
        /// [`Backend::supplied_by`](crate::sandbox::Backend::supplied_by).
        supplied_by: &'static str,
    },

    /// The platform has no sandbox backend to look for at all.
    #[error(
        "{platform} has no sandbox backend, so no generator ran.\n\
        \n\
        The envelope a generator's fragments run inside is bubblewrap on linux\n\
        and /usr/bin/sandbox-exec on darwin, which is the pair clan's own\n\
        executor uses and the pair a fragment written for either system meets.\n\
        This platform has neither, and there is no flag that runs a generator\n\
        outside the envelope."
    )]
    SandboxUnsupported {
        /// The platform that has none, as the compiler names it.
        platform: &'static str,
    },

    /// No memory-backed filesystem could be found to stage plaintext on.
    ///
    /// Raised before anything is produced. There is deliberately no fallback to
    /// a conventional temporary directory: this fleet's is disk-backed, so a
    /// silent fallback would be the exact failure the rule prevents, occurring
    /// under a code path that looks like it succeeded.
    #[error(
        "no memory-backed filesystem is available to stage plaintext on; nothing ran.\n\
        \n\
        A generator's inputs and outputs are files, and safix places them in a\n\
        private directory on tmpfs so that no plaintext reaches a block device.\n\
        Tried:{}\n\
        Of those, disk-backed:{}\n\
        \n\
        Re-run with --allow-disk-staging to accept that plaintext will be\n\
        written to a disk-backed filesystem, where an unlink leaves the bytes\n\
        in free blocks. Set SAFIX_STAGING_DIR to name a tmpfs mount instead.",
        bulleted(.candidates),
        bulleted(.disk_backed)
    )]
    StagingNotMemoryBacked {
        /// Every location that was tried, in the order they were tried.
        candidates: Vec<String>,
        /// Those of them that answered, and answered disk-backed.
        disk_backed: Vec<String>,
    },

    /// The staging root, or something inside it, could not be created.
    #[error("could not stage plaintext at {path}")]
    StagingUnusable {
        /// The path that could not be created or written.
        path: String,
        /// What the filesystem objected to.
        #[source]
        cause: io::Error,
    },

    /// A generator exited without writing one of its declared outputs.
    ///
    /// The listing of what the output directory did contain is clan's, and is
    /// copied deliberately: a refusal naming only what is absent leaves the
    /// operator to guess between a script that wrote nothing, one that wrote
    /// somewhere else, and one that misspelled a name.
    #[error(
        "'{generator}' did not write a file for '{output}' at $out/{output}; \
        nothing was written.\n\
        \n\
        $out held:{}",
        bulleted(.produced)
    )]
    GeneratorOutputMissing {
        /// The generator that was run.
        generator: String,
        /// The declared output that is absent.
        output: String,
        /// Every name the output directory did hold, in name order.
        produced: Vec<String>,
    },

    /// The stream carrying a prompt's answer ended before the answer did.
    #[error("no value read for prompt '{name}'")]
    NoValueForPrompt {
        /// The prompt that went unanswered.
        name: String,
    },

    /// The operator answered a prompt with nothing.
    #[error("prompt '{name}' was answered with nothing; refusing to generate from an empty input")]
    PromptUnanswered {
        /// The prompt that was answered with nothing.
        name: String,
    },

    /// The generator script exited non-zero.
    #[error(
        "the generator for '{generator}' exited {status}; nothing was written. \
        Its diagnostics are above, on stderr."
    )]
    GeneratorFailed {
        /// The generator that failed.
        generator: String,
        /// What its script exited with.
        status: i32,
    },

    /// A generator produced an empty value for one of its outputs.
    #[error(
        "'{generator}' produced nothing for '{output}'; an empty value is the state a \
        truncated write leaves behind, so it is refused"
    )]
    GeneratorProducedNothing {
        /// The generator that produced it.
        generator: String,
        /// The output it produced nothing for.
        output: String,
    },

    /// The entry's own validation refused a candidate value.
    ///
    /// The values are still only in this process's memory when this is raised,
    /// so nothing has to be undone.
    #[error(
        "the validation for '{generator}' rejected the candidate value for '{output}'; \
        nothing was written"
    )]
    ValidationRejected {
        /// The generator whose validation refused.
        generator: String,
        /// The output whose candidate value was refused.
        output: String,
    },

    /// The operator declined the cascade a rotation carries.
    #[error("declined; nothing was written. Pass --yes to answer this in advance.")]
    CascadeDeclined,

    /// Neither editor variable is set, so there is no editor to open.
    ///
    /// No fallback to a named program, and that absence is the decision rather
    /// than an omission. Dropping an operator who has never used it into `vi`,
    /// with a secret in the buffer, produces either an accidental write or an
    /// accidental abandonment — and safix cannot tell the two apart, so the
    /// value it stores would be one nobody chose.
    #[error(
        "neither $VISUAL nor $EDITOR is set, so there is no editor to open; \
        nothing was decrypted or staged.\n\
        \n\
        Set one of them and re-run. safix opens no editor of its own choosing: \
        a program you did not pick, holding your plaintext, can be left or \
        saved by accident and safix cannot tell which happened."
    )]
    NoEditor,

    /// `edit` was asked for an output whose value is not encrypted.
    ///
    /// A public output has no ciphertext, no key inside a document and no
    /// creation rule, so there is nothing for the encrypting write path to do
    /// with it. Refused by name rather than allowed to create a document
    /// alongside the plaintext, which would leave one value in two places.
    #[error(
        "'{name}' is a public output, so its value is already plaintext at {path}; \
        there is nothing to decrypt and nothing to encrypt.\n\
        \n\
        A public output is minted by the generator that declares it. Re-run \
        `safix generate` to replace it, or edit {path} directly — it is an \
        ordinary file in the repository, which is what declaring it \
        `secret = false` means."
    )]
    PublicNotEditable {
        /// The output that was asked for.
        name: String,
        /// Where its plaintext is.
        path: String,
    },

    /// The editor exited non-zero, so its buffer is not a value.
    #[error(
        "the editor exited {status}; nothing was written and nothing was committed. \
        The staged buffer has been shredded."
    )]
    EditorFailed {
        /// What the editor exited with.
        status: i32,
    },

    /// `keygen` was asked to mint an identity for somebody else.
    #[error("{}", keygen_for_someone_else(.user))]
    KeygenForSomeoneElse {
        /// The user it was asked to mint for.
        user: String,
    },

    /// `age-keygen` could not be run, or refused.
    #[error("age-keygen failed; nothing was appended")]
    KeygenFailed,

    /// `age-keygen` ran and named no public key.
    #[error("age-keygen wrote no public key; check {file} before re-running")]
    KeygenNoPublicKey {
        /// The identity file it was appending to.
        file: String,
    },

    /// `keygen --show` was asked and no identity has been minted on this
    /// machine yet.
    #[error("{}", prose::keygen_no_identity_yet(file))]
    KeygenNoIdentityYet {
        /// The identity file that holds no identity yet, so no public-key
        /// comment could be found in it.
        file: String,
    },

    /// The name is outside the alphabet a path and a `path_regex` are built
    /// from.
    #[error("{}", bad_user_name(.name, .pattern))]
    BadUserName {
        /// The name that was asked for.
        name: String,
        /// The pattern the nix half declares, unanchored.
        pattern: String,
    },

    /// The recipient needs a person present to decrypt with.
    #[error("{}", hardware_recipient(.recipient))]
    HardwareRecipient {
        /// The recipient that was given.
        recipient: String,
    },

    /// The recipient is not shaped like an age X25519 public key.
    #[error("{}", bad_recipient(.recipient))]
    BadRecipient {
        /// The recipient that was given.
        recipient: String,
    },

    /// The declarations already name this person.
    #[error("{}", already_declared(.user))]
    AlreadyDeclared {
        /// The name that was asked for.
        user: String,
    },

    /// The scaffold's path is occupied by something that declares nobody.
    #[error(
        "{file} already exists but declares no user; \
        resolve that by hand before scaffolding over it"
    )]
    ScaffoldExists {
        /// The repository-relative path the scaffold would occupy.
        file: String,
    },

    /// `--host` was given and no hook is configured to receive it.
    #[error("{HOST_WITHOUT_HOOK}")]
    HostWithoutHook,

    /// A generated file does not parse, so nothing is staged.
    #[error("generated {path} does not parse; nothing was staged")]
    Unparsable {
        /// The path that was written and then read back.
        path: String,
    },

    /// The operator declined the scaffold.
    #[error("aborted; nothing was written")]
    ScaffoldDeclined,

    /// The policy could not be regenerated after the scaffold was written.
    ///
    /// Its own variant rather than [`Error::NixEvalFailed`] because what the
    /// operator needs to know is the state this leaves: one untracked file
    /// written, the policy untouched, and no commit.
    #[error(
        "could not evaluate flake.safix.lib.policyText in {root}; the scaffold is written \
        but .sops.yaml is untouched and nothing is committed"
    )]
    PolicyEvalAfterScaffold {
        /// The repository the evaluation was rooted at.
        root: String,
    },

    /// The consumer's onboarding hook exited non-zero.
    #[error(
        "the onboarding hook exited {status}. The scaffold and the policy are committed; \
        whatever the hook left behind is yours to review."
    )]
    HookFailed {
        /// What the hook exited with.
        status: i32,
    },

    /// The kernel's entropy pool could not be read, so no credential was minted.
    #[error("could not read {source}, so no PIN or PUK was generated")]
    EntropyUnreadable {
        /// The source that was being read.
        source: &'static str,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// The `ykman` binary could not be run.
    #[error("could not run {program}, so the card's access was not touched")]
    YkmanUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// No smartcard service answered, so no reader could be asked.
    #[error("{PCSCD_UNAVAILABLE}")]
    PcscdUnavailable,

    /// No card is connected.
    #[error(
        "no card is connected. Insert the one you mean to enroll and re-run; nothing \
        was touched."
    )]
    NoCardConnected,

    /// More than one card is connected and none was named.
    #[error("{}", cards_ambiguous(.serials))]
    CardsAmbiguous {
        /// Every serial that answered, in the order they were listed.
        serials: Vec<String>,
    },

    /// `ykman` ran and refused, and this is what it said.
    ///
    /// The arguments are carried redacted of the flags that hold a credential:
    /// the PIN reaches `ykman`'s argument vector because `ykman` has no other
    /// interface for it, and a refusal is a string that travels further than one
    /// process.
    #[error("ykman {arguments} failed:\n{output}")]
    CardCommandFailed {
        /// What `ykman` was given, with every credential replaced.
        arguments: String,
        /// `ykman`'s own standard error, verbatim.
        output: String,
    },

    /// The card refused the PIN, and one attempt is all a run makes.
    #[error("{}", card_pin_rejected(.serial))]
    CardPinRejected {
        /// The card that refused it.
        serial: String,
    },

    /// An OTP slot was asked for.
    #[error("{OTP_REFUSED}")]
    OtpRefused,

    /// A card was asked for with no touch required.
    #[error(
        "touch-policy never is refused. The touch is the property a card is for, and an \
        identity generated without it is a smartcard emulating a file.\n\
        \n\
        The policies safix generates with are pin-policy once and touch-policy cached, \
        which is what this fleet's enrolled cards carry: one PIN entry per session, and \
        one touch cached for fifteen seconds."
    )]
    TouchPolicyNever,

    /// The run has no terminal, so nobody can be told to touch the card.
    #[error("{NO_TERMINAL}")]
    NoTerminal,

    /// A pseudo-terminal could not be opened, read or written.
    #[error("could not drive a pseudo-terminal, so the generator's PIN prompt was not reached")]
    PtyUnusable {
        /// What the kernel objected to.
        #[source]
        cause: io::Error,
    },

    /// The age plugin could not be run.
    #[error("could not run {program}, so no identity was generated")]
    PluginUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// The age plugin ran and refused.
    ///
    /// Its own standard error crossed the terminal as it was written, so nothing
    /// of it is repeated here: what this adds is that the run stopped and that no
    /// identity was appended.
    #[error(
        "the age plugin exited {status}; nothing was appended and no recipient was added. \
        Its own message is above."
    )]
    PluginFailed {
        /// What the plugin exited with.
        status: i32,
    },

    /// The age plugin said nothing for the idle limit and was ended.
    #[error(
        "the age plugin said nothing for {seconds} seconds and was ended; nothing was \
        appended.\n\
        \n\
        A card that is not inserted, a reader another agent holds exclusively, or a \
        touch nobody made all look like this."
    )]
    PluginStalled {
        /// How long it was given.
        seconds: u64,
    },

    /// The age plugin ran, succeeded, and printed no identity block.
    #[error(
        "the age plugin succeeded and printed no identity block, so there is nothing to \
        append and no recipient to add"
    )]
    PluginNoIdentity,

    /// The person's custody record is not one this can extend.
    #[error("{}", no_declaration_file(.user, .file))]
    NoDeclarationFile {
        /// The person whose record was being edited.
        user: String,
        /// The path it was expected at.
        file: String,
    },

    /// A re-wrap dropped a recipient a file had before the run.
    #[error("{}", recipients_lost(.file, .lost))]
    RecipientsLost {
        /// The file that lost them.
        file: String,
        /// Every recipient that could open it before and cannot now.
        lost: Vec<String>,
    },

    /// The person's audience covers no file the proof could use.
    #[error("{}", no_file_to_prove_with(.user))]
    NoFileToProveWith {
        /// The person whose audience was searched.
        user: String,
    },

    /// A password-store command could not be run.
    #[error("could not run {program}, so the credentials were not mirrored")]
    StoreUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// A store the credentials were being written to refused.
    #[error(
        "{transport} exited {status}, so the credentials are not there:\n{output}\n\
        \n\
        The card is enrolled and everything else is committed. This is a copy that \
        was not made, not a step that failed halfway."
    )]
    StoreMirrorFailed {
        /// Which store refused, in the words the report prints.
        transport: &'static str,
        /// What its command exited with.
        status: i32,
        /// Its own standard error, verbatim.
        output: String,
    },

    /// The mirror declares mappings and no database for them to reach.
    #[error("{}", prose::no_store_database(*.mappings))]
    NoStoreDatabase {
        /// How many mappings are declared.
        mappings: usize,
    },

    /// A mirror mapping name the declarations do not carry.
    #[error("{}", prose::unknown_sync_mapping(mapping, &declared.join(", ")))]
    UnknownSyncMapping {
        /// The name that was asked for.
        mapping: String,
        /// Every mapping that is declared.
        declared: Vec<String>,
    },

    /// The database needs a password and there is no terminal to ask on.
    #[error("{}", prose::store_locked(database))]
    StoreLocked {
        /// The database that was not opened.
        database: String,
    },

    /// The store's own command would not open the database.
    #[error("{}", prose::database_unreadable(database, output))]
    DatabaseUnreadable {
        /// The database that did not open.
        database: String,
        /// The command's own standard error, verbatim.
        output: String,
    },

    /// The store's own command was started with a pipe that was not there.
    #[error("the store's own command was started without the pipe its value travels")]
    StorePipeMissing,

    /// The store's own command refused over one entry.
    #[error("{}", prose::store_command_failed(entry, arguments, output))]
    StoreCommandFailed {
        /// The entry, or the group, it refused over.
        entry: String,
        /// The argument vector it was run with, which carries no value.
        arguments: String,
        /// Its own standard error, verbatim.
        output: String,
    },

    /// A value the store's own command cannot carry whole.
    #[error("{}", prose::value_spans_lines(entry))]
    ValueSpansLines {
        /// The entry it would have been written to.
        entry: String,
    },

    /// A mirror mapping whose safix side holds nothing to mirror.
    #[error("{}", prose::sync_source_empty(mapping, user, name, file, *.generated))]
    SyncSourceEmpty {
        /// The mapping being converged.
        mapping: String,
        /// The person whose entry it names.
        user: String,
        /// The entry, as they hold it.
        name: String,
        /// The file that does not carry the key.
        file: String,
        /// Whether a generator would mint it, which decides the remedy.
        generated: bool,
    },

    /// A mirror mapping whose database side holds no entry to read.
    #[error("{}", prose::store_entry_absent(mapping, entry, mode))]
    StoreEntryAbsent {
        /// The mapping being converged.
        mapping: String,
        /// The entry path the database holds nothing at.
        entry: String,
        /// The mapping's mode, which is what makes the database the source.
        mode: &'static str,
    },

    /// clan refused to register the recipient, both ways of asking.
    #[error(
        "clan would not register the card as {user}'s key:\n{output}\n\
        \n\
        The identity, the recipient and the re-wrap are committed on the safix side. \
        clan owns its own store, so the registration is clan's to accept and safix \
        writes nothing into it directly."
    )]
    ClanUserRegistrationFailed {
        /// The person the key was being registered for.
        user: String,
        /// clan's own standard error, verbatim.
        output: String,
    },

    /// The consumer's enrollment hook exited non-zero.
    #[error(
        "the enrollment hook exited {status}. The identity, the recipient and the policy \
        are committed; whatever the hook left behind is yours to review."
    )]
    EnrollHookFailed {
        /// What the hook exited with.
        status: i32,
    },

    /// A delegation check was reached and the commit's identity names no declared
    /// person.
    ///
    /// Its own refusal rather than an out-of-scope one, because the two have
    /// different remedies: this is a repository whose identity says nothing the
    /// declarations can match, and no edit to a `managers` list would change that.
    #[error("{}", actor_undeclared(.name, .email, .delegation, .declared))]
    ActorUndeclared {
        /// `user.name`, as this repository resolves it.
        name: String,
        /// `user.email`, as this repository resolves it.
        email: String,
        /// The delegation that asked who is acting.
        delegation: Box<crate::delegation::Refused>,
        /// Every person the declarations name, in name order.
        declared: Vec<String>,
    },

    /// The declarations name no such group.
    #[error(
        "'{group}' is not a declared group of flake.safix.groups.\n\nDeclared groups:{}",
        bulleted(.declared)
    )]
    UnknownGroup {
        /// The name that was asked for.
        group: String,
        /// Every group the declarations do name, in name order.
        declared: Vec<String>,
    },

    /// The declarations name no such subject.
    #[error("{}", unknown_subject(.subject, .declared))]
    UnknownSubject {
        /// The name that was asked for.
        subject: String,
        /// Every subject the declarations do name, in name order.
        declared: Vec<String>,
    },

    /// The group's declaration is not at the path this verb edits.
    #[error("{}", no_group_declaration(.group, .file))]
    NoGroupDeclaration {
        /// The group whose membership was being edited.
        group: String,
        /// The path it was expected at.
        file: String,
    },

    /// A declared person acting outside the delegation covering what they were
    /// about to edit.
    #[error("{}", scaffold_out_of_scope(.actor, .delegation, .managers))]
    ScaffoldOutOfScope {
        /// The declared person a commit made here would name.
        actor: String,
        /// The delegation that refused them.
        delegation: Box<crate::delegation::Refused>,
        /// Every manager the organizations it names declare, in name order.
        managers: Vec<String>,
    },

    /// clan's own `machines list` failed while searching for the machine
    /// that addresses a shared mapping.
    #[error("{}", prose::clan_machines_list_failed(output))]
    ClanMachinesListFailed {
        /// clan's own standard error, verbatim.
        output: String,
    },

    /// No machine in clan's own fleet resolves a shared mapping's generator.
    ///
    /// Raised only after every machine `Clan::machines` returned was tried
    /// and refused with clan's own "no such var", which is what tells "this
    /// machine does not see this generator" apart from a genuine failure.
    #[error("{}", prose::clan_address_unresolved(mapping, generator, file))]
    ClanAddressUnresolved {
        /// The mapping whose shared placement could not be addressed.
        mapping: String,
        /// The generator that was searched for.
        generator: String,
        /// The file that was searched for.
        file: String,
    },

    /// The named target is not a declared machine — because nothing names it,
    /// or because it names a person instead.
    ///
    /// One message for both: this verb targets machines only and carries no
    /// separate code path for a person's name, so a distinct wording for that
    /// case would be a second answer to a question this one already answers.
    #[error(
        "'{machine}' is not a declared machine of flake.safix.machines.\n\nDeclared machines:{}",
        bulleted(.declared)
    )]
    UnknownMachine {
        /// The name that was asked for.
        machine: String,
        /// Every machine the declarations do name, in name order.
        declared: Vec<String>,
    },

    /// The named machine is declared and names no recipient.
    #[error(
        "'{machine}' is a declared machine with no recipient, so there is nothing to check \
        supplied identity material against. Declare flake.safix.machines.{machine}.recipient."
    )]
    MachineHasNoRecipient {
        /// The machine whose recipient is null.
        machine: String,
    },

    /// A write was asked for and no `--identity` named material to write.
    #[error("--identity is required to write a host identity; nothing was written")]
    UploadNeedsIdentity,

    /// The key at `--identity` does not derive to the machine's declared
    /// recipient.
    #[error(
        "the identity at {path} derives to {supplied}, which is not {machine}'s declared \
        recipient {declared}. Seeding it would not match what {machine}'s audience is \
        already wrapped to."
    )]
    SuppliedIdentityMismatch {
        /// The machine being provisioned.
        machine: String,
        /// The `--identity` file that was read.
        path: String,
        /// The machine's declared recipient.
        declared: String,
        /// The recipient the supplied key derives to.
        supplied: String,
    },

    /// The target already presents an ed25519 host key, and it is neither
    /// absent nor the machine's declared recipient.
    #[error(
        "{machine} already presents an ed25519 host key, and its age form {presented} is \
        neither absent nor {machine}'s declared recipient {declared}. Seed it anyway with \
        --force and a matching --identity, or investigate before overriding a host that may \
        already be live."
    )]
    PresentedIdentityMismatch {
        /// The machine being provisioned.
        machine: String,
        /// The machine's declared recipient.
        declared: String,
        /// The recipient the presented key derives to.
        presented: String,
    },

    /// One of `upload`'s external tools — `ssh-keygen`, `ssh-to-age`,
    /// `ssh-keyscan`, `ssh` or `tar` — could not be run at all.
    #[error("could not run {program}")]
    UploadToolUnavailable {
        /// The program that was reached for.
        program: String,
        /// The underlying failure.
        #[source]
        cause: io::Error,
    },

    /// `ssh-to-age` was started without the pipe its input travels.
    #[error("ssh-to-age was started without the pipe its input travels")]
    UploadPipeMissing,

    /// One of `upload`'s external tools ran and refused.
    #[error("{program} exited refusing: {output}")]
    UploadToolFailed {
        /// The program that refused.
        program: String,
        /// Its own standard error, verbatim.
        output: String,
    },

    /// The fixed remote destination fails the path-depth safety the transport
    /// itself enforces.
    ///
    /// Unreachable while the destination is the constant this crate declares;
    /// carried as a real refusal rather than an assertion so that a future
    /// change narrowing the destination fails safely rather than by panicking
    /// mid-wipe.
    #[error(
        "{destination} is shallower than the wipe-then-extract transport allows; refusing \
        before touching it"
    )]
    UploadDestinationUnsafe {
        /// The destination that failed the check.
        destination: String,
    },

    /// clan's own `vars list` failed while enumerating one machine's vars for
    /// the audit's lingering report.
    ///
    /// Raised before any mapping is compared or any other machine is
    /// enumerated, the same way [`Self::ClanUnavailable`] already is: a
    /// lingering section that silently dropped one machine's contribution
    /// would read as complete while being partial.
    #[error("{}", prose::clan_machine_list_failed(machine, output))]
    ClanMachineListFailed {
        /// The machine whose vars could not be listed.
        machine: String,
        /// clan's own standard error, verbatim.
        output: String,
    },
}

/// The result type this crate returns.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_naming_a_list_ends_without_a_newline() {
        let refusal = Error::UnknownUser {
            user: "dave".into(),
            declared: vec!["alice".into(), "bob".into(), "carol".into()],
        };
        assert_eq!(
            refusal.to_string(),
            "'dave' is not a declared user of flake.safix.users.\n\nDeclared users:\n  - alice\n  - bob\n  - carol"
        );
    }

    #[test]
    fn a_refusal_naming_an_empty_list_ends_at_its_heading() {
        let refusal = Error::UnknownUser {
            user: "dave".into(),
            declared: Vec::new(),
        };
        assert!(refusal.to_string().ends_with("Declared users:"));
    }

    #[test]
    fn the_unknown_name_refusal_keeps_the_shell_s_literal_owner_placeholder() {
        let refusal = Error::UnknownName {
            user: "carol".into(),
            name: "api-token".into(),
            held: Vec::new(),
        };
        let rendered = refusal.to_string();
        assert!(rendered.contains(
            "  3. flake.safix.users.<owner>.sharedWith.carol.api-token — granted from outside\n"
        ));
        assert!(rendered.ends_with("Names flake.safix.users.carol holds:"));
    }
}
