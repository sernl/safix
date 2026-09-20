# safix

safix is a custody-first secrets manager for nix.
You declare who holds a secret, and safix decides which encrypted file it lives in, which recipients that file is wrapped to, and what the recipient policy in `.sops.yaml` says.
It serves NixOS and home-manager through consumption modules of its own, and installs what they resolve with an installer of its own.

This page is one worked example, start to finish: a person called alice, a host called `web`, and the grafana running on it.
Every nix block below is a region of [`examples/quickstart/`](examples/quickstart), a flake this repository evaluates, so a block you copy is a block that builds.

## Install it

Add the input.

```nix title="examples/quickstart/flake.nix#inputs"
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    safix.url = "path:../..";
  };
```

Outside this repository, `safix.url = "github:sernl/safix"` is the input.
home-manager is optional and is here because the last step of this page uses it.

Two modules arrive with the input.
`nixosModules.safix` is imported by a system configuration, `homeModules.safix` by a home-manager profile, and each is also published as `.default`.
Both appear in their own step below.

## Declare what exists

Declarations are a module of their own.
This one file is everything safix knows about the fleet.

```nix title="examples/quickstart/secrets.nix#declare"
  flake.safix.users.alice = {
    recipient = "age1exampleaaa00000000000000000000000000000000000000000000000";
    recipientNote = "alice — example identity, decrypts nothing";

    # A value a person types. Declaring it says alice holds it and nobody else
    # does; the grant below hands the same name to the machine, so the file is
    # encrypted to alice and to web and to no one further.
    private."grafana-admin-password".restartUnits = [ "grafana.service" ];

    sharedWith.web."grafana-admin-password" = { };
  };

  flake.safix.machines.web = {
    recipient = "age1exampleweb00000000000000000000000000000000000000000000000";
    recipientNote = "web — the age form of a host identity that does not exist";
    owner = "alice";
  };
```

A recipient is a public key: native age, an SSH public key, or `pgp:` and a GnuPG fingerprint.
Both values above are placeholders, and the next step prints the real ones.

An entry names no file and no directory.
The audience does: alice and `web` can read this value, so it lands in the file that exactly those two can open.
You never choose the path, which is why moving a grant moves a file and nothing else.

Every field an entry may carry is listed in [Declarations](docs/reference/declarations.md), and the subjects that may hold one — people, machines, services, groups, organizations — in [Custody](docs/concepts/custody.md).

## Bind the declarations

```nix title="examples/quickstart/flake.nix#outputs"
  outputs =
    inputs@{ nixpkgs, home-manager, ... }:
    {
      # What the command reads: every verb evaluates an attribute of
      # `safix.lib`, so this output is what binds the declarations to the tree.
      safix.lib = inputs.safix.lib.mkVault {
        modules = [ ./secrets.nix ];
        root = ./.;
      };

      nixosConfigurations.web = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs; };
        modules = [ ./hosts/web.nix ];
      };

      homeConfigurations.alice = home-manager.lib.homeManagerConfiguration {
        pkgs = nixpkgs.legacyPackages.x86_64-linux;
        extraSpecialArgs = { inherit inputs; };
        modules = [ ./home/alice.nix ];
      };
    };
```

`mkVault` evaluates the modules you hand it together with safix's own resolver and returns the projection.
`modules` merges the way a flake's own module list does, so declarations may scatter across as many files as you like.
`specialArgs` is how `inputs` reaches the two profiles; safix asks for no other plumbing.

## The first commands

```console
$ safix keygen
```

It appends a new age identity to your own key file and prints the public half; the private half is never printed.
Paste the printed recipient into `flake.safix.users.alice.recipient`.
Run `ssh-to-age < /etc/ssh/ssh_host_ed25519_key.pub` on the host for the machine's, and paste that into `flake.safix.machines.web.recipient`.

```console
$ safix fix
$ safix set alice grafana-admin-password
$ safix list alice
```

`fix` writes `.sops.yaml` from the declarations and re-wraps each governed file to the audience that policy names.
`set` prompts twice without echoing, then writes and commits that one file.
`list` prints a row per name alice holds, with where it came from, whether a generator mints it, when it was last written and how long it has left.

```console
$ safix view alice
```

With no name, `view` offers the same rows for selection and decrypts the one under the cursor into a preview pane.
Enter reads it and escape leaves.
Up and down move the cursor, left and right move a caret through what you typed, and `Ctrl` with left or right scrolls the columns.
Tab shows the remaining columns, `^P` hides the preview, and typing narrows the list with the letters that matched lit inside their cells.
Everything a verb does is in [CLI](docs/reference/cli.md).

## The host

```nix title="examples/quickstart/hosts/web.nix#bind"
  imports = [ inputs.safix.nixosModules.safix ];

  safix.flake = inputs.self;
  safix.machine = "web";
  safix.identity.deriveHostKeys = true;
```

[`safix.flake`](docs/reference/profile-options.md#safixflake) reaches the `safix.lib` output the previous step wrote; [`safix.lib`](docs/reference/profile-options.md#safixlib) takes the projection directly where your flake hands it over some other way.
[`safix.machine`](docs/reference/profile-options.md#safixmachine) says which declared machine this configuration is.
[`safix.identity.deriveHostKeys`](docs/reference/profile-options.md#safixidentityderivehostkeys) takes the identity from the host's own ed25519 SSH keys, so naming a machine mints no second key and adds no enrolment step.

```nix title="examples/quickstart/hosts/web.nix#service"
  services.grafana.enable = true;

  systemd.services.grafana.serviceConfig.LoadCredential = [
    "admin-password:${config.safix.secrets."grafana-admin-password".path}"
  ];
```

`config.safix.secrets.<name>.path` is where the decrypted value lands.
It is read back from the same resolution the installer's manifest was built from, so what your module reads and what arrives cannot disagree.

Nothing outside the `safix.*` namespace is set here for safix's benefit — see the namespace rule in [The model](docs/concepts/model.md).

## Rebuild, and what happens on the machine

```console
$ sudo nixos-rebuild switch --flake .#web
```

The switch builds safix's manifest, opens the files with the host's own key, and publishes each value into a private runtime generation.
The entry lands root-owned at mode `0400`, so systemd reads it before dropping privileges and hands grafana a credential.

`restartUnits = [ "grafana.service" ]` on the entry is why the unit restarts when the value changes and not on every rebuild.
`reloadUnits` is the same hook with a reload, and restart wins where a unit is named by both — see [Templates and services](docs/guides/templates-and-services.md).

## A value that mints itself

A password someone else already knows can only be transcribed.
A key nobody has yet should be minted, and a generator is how a declaration says so.

```nix title="examples/quickstart/secrets.nix#generator"
  flake.safix.users.alice.private."grafana-secret-key" = {
    generator = {
      script = ''openssl rand -hex 32 > "$out/grafana-secret-key"'';
      runtimeInputs = [ "openssl" ];
      validation = "grep -Eq '^[0-9a-f]{64}$' ";
    };

    rotation = "quarterly";
    restartUnits = [ "grafana.service" ];
  };

  flake.safix.users.alice.sharedWith.web."grafana-secret-key" = { };
```

```console
$ safix generate alice
```

The script runs in a sandbox whose only writable path is the staging root, with no network and with `runtimeInputs` as the whole of its `PATH`.
`validation` judges the candidate before anything is written, and a non-zero exit refuses the run while the value is still only in memory.
Prompts, dependencies between generators, and outputs a nix module may read in the clear are in [Generators](docs/guides/generators.md).

## A deadline on a value

`rotation = "quarterly"` above names a policy, and the policy declares the interval once.

```nix title="examples/quickstart/secrets.nix#policy"
  flake.safix.rotation.quarterly.every = "90d";
```

An entry naming that policy is due an interval after its last recorded write.
Shortening the interval moves every entry under it at the next evaluation, with no entry edited.

```console
$ safix check
$ safix rotate --due
```

`check` reports drift and changes nothing, and each finding prints the command that resolves it; a value past its deadline is one such finding.
`rotate --due` re-mints everything past its deadline, along with every generator reading what it replaced, and announces the set before the first commit.
A due value no generator declares is listed with the `safix set` it needs rather than refused, because one un-mintable value should not block the rest.

The same run can be a timer on the workstation where the declarations and your git identity live.

```nix title="examples/quickstart/home/alice.nix#rotation"
  safix.rotation = {
    enable = false;
    repository = "/home/alice/secrets";
    onCalendar = "daily";
  };
```

It is off here so the example shows the option rather than its default.
Turned on, it installs a user timer that commits locally and pushes nothing; reviewing and pushing stay yours.
The identity it runs under has to decrypt without a prompt and without a card — see [Rotating secrets](docs/guides/rotating-secrets.md).

## A file assembled from several values

A program that wants one configuration file rather than two paths gets a template.

```nix title="examples/quickstart/hosts/web.nix#template"
  safix.templates."grafana.env" = {
    content = ''
      GF_SECURITY_ADMIN_PASSWORD=${config.safix.placeholder."grafana-admin-password"}
      GF_SECURITY_SECRET_KEY=${config.safix.placeholder."grafana-secret-key"}
    '';
    restartUnits = [ "grafana.service" ];
  };
```

`content` is public text and enters the nix store, so put placeholders there and never plaintext.
`config.safix.placeholder.<name>` is a literal token the installer replaces inside a private runtime generation, and nix never sees a value.
A template may name only what this profile resolves, which is why both entries above were granted to the machine.

## alice's own profile

A person's custody does not depend on the host they log into, so the same declarations serve her own scope.

```nix title="examples/quickstart/home/alice.nix#bind"
  imports = [ inputs.safix.homeModules.safix ];

  safix.flake = inputs.self;
  safix.user = "alice";
  safix.hostname = "web";
  safix.identity.keyFile = "/home/alice/.config/sops/age/keys.txt";
```

[`safix.user`](docs/reference/profile-options.md#safixuser) selects the declared person and defaults to the profile's own username.
[`safix.identity.keyFile`](docs/reference/profile-options.md#safixidentitykeyfile) is the file `safix keygen` appended to, as a string rather than a nix path: a nix path would copy the identity into the world-readable store.
[`safix.hostname`](docs/reference/profile-options.md#safixhostname) is read from the enclosing system configuration where there is one, and named here because a standalone profile has none.

Activation refuses before it links anything when that key file is absent or unreadable.
Ownership is a system-scope axis only, so a user-scope entry carrying `owner` or `group` is refused rather than having the field dropped.

## Coming from sops-nix or agenix

`safix migrate plan.json` converts explicit sources into new files, decrypts every candidate independently with the target identities, and compares the bytes before publishing anything.
It needs no declarations of its own, and it never replaces or deletes a source.
An interrupted run leaves a journal beside the receipt, so rerunning the same plan resumes it instead of refusing its outputs, and `safix migrate --abandon` discards it.

The plan's schema is in [Migration plan](docs/reference/migration-plan.md), and a plan walked end to end is in [Migrating from sops-nix and agenix](docs/guides/migrating-from-sops-nix-and-agenix.md).

## flake-parts

flake-parts is fully supported and is the shorter route.
Import `inputs.safix.flakeModules.default`, write your declarations as `flake.safix.*` anywhere in your module tree, and the projection appears at `flake.safix.lib` with no `mkVault` call of your own.
Profiles then bind through `safix.flake = inputs.self;` exactly as above.

[`examples/dendritic/`](examples/dendritic) is the worked example: one declaration per file, discovered by directory, with no hand-maintained module list.
The route is described in [flake-parts](docs/guides/flake-parts.md).

## What safix will not do

- It will not let you author a governed placement: the audience and the format decide the file, always.
- It will not write a catch-all recipient rule, so an unmatched path fails closed rather than acquiring a default audience.
- It will not treat narrowing an audience as revocation — see the revocation rule in [Custody](docs/concepts/custody.md).
- It will not read or define an option outside its own namespace — see the namespace rule in [The model](docs/concepts/model.md).
- It will not mint another person's identity without an explicit flag saying that is what you are doing.

## Documentation

- [Your first secret](docs/tutorials/first-secret.md) — this page again, slower, with a second scope at the end.
- [Guides](docs/index.md#guides) — unattended hosts, inspectable recipients, removing access, rotation, generators, templates, migration, hardware keys, identity backup, vaults, password managers, flake-parts.
- [Concepts](docs/index.md#concepts) — the model, custody, and the storage layout.
- [Reference](docs/index.md#reference) — the verbs, the `flake.safix.*` declarations, the `safix.*` profile options, the migration plan, the environment variables and the refusal codes.
- [`examples/`](examples) — the fleets and profiles this repository evaluates, indexed by [`examples/README.md`](examples/README.md).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to work on this repository.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
