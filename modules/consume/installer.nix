# System installer: normal and early-user manifests never share a generation.
{
  config,
  options,
  lib,
  pkgs,
  ...
}:
let
  common = import ./common.nix {
    inherit lib;
    scope = "system";
  };
  cfg = config.safix;
  entries = builtins.attrValues cfg.secrets;
  earlyEntries = lib.filter (e: e.neededForUsers) entries;
  normalEntries = lib.filter (e: !e.neededForUsers) entries;
  hasEarly = earlyEntries != [ ];
  identitySshKeyPaths =
    if cfg.identity.sshKeyPaths != [ ] then cfg.identity.sshKeyPaths else cfg.identity.derivedHostKeys;
  manifest = common.makeManifest {
    inherit cfg pkgs;
    name = "safix-manifest.json";
    entries = normalEntries;
    templates = common.templateManifests cfg;
    sshKeyPaths = identitySshKeyPaths;
  };
  usersManifest = common.makeManifest {
    inherit cfg pkgs;
    name = "safix-users-manifest.json";
    entries = earlyEntries;
    templates = [ ];
    sshKeyPaths = identitySshKeyPaths;
    early = true;
  };
  environment = common.toolEnvironment { inherit cfg pkgs; };
  makeScript =
    early:
    pkgs.writeShellScript (if early then "safix-install-users-secrets" else "safix-install-secrets") ''
      set -euo pipefail
      ${common.exportEnvironment environment}
      ${lib.optionalString ((if early then earlyEntries else normalEntries) != [ ]) (
        common.identityPreflight {
          inherit cfg;
          sshKeyPaths = identitySshKeyPaths;
        }
      )}
      exec ${cfg.installer.package}/bin/safix install ${lib.optionalString early "--ignore-passwd "}${
        if early then usersManifest else manifest
      }
    '';
  installerScript = makeScript false;
  usersInstallerScript = makeScript true;
  identityMounts =
    identitySshKeyPaths
    ++ lib.optional (cfg.identity.keyFile != null) (toString cfg.identity.keyFile)
    ++ lib.optional (cfg.identity.gnupgHome != null) (toString cfg.identity.gnupgHome);
  defaultDependenciesOff = unit: lib.elem (unit.unitConfig.DefaultDependencies or true) [
    false
    "no"
    "false"
    "0"
  ];
  lateAfterUnits = lib.filter (
    name:
    let
      unit = config.systemd.services.${lib.removeSuffix ".service" name} or null;
    in
    lib.hasSuffix ".service" name && unit != null && !defaultDependenciesOff unit
  ) cfg.installer.afterUnits;
  unitBase =
    { script, before }:
    {
      environment = cfg.installer.environment // {
        SOPS_RESTART_UNITS_VIA_SYSTEMCTL = "1";
      };
      path = cfg.installer.agePlugins ++ [
        pkgs.age
        pkgs.gnupg
        pkgs.sops
        pkgs.ssh-to-age
        pkgs.systemd
      ];
      serviceConfig = {
        Type = "oneshot";
        ExecStart = [ "${script}" ];
        RemainAfterExit = true;
      };
      # Being wanted by sysinit.target pulls a unit in without ordering it, so
      # services with default dependencies would race the installer without the
      # explicit edge. The mounts are wanted, not required: a key mount stopped
      # at shutdown must not pull these units, or the user-creation units they
      # precede, into a dependency loop.
      before = [ "sysinit.target" ] ++ before;
      unitConfig = {
        DefaultDependencies = "no";
        WantsMountsFor = identityMounts;
      };
    };
in
{
  options.safix.installer = common.installerOptions { inherit cfg pkgs; } // {
    useTmpfs = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Use tmpfs instead of ramfs for both secret stores. The installer mounts tmpfs with noswap; a kernel that refuses noswap gets ramfs instead, never a swappable tmpfs. Turning this on for a running host keeps the existing ramfs until reboot; turning it off mounts ramfs over the existing tmpfs, whose earlier generations stay in memory until reboot.";
    };
    useSystemdActivation = lib.mkOption {
      type = lib.types.bool;
      default =
        (options.systemd ? sysusers && config.systemd.sysusers.enable)
        || (options.services ? userborn && config.services.userborn.enable);
      defaultText = lib.literalExpression "(options.systemd ? sysusers && config.systemd.sysusers.enable) || (options.services ? userborn && config.services.userborn.enable)";
      description = "Install with units instead of legacy activation scripts. Defaults from the host's user-management mechanism.";
    };
    afterActivation = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "Additional legacy activation steps the normal installer waits for. Early identities must be available before user creation.";
    };
    afterUnits = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "Additional units the normal installer waits for. These are not added to the pre-user unit, to avoid a user-creation ordering cycle. The installer runs before sysinit.target, so each named unit must itself run before sysinit.target, which for a service means default dependencies off. Evaluation refuses a named service defined through systemd.services with default dependencies on; units defined elsewhere are not checked.";
    };
  };

  options.safix.identity.deriveHostKeys = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Derive SSH identities from OpenSSH ed25519 host keys when no explicit identity is configured. Keys in either safix store are excluded.";
  };
  options.safix.identity.derivedHostKeys = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    readOnly = true;
    internal = true;
    default =
      if
        cfg.identity.deriveHostKeys
        && cfg.identity.keyFile == null
        && cfg.identity.gnupgHome == null
        && cfg.identity.sshKeyPaths == [ ]
        && (
          config.services.openssh.enable
          || (options.services.openssh ? generateHostKeys && config.services.openssh.generateHostKeys)
        )
      then
        map (e: e.path) (
          lib.filter (
            e:
            e.type == "ed25519"
            && !(lib.any (root: lib.hasPrefix root e.path) [
              cfg.installer.symlinkPath
              cfg.installer.secretsMountPoint
            ])
          ) config.services.openssh.hostKeys
        )
      else
        [ ];
    defaultText = lib.literalMD "eligible ed25519 host keys, when no explicit identity was supplied and `services.openssh.enable` or `services.openssh.generateHostKeys` is on";
    description = "Derived host identity paths shared by the manifest, preflight and mount ordering.";
  };

  config = lib.mkIf cfg.enable {
    assertions =
      map (message: {
        assertion = false;
        inherit message;
      }) (common.deploymentErrors cfg)
      ++ [
        {
          assertion = lib.all common.privatePath identitySshKeyPaths;
          message = "safix: derived private identity paths must be absolute and outside the Nix store.";
        }
      ]
      ++ map (unit: {
        assertion = false;
        message = "safix: safix.installer.afterUnits names ${unit}, which has default dependencies and so runs after sysinit.target; safix-install-secrets runs before sysinit.target, so the ordering is a cycle. Set DefaultDependencies=no on ${unit} or remove it from afterUnits.";
      }) lateAfterUnits;

    system.build.safix-manifest = manifest;
    system.build.safix-users-manifest = usersManifest;
    system.build.safix-installer = installerScript;

    # The dependency belongs to users, not just to the normal installer:
    # passwordFile consumers must see early secrets before user creation.
    system.activationScripts.users = lib.mkIf (!cfg.installer.useSystemdActivation && hasEarly) {
      deps = [ "safixInstallUsersSecrets" ];
    };
    system.activationScripts.safixInstallUsersSecrets =
      lib.mkIf (!cfg.installer.useSystemdActivation && hasEarly)
        (
          lib.stringAfter [ "specialfs" ] ''
            ${usersInstallerScript}
          ''
        );
    system.activationScripts.safixInstallSecrets = lib.mkIf (!cfg.installer.useSystemdActivation) (
      lib.stringAfter
        (
          [
            "specialfs"
            "users"
            "groups"
          ]
          ++ lib.optional hasEarly "safixInstallUsersSecrets"
          ++ cfg.installer.afterActivation
        )
        ''
          [ -e /run/current-system ] || echo installing safix secrets...
          ${installerScript}
        ''
      // lib.optionalAttrs (config.system ? dryActivationScript) {
        supportsDryActivation = true;
      }
    );

    systemd.services.safix-install-users-secrets =
      lib.mkIf (cfg.installer.useSystemdActivation && hasEarly)
        (
          unitBase {
            script = usersInstallerScript;
            before = [
              "systemd-sysusers.service"
              "userborn.service"
              "sysinit-reactivation.target"
            ];
          }
          // {
            description = "Install root-only safix secrets before user creation";
            wantedBy = [
              "sysinit.target"
              "sysinit-reactivation.target"
            ];
            requiredBy = [
              "systemd-sysusers.service"
              "userborn.service"
              "sysinit-reactivation.target"
            ];
            # No local-fs.target edge and no shutdown edges. This unit is
            # ordered after its key mount, and userborn, systemd-tmpfiles-setup-dev
            # and local-fs-pre.target lie between it and that mount, so both close
            # a cycle: local-fs.target at boot, and a stop job at shutdown.
          }
        );
    systemd.services.safix-install-secrets = lib.mkIf cfg.installer.useSystemdActivation (
      unitBase {
        script = installerScript;
        before = [
          "shutdown.target"
          "sysinit-reactivation.target"
        ];
      }
      // {
        # With default dependencies off nothing else stops this unit at shutdown.
        conflicts = [ "shutdown.target" ];
        description = "Install safix secrets and render public templates";
        wantedBy = [ "sysinit.target" ];
        requiredBy = [ "sysinit-reactivation.target" ];
        requires = lib.optional hasEarly "safix-install-users-secrets.service";
        after = [
          "local-fs.target"
          "systemd-sysusers.service"
          "userborn.service"
        ]
        ++ lib.optional hasEarly "safix-install-users-secrets.service"
        ++ cfg.installer.afterUnits;
      }
    );
  };
}
