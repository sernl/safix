---
title: "Your first secret"
---

## What you will have at the end

This lesson takes you from a plain NixOS flake to a running service reading a
decrypted value, and then to the same value arriving in a person's
home-manager profile.

You will declare one person, `alice`, one host, `web`, and one entry,
`grafana-admin-password`. No flake-parts is involved at any point.

You need a flake with at least one `nixosConfigurations` output, a git
repository around it, and a terminal on a machine where you can run
`nixos-rebuild`. Every `safix` command below runs inside that repository.

## Step 1: add the input

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    safix.url = "github:you/safix";
  };

  outputs =
    { nixpkgs, ... }@inputs:
    {
      nixosConfigurations.web = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs; };
        modules = [ ./hosts/web.nix ];
      };
    };
}
```

`specialArgs` is how `inputs` reaches the host module. safix needs no other
plumbing in the flake: it declares no flake output of yours and reads none.

```console
$ nix flake lock
```

The lock file gains one entry for safix, and nothing else changes.

## Step 2: declare the entry

Declarations live in a module of their own. This one file is everything safix
knows about your fleet.

```nix
# secrets.nix
{
  flake.safix.users.alice = {
    recipient = "age1...";
    recipientNote = "alice — her workstation's software identity";

    private."grafana-admin-password" = { };
    sharedWith.web."grafana-admin-password" = { };
  };

  flake.safix.machines.web = {
    recipient = "age1...";
    recipientNote = "web — the host running grafana";
    owner = "alice";
  };
}
```

`private."grafana-admin-password"` says the entry exists and that alice holds
it. `sharedWith.web` hands that one name to the machine, so the file is
encrypted to alice and to `web` and to nobody else.

Both `recipient` values are placeholders for now. The next step prints the
first, and the second is the age form of the host's own SSH identity.

## Step 3: mint alice's identity

```console
$ safix keygen
```

It appends a new age identity to your own key file, then prints the public
recipient and what to do with it; the private half is never printed.

Paste that recipient into `flake.safix.users.alice.recipient`.

```console
$ ssh-to-age < /etc/ssh/ssh_host_ed25519_key.pub
```

One line: the age recipient corresponding to the host's ed25519 SSH host key.
Paste it into `flake.safix.machines.web.recipient`.

Minting an identity is not enrolling it. Until the public half is in a
declaration, nothing is encrypted to it — see
[Custody](../concepts/custody.md).

## Step 4: bind the declarations in the host

```nix
# hosts/web.nix
{ inputs, ... }:
{
  imports = [ inputs.safix.nixosModules.safix ];

  networking.hostName = "web";
  services.openssh.enable = true;

  safix = {
    lib = inputs.safix.lib.mkVault {
      modules = [ ../secrets.nix ];
      root = ../.;
    };

    machine = "web";
    identity.deriveHostKeys = true;
  };
}
```

`mkVault` takes `modules` and `root`. It evaluates those modules together with
safix's own resolver, hands `root` to the modules as `self`, and returns the
resolver projection — the same value a flake-parts consumer reads at
`flake.safix.lib`. Declarations passed through `modules` scatter and merge the
way an `imports` list would.

[`safix.machine`](../reference/profile-options.md#safixmachine) selects the
declared machine this configuration serves. A machine needs no hostname,
because it is the host.

[`safix.identity.deriveHostKeys`](../reference/profile-options.md#safixidentityderivehostkeys)
is on by default. It derives the identity from the ed25519 entries of
`services.openssh.hostKeys` when no identity is named explicitly, and it needs
`services.openssh.enable` for those keys to exist. Keys inside either safix
store are excluded.

Nothing outside the `safix.*` namespace is set for safix's benefit here — see
the namespace rule in [The model](../concepts/model.md).

## Step 5: write the recipient policy

```console
$ safix fix
```

It regenerates `.sops.yaml` from the declarations and re-wraps each governed
file's data key to the audience that policy declares, printing what it touched
and committing nothing.

Read the diff, then commit it. The encryption tool reads the committed
`.sops.yaml` off disk, and that version decides what a new file is encrypted to.

## Step 6: write the value

```console
$ safix set alice grafana-admin-password
```

It prompts twice without echoing, writes the value into the file the
declarations place that name in, then stages and commits that file alone.

The file it wrote is `secrets/safix/shared/alice,web/secrets.yaml`, under the
key `grafana-admin-password`. You chose neither: the audience picked the
directory and the name picked the key — see
[Storage layout](../concepts/storage-layout.md).

```console
$ safix list alice
```

One row per name alice holds: where it came from, whether it has a generator,
and the key it is read under. Three more columns follow: when the value was
created and last updated, the file serving it, and how long it has before its
rotation deadline. The last of those reads `-`, because nothing yet says this
value expires.

Say how long it may live. A policy is an interval named once, and an entry opts
in by naming it:

```nix
# secrets.nix
{
  flake.safix.rotation.quarterly.every = "90d";

  flake.safix.users.alice.private."grafana-admin-password".rotation = "quarterly";
}
```

`safix list alice` now counts the value down from ninety days, and once that
passes, `safix check` reports it and names `safix set` as the remedy — nothing
can mint a password a person chose. [Rotating secrets](../guides/rotating-secrets.md)
covers generated values, the `rotate` verb and the timer that runs it.

```console
$ safix check
```

Silence and exit status zero. Anything else is drift, and each finding prints
the command that resolves it.

## Step 7: rebuild, and read the path in a service

```console
$ sudo nixos-rebuild switch --flake .#web
```

The switch builds safix's manifest, decrypts under the host identity, and
publishes the value at `/run/safix/grafana-admin-password`.

`config.safix.secrets.<name>` is that resolution, read back from the same value
the manifest was built from.

```nix
# hosts/web.nix
{ config, ... }:
{
  services.grafana.enable = true;
  services.grafana.settings.security.admin_password = "$__file{%d/admin-password}";

  systemd.services.grafana.serviceConfig.LoadCredential = [
    "admin-password:${config.safix.secrets."grafana-admin-password".path}"
  ];
}
```

The entry lands root-owned with mode `0400`, so systemd reads it before
dropping privileges and hands grafana a credential. Adding
`restartUnits = [ "grafana.service" ]` to the entry's declaration makes the
installer restart the unit whenever the value changes — see
[Templates and services](../guides/templates-and-services.md).

Rebuild once more, and the service starts with the password you typed.

## Step 8: the same value in alice's home profile

A person's custody does not depend on the host, so the same declaration serves
alice's own profile. Add home-manager to the flake, then give alice a profile.

```nix
# hosts/web.nix
{ inputs, ... }:
{
  imports = [ inputs.home-manager.nixosModules.home-manager ];

  home-manager.extraSpecialArgs = { inherit inputs; };
  home-manager.users.alice = ./home/alice.nix;
}
```

```nix
# home/alice.nix
{ inputs, ... }:
{
  imports = [ inputs.safix.homeModules.safix ];

  safix = {
    lib = inputs.safix.lib.mkVault {
      modules = [ ../secrets.nix ];
      root = ../.;
    };

    user = "alice";
    identity.keyFile = "/home/alice/.config/sops/age/keys.txt";
  };
}
```

[`safix.user`](../reference/profile-options.md#safixuser) selects the declared
person, and defaults to the profile's own username.
[`safix.identity.keyFile`](../reference/profile-options.md#safixidentitykeyfile)
is the file `safix keygen` appended to, as a string rather than a nix path: a
nix path would copy the identity into the world-readable store.

The hostname comes from the enclosing system configuration. A standalone
home-manager profile has none to read, so it sets
[`safix.hostname`](../reference/profile-options.md#safixhostname) itself.

```console
$ sudo nixos-rebuild switch --flake .#web
```

Alice's home activation refuses before it links anything when her key file is
absent or unreadable, and otherwise publishes the value under her own runtime
directory.

## Where to go next

For an unattended host with no person on it, read
[Unattended hosts](../guides/unattended-hosts.md).

For a value that mints itself rather than being typed, read
[Generators](../guides/generators.md).

For what happens when alice leaves, read
[After removing access](../guides/after-removing-access.md).

For putting the fleet's values on a schedule, read
[Rotating secrets](../guides/rotating-secrets.md).

For the flake-parts route, which is fully supported and shorter than this one,
read [flake-parts](../guides/flake-parts.md).
