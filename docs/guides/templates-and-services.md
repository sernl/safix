---
title: "Templates and services"
---

## What you will have

This page shows you how to hand a secret to a service that wants a configuration file rather than a bare value, and how to get that service restarted when the value changes. At the end a rendered file sits at a path you chose, owned by the account you named, and one unit is notified when its contents move.

## Steps

1. Write the template. The text is public and enters the Nix store, so put placeholders in it and never plaintext:

   ```nix
   { config, ... }:
   {
     safix.templates."grafana.env" = {
       content = ''
         GF_SECURITY_ADMIN_PASSWORD=${config.safix.placeholder.grafana-admin-password}
       '';
       mode = "0400";
       owner = "grafana";
       group = "grafana";
       restartUnits = [ "grafana.service" ];
     };
   }
   ```

   See [`safix.templates`](../reference/profile-options.md#safixtemplates) and [`safix.placeholder`](../reference/profile-options.md#safixplaceholder).

2. Point the service at the rendered file. A template's default path is its name under the installer's symlink path, and an explicit one is kept:

   ```nix
   { config, ... }:
   {
     systemd.services.grafana.serviceConfig.EnvironmentFile = config.safix.templates."grafana.env".path;
   }
   ```

   See [`safix.installer.symlinkPath`](../reference/profile-options.md#safixinstallersymlinkpath).

3. Read a plain entry directly where no file format is needed:

   ```nix
   { config, ... }:
   {
     systemd.services.grafana.serviceConfig.LoadCredential = [
       "admin:${config.safix.secrets.grafana-admin-password.path}"
     ];
   }
   ```

   See [`safix.secrets.<name>.path`](../reference/profile-options.md#safixsecretsnamepath).

4. Declare the units to notify, either on the template as above or on the entry itself:

   ```nix
   {
     flake.safix.users.alice.private.grafana-admin-password.restartUnits = [
       "grafana.service"
     ];
   }
   ```

   A grant carries no fields of its own, so the units live on the owner's own record of the entry and travel with it to every subject the grant reaches.

## When a unit is notified

Rendering happens inside a private runtime generation, and publication swaps the generation symlink. Only then are units acted on.

A unit is notified when the value or the rendered text it depends on differs from what the previous generation held. A value that reads back identically neither restarts nor reloads anything.

A first install notifies nothing. With no previous generation every entry reads as changed, and a machine still coming up has nothing running to restart.

Restart takes precedence over reload for the same unit: a unit named in both lists is restarted once, not both restarted and reloaded.

At system scope the notification reaches the host's activation machinery. At user scope on Linux it is a checked `systemctl --user` hook, run after the new generation is live, and a hook failure is reported without undoing publication. A dry run invokes neither.

## Ownership, paths and modes

`mode` and `path` mean the same thing at both scopes, and nothing in a declaration names a scope.

`owner` and `group` are the system scope's alone. A user-scope profile refuses an entry that sets either, rather than dropping it, because a dropped ownership field reads afterwards as a claim that was honoured.

A user-mode install mounts no filesystem and changes no ownership, so there is no ownership axis to refuse into.

Two entries resolving onto one path are refused at either scope, because whichever activates second unlinks the first's output. A template whose name collides with a secret's name is refused for the same reason.

## Values needed before accounts exist

A password file read while users are being created cannot wait for the ordinary install. Mark the entry:

```nix
{
  flake.safix.users.alice.private.login-hash.neededForUsers = true;
}
```

Those entries use a separate store and symlink, with `-for-users` appended to the configured roots. The entry field `neededForUsers` is described in [Declarations](../reference/declarations.md).

Legacy activation installs them before `users`. Systemd activation orders them before `systemd-sysusers` or `userborn`. Ordinary entries stay after user creation.

Provision the decryption identity before that phase. [`safix.installer.afterActivation`](../reference/profile-options.md#safixinstallerafteractivation) and [`safix.installer.afterUnits`](../reference/profile-options.md#safixinstallerafterunits) order the normal installer and do not postpone the early one past the users it has to precede.

Three things are refused around early entries. Home-manager cannot request one at all. An early entry that is not root-owned, or that carries group or other permission bits, is refused. A template referencing an early secret is refused, because the template renders after the phase that secret exists for.

## What can go wrong

- A placeholder naming an entry this profile does not resolve refuses before anything is promoted, as do a path traversal and two outputs colliding on one path.
- A profile bound to declarations that names neither a person nor a host refuses, naming the option that is unset.
- `safix::manifest_key_missing` — a document the manifest names does not hold a key the manifest declares.
- `safix::manifest_owner_unknown` and `safix::manifest_group_unknown` — the account or group named on an entry does not resolve on this host.
- `safix::manifest_mode_unparsable` — a mode is not an octal string.
- `safix::identity_key_file_unreadable` — the configured identity was absent or unreadable, checked before anything is decrypted.
- `safix::install_decrypt_failed` — the identity is readable and is not a recipient of the document.

Every code is listed in [Refusals](../reference/refusals.md).
