# Proposal

## Why

Three findings from the 2026-11 upstream survey (`docs/notes/research/upstream-survey-2026-11.md`, sops §3.1, agenix-rekey §3.1 and clan-core F3 as corrected) concern the environment safix hands to sops and the paths it shares with standalone tools.
sops has no fix for GHSA-jgf3-f6rg-8x3h, a decrypt that sends the operator's Vault token to whatever `vault_address` a file's metadata names, and safix leaves the opt-in defence unset.
The packaged command does not put `age-plugin-yubikey` on its `PATH`, so a write whose audience includes an enrolled card's `age1yubikey1…` recipient depends on the operator's own `PATH`.
On macOS, `safix keygen` writes an identity where standalone sops and clan do not look when `XDG_CONFIG_HOME` is unset, and says nothing about it.

## What Changes

- Every sops child safix starts carries `SOPS_HC_VAULT_ALLOWLIST=none` unless the operator's environment already sets a non-empty value, which is then passed through unchanged.
- A sops child of a run that must not inherit ambient identity material (installation, migration verification) also has `VAULT_TOKEN` and `VAULT_ADDR` removed from its environment.
- The packaged `safix` command appends `age-plugin-yubikey` to the end of its `PATH`, after the operator's own `PATH`, so an operator's plugin still wins and the packaged one is the fallback.
- On macOS, `safix keygen` and `safix keygen --kind age` print an `export SOPS_AGE_KEY_FILE=<file>` line when the file they wrote is not the one standalone sops reads by default and `SOPS_AGE_KEY_FILE` does not already name it.
  Where safix itself looks for identities does not change.
- `docs/reference/environment.md` gains entries for `SOPS_HC_VAULT_ALLOWLIST` and the removed Vault variables naming the advisory, a macOS paragraph under "Which identity decrypts", and a corrected `SAFIX_AGE_PLUGIN_YUBIKEY` row.
- `docs/guides/hardware-keys.md` states that writing to a card's recipient runs the plugin from `PATH`, and that the packaged command carries one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `safix-cli`: adds requirements for the environment every sops child receives, for the card plugin the packaged command carries, and for the macOS identity-file notice `keygen` prints.

## Impact

- Code: `crates/safix-core/src/ciphertext.rs` (`sops_command`), `crates/safix-core/src/sops/mod.rs` (`Sops::command`), `crates/safix-core/src/keygen.rs` (`run`, `epilogue`, a new notice function), `crates/safix/src/main.rs` (the `keygen --kind age` output), `modules/flake/rust.nix` (the wrapper and one new check).
- Tests: a new integration target `crates/safix/tests/sops_environment.rs`, one new test in `crates/safix/tests/install.rs`, unit tests in `crates/safix-core/src/keygen.rs`, and new checks wired in `modules/flake/checks/cli.nix` and `modules/flake/rust.nix`.
- Docs: `docs/reference/environment.md`, `docs/guides/hardware-keys.md`, `docs/reference/cli.md` (the `keygen` section).
- Closure: the packaged command's runtime closure gains `age-plugin-yubikey` (0.5.1 at the pinned nixpkgs, available on every system the flake declares).
- Behaviour visible to operators: a safix-launched sops refuses to contact any Vault server unless the operator set an allowlist; safix supports no Vault recipients, so no supported file changes behaviour.
