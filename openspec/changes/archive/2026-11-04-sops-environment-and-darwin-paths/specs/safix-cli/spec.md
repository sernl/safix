## ADDED Requirements

### Requirement: The encrypting backend is denied every key-service address unless the operator says otherwise

Every invocation of the encrypting backend SHALL carry `SOPS_HC_VAULT_ALLOWLIST=none` unless the operator's environment sets it to a non-empty value, which SHALL reach the backend unchanged.
An invocation for a run that must not inherit ambient identity material SHALL additionally carry neither `VAULT_TOKEN` nor `VAULT_ADDR`.
The environment reference SHALL name both behaviours and GHSA-jgf3-f6rg-8x3h.

#### Scenario: An unset allowlist is set to none

- **WHEN** a value is written to a declared secret whose file does not exist yet, with `SOPS_HC_VAULT_ALLOWLIST` unset
- **THEN** every backend invocation the run makes, the one creating the file and the one writing the value, carries `SOPS_HC_VAULT_ALLOWLIST=none`

#### Scenario: An operator's allowlist wins

- **WHEN** the same write runs with `SOPS_HC_VAULT_ALLOWLIST` set to a non-empty value by the operator
- **THEN** every backend invocation carries exactly that value
- **AND** none carries `none`

#### Scenario: An isolated run hands the backend no Vault credentials

- **WHEN** an installation decrypts with `VAULT_TOKEN` and `VAULT_ADDR` present in its own environment
- **THEN** the backend invocation carries neither variable
- **AND** it carries `SOPS_HC_VAULT_ALLOWLIST=none`

### Requirement: The packaged command carries the card plugin as a fallback on its search path

The packaged command SHALL append `age-plugin-yubikey` from its own closure to the end of its executable search path, after whatever the operator's search path already holds, so a write to a card's recipient succeeds without the operator installing the plugin and an operator's own plugin is still the one found first.
The packaged command SHALL evaluate on every system the flake declares.

#### Scenario: The plugin is found with nothing else installed

- **WHEN** the packaged command runs with an operator search path that holds no `age-plugin-yubikey`
- **THEN** the search path its children inherit ends with the directory holding the packaged plugin

#### Scenario: The operator's plugin comes first

- **WHEN** the packaged command runs with an operator search path
- **THEN** every operator entry precedes the packaged plugin's directory in the search path its children inherit

### Requirement: keygen names its identity file to standalone tools where their default differs

On macOS, after appending an age identity, `keygen` and `keygen --kind age` SHALL print an `export SOPS_AGE_KEY_FILE=<file>` line naming the file written, unless that file is the one standalone sops reads by default or `SOPS_AGE_KEY_FILE` already names it.
Standalone sops's default on macOS is `$XDG_CONFIG_HOME/sops/age/keys.txt` where `XDG_CONFIG_HOME` is non-empty and `$HOME/Library/Application Support/sops/age/keys.txt` otherwise.
Where safix itself looks for identities SHALL NOT change.

#### Scenario: A macOS identity under the XDG default is named

- **WHEN** `keygen` appends an identity on macOS with `XDG_CONFIG_HOME` and `SOPS_AGE_KEY_FILE` unset
- **THEN** the output includes `export SOPS_AGE_KEY_FILE=` followed by the file it appended to

#### Scenario: Nothing is printed where standalone sops already looks

- **WHEN** `keygen` appends an identity on macOS with `XDG_CONFIG_HOME` set, so the file written is the one standalone sops reads
- **THEN** the output includes no `export SOPS_AGE_KEY_FILE=` line

#### Scenario: Nothing is printed off macOS

- **WHEN** `keygen` appends an identity on Linux
- **THEN** the output includes no `export SOPS_AGE_KEY_FILE=` line

#### Scenario: safix's own discovery is unchanged

- **WHEN** a run on macOS discovers its age identity file
- **THEN** it consults `SOPS_AGE_KEY_FILE`, then `SAFIX_AGE_KEY_FILE`, then `$XDG_CONFIG_HOME/sops/age/keys.txt` falling back to `$HOME/.config`, exactly as on Linux
