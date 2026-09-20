---
title: "Environment variables"
---

## What this page is for

This page lets you look up every environment variable the safix runtime reads,
what it means, and what wins where two of them answer the same question.

Variables whose names begin `SAFIX_TEST_`, and the stub and fixture variables the
repository's own tests set, are not listed: they exist to drive a test double and
have no meaning in a real run.

## Where a run is rooted

#### `SAFIX_REPO_ROOT`

The repository a run stages and commits in, instead of the one git reports for
the current directory. A path here that is not a repository takes the same
refusal a missing repository takes.

#### `SAFIX_ENTRY`

The file to evaluate instead of the repository's flake, exactly as `--entry`
does. The flag wins where both are given.

#### `SAFIX_NIXPKGS`

The flake reference a generator's sandbox resolves its tools against, exactly as
`--nixpkgs` does. The flag wins where both are given. Under `--entry` with
something to mint, a run with neither this nor the flag refuses before the
sandbox is probed.

#### `SAFIX_VAULT_ROOT`

The vault repository's own working tree — a mutable path, and a different one
from the locked, store-copied path the declaration resolves at evaluation. It is
unset and unread when no vault is declared, and required whenever one is; a
declared vault with this unset, and this set with no vault declared, are each
their own refusal. See [The vault](../guides/vault.md).

## Where plaintext is staged

#### `SAFIX_STAGING_DIR`

The mount to stage plaintext on. It replaces the conventional candidates —
`/dev/shm`, then the per-user runtime directory — rather than being tried ahead
of them, so a mount you named and safix rejected is a refusal rather than a
silent fall back to somewhere else. What it names is verified exactly as a
conventional candidate is, so pointing it at a disk-backed filesystem earns the
refusal that `--allow-disk-staging` answers.

## Which identity decrypts

Discovery reads three sources in order and takes the first that answers.

For the age identity file: `SOPS_AGE_KEY_FILE`, then `SAFIX_AGE_KEY_FILE`, then
`$XDG_CONFIG_HOME/sops/age/keys.txt` — falling back to `$HOME/.config` where
`XDG_CONFIG_HOME` is unset — used only where that file exists.

For the GnuPG keyring: `GNUPGHOME`, then `SAFIX_GNUPGHOME`, then
`$XDG_DATA_HOME/safix/gnupg` — falling back to `$HOME/.local/share` — used only
where that directory exists.

#### `SAFIX_AGE_KEY_FILE`

The managed age identity file. It is the second source for discovery, behind
`SOPS_AGE_KEY_FILE`, and the first one for the file `safix keygen` appends to.

#### `SAFIX_AGE_SSH_KEY_PATHS`

Private SSH keys native age accepts directly, as a list in the platform's path
separator form.

#### `SAFIX_GNUPGHOME`

The managed GnuPG keyring. It is the second source for discovery, behind
`GNUPGHOME`, and the first one for the keyring `safix keygen --kind pgp`
protects and writes into.

## Which program is run

Each variable below names an executable to run instead of the one PATH would
find. An empty value counts as unset. The nix half of safix sets these for the
installer, and leaves any value the surrounding environment already carries
alone.

| Variable | The program |
| --- | --- |
| `SAFIX_SOPS` | `sops` |
| `SAFIX_AGE` | `age` |
| `SAFIX_AGE_KEYGEN` | `age-keygen` |
| `SAFIX_AGE_PLUGIN_YUBIKEY` | the age plugin that generates a card identity |
| `SAFIX_SSH_TO_AGE` | `ssh-to-age` |
| `SAFIX_GPG` | `gpg` |
| `SAFIX_GPGCONF` | `gpgconf` |
| `SAFIX_GIT` | `git` |
| `SAFIX_NIX` | `nix` |
| `SAFIX_CLAN` | `clan` |
| `SAFIX_KEEPASSXC_CLI` | `keepassxc-cli` |
| `SAFIX_SECRET_TOOL` | the session secret service's own client |
| `SAFIX_PASS` | `pass` |
| `SAFIX_BW` | the Bitwarden client |
| `SAFIX_OP` | the 1Password client |
| `SAFIX_YKMAN` | `ykman` |
| `SAFIX_SSH` | `ssh` |
| `SAFIX_SSH_KEYGEN` | `ssh-keygen` |
| `SAFIX_SSH_KEYSCAN` | `ssh-keyscan` |
| `SAFIX_TAR` | `tar` |
| `SAFIX_SYSTEMCTL` | `systemctl` |

## How a run behaves

#### `SAFIX_ERROR_FORMAT`

Set to `plain` to render a refusal in the retired shell runtime's shape, rather
than the command's own. Scripts should branch on the refusal code instead — see
[Refusal codes](refusals.md).

#### `SAFIX_FIX_CONCURRENCY`

How many governed files `safix fix` re-wraps at once. Four, where the variable
says nothing.

## Upstream variables safix honours

#### `SOPS_AGE_KEY_FILE`

Read as the first source of the age identity file. Where an operation must not
inherit ambient identity material, safix removes it from the child's environment
and points it at `/dev/null` instead, so that a verification cannot be satisfied
by a key it was not given.

#### `SOPS_AGE_KEY`

Inherited by the encryption tool for operations that inherit the ambient
environment, and removed for the ones that must not. safix never sets it.

#### `SOPS_AGE_KEY_CMD`

Inherited and removed on exactly the same terms as `SOPS_AGE_KEY`.

#### `SOPS_AGE_SSH_PRIVATE_KEY_FILE`

Inherited and removed on the same terms, and additionally set by safix for the
child that must use one named SSH identity.

#### `GNUPGHOME`

Read as the first source of the GnuPG keyring. Where an operation must not
inherit ambient identity material, safix replaces it with an empty directory in
the child's environment.

#### `SOPS_GPG_EXEC`

The GnuPG binary the encryption tool runs. safix sets it from `SAFIX_GPG` only
where it is unset or empty, so a value an operator chose is left alone.

#### `VISUAL` and `EDITOR`

The editor `safix edit` opens, `VISUAL` first. Neither set is a refusal naming
both: the verb opens no editor of its own choosing. The value is split on
whitespace and run directly rather than through a shell, so `code --wait` works.

#### `PASSWORD_STORE_DIR`

The `pass` store root, which reaches the child process through this variable
alone — a location rather than a value. safix sets it from the declared store,
expanding a leading `~` itself.

#### `BW_SESSION`

The Bitwarden session key. safix places the key the unlock returned here for the
child invocations that need it, and in no argument vector; a session an operator
already established reaches the same client the same way.

#### `OP_SERVICE_ACCOUNT_TOKEN`

A 1Password service-account token. safix signs nothing in and holds no session:
whatever authentication material the surrounding environment carries is what the
child inherits, and this variable is named in the refusal a signed-out session
earns.

#### `NIXOS_ACTION`

`dry-activate` makes `safix install` behave as though `--dry-run` were passed.

#### `XDG_CONFIG_HOME`

The root the default age identity file is looked for under, falling back to
`$HOME/.config`.

#### `XDG_DATA_HOME`

The root the managed GnuPG keyring is looked for under, falling back to
`$HOME/.local/share`.

#### `XDG_STATE_HOME`

Where the picker remembers its panes and your last choice, in
`safix/picker.json`, created 0600. It is used only where it is an absolute path,
and falls back to `$HOME/.local/state`; neither available is no file, and the
picker then behaves as it does on a first run.

#### `XDG_RUNTIME_DIR`

The second staging candidate, after `/dev/shm`, and the directory a user-scope
manifest's `%r` expands to. A user-scope install refuses where it names no
runtime directory.

#### `HOME`

The fallback root for every path above that has one. A verb that needs it and
cannot read it refuses rather than guessing.
