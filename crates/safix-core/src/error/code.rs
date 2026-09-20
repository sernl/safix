//! The stable name of a refusal.
//!
//! Scripts branch on codes, not diagnostic prose. `safix::recipient_drift`
//! names the same refusal when its explanation or terminal rendering changes.
//!
//! The table below is the whole of it, and it lives beside [`Error`] rather
//! than in the command's reporter for one reason: [`Error`] is
//! `#[non_exhaustive]`, so a match on it outside this crate needs a wildcard
//! arm, and a wildcard arm is a place a new refusal can arrive at without
//! anyone naming it. Inside this crate no wildcard is permitted, so a variant
//! added to [`Error`] does not compile until it is given a line here.
//!
//! [`Code`] is closed while [`Error`] is extensible. Consumers may enumerate
//! [`Code::ALL`] or match the stable strings returned by [`Code::as_str`].

use std::fmt::{Display, Formatter, Result as FmtResult};

use super::Error;

/// Both halves of the table, from one list: the code every refusal carries, and
/// the mapping from a refusal to it.
///
/// Written as a macro so the enumeration and the mapping cannot disagree. A
/// hand-written `ALL` is a list somebody has to remember to extend, which is
/// the failure this exists to make impossible.
macro_rules! refusal_codes {
    ($($variant:ident => $code:literal,)+) => {
        /// The stable name of a refusal.
        ///
        /// Closed where [`Error`] is `#[non_exhaustive]`, so callers can cover
        /// every code exhaustively. Callers that need an open representation
        /// use [`Code::as_str`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Code {
            $(
                #[doc = concat!("The code of [`Error::", stringify!($variant), "`], `", $code, "`.")]
                $variant,
            )+
        }

        impl Code {
            /// Every code, in the order [`Error`] declares its variants.
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            /// The code as it is printed and as a script greps for it.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)+
                }
            }
        }

        impl Error {
            /// The stable name of this refusal.
            #[must_use]
            pub const fn code(&self) -> Code {
                match self {
                    $(Self::$variant { .. } => Code::$variant,)+
                }
            }
        }
    };
}

refusal_codes! {
    DocumentOperation => "safix::document_operation",
    MigrationRefused => "safix::migration_refused",
    KeyManagement => "safix::key_management",
    SecretRead => "safix::secret_unreadable",
    NotInsideRepository => "safix::not_a_repository",
    NixEvalFailed => "safix::nix_eval_failed",
    NixSchemaMismatch => "safix::nix_schema_mismatch",
    UnknownUser => "safix::unknown_user",
    UnknownName => "safix::unknown_name",
    NoFileForName => "safix::no_file_for_name",
    NotAYamlPath => "safix::not_a_yaml_path",
    NoDefaultUser => "safix::no_default_user",
    NoValueYet => "safix::no_value_yet",
    RecipientsUnreadable => "safix::recipients_unreadable",
    SopsDocumentUnreadable => "safix::document_unreadable",
    SopsStanzaUnreadable => "safix::stanza_unreadable",
    SopsUnavailable => "safix::sops_unavailable",
    SopsPipeMissing => "safix::sops_pipe_missing",
    SopsKeyIndex => "safix::sops_key_index",
    ClanUnavailable => "safix::clan_unavailable",
    ClanPipeMissing => "safix::clan_pipe_missing",
    ClanVarUnknown => "safix::clan_var_unknown",
    ClanCommandFailed => "safix::clan_command_failed",
    UnknownMapping => "safix::unknown_mapping",
    MappingWrongDirection => "safix::mapping_wrong_direction",
    ReservedMappingWord => "safix::reserved_mapping_word",
    MappingNameNeedsTarget => "safix::mapping_name_needs_target",
    DirectionOnWrongTarget => "safix::direction_on_wrong_target",
    SourceHasNoValue => "safix::source_has_no_value",
    SourceUnreadable => "safix::source_unreadable",
    GeneratorDefinitionDrifted => "safix::generator_definition_drifted",
    NoClanFlake => "safix::no_clan_flake",
    FileUnreadable => "safix::file_unreadable",
    StampRecordUnparsable => "safix::stamp_record_unparsable",
    GitUnavailable => "safix::git_unavailable",
    GitCommandFailed => "safix::git_command_failed",
    GitOutputNotText => "safix::git_output_not_text",
    MidOperation => "safix::mid_operation",
    ConflictEntries => "safix::conflict_entries",
    UncommittedChanges => "safix::uncommitted_changes",
    NoValueRead => "safix::no_value_read",
    NoConfirmationRead => "safix::no_confirmation_read",
    EmptyValue => "safix::empty_value",
    EntriesDiffer => "safix::entries_differ",
    FileUnwritable => "safix::file_unwritable",
    NoAudienceForFile => "safix::no_audience_for_file",
    CandidateRecipientsUnreadable => "safix::candidate_recipients_unreadable",
    RecipientDrift => "safix::recipient_drift",
    NoCreationRule => "safix::no_creation_rule",
    RewrapUnschedulable => "safix::rewrap_unschedulable",
    SopsCreateFailed => "safix::sops_create_failed",
    GeneratorCycle => "safix::generator_cycle",
    NoGenerator => "safix::no_generator",
    DependencyHasNoValue => "safix::dependency_has_no_value",
    GenerateNeedsNixpkgs => "safix::generate_needs_nixpkgs",
    SandboxUnavailable => "safix::sandbox_unavailable",
    SandboxUnsupported => "safix::sandbox_unsupported",
    StagingNotMemoryBacked => "safix::staging_not_memory_backed",
    StagingUnusable => "safix::staging_unusable",
    GeneratorOutputMissing => "safix::generator_output_missing",
    NoValueForPrompt => "safix::no_value_for_prompt",
    PromptUnanswered => "safix::prompt_unanswered",
    GeneratorFailed => "safix::generator_failed",
    GeneratorProducedNothing => "safix::generator_produced_nothing",
    ValidationRejected => "safix::validation_rejected",
    CascadeDeclined => "safix::cascade_declined",
    NoEditor => "safix::no_editor",
    PublicNotEditable => "safix::public_not_editable",
    EditorFailed => "safix::editor_failed",
    KeygenForSomeoneElse => "safix::keygen_for_someone_else",
    KeygenFailed => "safix::keygen_failed",
    KeygenNoPublicKey => "safix::keygen_no_public_key",
    KeygenNoIdentityYet => "safix::keygen_no_identity_yet",
    BadUserName => "safix::bad_user_name",
    HardwareRecipient => "safix::hardware_recipient",
    BadRecipient => "safix::bad_recipient",
    AlreadyDeclared => "safix::already_declared",
    ScaffoldExists => "safix::scaffold_exists",
    HostWithoutHook => "safix::host_without_hook",
    Unparsable => "safix::unparsable",
    ScaffoldDeclined => "safix::scaffold_declined",
    PolicyEvalAfterScaffold => "safix::policy_eval_after_scaffold",
    HookFailed => "safix::hook_failed",
    EntropyUnreadable => "safix::entropy_unreadable",
    YkmanUnavailable => "safix::ykman_unavailable",
    PcscdUnavailable => "safix::pcscd_unavailable",
    NoCardConnected => "safix::no_card_connected",
    CardsAmbiguous => "safix::cards_ambiguous",
    CardCommandFailed => "safix::card_command_failed",
    CardPinRejected => "safix::card_pin_rejected",
    OtpRefused => "safix::otp_refused",
    TouchPolicyNever => "safix::touch_policy_never",
    NoTerminal => "safix::no_terminal",
    PickerNeedsTerminal => "safix::picker_needs_terminal",
    NothingToPick => "safix::nothing_to_pick",
    SelectionCancelled => "safix::selection_cancelled",
    PtyUnusable => "safix::pty_unusable",
    PluginUnavailable => "safix::plugin_unavailable",
    PluginFailed => "safix::plugin_failed",
    PluginStalled => "safix::plugin_stalled",
    PluginNoIdentity => "safix::plugin_no_identity",
    NoDeclarationFile => "safix::no_declaration_file",
    RecipientsLost => "safix::recipients_lost",
    NoFileToProveWith => "safix::no_file_to_prove_with",
    StoreUnavailable => "safix::store_unavailable",
    StoreMirrorFailed => "safix::store_mirror_failed",
    NoStoreDatabase => "safix::no_store_database",
    UnknownSyncMapping => "safix::unknown_sync_mapping",
    StoreLocked => "safix::store_locked",
    DatabaseUnreadable => "safix::database_unreadable",
    StorePipeMissing => "safix::store_pipe_missing",
    StoreCommandFailed => "safix::store_command_failed",
    ValueSpansLines => "safix::value_spans_lines",
    FieldUnsupported => "safix::field_unsupported",
    FieldSourceInArgv => "safix::field_source_in_argv",
    SyncSourceEmpty => "safix::sync_source_empty",
    StoreEntryAbsent => "safix::store_entry_absent",
    PassUnavailable => "safix::pass_unavailable",
    PassLocked => "safix::pass_locked",
    PassCommandFailed => "safix::pass_command_failed",
    NoPassStore => "safix::no_pass_store",
    PassEntryAbsent => "safix::pass_entry_absent",
    BitwardenUnavailable => "safix::bitwarden_unavailable",
    BitwardenLocked => "safix::bitwarden_locked",
    BitwardenCommandFailed => "safix::bitwarden_command_failed",
    BitwardenServerMismatch => "safix::bitwarden_server_mismatch",
    BitwardenItemAmbiguous => "safix::bitwarden_item_ambiguous",
    BitwardenItemAbsent => "safix::bitwarden_item_absent",
    BitwardenStale => "safix::bitwarden_stale",
    OnePasswordUnavailable => "safix::onepassword_unavailable",
    OnePasswordSignedOut => "safix::onepassword_signed_out",
    OnePasswordCommandFailed => "safix::onepassword_command_failed",
    OnePasswordItemAbsent => "safix::onepassword_item_absent",
    OnePasswordVaultAbsent => "safix::onepassword_vault_absent",
    ClanUserRegistrationFailed => "safix::clan_user_registration_failed",
    EnrollHookFailed => "safix::enroll_hook_failed",
    ActorUndeclared => "safix::actor_undeclared",
    UnknownGroup => "safix::unknown_group",
    UnknownRotationPolicy => "safix::unknown_rotation_policy",
    NoEntryDeclaration => "safix::no_entry_declaration",
    UnknownSubject => "safix::unknown_subject",
    NoGroupDeclaration => "safix::no_group_declaration",
    ScaffoldOutOfScope => "safix::scaffold_out_of_scope",
    ClanMachinesListFailed => "safix::clan_machines_list_failed",
    ClanAddressUnresolved => "safix::clan_address_unresolved",
    UnknownMachine => "safix::unknown_machine",
    MachineHasNoRecipient => "safix::machine_has_no_recipient",
    UploadNeedsIdentity => "safix::upload_needs_identity",
    SuppliedIdentityMismatch => "safix::supplied_identity_mismatch",
    PresentedIdentityMismatch => "safix::presented_identity_mismatch",
    UploadToolUnavailable => "safix::upload_tool_unavailable",
    UploadPipeMissing => "safix::upload_pipe_missing",
    UploadToolFailed => "safix::upload_tool_failed",
    UploadDestinationUnsafe => "safix::upload_destination_unsafe",
    ClanMachineListFailed => "safix::clan_machine_list_failed",
    VaultDeclaredWithoutRoot => "safix::vault_declared_without_root",
    VaultRootWithoutDeclaration => "safix::vault_root_without_declaration",
    VaultNotARepository => "safix::vault_not_a_repository",
    VaultRootNotTopLevel => "safix::vault_root_not_top_level",
    VaultCommitHalfLanded => "safix::vault_commit_half_landed",
    VaultRelocationUnreadable => "safix::vault_relocation_unreadable",
    ManifestUnreadable => "safix::manifest_unreadable",
    ManifestUnparsable => "safix::manifest_unparsable",
    ManifestVersionUnknown => "safix::manifest_version_unknown",
    ManifestModeUnparsable => "safix::manifest_mode_unparsable",
    ManifestOwnerUnknown => "safix::manifest_owner_unknown",
    ManifestGroupUnknown => "safix::manifest_group_unknown",
    ManifestKeyMissing => "safix::manifest_key_missing",
    IdentityKeyFileUnreadable => "safix::identity_key_file_unreadable",
    InstallSshKeyUnconvertible => "safix::install_ssh_key_unconvertible",
    InstallDecryptFailed => "safix::install_decrypt_failed",
    InstallDocumentUnparsable => "safix::install_document_unparsable",
    InstallMountFailed => "safix::install_mount_failed",
    InstallRuntimeDirUnknown => "safix::install_runtime_dir_unknown",
}

/// The namespace shared by every refusal code.
pub const NAMESPACE: &str = "safix::";

impl Code {
    /// The code without its namespace.
    ///
    /// Useful when a consumer has already supplied the namespace.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.as_str().strip_prefix(NAMESPACE).unwrap_or_else(|| {
            unreachable!("every code in the table above is written under the namespace")
        })
    }
}

impl Display for Code {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_under_the_namespace_and_distinct_from_the_rest() {
        let mut seen: Vec<&str> = Code::ALL.iter().map(|code| code.as_str()).collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total, "two refusals share one code");
        for code in Code::ALL {
            assert!(
                code.as_str().starts_with(NAMESPACE),
                "{code} is outside the namespace"
            );
            assert!(!code.name().is_empty(), "{code} has no name under it");
        }
    }
}
