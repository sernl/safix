# safix in a NixOS configuration: the same `safix.*` namespace as the
# home-manager module, materializing at system scope.
#
# This module imports nothing outside safix. `nixosModules.default` is this file
# plus sops-nix's own NixOS module; a tree that already imports sops-nix at a
# revision of its own imports this file instead, as `nixosModules.safix`. See
# `./home.nix` for the measured reason the choice exists. `./installer.nix` is
# safix's own and travels with this file in both forms.
#
# ── no activation guard here, and why ──
# The home-manager module installs a read-only identity preflight that sorts
# ahead of `checkLinkTargets`, where refusing is still atomic: nothing has been
# linked, no unit restarted, no secret written. Nothing equivalent is installed
# here, because no equivalent point has been demonstrated at system activation.
# sops-nix's system-scope secret installation is ordered by its own units and by
# `system.activationScripts`, and safix has not shown a place in that sequence
# where a refusal leaves the previous generation intact. Claiming one would be
# documenting a guarantee that no code in this repository enforces.
#
# The failure the home-scope guard exists for is also rarer here. sops-nix
# defaults `sops.age.sshKeyPaths` to the host's ed25519 keys, which exist on any
# host that runs sshd, so a system configuration usually has an identity without
# naming one — which is why `safix.identity.sshKeyPaths` is defined onto the
# provisioner only when it is set.
{
  config,
  options,
  lib,
  ...
}:
let
  common = import ./common.nix {
    inherit lib;
    scope = "system";
  };

  cfg = config.safix;

  # The provisioner's own derivation rule (`modules/sops/default.nix:181-191`)
  # with the exclusion prefix that is actually safix's. The exclusion avoids a
  # catch-22 — decrypting with a key this installer itself deploys — which is
  # a statement about safix's store, now `safix.installer.symlinkPath`, and
  # not about `/run/secrets`, where a foreign store's keys were placed before
  # safix runs and are exactly the identity to decrypt with.
  derivedHostKeys =
    if cfg.identity.deriveHostKeys && config.services.openssh.enable then
      map (e: e.path) (
        lib.filter (
          e: e.type == "ed25519" && !(lib.hasPrefix cfg.installer.symlinkPath e.path)
        ) config.services.openssh.hostKeys
      )
    else
      [ ];
in
{
  imports = [ ./installer.nix ];

  options.safix =
    common.sharedOptions {
      inherit cfg;

      # A system configuration knows its own host, and knows no person: which
      # people's system-scope entries land here is exactly what this module is
      # asked.
      userDefault = null;
      userDefaultText = lib.literalExpression "null";

      hostnameDefault = config.networking.hostName;
      hostnameDefaultText = lib.literalExpression "config.networking.hostName";
    }
    // {
      installed = common.installedOptionFor options;
    };

  config = lib.mkMerge [
    {
      safix.secrets = common.resolvedFor {
        inherit cfg;
        target = config;
      };

      # Outside the enable gate deliberately. Each of these fires exactly when
      # the resolution is empty for want of the option it names, which is when
      # `enable` defaults to false — so an assertion inside the gate would be a
      # refusal that only speaks once the mistake has already been repaired.
      #
      # `configured` is read off `options` rather than off `cfg` because
      # `safix.hostname` defaults to this configuration's own hostname and is
      # therefore never null: the value cannot tell a consumer's selection from
      # the module's own default, and only a definition can.
      assertions = common.assertionsFor {
        inherit cfg;
        configured =
          common.wasSet options.safix.user
          || common.wasSet options.safix.machine
          || common.wasSet options.safix.hostname;
      };
    }

    (lib.mkIf cfg.enable {
      # The resolved set arrives in safix's own typed option rather than in
      # `sops.secrets`, which stays empty: two installers cannot both be right
      # about one resolved set, and the provisioner gates its entire installer
      # — activation entry, unit and key-source assertion — on its own secrets
      # option being non-empty.
      #
      # An entry that declares no path parks at `<symlinkPath>/<name>`, minted
      # here rather than left to the provisioner's `/run/secrets/<name>`
      # default: the installer creates a symlink at any path that is not
      # `<symlinkPath>/<name>` (`main.go:254-268`), so safix's store root and
      # this default must move together or a moved root writes symlinks into
      # the other store's directory.
      safix.installed = lib.mapAttrs (
        name: entry:
        entry // lib.optionalAttrs (!(entry ? path)) { path = "${cfg.installer.symlinkPath}/${name}"; }
      ) cfg.secrets;

      sops = {
        age.keyFile = cfg.identity.keyFile;

        # Defined unconditionally, replacing the provisioner's own default —
        # whose exclusion prefix is the store safix no longer writes — with
        # safix's derivation. A consumer-named identity always wins, and a
        # derivation that yields nothing leaves the list empty rather than
        # borrowing a rule made for another installer's store.
        age.sshKeyPaths =
          if cfg.identity.sshKeyPaths != [ ] then cfg.identity.sshKeyPaths else derivedHostKeys;
      };
    })
  ];
}
