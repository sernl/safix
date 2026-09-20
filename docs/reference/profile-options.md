---
title: "Profile options"
---

## What this page is for

This page lets you look up every option under `safix.*` — what a NixOS
configuration or a home-manager profile declares to consume secrets — with its
type, its default and what it means.

Most options exist at both scopes. Where one exists at a single scope, the entry
says so: **system only** for the NixOS module, **home only** for the
home-manager module. Where a default differs between the two, both are given.

The declarations these profiles bind to are in
[Declaration options](declarations.md). safix reads no option outside its own
namespace — see the namespace rule in [The model](../concepts/model.md).

## Binding

#### `safix.enable`

*Type:* boolean. *Default:* `config.safix.secrets != { } || config.safix.templates != { }`.

Whether to install the resolved secrets and public templates.

#### `safix.flake`

*Type:* null or raw. *Default:* `null`.

The consumer flake publishing `safix.lib`; leave it null for imported-only
deployments, or supply `safix.lib` directly.

#### `safix.lib`

*Type:* null or raw. *Default:* `config.safix.flake.safix.lib`.

The registry resolver projection this profile resolves through.

#### `safix.user`

*Type:* null or string. *Default:* `null` at system scope, `config.home.username`
at home scope.

The registry person selected by this profile, mutually exclusive with
`safix.machine`.

#### `safix.machine`

*Type:* null or string. *Default:* `null`.

The registry machine selected instead of a person; a machine needs no hostname.

#### `safix.hostname`

*Type:* null or string. *Default:* `config.networking.hostName` at system scope;
at home scope the enclosing system's hostname where one is available, and `null`
standalone.

The hostname used for a registry person's host-scoped resolution.

#### `safix.tags`

*Type:* list of strings. *Default:* the selected machine's declared tags, or an
empty list.

The tags used by registry per-tag selection.

## Resolved secrets

#### `safix.secrets`

*Type:* attribute set of resolved entries. *Read-only.*

The resolved registry entries merged once with `safix.importedSecrets`, with
deployment defaults applied. This is what a service reads: pass
`config.safix.secrets.<name>.path` to the unit that needs the value.

#### `safix.importedSecrets`

*Type:* attribute set of entries, taking the same submodule as `safix.secrets`.
*Default:* `{ }`.

Explicit encrypted sources, including migration-produced declarations, merged
with the registry resolution; a colliding entry is refused, and nothing here
declares recipient policy.

### The fields of one entry

Every field below is a field of `safix.secrets.<name>`, where it is a read-only
result of resolution, and of `safix.importedSecrets.<name>`, where you may set
it. See [Templates and services](../guides/templates-and-services.md).

#### `safix.secrets.<name>.name`

*Type:* string. *Default:* the attribute name.

The entry name inside the generation.

#### `safix.secrets.<name>.key`

*Type:* string. *Default:* the entry's own name.

The slash-separated document key; an empty string selects the whole document.

#### `safix.secrets.<name>.path`

*Type:* string. *Default:* the entry name below `safix.installer.symlinkPath`, or
below `symlinkPath-for-users` for early entries.

The installed path; an explicit path is preserved.

#### `safix.secrets.<name>.sopsFile`

*Type:* path. *No default.*

The encrypted source document; ciphertext, unlike private identities, may be
stored in the Nix store.

#### `safix.secrets.<name>.format`

*Type:* one of `"yaml"`, `"json"`, `"dotenv"`, `"ini"`, `"binary"`, `"age"`.
*Default:* `"yaml"`.

The explicit ciphertext format; `age` selects a raw age document.

#### `safix.secrets.<name>.mode`

*Type:* string. *Default:* `"0400"`.

The installed permission bits, as an octal string.

#### `safix.secrets.<name>.owner`

*Type:* null or string. *Default:* `null`.

The owner name, or `null` to use the uid; a user install rejects ownership
overrides.

#### `safix.secrets.<name>.group`

*Type:* null or string. *Default:* `null`.

The group name, or `null` to use the gid; a user install rejects ownership
overrides.

#### `safix.secrets.<name>.uid`

*Type:* integer. *Default:* `0`.

The numeric owner used when `owner` is null.

#### `safix.secrets.<name>.gid`

*Type:* integer. *Default:* `0`.

The numeric group used when `group` is null.

#### `safix.secrets.<name>.restartUnits`

*Type:* list of strings. *Default:* `[ ]`.

The units restarted when the installed value changes; a user installation uses
`systemctl --user`.

#### `safix.secrets.<name>.reloadUnits`

*Type:* list of strings. *Default:* `[ ]`.

The units reloaded when the installed value changes; a user installation uses
`systemctl --user`.

#### `safix.secrets.<name>.neededForUsers`

*Type:* boolean. *Default:* `false`.

Install before user creation, in an independent root-only store; unsupported at
home scope.

## Templates

#### `safix.templates`

*Type:* attribute set of templates. *Default:* `{ }`.

Public templates rendered with secrets only in private runtime state.

#### `safix.templates.<name>.content`

*Type:* lines. *No default.*

The public template text containing `safix.placeholder` tokens; this text enters
the Nix store, so never put plaintext secrets in it — substitution happens only
at runtime.

#### `safix.templates.<name>.path`

*Type:* string. *Default:* the template name below `safix.installer.symlinkPath`.

The installed rendered-template path.

#### `safix.templates.<name>.mode`

*Type:* string. *Default:* `"0400"`.

The installed permission bits, as an octal string.

#### `safix.templates.<name>.owner`

*Type:* null or string. *Default:* `null`.

The owner name, or `null` to use the uid; a user install rejects ownership
overrides.

#### `safix.templates.<name>.group`

*Type:* null or string. *Default:* `null`.

The group name, or `null` to use the gid; a user install rejects ownership
overrides.

#### `safix.templates.<name>.uid`

*Type:* integer. *Default:* `0`.

The numeric owner used when `owner` is null.

#### `safix.templates.<name>.gid`

*Type:* integer. *Default:* `0`.

The numeric group used when `group` is null.

#### `safix.templates.<name>.restartUnits`

*Type:* list of strings. *Default:* `[ ]`.

The units restarted when the rendered template changes; a user installation uses
`systemctl --user`.

#### `safix.templates.<name>.reloadUnits`

*Type:* list of strings. *Default:* `[ ]`.

The units reloaded when the rendered template changes; a user installation uses
`systemctl --user`.

#### `safix.placeholder`

*Type:* attribute set of strings. *Read-only;* a runtime placeholder for every
resolved secret name.

The literal runtime placeholder tokens; nix never substitutes secret plaintext.

## Identity

#### `safix.identity.keyFile`

*Type:* null, or an absolute path outside the Nix store. *Default:* `null`.

The absolute private age identity path.

#### `safix.identity.sshKeyPaths`

*Type:* list of strings. *Default:* `[ ]`.

The absolute private SSH identity paths, outside the Nix store; an unreadable
individual SSH key is skipped.

#### `safix.identity.gnupgHome`

*Type:* null, or an absolute path outside the Nix store. *Default:* `null`.

The private GnuPG home directory; where set, it must be a readable directory at
activation.

#### `safix.identity.deriveHostKeys`

*Type:* boolean. *Default:* `true`. **System only.**

Derive SSH identities from OpenSSH ed25519 host keys where no explicit identity
is configured; keys in either safix store are excluded. See
[Unattended hosts](../guides/unattended-hosts.md).

#### `safix.identity.derivedHostKeys`

*Type:* list of strings. *Read-only and internal.* **System only.**

The derived host identity paths, shared by the manifest, the preflight and the
mount ordering.

#### `safix.identity.generateKey`

*Type:* boolean. *Default:* `false`. **Home only.**

Generate the configured age `keyFile` at activation where it is absent; its
public recipient must still be enrolled separately, and an existing private
identity is never overwritten.

#### `safix.identityPreflight`

*Type:* boolean. *Default:* `true`. **Home only.**

Check identity presence and readability before `checkLinkTargets`; this does not
check decryption and cannot undo an enclosing NixOS switch.

## Scheduled rotation

These four are the workstation timer that re-mints aged values. They are **home
only**, they take effect on Linux alone, and they are independent of
`safix.enable`: a machine that consumes secrets need not rotate any. See
[Rotating secrets](../guides/rotating-secrets.md).

#### `safix.rotation.enable`

*Type:* boolean. *Default:* `false`. **Home only.**

Install a systemd user timer running `safix rotate --due --yes` against
`safix.rotation.repository`. The identity it runs under must decrypt without a
prompt and without a card, since a unit has no terminal and no pinentry. The
timer commits locally and pushes nothing; machines receive the rotated values on
their next rebuild.

#### `safix.rotation.repository`

*Type:* null or string. *Default:* `null`. **Home only.**

The absolute path of the declaring repository the verb runs in, as the unit's
`WorkingDirectory`. A string rather than a path, so the repository is not copied
into the nix store at evaluation. Enabling the timer without one is an assertion
failure.

#### `safix.rotation.onCalendar`

*Type:* string. *Default:* `"daily"`. **Home only.**

When the timer fires, as `OnCalendar` takes it. The unit is persistent, so a
machine that was asleep at the hour catches up on its next boot rather than
skipping the interval.

#### `safix.rotation.environment`

*Type:* attribute set of strings. *Default:* `{ }`. **Home only.**

The environment the rotation unit runs with: the identity the values are
decrypted and re-encrypted under, and anything a generator's own script needs.
It names no secret value, because a value set here would be world-readable in
the nix store.

## Installer

#### `safix.installer.package`

*Type:* package. *Default:* `pkgs.safix`.

The safix package used at activation.

#### `safix.installer.validationPackage`

*Type:* package. *Default:* the installer package, or the build-platform safix
package for cross builds.

The build-platform package used to validate manifests.

#### `safix.installer.validate`

*Type:* boolean. *Default:* `true`.

Validate ciphertext documents and include their input hash; where false, validate
only manifest metadata.

#### `safix.installer.keepGenerations`

*Type:* integer. *Default:* `1`.

The number of generations retained; zero retains all.

#### `safix.installer.log`

*Type:* list of `"keyImport"` or `"secretChanges"`. *Default:* `[ ]`.

The optional installer logging classes; neither logs secret values.

#### `safix.installer.secretsMountPoint`

*Type:* string. *Default:* `"/run/safix.d"` at system scope, `"%r/safix.d"` at
home scope.

The generation root; at home scope the installer expands `%r` to the session
runtime directory.

#### `safix.installer.symlinkPath`

*Type:* string. *Default:* `"/run/safix"` at system scope, `"%r/safix"` at home
scope.

The current-generation symlink, and the default parent of installed paths.

#### `safix.installer.environment`

*Type:* attribute set of strings. *Default:* `{ }`.

The installer environment; safix supplies `HOME`, tool paths and a PATH including
age plugins, and a variable set here wins — see
[Environment](environment.md).

#### `safix.installer.agePlugins`

*Type:* list of packages. *Default:* `[ ]`.

The age plugin packages available to the installer and to crypto subprocesses.

#### `safix.installer.useTmpfs`

*Type:* boolean. *Default:* `false`. **System only.**

Use tmpfs instead of ramfs for both secret stores; the installer requests
`noswap` where that is supported.

#### `safix.installer.useSystemdActivation`

*Type:* boolean. *Default:* true where the host enables `systemd.sysusers` or
`services.userborn`. **System only.**

Install with units instead of legacy activation scripts.

#### `safix.installer.afterActivation`

*Type:* list of strings. *Default:* `[ ]`. **System only.**

Additional legacy activation steps the normal installer waits for; early
identities must be available before user creation.

#### `safix.installer.afterUnits`

*Type:* list of strings. *Default:* `[ ]`. **System only.**

Additional units the normal installer waits for; these are not added to the
pre-user unit, which would create a user-creation ordering cycle.

#### `safix.installer.manifest`

*Type:* package. *Read-only;* the user-mode installer manifest. **Home only.**

The public manifest used by activation and the user unit; secret plaintext is
never rendered in nix.
