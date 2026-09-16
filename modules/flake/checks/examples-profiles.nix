# Holds ../../../examples/profiles/: the two consumption profiles, over the one
# fleet `examples/plain-nix/fleet.nix` declares, plus the storage-and-vault
# overlay merged beside it.
#
# ── what it reads ──
# `examples/profiles/nixos.nix` through a real `nixosSystem` and
# `examples/profiles/home.nix` through home-manager's own library, the way
# `./portability.nix` and `./installer.nix` already do. Each is imported beside
# safix's own bare consumption module rather than the published wrapper, and
# `safix` reaches the package set through one overlay: the bare modules import
# nothing by contract, so their own default for `safix.installer.package` is
# `pkgs.safix`, and `flake.nix` is where the published names supply it instead.
#
# ── why this is not a field of `./examples.nix` ──
# What a profile resolves is a materialization under a scope, and the fields
# that check compares are the pre-scope projection. Three features of the
# declaration surface are therefore invisible there and observable only here:
# an entry's `mode` and its `path` are not placement fields, and `path` is
# `functionTo str`, so it cannot be serialized at all — it is compared applied,
# under this profile's own home directory. A profile evaluation also needs a
# host module system and home-manager, and `./examples.nix`' sandbox
# deliberately copies neither.
#
# ── the roots the rows are bound against ──
# The committed profiles bind `safix.lib` against their own tree, which is what
# a consumer writes and what `bindingAsWritten` resolves through. Every row
# that reads a resolved entry is bound against a root-relative projection
# instead, and that is not a preference: a resolved `sopsFile` is
# `lib.types.path`, so forcing one resolves a path inside this flake's own
# source, and this repository commits no ciphertext for a fixture fleet to be
# read out of. An empty root is what makes those paths read as the
# repository-relative strings these rows compare, the way `./portability.nix`
# reads its own.
#
# ── the manifests are read as text, not built ──
# `config.system.build.safix-manifest` and `config.safix.installer.manifest`
# are `writeTextFile` derivations whose `text` is the manifest JSON, so the
# rows below parse that attribute rather than realising the derivation. What
# realising it would add is a compile of the runtime and a run of its own
# manifest check, which `./installer.nix` already holds; what it would cost is
# this check reddening whenever the runtime does not build.
#
# ── the relocation is read twice, and why ──
# A declared vault replaces all three storage roots: every resolved name
# becomes a keyed hash under the vault's own flat buckets, so no string a
# profile reads carries `.safix/encrypted` while a vault is declared. The two
# halves of `examples/profiles/relocated.nix` are therefore observed in two
# evaluations — the storage roots with the vault forced back to null, and the
# vault as declared, whose opacity is read off the keys a profile is handed
# rather than off a document path, since a vault-rooted document path is a
# path into a tree this repository does not commit either. The forcing module
# is assembled here and is not an example file, for the reason
# `ownershipRefused`'s profile is not one.
#
# ── severity: four drills, all observed ──
# Dropping `safix.machine = "deck"` from `nixos.nix` empties `systemSecrets`
# and moves the two other rows that read the same resolution —
# `bindingAsWritten` goes empty and `ownershipAxis` goes null — while every
# user-scope row stays green, which is the scope boundary this pair of
# profiles exists to show. Observed.
# Adding `owner = "web"` to an entry the home profile resolves makes the real
# home profile fail to evaluate rather than dropping the field: "safix
# ownership: flake.safix.users.alice materializes at user scope … but
# 'app-credentials' sets owner". That is the counterpart of
# `ownershipRefused` — a dropped ownership field reads afterwards as an
# ownership claim that was honoured. Observed.
# Reverting `relocated.nix`'s `storage.encrypted` to its default reddens
# `relocatedStorage`'s `sopsFiles` row while `relocatedVault.vaultDeclared`
# stays green, which is what says the two halves are read independently.
# Observed.
# Dropping `catalogue.deploy-key`'s `mode` from both examples at once reddens
# `homeSecrets` — the entry resolves at `0400` instead of `0440` — while
# `safix-examples` stays green, which is why this check exists at all.
# Observed.
{
  inputs,
  lib,
  ...
}:
let
  examplesRoot = ../../../examples;
  plainNixRoot = examplesRoot + "/plain-nix";
  profilesRoot = examplesRoot + "/profiles";
  libRoot = ../../../lib;

  mkVault = (import libRoot { inherit lib; }).mkVault;

  rootRelative =
    modules:
    mkVault {
      inherit modules;
      root = "";
    };

  fleet = rootRelative [ (plainNixRoot + "/fleet.nix") ];

  relocatedVault = rootRelative [
    (plainNixRoot + "/fleet.nix")
    (profilesRoot + "/relocated.nix")
  ];

  # The same declarations with the vault half withdrawn, so the three storage
  # roots are observable at all: a declared vault subsumes every one of them.
  relocatedStorage = rootRelative [
    (plainNixRoot + "/fleet.nix")
    (profilesRoot + "/relocated.nix")
    { flake.safix.vault = lib.mkForce null; }
  ];

  fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

  # The nine fields of a resolved system-scope entry that safix decides. `uid`
  # and `gid` are excluded: both are the provisioner's numeric echo of `owner`
  # and `group`, which are already here.
  systemFields = [
    "key"
    "path"
    "mode"
    "owner"
    "group"
    "sopsFile"
    "format"
    "restartUnits"
    "reloadUnits"
  ];

  # A readable name is what a vault-rooted name may not carry. The entry names
  # and the declaration-root spellings are the readable identities every
  # vault name is a keyed hash of instead.
  readableNames = [
    "fleet-token"
    "web-token"
    "grafana-password"
    "alice"
    "secrets/safix"
  ];
in
{
  perSystem =
    {
      pkgs,
      self',
      system,
      ...
    }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # One overlay, because supplying the package is the whole of what a bare
      # consumption module cannot derive for itself.
      safixOverlay = _: _: { safix = self'.packages.safix; };

      hostFor =
        extra:
        (inputs.nixpkgs.lib.nixosSystem {
          modules = [
            ../../consume/nixos.nix
            (profilesRoot + "/nixos.nix")
            {
              nixpkgs.hostPlatform = system;
              nixpkgs.overlays = [ safixOverlay ];
              networking.hostName = "deck";
              system.stateVersion = "24.05";

              # A machine's declared recipient is `ssh-to-age` of a host key, so
              # a host that has one is the case this profile stands for, and it
              # is what `safix.identity.deriveHostKeys` reads.
              services.openssh.enable = true;
            }
            extra
          ];
        }).config;

      homeFor =
        extra:
        (inputs.home-manager.lib.homeManagerConfiguration {
          pkgs = pkgs.extend safixOverlay;
          modules = [
            ../../consume/home.nix
            (profilesRoot + "/home.nix")
            {
              home = {
                username = "alice";
                homeDirectory = "/home/alice";
                stateVersion = "24.05";
              };
            }
            extra
          ];
        }).config;

      boundTo = projection: { safix.lib = lib.mkForce projection; };

      systemProfile = hostFor (boundTo fleet);
      homeProfile = homeFor (boundTo fleet);
      systemAsWritten = hostFor { };

      relocatedStorageProfile = hostFor (boundTo relocatedStorage);
      relocatedVaultProfile = homeFor (boundTo relocatedVault);

      # A user-scope profile serving the machine. `deck` reads the service
      # `web`'s entry, and `web` declares an account and a group, so this scope
      # has a claim it cannot honour and refuses rather than dropping it.
      ownershipRefusedProfile = homeFor (
        boundTo fleet
        // {
          safix = {
            lib = lib.mkForce fleet;
            user = lib.mkForce null;
            machine = "deck";
          };
        }
      );

      manifestOf = drv: builtins.fromJSON drv.text;

      readManifest = m: {
        inherit (m)
          keepGenerations
          secretsMountPoint
          symlinkPath
          useTmpfs
          ;
      };

      sopsFilesOf = profile: lib.unique (map (e: e.sopsFile) (builtins.attrValues profile.safix.secrets));

      rows = {
        # Every entry the machine `deck` holds: the grants aimed at it, plus the
        # entries the service `web` carries onto it.
        systemSecrets = lib.mapAttrs (_name: lib.getAttrs systemFields) systemProfile.safix.secrets;

        # The user scope's set is the untyped projection — what the resolver
        # emitted, with an entry's `path` already applied to this
        # configuration — so `deploy-key`'s declared mode and `web-token`'s
        # `perHost.deck.add` override are both readable here and nowhere in
        # `./examples.nix`.
        homeSecrets = homeProfile.safix.secrets;

        entryPath = homeProfile.safix.secrets.app-credentials.path;

        # The committed binding, resolved: the names alone, because forcing a
        # resolved document would resolve a path this repository does not
        # commit.
        bindingAsWritten = builtins.attrNames systemAsWritten.safix.secrets;

        # Read through a default, so a profile that resolves nothing at all
        # reports two null fields here rather than aborting the whole check on
        # a missing attribute and taking every other row's evidence with it.
        ownershipAxis =
          let
            entry = systemProfile.safix.secrets."web/web-token" or null;
          in
          {
            owner = if entry == null then null else entry.owner;
            group = if entry == null then null else entry.group;
          };
        ownershipRefused = fires ownershipRefusedProfile.safix.secrets;

        identity = {
          derivedHostKeysAreNotEmpty = systemProfile.safix.identity.derivedHostKeys != [ ];
          derivedHostKeys = systemProfile.safix.identity.derivedHostKeys;
          preflight = homeProfile.safix.identityPreflight;
        };

        installerSurface = {
          systemScope = readManifest (manifestOf systemProfile.system.build.safix-manifest);
          userScope = readManifest (manifestOf homeProfile.safix.installer.manifest);
        };

        relocatedStorage = {
          sopsFiles = sopsFilesOf relocatedStorageProfile;
          vaultDeclared = relocatedStorage.vaultDeclared;
        };

        # A vault-rooted name is a keyed hash of the entry's identity, so the
        # readable half of what a profile reads is the entry names alone. The
        # hashes themselves are not asserted one by one: a list of them says
        # nothing a reader can check, where "every one is opaque and none
        # carries a declared name" is the property the vault exists for.
        relocatedVault =
          let
            keys = map (e: e.key) (builtins.attrValues relocatedVaultProfile.safix.secrets);
          in
          {
            vaultDeclared = relocatedVaultProfile.safix.lib.vaultDeclared;
            names = builtins.attrNames relocatedVaultProfile.safix.secrets;
            everyKeyIsAnOpaqueHash = lib.all (key: builtins.match "[0-9a-f]{64}" key != null) keys;
            carriesNoReadableName = !(lib.any (key: lib.any (name: lib.hasInfix name key) readableNames) keys);
          };
      };

      expected = {
        systemSecrets = {
          fleet-token = {
            format = "yaml";
            group = null;
            key = "fleet-token";
            mode = "0400";
            owner = null;
            path = "/run/safix-example/fleet-token";
            reloadUnits = [ ];
            restartUnits = [ ];
            sopsFile = "/secrets/safix/shared/alice,deck/secrets.yaml";
          };
          "web/web-token" = {
            format = "yaml";
            group = "web";
            key = "web-token";
            mode = "0400";
            owner = "web";
            path = "/run/safix-example/web/web-token";
            reloadUnits = [ ];
            restartUnits = [ ];
            sopsFile = "/secrets/safix/shared/%web,alice/secrets.yaml";
          };
        };
        homeSecrets = {
          app-credentials = {
            mode = "0400";
            path = "/home/alice/.config/example-app/credentials.toml";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          corp-handover = {
            mode = "0400";
            sopsFile = "/secrets/safix/shared/@~rack,alice/secrets.yaml";
          };
          deploy-key = {
            mode = "0440";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          deploy-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          deploy-username = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          derived-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          escrow-note = {
            mode = "0400";
            sopsFile = "/secrets/safix/shared/=acme,alice/secrets.yaml";
          };
          fetched-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          fleet-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/shared/alice,deck/secrets.yaml";
          };
          generated-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          grafana-password = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          handoff-note = {
            mode = "0400";
            sopsFile = "/secrets/safix/shared/alice,bob/secrets.yaml";
          };
          ntfy-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          pager-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/shared/@oncall,alice/secrets.yaml";
          };
          prompted-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          registry-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          shelf-item = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          team-wifi = {
            mode = "0400";
            sopsFile = "/secrets/safix/shared/alice,bob/secrets.yaml";
          };
          validated-token = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          vpn-password = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          web-token = {
            mode = "0440";
            sopsFile = "/secrets/safix/shared/%web,alice/secrets.yaml";
          };
          wg-key = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
          wg-private = {
            mode = "0400";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
        };
        entryPath = "/home/alice/.config/example-app/credentials.toml";
        bindingAsWritten = [
          "fleet-token"
          "web/web-token"
        ];
        ownershipAxis = {
          owner = "web";
          group = "web";
        };
        ownershipRefused = true;
        identity = {
          derivedHostKeysAreNotEmpty = true;
          derivedHostKeys = [ "/etc/ssh/ssh_host_ed25519_key" ];
          preflight = false;
        };
        installerSurface = {
          systemScope = {
            keepGenerations = 3;
            secretsMountPoint = "/run/safix-example.d";
            symlinkPath = "/run/safix-example";
            useTmpfs = true;
          };
          userScope = {
            keepGenerations = 3;
            secretsMountPoint = "%r/safix-example.d";
            symlinkPath = "%r/safix-example";
            useTmpfs = false;
          };
        };
        relocatedStorage = {
          sopsFiles = [
            "/.safix/encrypted/shared/alice,deck/secrets.yaml"
            "/.safix/encrypted/shared/%web,alice/secrets.yaml"
          ];
          vaultDeclared = false;
        };
        relocatedVault = {
          vaultDeclared = true;
          names = [
            "app-credentials"
            "corp-handover"
            "deploy-key"
            "deploy-token"
            "deploy-username"
            "derived-token"
            "escrow-note"
            "fetched-token"
            "fleet-token"
            "generated-token"
            "grafana-password"
            "handoff-note"
            "ntfy-token"
            "pager-token"
            "prompted-token"
            "registry-token"
            "shelf-item"
            "team-wifi"
            "validated-token"
            "vpn-password"
            "web-token"
            "wg-key"
            "wg-private"
          ];
          everyKeyIsAnOpaqueHash = true;
          carriesNoReadableName = true;
        };
      };
    in
    {
      checks.safix-examples-profiles = mkStructuralCheck {
        name = "examples-profiles";
        actual = rows;
        expected = expected;
      };
    };
}
