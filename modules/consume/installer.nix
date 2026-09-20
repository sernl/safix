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
  unitBase = script: {
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
    unitConfig = {
      DefaultDependencies = "no";
      RequiresMountsFor = identityMounts;
    };
  };
in
{
  options.safix.installer = common.installerOptions { inherit cfg pkgs; } // {
    useTmpfs = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Use tmpfs instead of ramfs for both secret stores. The installer requests noswap where supported.";
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
      description = "Additional units the normal installer waits for. These are not added to the pre-user unit, to avoid a user-creation ordering cycle.";
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
        && config.services.openssh.enable
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
    defaultText = lib.literalMD "eligible ed25519 host keys, when no explicit identity was supplied";
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
      ];

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
          unitBase usersInstallerScript
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
            before = [
              "systemd-sysusers.service"
              "userborn.service"
              "sysinit-reactivation.target"
            ];
            after = [ "local-fs.target" ];
          }
        );
    systemd.services.safix-install-secrets = lib.mkIf cfg.installer.useSystemdActivation (
      unitBase installerScript
      // {
        description = "Install safix secrets and render public templates";
        wantedBy = [ "sysinit.target" ];
        requiredBy = [ "sysinit-reactivation.target" ];
        before = [ "sysinit-reactivation.target" ];
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
