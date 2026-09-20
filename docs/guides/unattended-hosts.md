---
title: "Unattended hosts"
---

## What you will have

This page shows you how to give a host an identity of its own, so it decrypts at boot with nobody logged in. At the end, a machine is declared as a recipient, a service on it reads one entry, and the machine's private half is on its disk before its first activation.

## Why a host needs its own identity

A person's identity is protected by something a person supplies: a passphrase, a card, a touch. A host that reboots at four in the morning supplies none of those.

So custody splits. People keep passphrase-protected or hardware-backed keys, and each unattended host holds a key of its own that no person types a secret into.

An unlocked `gpg-agent` is session state. It is gone after a reboot, and the machine's first activation after a power cut then has no way to decrypt. A GnuPG identity is usable at system scope only when it is unlockable without a person present — see [`safix.identity.gnupgHome`](../reference/profile-options.md#safixidentitygnupghome).

## Steps

1. Take the age form of the host's own ed25519 key. On a machine that already runs, that is its public host key:

   ```console
   $ ssh-to-age < /etc/ssh/ssh_host_ed25519_key.pub
   age1...
   ```

2. Declare the machine, and declare the service that runs on it:

   ```nix
   {
     flake.safix.machines.web = {
       recipient = "age1...";
       recipientNote = "web — the age form of its ed25519 host key";
       owner = "alice";
     };

     flake.safix.services.grafana = {
       machines = [ "web" ];
       owner = "alice";
       user = "grafana";
       group = "grafana";
     };
   }
   ```

   See [`flake.safix.machines.<name>.recipient`](../reference/declarations.md#flakesafixmachinesnamerecipient) and [`flake.safix.services.<name>.machines`](../reference/declarations.md#flakesafixservicesnamemachines).

3. Grant the host only what it needs. A grant to the service puts the service in the audience and lands the file under the service's account:

   ```nix
   {
     flake.safix.users.alice.sharedWith.grafana.grafana-admin-password = { };
   }
   ```

   Grant to the machine itself only for a value no single service owns, with [`flake.safix.users.<name>.sharedWith`](../reference/declarations.md#flakesafixusersnamesharedwith) naming `web` in place of `grafana`.

4. Wire the profile. The system scope derives its identity from the host's own ssh keys by default:

   ```nix
   {
     imports = [ inputs.safix.nixosModules.default ];

     safix.flake = inputs.self;
     safix.machine = "web";
   }
   ```

   See [`safix.identity.deriveHostKeys`](../reference/profile-options.md#safixidentityderivehostkeys) for what that derivation picks, and [`safix.identity.derivedHostKeys`](../reference/profile-options.md#safixidentityderivedhostkeys) for reading back what it chose.

5. Use a dedicated age key file instead where the host's ssh keys are unsuitable, or where you want the secrets identity to be rotatable apart from the ssh one:

   ```nix
   {
     safix.identity.deriveHostKeys = false;
     safix.identity.keyFile = "/var/lib/safix/identity.txt";
   }
   ```

   The recipient you declare in step 2 is then the age recipient of that file rather than of the ssh key.

6. Put the private half on the machine before its first activation. Declaring a recipient mints nothing and copies nothing:

   ```console
   $ safix upload web --directory ./preseed --identity /secure/hosts/web_ed25519
   $ safix upload web --to root@web.example --identity /secure/hosts/web_ed25519
   ```

   `--directory` writes a plain tree for `nixos-anywhere --extra-files` or for installer media, and makes no connection. `--to` reads the key the address currently presents before it writes anything. Neither form triggers a deploy: the machine's own next rebuild activates what was written. See [the verb table](../reference/cli.md).

## Scope the grant narrowly

A host key is unattended by construction, so it is the key an attacker with the disk gets. What that key opens is exactly what you granted the machine and the services on it.

Grant per service rather than per machine where a service exists to hold the value. The audience then names the service, the landed file belongs to the service's account, and a second service on the same host is not in that audience.

Removing a grant afterwards narrows future ciphertext and takes nothing back — see the revocation rule in [Custody](../concepts/custody.md). [After removing access](after-removing-access.md) is the procedure.

## What can go wrong

- `safix::unknown_machine` — the name you passed to `upload` is not declared. A person's name is refused here the same way.
- `safix::machine_has_no_recipient` — the machine is declared with no recipient, so nothing wraps to it.
- `safix::upload_needs_identity` — `--to` found no key presented and has no `--identity` to write.
- `safix::supplied_identity_mismatch` — the private key you passed derives a recipient the declaration does not carry. Nothing is written.
- `safix::presented_identity_mismatch` — the address already presents a different ed25519 key. `--force` together with `--identity` proceeds; both recipients are named first.
- `safix::identity_key_file_unreadable` — the installer found the configured identity path absent or unreadable, and refused before decrypting.

Every code is listed in [Refusals](../reference/refusals.md).
