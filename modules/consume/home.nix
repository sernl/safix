# Home scope uses the same manifest schema, without mounts or chown. Unit hooks
# are retained and run through systemctl --user by the installer.
{
  config,
  options,
  lib,
  pkgs,
  osConfig ? null,
  ...
}:
let
  common = import ./common.nix {
    inherit lib;
    scope = "user";
  };
  cfg = config.safix;
  hasIdentity =
    cfg.identity.keyFile != null || cfg.identity.gnupgHome != null || cfg.identity.sshKeyPaths != [ ];
  hasSecrets = cfg.secrets != { };
  hasWork = hasSecrets || cfg.templates != { };
  manifest = common.makeManifest {
    inherit cfg pkgs;
    name = "safix-user-manifest.json";
    entries = builtins.attrValues cfg.secrets;
    templates = common.templateManifests cfg;
    sshKeyPaths = cfg.identity.sshKeyPaths;
  };
  identityPreflight = common.identityPreflight {
    inherit cfg;
    sshKeyPaths = cfg.identity.sshKeyPaths;
    generateKey = cfg.identity.generateKey;
  };
  keyGeneration = lib.optionalString (cfg.identity.generateKey && cfg.identity.keyFile != null) ''
    export SAFIX_AGE_KEYGEN="''${SAFIX_AGE_KEYGEN:-${pkgs.age}/bin/age-keygen}"
    if [ -L ${lib.escapeShellArg (toString cfg.identity.keyFile)} ]; then
      echo 'safix: refusing to generate an identity through a symlink' >&2
      exit 1
    fi
    if [ ! -e ${lib.escapeShellArg (toString cfg.identity.keyFile)} ]; then
      (
        umask 077
        mkdir -p ${lib.escapeShellArg (builtins.dirOf (toString cfg.identity.keyFile))}
        "$SAFIX_AGE_KEYGEN" -o ${lib.escapeShellArg (toString cfg.identity.keyFile)}
      )
    fi
  '';
  installScript = pkgs.writeShellScript "safix-install-secrets-user" ''
    set -euo pipefail
    ${common.exportEnvironment (common.toolEnvironment { inherit cfg pkgs; })}
    ${keyGeneration}
    ${lib.optionalString hasSecrets (
      common.identityPreflight {
        inherit cfg;
        sshKeyPaths = cfg.identity.sshKeyPaths;
      }
    )}
    exec ${cfg.installer.package}/bin/safix install ${manifest}
  '';
in
{
  options.safix =
    lib.recursiveUpdate
      (common.sharedOptions {
        inherit cfg;
        userDefault = config.home.username;
        userDefaultText = lib.literalExpression "config.home.username";
        hostnameDefault =
          if osConfig == null || !(osConfig ? networking) then null else osConfig.networking.hostName;
        hostnameDefaultText = lib.literalExpression "osConfig.networking.hostName, where available; null standalone";
        secretsType = lib.types.attrsOf (common.secretEntryType { inherit cfg; });
      })
      {
        identityPreflight = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Check identity presence and readability before checkLinkTargets. This does not check decryption and cannot undo an enclosing NixOS switch.";
        };
        identity.generateKey = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Generate the configured age keyFile at activation if absent. Its public recipient must still be enrolled separately. Existing private identities are not overwritten.";
        };
        installer = common.installerOptions { inherit cfg pkgs; } // {
          manifest = lib.mkOption {
            type = lib.types.package;
            readOnly = true;
            default = manifest;
            defaultText = lib.literalMD "the user-mode installer manifest";
            description = "Public manifest used by activation and the user unit; secret plaintext is never rendered in Nix.";
          };
        };
      };

  config = lib.mkMerge [
    {
      safix.secrets =
        let
          resolved = common.resolvedFor {
            inherit cfg;
            target = config;
          };
        in
        if resolved != { } && !hasIdentity then
          throw (common.noIdentityMessage { inherit cfg resolved; })
        else
          resolved;

      # Defaults for username/hostname are not explicit registry requests.
      # Conversely, imports must never suppress a broken explicit binding.
      assertions = common.assertionsFor {
        inherit cfg;
        configured =
          common.wasSet options.safix.user
          || common.wasSet options.safix.machine
          || common.wasSet options.safix.hostname
          || common.wasSet options.safix.flake
          || common.wasSet options.safix.lib;
      };
    }
    (lib.mkIf cfg.enable {
      assertions = map (message: {
        assertion = false;
        inherit message;
      }) (common.deploymentErrors cfg);

      home.activation.safixInstall = lib.mkIf hasWork (
        lib.hm.dag.entryAfter [ "writeBoundary" ] ''
          run ${installScript}
        ''
      );
      home.activation.safixIdentityPreflight = lib.mkIf (cfg.identityPreflight && hasSecrets) (
        lib.hm.dag.entryBefore [ "checkLinkTargets" ] identityPreflight
      );

      systemd.user.services = lib.mkIf (pkgs.stdenv.hostPlatform.isLinux && hasWork) {
        safix = {
          Unit = {
            Description = "Install safix secrets and render public templates";
            RefuseManualStop = false;
          };
          Service = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = "${installScript}";
          };
          Install.WantedBy = [ "default.target" ];
        };
      };
    })
  ];
}
