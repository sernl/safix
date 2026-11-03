# Design

## Context

See `proposal.md` for motivation and `specs/safix-cli/spec.md` for the requirements.

safix builds sops children in two places.
`crates/safix-core/src/ciphertext.rs:241-274` (`sops_command`) serves `read`, `set_value` and `write`, and is the only builder with an isolated branch (`ciphertext.rs:244-261`): when `Identities::inherit_environment` is false it removes the identity variables, points `HOME` and `XDG_CONFIG_HOME` at `/var/empty` and `SOPS_AGE_KEY_FILE` at `/dev/null`.
The isolated identities come from `install.rs:1167` and `migrate.rs:386`.
`crates/safix-core/src/sops/mod.rs:318-334` (`Sops::command`) serves `decrypt_key`, `decrypt_document_json`, `create_empty_document` and `update_keys_command`, and always inherits.
Both builders already set `SOPS_GPG_EXEC` from `SAFIX_GPG` only where the operator left it unset or empty (`ciphertext.rs:268-272`, `mod.rs:321-325`).

sops 3.13.3 names `SOPS_HC_VAULT_ALLOWLIST` in `hcvault/keysource.go:31` with default `all` (`:37`), and checks it in `vaultClient` (`:397-407`) before the Vault client reads `VAULT_TOKEN` or falls back to `$HOME/.vault-token` (`userVaultToken`, `:437`).
The isolated branch already makes `$HOME/.vault-token` unreachable through `HOME=/var/empty`, but does not remove `VAULT_TOKEN` or `VAULT_ADDR`.
safix supports no `hc_vault` recipients: `crates/safix-core/src/sops/document.rs:151-157` reports them as `<unsupported sops hc_vault recipients>`.

The packaged command's wrapper is `modules/flake/rust.nix:81-100`.
It prefixes `PATH` with age, sops, gnupg, openssh, ssh-to-age, git and nix, and `--set-default`s `SAFIX_AGE`, `SAFIX_AGE_KEYGEN`, `SAFIX_SOPS`, `SAFIX_GPG`, `SAFIX_GPGCONF` and `SAFIX_SSH_TO_AGE`; git and nix are on `PATH` without their `SAFIX_GIT` and `SAFIX_NIX` defaults.
`enroll` finds the plugin through `SAFIX_AGE_PLUGIN_YUBIKEY`, else `age-plugin-yubikey` from `PATH` (`crates/safix-core/src/enroll/identity.rs:48-51,230-237`), and uses it only to generate the card identity.
sops wraps to an `age1yubikey1…` recipient through `plugin.NewRecipient`, which runs `age-plugin-yubikey` from `PATH` (`age/keysource.go:498-503` at 3.13.3).
At the pinned nixpkgs, `age-plugin-yubikey` is 0.5.1 and `meta.available` is true on `aarch64-darwin` as well as both Linux systems.

On macOS, sops 3.13.3 resolves its default identity directory with `getUserConfigDir` (`age/keysource.go:393-400`): a non-empty `XDG_CONFIG_HOME`, else `os.UserConfigDir()`, which is `$HOME/Library/Application Support`.
safix's discovery (`ciphertext.rs:91-102`), its custody path (`identity.rs:112-119`) and `keygen::identity_file` (`keygen.rs:138-150`) use `XDG_CONFIG_HOME`, else `$HOME/.config`, on every platform.
safix passes `SOPS_AGE_KEY_FILE` to every sops child it starts (`ciphertext.rs:265-267`, `mod.rs:330-332`), so this is a mismatch with standalone sops and clan, not a safix decryption failure.
`keygen` has two output sites: `keygen::run` prints `epilogue` (`keygen.rs:90`, `:183-205`) and `keygen --kind age` prints its own lines (`crates/safix/src/main.rs:916-917`).

## Goals / Non-Goals

**Goals:**

- A safix-launched sops cannot send Vault credentials to an address a committed file names, unless the operator opted in.
- A write to an enrolled card's recipient works from the packaged command on a machine with no plugin installed.
- A macOS operator who also runs standalone sops or clan is told the one line that makes them find the identity `keygen` wrote.

**Non-Goals:**

- Changing where safix looks for or writes identities on any platform.
- Supporting Vault recipients, or plugins other than `age-plugin-yubikey` on the CLI path.
- Following sops's announced flip of the allowlist default; safix sets the value it needs regardless of the default.
- FIDO2 hmac-secret keys: `safix enroll` does not handle them (owner decision D4), and the hardware-keys guide, which this change edits, says so.

## Decisions

### Set the allowlist only where the operator has not

Both builders set `SOPS_HC_VAULT_ALLOWLIST=none` when the variable is unset or empty, with the same test `SOPS_GPG_EXEC` uses, and pass a non-empty operator value through untouched.
An operator who decrypts Vault-backed files with sops outside safix has stated a value deliberately; overriding it would break nothing safix supports but would contradict the precedence the environment reference already promises for upstream variables.
Alternative considered: set `none` unconditionally.
Rejected because the precedence rule would then have an exception, and safix supports no Vault recipients for which an operator value could matter.
The test for "unset" moves into one helper both builders call, so the two builders cannot drift; `ciphertext.rs` already has `nonempty_environment` (`:149-151`) and `mod.rs` repeats the test inline.

### Remove `VAULT_TOKEN` and `VAULT_ADDR` in the isolated branch

The isolated branch exists so that a verification is satisfied only by material the run was given.
Vault credentials are ambient material of the same kind, and `HOME=/var/empty` already removes the token file; removing the two variables closes the remaining path.
The two names join the existing `env_remove` list at `ciphertext.rs:245-253`.
The operator's allowlist value is still honoured in the isolated branch: with no token and no address, a permissive value has nothing to send.
Alternative considered: also remove them from inherited runs.
Rejected because the allowlist already refuses the connection before the token is read, and an operator who set a permissive allowlist chose to send it.

### Append the plugin after the operator's `PATH`, with no `SAFIX_AGE_PLUGIN_YUBIKEY` default

The wrapper gains `--suffix PATH : ${lib.makeBinPath [ pkgs.age-plugin-yubikey ]}`, after the existing `--prefix PATH` block, as agenix-rekey does (`nix/lib.nix:48`).
An operator's own plugin, for example a newer one supporting newer card firmware, is found first; the packaged one is the fallback.
The wrapper does not `--set-default SAFIX_AGE_PLUGIN_YUBIKEY`.
With the plugin appended rather than prefixed, a default would let `enroll` generate with the packaged plugin while sops, which reads only `PATH`, wraps with the operator's; without a default, `enroll`'s fallback lookup and sops resolve the same binary.
This follows the existing precedent of git and nix, which the wrapper puts on `PATH` without their `SAFIX_*` defaults.
`SAFIX_AGE_PLUGIN_YUBIKEY` keeps its meaning as a generation-only override, and the environment reference says so.
The plugin is wrapped in `lib.optionals (lib.meta.availableOn pkgs.stdenv.hostPlatform pkgs.age-plugin-yubikey)`, so a future nixpkgs that drops a platform does not break evaluation of the package there; at the current pin the guard admits every declared system.
Alternative considered: append to the `--prefix` list.
Rejected because the survey's recommendation and agenix-rekey both place plugins after the operator's `PATH`, and a prefixed plugin could not be replaced by the operator.

### Print the export line on macOS, decided by a pure function

`keygen.rs` gains a function that takes the platform, the file written and the values of `XDG_CONFIG_HOME`, `HOME` and `SOPS_AGE_KEY_FILE`, and returns the notice or nothing.
It returns nothing off macOS, nothing when `SOPS_AGE_KEY_FILE` already names the file written, and nothing when the file written equals standalone sops's default (`$XDG_CONFIG_HOME/sops/age/keys.txt` for a non-empty `XDG_CONFIG_HOME`, else `$HOME/Library/Application Support/sops/age/keys.txt`).
Otherwise it returns a line pointing out that standalone sops and clan on macOS read a different default, followed by `export SOPS_AGE_KEY_FILE=<file>`, with the path single-quoted for a POSIX shell.
`run` and `main.rs`'s `--kind age` arm call it with `cfg!(target_os = "macos")` and the process environment, and append its output.
Taking the platform as an argument lets Linux CI exercise the macOS branch as a unit test.
Alternative considered: change `identity_file` to follow sops's macOS default.
Rejected by the scope: it would move existing identities' expected location and diverge `keygen` from discovery.
Alternative considered: a docs paragraph only.
Rejected because the operator who needs it is the one reading `keygen`'s output, not the reference.

## Risks / Trade-offs

- [An operator with `SOPS_HC_VAULT_ALLOWLIST=all` exported for other work keeps the exposure inside safix] → The environment reference names the advisory and states that safix sets `none` only where the variable is unset.
- [The packaged closure grows by the plugin and its PC/SC dependency] → Accepted; agenix-rekey ships the same plugin by default, and the plugin is what makes an enrolled card usable from the packaged command.
- [An operator sets `SAFIX_AGE_PLUGIN_YUBIKEY` to a plugin not on `PATH`; enrollment succeeds and later writes fail in sops] → The environment reference states the variable governs generation only and that writes find the plugin on `PATH`.
- [The macOS branch is never executed in CI] → The decision is a pure function under unit test for every branch; only the `cfg!` call site is platform-specific.

## Migration Plan

No data migrates.
A rollback is a revert of the change; no stored artifact depends on it.
