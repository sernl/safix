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
        rotation = {
          enable = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = ''
              Install a systemd user timer running `safix rotate --due --yes`
              against `safix.rotation.repository`.

              Off by default, and independent of `safix.enable`: installing
              secrets and re-minting them are different acts on different
              machines. This is the operator's workstation, where the
              declarations and the git identity are; a machine that only
              consumes secrets holds neither.

              The identity it runs under must decrypt without a prompt and
              without a card. A unit has no terminal and no pinentry, so a
              rotation needing either fails the unit with the underlying
              refusal rather than hanging, and the repository is left
              uncommitted.

              The timer commits locally and pushes nothing. Reviewing and
              pushing are yours; machines receive the rotated values on their
              next rebuild.
            '';
          };

          repository = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = "/home/alice/fleet";
            description = ''
              The absolute path of the declaring repository the verb runs in,
              as the unit's `WorkingDirectory`.

              A string rather than a path: a nix path would copy the
              repository into the store at evaluation, and what is wanted is
              the mutable working tree on this machine.

              `safix generate` refuses a dirty tree, so a timer that fires over
              uncommitted work fails visibly and rotates nothing.
            '';
          };

          onCalendar = lib.mkOption {
            type = lib.types.str;
            default = "daily";
            example = "Mon *-*-* 03:00:00";
            description = ''
              When the timer fires, as `OnCalendar` takes it. The unit is
              persistent, so a machine that was asleep at the hour catches up
              on its next boot rather than skipping the interval.
            '';
          };

          environment = lib.mkOption {
            type = lib.types.attrsOf lib.types.str;
            default = { };
            example = lib.literalExpression ''{ SOPS_AGE_KEY_FILE = "/home/alice/.config/sops/age/keys.txt"; }'';
            description = ''
              Environment the rotation unit runs with: the identity the values
              are decrypted and re-encrypted under, and anything a generator's
              own script needs.

              It names no secret value. A key file's path is not its contents,
              and a value written here would be world-readable in the nix
              store.
            '';
          };
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
    # Outside the `cfg.enable` gate, and Linux-only like the install unit: a
    # workstation that rotates values need not install any, and a machine with
    # no user manager has nowhere to put a timer.
    (lib.mkIf (cfg.rotation.enable && pkgs.stdenv.hostPlatform.isLinux) {
      assertions = [
        {
          assertion = cfg.rotation.repository != null;
          message = "safix.rotation.enable is set, but safix.rotation.repository names no repository for `safix rotate --due --yes` to run in.";
        }
      ];

      systemd.user.services.safix-rotate = {
        Unit.Description = "Re-mint every safix value past its rotation deadline";
        Service = {
          Type = "oneshot";
          WorkingDirectory = cfg.rotation.repository;
          Environment = lib.mapAttrsToList (name: value: "${name}=${value}") cfg.rotation.environment;
          # `--yes` answers the cascade confirmation, which is the one decision
          # a unit answers on the operator's behalf: a rotation that stopped to
          # ask would hang a timer rather than fail it.
          ExecStart = "${cfg.installer.package}/bin/safix rotate --due --yes";
        };
      };

      systemd.user.timers.safix-rotate = {
        Unit.Description = "Schedule safix rotation";
        Timer = {
          OnCalendar = cfg.rotation.onCalendar;
          Persistent = true;
        };
        Install.WantedBy = [ "timers.target" ];
      };
    })
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
