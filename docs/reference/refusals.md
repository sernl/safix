---
title: "Refusal codes"
---

## What this page is for

This page lets you find out what a refusal code means and what the message told
you to do about it. Every refusal safix raises carries one of the codes below,
and the set is closed: a new refusal does not compile until it is given a code.

Branch on the code, not on the prose. Diagnostic wording may change between
releases; the code names the same refusal after it does. Every code is prefixed
`safix::`, and a program may enumerate the whole set from the library.

Set `SAFIX_ERROR_FORMAT=plain` to render a refusal in the retired shell
runtime's shape — see [Environment](environment.md#safix_error_format).

## Documents and evaluation

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::document_operation` | A ciphertext format or cryptographic subprocess operation failed | It names the operation, the path and a diagnostic carrying no plaintext |
| `safix::not_a_repository` | Nothing here is a repository, so no path resolves against one | Run inside the repository holding the declarations, or set `SAFIX_REPO_ROOT` |
| `safix::nix_eval_failed` | The nix half could not be evaluated | It names the attribute and the root; a nix that ran and refused has already said why on its own stderr |
| `safix::nix_schema_mismatch` | The nix half evaluated to a shape this runtime does not read | It names the attribute and what the deserializer objected to; every schema denies unknown fields, so a new field on the nix side arrives here rather than being dropped |
| `safix::file_unreadable` | A file could not be read | It names the path |
| `safix::file_unwritable` | A file could not be written | It names the path |
| `safix::document_unreadable` | The bytes are not a YAML document | It names what the parser objected to |
| `safix::stanza_unreadable` | A `sops.age` entry is not a mapping carrying a string recipient | Repair the document's own metadata |
| `safix::stamp_record_unparsable` | A per-value record exists and is not the one line safix writes | Restore it from git history to keep the dates it held, or remove it — an entry with no record reads as having no dates |

## Naming and resolution

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::unknown_user` | The declarations name no such person | It lists every person they do name, in name order |
| `safix::unknown_name` | This person holds no secret by that name | It names the three places a secret is declared — the catalogue selected by `carries`, the person's `private`, and a grant from outside — and lists what they hold |
| `safix::no_default_user` | No person can be assumed | Name one: `safix <subcommand> <user> <name>` |
| `safix::no_file_for_name` | The placement record carries no file, so there is nowhere to read or write | It names the entry |
| `safix::not_a_yaml_path` | The placement is outside the suffix every creation rule ends in | Place the entry at a `*.yaml` path; every generated rule ends in `\.yaml$`, so no rule covers anything else |
| `safix::no_value_yet` | The file the name resolves to does not exist, so the name has no value | `safix set <user> <name>` |
| `safix::unknown_machine` | The declarations name no such machine | It lists every machine they do name |
| `safix::unknown_group` | The declarations name no such group | It lists every group they do name |
| `safix::unknown_subject` | No declaration of any subject kind covers that name | It lists the declared subjects, and says an organization is not one — an audience wanting its custody names the organization |

## Reading a value

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::secret_unreadable` | A value could not be read from the stream it was being read from | Whatever had been read was zeroed first, so no partial value survives; retry the read |
| `safix::recipients_unreadable` | A governed file's recipients could not be read | It names the file |
| `safix::sops_unavailable` | The encryption tool could not be run | Install it, or name the one to use in `SAFIX_SOPS` |
| `safix::sops_pipe_missing` | The tool was started with a pipe that was not there to read | Retry; nothing was written |
| `safix::sops_key_index` | A key name could not be rendered as the index the tool extracts by | It names the key and the parse failure |

## Writing a value

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_value_read` | Nothing was read at the value prompt | Re-run and supply one |
| `safix::no_confirmation_read` | Nothing was read at the confirmation prompt | Re-run and supply one |
| `safix::entries_differ` | The two entries differ, and nothing was written | Re-run and type the same value twice |
| `safix::empty_value` | The value is empty | Supply a value; an empty one is the state a truncated write leaves behind |
| `safix::mid_operation` | The repository is part-way through an operation a commit would disturb | Finish or abort it before setting a secret |
| `safix::conflict_entries` | The target file has unmerged conflict entries | Resolve them before setting a secret |
| `safix::uncommitted_changes` | The target file already has changes a commit here would sweep up | Commit or discard them first, then re-run |
| `safix::git_unavailable` | The git binary could not be run | Install it, or name the one to use in `SAFIX_GIT` |
| `safix::git_command_failed` | git ran and refused | It names the arguments it ran |
| `safix::git_output_not_text` | git printed something that is not text | It names what the decoder objected to |

## Audiences and the recipient policy

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_audience_for_file` | The audience map declares no audience for that file, so there is nothing to hold its recipients to | Declare the entry that places a value there |
| `safix::candidate_recipients_unreadable` | The recipients of the document prepared for a file could not be read | It names the file; nothing was published |
| `safix::recipient_drift` | A file is not encrypted to the audience declared for it | It lists who can open it and is not in its audience, and who is in its audience and cannot; re-wrap with `safix fix`, review `git diff`, then re-run |
| `safix::no_creation_rule` | The committed policy has no creation rule for that file | Regenerate the policy, review the diff, then re-run; there is deliberately no catch-all rule to fall back on |
| `safix::rewrap_unschedulable` | A governed file's re-wrap could not be run | It names the cause |
| `safix::sops_create_failed` | The tool could not create the file | It names the file and carries the tool's own output |

## Generators

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_generator` | The entry has no generator, so there is nothing to run | Declare one on the entry, or set the value by hand with `safix set <user> <name>` |
| `safix::generator_cycle` | The run order carries a cycle of generators, so nothing can run first | It prints the cycle; a cycle is refused at evaluation, so an order carrying one did not come from that refusal |
| `safix::dependency_has_no_value` | A declared dependency has no value yet, so its file cannot be written into `$in` | Give the dependency a value first |
| `safix::generate_needs_nixpkgs` | A generator's sandbox resolves its tools through a flake-only operation, and this run named `--entry` | Drop `--entry` and run against the declaring flake, or pass `--nixpkgs <flake-ref>` |
| `safix::sandbox_unavailable` | The sandbox backend is not available, so no generator ran | It names the backend and where it is supplied from; no flag runs a generator unsandboxed |
| `safix::sandbox_unsupported` | This platform has no sandbox backend | It names the platform and the two backends safix uses |
| `safix::generator_output_missing` | A generator did not write a file for a declared output | It lists what `$out` did contain; nothing was written |
| `safix::no_value_for_prompt` | Nothing was read for a declared prompt | Re-run and answer it |
| `safix::prompt_unanswered` | A prompt was answered with nothing | Answer it; an empty input is refused rather than generated from |
| `safix::generator_failed` | The generator exited non-zero | Its own diagnostics are above, on standard error; nothing was written |
| `safix::generator_produced_nothing` | A generator produced an empty value for an output | An empty value is the state a truncated write leaves behind, so fix the script |
| `safix::validation_rejected` | The generator's validation rejected the candidate value | Fix the script or the validation; nothing was written |
| `safix::cascade_declined` | The regeneration cascade was declined | Pass `--yes` to answer that confirmation in advance |

## Staging plaintext

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::staging_not_memory_backed` | No memory-backed filesystem is available to stage plaintext on | It lists what was tried and which of those are disk-backed; re-run with `--allow-disk-staging`, or set `SAFIX_STAGING_DIR` to a tmpfs mount |
| `safix::staging_unusable` | Plaintext could not be staged at the chosen path | It names the path |

## Editing a value

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_editor` | Neither `$VISUAL` nor `$EDITOR` is set | Set one and re-run; safix opens no editor of its own choosing |
| `safix::public_not_editable` | The entry is a public output, already plaintext in the repository | Re-run `safix generate` to replace it, or edit the file directly |
| `safix::editor_failed` | The editor exited non-zero | Nothing was written or committed, and the staged buffer has been shredded |

## Choosing from a list

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::picker_needs_terminal` | Choosing needs a terminal and there is none | Name the entry — `safix view <name>` — or see what is there with `safix list [<user>]` |
| `safix::nothing_to_pick` | The person holds nothing to choose from | It is a state of the declarations rather than of the session; `safix list <user>` prints the same emptiness without refusing |
| `safix::selection_cancelled` | You left the list without choosing | Nothing was written and nothing decrypted was kept; the message is the code alone |

## Minting an identity

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::key_management` | A key operation cannot preserve private-key custody | It names the violated precondition |
| `safix::keygen_for_someone_else` | The name given is not you, and this writes a private key into your own identity file | They should run it themselves and hand you the public half; or say so explicitly with `safix keygen --for-someone-else <user>` |
| `safix::keygen_failed` | The key generator failed | Nothing was appended |
| `safix::keygen_no_public_key` | The key generator wrote no public key | Check the identity file before re-running |
| `safix::keygen_no_identity_yet` | `--show` was asked and no identity has been minted here | Mint one first with `safix keygen` |
| `safix::identity_key_file_unreadable` | The age key file could not be read | It names the path |

## Onboarding a person

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::bad_user_name` | The name is outside the alphabet a path and a rule pattern are built from | It prints the anchored pattern: lowercase letters and digits, then any of those plus underscore and hyphen |
| `safix::bad_recipient` | The recipient is not a well-formed age recipient | They mint one with `safix keygen`, or convert an ed25519 ssh key with `ssh-to-age` |
| `safix::hardware_recipient` | A recipient needing a physical interaction cannot be the primary one | Pass their software recipient, then add the card to their `recoveryRecipients`, which is additive |
| `safix::already_declared` | The declarations already name that person | `safix list <user>` shows what they hold; changing their recipient is an edit to their declaration followed by `safix fix` |
| `safix::scaffold_exists` | The file exists and declares no person | Resolve that by hand before scaffolding over it |
| `safix::host_without_hook` | `--host` was given and no onboarding hook is set | Set the hook, which receives the name, the recipient and every `--host`; or drop `--host` |
| `safix::unparsable` | The generated file does not parse | Nothing was staged; report it |
| `safix::scaffold_declined` | The confirmation was declined | Nothing was written |
| `safix::policy_eval_after_scaffold` | The policy could not be evaluated after the scaffold was written | The scaffold is written, the policy is untouched and nothing is committed; fix the declarations and re-run |
| `safix::hook_failed` | The onboarding hook exited non-zero | The scaffold and the policy are committed; whatever the hook left behind is yours to review |

## Hardware keys

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::entropy_unreadable` | The entropy source could not be read, so no PIN or PUK was generated | It names the source |
| `safix::ykman_unavailable` | The card tool could not be run | Install it, or name the one to use in `SAFIX_YKMAN`; the card's access was not touched |
| `safix::pcscd_unavailable` | No smartcard service answered | Enable `services.pcscd`, re-insert the card and re-run; a card held exclusively by another agent presents the same way |
| `safix::no_card_connected` | No card is connected | Insert the one you mean to enroll and re-run |
| `safix::cards_ambiguous` | More than one card is connected | Name the one you mean with `safix enroll --serial <serial>`; guessing would reprovision the card that was not meant |
| `safix::card_command_failed` | The card tool ran and refused | It names the arguments and carries the tool's output |
| `safix::card_pin_rejected` | The card refused the PIN | One attempt is spent, not three; where safix provisioned the card, its PIN is in that person's custody or in the password store beside it |
| `safix::otp_refused` | An OTP slot was asked for | No flag accepts it: a programmed challenge-response slot is what opens a password database, and writing it ends that database permanently |
| `safix::touch_policy_never` | `--touch-policy never` was asked for | The touch is the property a card is for; safix generates with `pin-policy once` and `touch-policy cached` |
| `safix::no_terminal` | Enrollment needs a terminal and there is none | Run it where you can type; nothing was touched |
| `safix::pty_unusable` | A pseudo-terminal could not be driven, so the generator's PIN prompt was not reached | Re-run on a terminal that allows one |
| `safix::plugin_unavailable` | The age plugin could not be run | Install it, or name the one to use in `SAFIX_AGE_PLUGIN_YUBIKEY`; no identity was generated |
| `safix::plugin_failed` | The age plugin exited non-zero | Its own message is above; nothing was appended and no recipient was added |
| `safix::plugin_stalled` | The age plugin said nothing for long enough to be ended | A card that is not inserted, a reader another agent holds exclusively, or a touch nobody made all look like this |
| `safix::plugin_no_identity` | The plugin succeeded and printed no identity block | There is nothing to append and no recipient to add; re-run |
| `safix::no_declaration_file` | The person has no custody record this can extend | `safix adduser <user> <age-recipient>` writes one, or move an existing record to that path |
| `safix::recipients_lost` | A re-wrap took away a recipient the file had before the run | It lists who can no longer open it; run `safix check`, read `git diff`, converge deliberately, then re-run |
| `safix::no_file_to_prove_with` | The person's audience covers no file the proof could open | Give them a secret with `safix set <user> <name>`, then re-run `safix enroll <user>` |
| `safix::store_unavailable` | The password-store client could not be run, so the credentials were not mirrored | It names the program |
| `safix::store_mirror_failed` | The mirror transport exited non-zero, so the credentials are not there | The card is enrolled and everything else is committed: this is a copy that was not made, not a step that failed halfway |
| `safix::clan_user_registration_failed` | clan would not register the card as that person's key | The safix side is committed; clan owns its own store, so the registration is clan's to accept |
| `safix::enroll_hook_failed` | The enrollment hook exited non-zero | The identity, the recipient and the policy are committed; whatever the hook left behind is yours to review |

## The clan bridge

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_clan_flake` | A transfer was asked for and no clan is declared | Set the bridge's clan flake reference, declared once for the consumer |
| `safix::clan_unavailable` | The clan binary could not be run | Install clan, or name the one to use in `SAFIX_CLAN`; no mapping was transferred |
| `safix::clan_pipe_missing` | clan was started with a pipe that was not there to use | Retry |
| `safix::clan_var_unknown` | The clan half of a mapping names a var clan does not have | It prints the machine, generator and file, and names `clan vars list <machine>` |
| `safix::clan_command_failed` | clan ran and refused | clan's own message is carried verbatim |
| `safix::clan_machines_list_failed` | clan refused to list its machines | clan's own message is carried |
| `safix::clan_machine_list_failed` | clan refused to list one machine's vars | clan's own message is carried |
| `safix::clan_address_unresolved` | No machine in clan's fleet resolves a shared mapping's generator and file | Check them against what any machine sees with `clan vars list <machine>` |
| `safix::generator_definition_drifted` | clan already considers the mapping's generator outdated | Either run clan's generation now and accept the value it mints, or declare the mapping `clan-to-safix`; there is no option that exports anyway |
| `safix::source_has_no_value` | An export's source entry holds no value | Put a value there first — `safix generate` where a generator mints it, `safix set` where it is typed |
| `safix::source_unreadable` | An export's source did not decrypt for whoever is running | The tool has said why; a value that cannot be read cannot be verified |

## Selecting a mapping

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::unknown_mapping` | A bridge mapping name nothing declares | It lists the declared ones, or names the option to declare one under |
| `safix::unknown_sync_mapping` | A mirror mapping name nothing declares | It lists the declared ones, or names the option to declare one under |
| `safix::mapping_wrong_direction` | A named mapping is declared with a direction the `--direction` filter does not accept | Drop `--direction` to act on it, or narrow to its own declared direction |
| `safix::reserved_mapping_word` | A target keyword was given where a mapping name belongs | Name the mapping you meant, or drop it to act on every mapping of the target already named |
| `safix::mapping_name_needs_target` | A mapping name was given before any target was named | Name a target first; a mapping id may be declared under more than one target's namespace |
| `safix::direction_on_wrong_target` | `--direction` was given to a target that declares a mode instead | Drop `--direction`, or name the clan target |

## The password database

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_store_database` | Mappings are declared and no database is | Declare the database as a string path; there is no default, because which database holds a person's credentials is a fact about their machine |
| `safix::store_locked` | The database needs its password and there is no terminal to ask on | Run it where you can type, or leave the mapping to a run that can; the session's secret service is not a second way in |
| `safix::database_unreadable` | The database did not open, so no mapping was judged | The store's own message tells a wrong password from an unreadable file |
| `safix::store_pipe_missing` | The store's command was started without the pipe its value travels | Retry |
| `safix::store_command_failed` | The store's command refused over one entry | It prints the arguments it ran, which carry no value, and the command's own output |
| `safix::store_entry_absent` | A pulling mapping's database side holds no entry | Create it, and the next run converges safix onto it; or declare the mapping the other way round |
| `safix::value_spans_lines` | The value carries a newline and the store's command reads one line | Re-establish the value without the trailing byte — `printf '%s' "$VALUE" \| safix set <user> <name>` — or change the generator's last write to `printf` and regenerate |

## pass

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::pass_unavailable` | The store's own command could not be run | Install it, or point safix at the one you have with `SAFIX_PASS` |
| `safix::pass_locked` | One entry did not decrypt | The unlock belongs to your own agent, and gpg's own message is carried; safix does not interpose on it |
| `safix::pass_command_failed` | The store's command refused over one entry | It prints the arguments it ran, which carry no value or field, and the command's own output |
| `safix::no_pass_store` | The declared root is not a pass store | Either the location is wrong, or there is no store there yet and `pass init <your-gpg-id>` is yours to run |
| `safix::pass_entry_absent` | A pulling mapping's store side holds no entry | Create it, and the next run converges safix onto it; or declare the mapping the other way round |

## Bitwarden

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::bitwarden_unavailable` | The vault's client could not be run | Install it, or point safix at the one you have with `SAFIX_BW` |
| `safix::bitwarden_locked` | The client is locked with no terminal to ask on, or is not logged in at all | Unlock the client, or log in yourself — safix unlocks a vault and never logs one in |
| `safix::bitwarden_command_failed` | The client refused over one item | It prints the arguments it ran, which carry no value, and the client's own output |
| `safix::bitwarden_server_mismatch` | The declared server is not the one the unlocked client reports reaching | Point the client at the declared server, or declare the one it reaches; the refusal precedes every read |
| `safix::bitwarden_stale` | The client could not refresh its local copy | Its own message is carried; every mapping is refused rather than compared against a copy that may predate another device's change |
| `safix::bitwarden_item_ambiguous` | One declared address matches more than one item | Rename one of the items, or move it to another folder, so the address names exactly one |
| `safix::bitwarden_item_absent` | A pulling mapping's vault side holds no item | Create it, and the next run converges safix onto it; or declare the mapping the other way round |

## 1Password

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::onepassword_unavailable` | The service's own command could not be run | Install it, or point safix at the one you have with `SAFIX_OP` |
| `safix::onepassword_signed_out` | The command would not answer for the account | The session is yours: a service-account token in `OP_SERVICE_ACCOUNT_TOKEN`, or a session your own sign-in established; the command's own message is carried |
| `safix::onepassword_command_failed` | The command refused over one item | It prints the arguments it ran, which carry no value or field, and the command's own output; the rest of the run went on |
| `safix::onepassword_item_absent` | A pulling mapping's item holds no value, or does not exist | Create it, and the next run converges safix onto it; or declare the mapping the other way round |
| `safix::onepassword_vault_absent` | This session cannot see a vault by the declared name | Grant the session that vault or name one it already has; a service account cannot reach a built-in vault at all |

## Mapped fields

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::field_unsupported` | The mapping declares a field the target cannot carry | Remove the field, or move the mapping to a target whose own command can write it |
| `safix::field_source_in_argv` | A field is sourced from another entry, and the target carries that field in an argument vector | Write the field as a literal, or declare it on a target whose channel for it is a pipe |
| `safix::sync_source_empty` | A mapping's safix side holds no value to mirror | Give it a value — `safix generate` where a generator mints it, `safix set` where it is typed |

## Groups and delegation

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::no_group_declaration` | The file is not a group declaration this verb can edit | Move the declaration to that path, or edit the membership by hand — which owes the same disclosure this verb prints |
| `safix::actor_undeclared` | A commit made here would be authored by a name no person declares, where a delegation asked who is acting | Set this repository's commit identity to a declared name, or declare the person these commits already name |
| `safix::scaffold_out_of_scope` | The acting person is not among the managers the covering delegation names | A manager runs it, or this name joins them with one line under that organization's managers, committed first |

## Machines and host identities

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::machine_has_no_recipient` | The machine is declared with no recipient, so there is nothing to check supplied identity material against | Declare that machine's recipient |
| `safix::upload_needs_identity` | A host identity was to be written and none was supplied | Pass `--identity`; nothing was written |
| `safix::supplied_identity_mismatch` | The supplied identity derives to something other than the machine's declared recipient | It names both; seeding it would not match what that machine's audience is already wrapped to |
| `safix::presented_identity_mismatch` | The machine already presents a host key that is neither absent nor its declared recipient | Seed it anyway with `--force` and a matching `--identity`, or investigate first |
| `safix::upload_tool_unavailable` | A tool the transport needs could not be run | It names the program, and each has its own override variable |
| `safix::upload_pipe_missing` | The key converter was started without the pipe its input travels | Retry |
| `safix::upload_tool_failed` | A tool the transport ran exited refusing | It names the program and carries its output |
| `safix::upload_destination_unsafe` | The destination is shallower than the wipe-then-extract transport allows | Name a deeper destination; nothing was touched |

## The vault

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::vault_declared_without_root` | A vault is declared and `SAFIX_VAULT_ROOT` names no path | Set it to the vault repository's working tree before running safix again |
| `safix::vault_root_without_declaration` | `SAFIX_VAULT_ROOT` names a path and no vault is declared | Declare the vault or unset the variable; a declaration removed without first running the rollback is recovered by re-declaring it |
| `safix::vault_not_a_repository` | The named path is not a git repository | The vault root must name a git repository's working tree |
| `safix::vault_root_not_top_level` | The named path is not a repository's top level | It names the top level git reports instead; the vault root must be that top level itself |
| `safix::vault_commit_half_landed` | The vault committed and the declaration-root commit that was to follow it failed | It lists what is still staged; re-running the same command completes the operation and does not repeat the vault commit |
| `safix::vault_relocation_unreadable` | A file did not decrypt while it was being relocated | Nothing there was changed, and re-running `safix fix` completes it once the cause is fixed |

## Installing a manifest

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::manifest_unreadable` | The installer manifest could not be read | It names the path |
| `safix::manifest_unparsable` | The file is not an installer manifest this runtime reads | It names the path and what the deserializer objected to |
| `safix::manifest_version_unknown` | The manifest declares a schema version this build does not read | It names the version found and the one supported |
| `safix::manifest_mode_unparsable` | An entry's mode is not an octal file mode | It names the entry and the value |
| `safix::manifest_owner_unknown` | An entry names an owner this host declares no user for | Declare the user, or pass `--ignore-passwd` where no such user is meant to exist |
| `safix::manifest_group_unknown` | An entry names a group this host declares no group for | Declare the group, or pass `--ignore-passwd` |
| `safix::manifest_key_missing` | A document holds no value at the key an entry names | It names the document, the key and the entry |
| `safix::install_ssh_key_unconvertible` | An SSH identity did not convert to an age identity | It names the path and the reason |
| `safix::install_decrypt_failed` | The encryption tool exited non-zero decrypting a document | It names the document and the exit status |
| `safix::install_document_unparsable` | A document decrypted to something this runtime does not read | It names the document and what the parser objected to |
| `safix::install_mount_failed` | A secret store's filesystem could not be mounted | It names the filesystem and the path |
| `safix::install_runtime_dir_unknown` | The variable a user-mode manifest's `%r` expands from names no runtime directory | It names the variable |

## Migration

| Code | What it means | The remedy it names |
| --- | --- | --- |
| `safix::migration_refused` | A migration cannot preserve its source or destination guarantees | It names the violated precondition; sources are retained and nothing is published — see [Migration plan schema](migration-plan.md) |
