# Shared consumer types and deployment helpers. Recipient policy stays in the registry.
{ lib, scope }:
let
  inherit (lib) mkOption types;
  scopeNoun = if scope == "system" then "system configuration" else "home-manager profile";
  absolutePath = types.addCheck (types.either types.str types.path) (
    p: lib.hasPrefix "/" (toString p)
  );
  privatePath = p: lib.hasPrefix "/" (toString p) && !(lib.hasPrefix builtins.storeDir (toString p));
  missingLibMessage = "safix: safix.flake was set to a value carrying no safix.lib. Import the safix flake module or set safix.lib directly.";
  flakelessMessage = "safix: this ${scopeNoun} explicitly requests a registry binding but safix.lib is null. Set safix.flake or safix.lib.";
  violationMessage =
    cfg:
    "safix: the declarations this ${scopeNoun} is bound to do not resolve:\n"
    + lib.concatMapStrings (v: "  - ${v}\n") cfg.lib.violations;
  noIdentityMessage =
    { cfg, resolved }:
    "safix: this home-manager profile has ${toString (builtins.length (builtins.attrNames resolved))} secrets but no decryption identity. Set safix.identity.keyFile, safix.identity.sshKeyPaths or safix.identity.gnupgHome.";
  noSystemIdentityMessage =
    { cfg, resolved }:
    "safix: this system has ${toString (builtins.length (builtins.attrNames resolved))} secrets but no configured or derivable decryption identity. Set safix.identity.keyFile, safix.identity.sshKeyPaths or safix.identity.gnupgHome, or enable host key derivation.";
  installerPackageMessage = "safix: safix.installer.package is unset and pkgs.safix does not exist. Set the package explicitly; under cross compilation also set validationPackage.";
  sopsFileMissingMessage =
    { name, file }:
    "safix: cannot find '${toString file}', the ciphertext file of resolved entry '${name}'";
  sopsFileOutsideStoreMessage =
    { name, file }:
    "safix: '${toString file}', the ciphertext file of resolved entry '${name}', is not in the Nix store. Add it to the Nix store or set safix.installer.validate to false";
  wasSet = option: option.highestPrio < (lib.mkOptionDefault null).priority;

  deploymentOptions = {
    mode = mkOption {
      type = types.str;
      default = "0400";
      description = "Installed permission bits, as an octal string.";
    };
    owner = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Owner name, or null to use uid. User installs reject ownership overrides.";
    };
    group = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Group name, or null to use gid. User installs reject ownership overrides.";
    };
    uid = mkOption {
      type = types.int;
      default = 0;
      description = "Numeric owner when owner is null.";
    };
    gid = mkOption {
      type = types.int;
      default = 0;
      description = "Numeric group when group is null.";
    };
    restartUnits = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Units restarted when the installed value changes; user installations use systemctl --user.";
    };
    reloadUnits = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Units reloaded when the installed value changes; user installations use systemctl --user.";
    };
  };
  secretEntryType =
    { cfg }:
    types.submodule (
      { config, name, ... }: {
        options = deploymentOptions // {
          name = mkOption {
            type = types.str;
            default = name;
            description = "Entry name inside the generation.";
          };
          key = mkOption {
            type = types.str;
            default = config.name;
            description = "Slash-separated document key; an empty string selects the whole document.";
          };
          path = mkOption {
            type = types.str;
            default = "${cfg.installer.symlinkPath}${lib.optionalString config.neededForUsers "-for-users"}/${config.name}";
            defaultText = lib.literalMD "the entry name below symlinkPath, or symlinkPath-for-users for early entries";
            description = "Installed path. Explicit paths are preserved.";
          };
          sopsFile = mkOption {
            type = types.path;
            description = "Encrypted source document. Ciphertext, unlike private identities, may be stored in the Nix store.";
          };
          format = mkOption {
            type = types.enum [
              "yaml"
              "json"
              "dotenv"
              "ini"
              "binary"
              "age"
            ];
            default = "yaml";
            description = "Explicit ciphertext format; age selects a raw age document.";
          };
          neededForUsers = mkOption {
            type = types.bool;
            default = false;
            description = "Install before user creation in an independent root-only store. Unsupported at home scope.";
          };
        };
      }
    );
  templateType =
    { cfg }:
    types.submodule (
      { name, ... }: {
        options = deploymentOptions // {
          content = mkOption {
            type = types.lines;
            description = "Public template text containing safix.placeholder tokens. Never put plaintext secrets here: this text enters the Nix store. Substitution happens only at runtime.";
          };
          path = mkOption {
            type = types.str;
            default = "${cfg.installer.symlinkPath}/${name}";
            description = "Installed rendered-template path.";
          };
        };
      }
    );
  entryManifest = entry: {
    inherit (entry)
      name
      key
      path
      mode
      owner
      group
      uid
      gid
      sopsFile
      format
      restartUnits
      reloadUnits
      neededForUsers
      ;
  };
  templateManifests =
    cfg:
    lib.mapAttrsToList (name: t: {
      inherit name;
      inherit (t)
        content
        path
        mode
        owner
        group
        uid
        gid
        restartUnits
        reloadUnits
        ;
    }) cfg.templates;
  noOwnership = e: e.owner == null && e.group == null && e.uid == 0 && e.gid == 0;
  rootOnly =
    e:
    (e.owner == null || e.owner == "root")
    && (e.group == null || e.group == "root")
    && e.uid == 0
    && e.gid == 0
    && builtins.match "0?[0-7]00" e.mode != null;

  # Validate public deployment metadata without rendering any template.
  deploymentErrors =
    cfg:
    let
      entries = builtins.attrValues cfg.secrets;
      templates = templateManifests cfg;
      early = lib.filter (e: e.neededForUsers) entries;
      earlyReferences = lib.concatMap (
        t:
        lib.map (
          e:
          "safix: template '${t.name}' references early secret '${e.name}'. Templates may reference only normal secrets; early and normal stores cannot be mixed."
        ) (lib.filter (e: lib.hasInfix "<safix:${e.name}>" t.content) early)
      ) templates;
      names = map (e: e.name) entries;
    in
    lib.optional (
      builtins.length names != builtins.length (lib.unique names)
    ) "safix: resolved secret names must be unique."
    ++ lib.optional (lib.any (
      t: builtins.elem t.name names
    ) templates) "safix: secret and template names must not collide."
    ++ earlyReferences
    ++ (
      if scope == "system" then
        lib.map (
          e: "safix: early secret '${e.name}' must be root-owned with no group or other permission bits."
        ) (lib.filter (e: !rootOnly e) early)
      else
        lib.map (e: "safix: home secret '${e.name}' cannot request neededForUsers or ownership.") (
          lib.filter (e: e.neededForUsers || !noOwnership e) entries
        )
        ++ lib.map (t: "safix: home template '${t.name}' cannot request ownership.") (
          lib.filter (t: !noOwnership t) templates
        )
    );

  sharedOptions =
    {
      cfg,
      userDefault,
      userDefaultText,
      hostnameDefault,
      hostnameDefaultText,
      secretsType,
    }:
    {
      enable = mkOption {
        type = types.bool;
        default = cfg.secrets != { } || cfg.templates != { };
        defaultText = lib.literalExpression "config.safix.secrets != { } || config.safix.templates != { }";
        description = "Whether to install the resolved secrets and public templates.";
      };
      flake = mkOption {
        type = types.nullOr types.raw;
        default = null;
        description = "Consumer flake publishing safix.lib. Leave null for imported-only deployments or supply lib directly.";
      };
      lib = mkOption {
        type = types.nullOr types.raw;
        default = if cfg.flake == null then null else cfg.flake.safix.lib or (throw missingLibMessage);
        defaultText = lib.literalExpression "config.safix.flake.safix.lib";
        description = "Registry resolver projection.";
      };
      user = mkOption {
        type = types.nullOr types.str;
        default = if cfg.machine == null then userDefault else null;
        defaultText = userDefaultText;
        description = "Registry user selected by this profile, mutually exclusive with machine.";
      };
      machine = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Registry machine selected instead of a user. A machine needs no hostname.";
      };
      hostname = mkOption {
        type = types.nullOr types.str;
        default = hostnameDefault;
        defaultText = hostnameDefaultText;
        description = "Hostname used for a registry user's host-scoped resolution.";
      };
      tags = mkOption {
        type = types.listOf types.str;
        default =
          if cfg.machine == null || cfg.lib == null then
            [ ]
          else
            cfg.lib.subjects.machines.${cfg.machine}.tags;
        defaultText = lib.literalMD "the selected machine's declared tags, or an empty list";
        description = "Tags used by registry per-tag selection.";
      };
      importedSecrets = mkOption {
        type = types.attrsOf (secretEntryType {
          inherit cfg;
        });
        default = { };
        description = "Explicit encrypted sources, including migration-produced declarations. Merged with the registry resolution; colliding entries are refused. This does not declare recipient policy.";
      };
      secrets = mkOption {
        type = secretsType;
        readOnly = true;
        description = "Resolved registry entries merged once with importedSecrets, with deployment defaults applied.";
      };
      templates = mkOption {
        type = types.attrsOf (templateType {
          inherit cfg;
        });
        default = { };
        description = "Public templates rendered with secrets only in private runtime state.";
      };
      placeholder = mkOption {
        type = types.attrsOf types.str;
        readOnly = true;
        default = lib.listToAttrs (
          map (e: {
            name = e.name;
            value = "<safix:${e.name}>";
          }) (builtins.attrValues cfg.secrets)
        );
        defaultText = lib.literalMD "a runtime placeholder for every resolved secret name";
        description = "Literal runtime placeholder tokens. Nix never substitutes secret plaintext.";
      };
      identity = {
        keyFile = mkOption {
          type = types.nullOr absolutePath;
          default = null;
          description = "Absolute private age identity path, outside the Nix store.";
        };
        sshKeyPaths = mkOption {
          type = types.listOf types.str;
          default = [ ];
          description = "Absolute private SSH identity paths, outside the Nix store. Unreadable individual SSH keys are skipped.";
        };
        gnupgHome = mkOption {
          type = types.nullOr absolutePath;
          default = null;
          description = "Private GnuPG home directory, outside the Nix store. If set, it must be a readable directory at activation.";
        };
      };
    };

  resolvedFor =
    { cfg, target }:
    let
      unbound = cfg.lib == null || (cfg.user == null && cfg.machine == null);
      unaddressed = cfg.machine == null && cfg.hostname == null;
      registry =
        if cfg.lib != null && cfg.lib.violations != [ ] then
          throw (violationMessage cfg)
        else if unbound || unaddressed || (cfg.user != null && cfg.machine != null) then
          { }
        else
          cfg.lib.materialize {
            inherit (cfg)
              user
              machine
              hostname
              tags
              ;
            inherit scope;
          } target;
      collisions = builtins.attrNames (builtins.intersectAttrs registry cfg.importedSecrets);
    in
    if collisions != [ ] then
      throw "safix: importedSecrets collide with registry entries: ${lib.concatStringsSep ", " collisions}"
    else
      registry // cfg.importedSecrets;

  assertionsFor = { cfg, configured }: [
    {
      assertion = cfg.lib != null || !configured;
      message = flakelessMessage;
    }
    {
      assertion = cfg.flake == null || cfg.flake ? safix.lib;
      message = missingLibMessage;
    }
    {
      assertion = cfg.lib == null || cfg.user != null || cfg.machine != null;
      message = "safix: the registry binding needs safix.user or safix.machine.";
    }
    {
      assertion = cfg.user == null || cfg.machine == null;
      message = "safix: select only one of safix.user and safix.machine.";
    }
    {
      assertion = cfg.lib == null || cfg.machine != null || cfg.hostname != null;
      message = "safix: a registry user binding requires safix.hostname.";
    }
    {
      assertion = lib.all privatePath (
        cfg.identity.sshKeyPaths
        ++ lib.optional (cfg.identity.keyFile != null) cfg.identity.keyFile
        ++ lib.optional (cfg.identity.gnupgHome != null) cfg.identity.gnupgHome
      );
      message = "safix: private identity paths must be absolute and must never be in the Nix store.";
    }
  ];

  # Presence/readability only: this deliberately does not decrypt during preflight.
  identityPreflight =
    {
      cfg,
      sshKeyPaths,
      generateKey ? false,
    }:
    let
      key = cfg.identity.keyFile;
      gpg = cfg.identity.gnupgHome;
      q = lib.escapeShellArg;
    in
    ''
      safixIdentityFailed=0
      ${lib.optionalString (key != null && !generateKey) ''
        if [ ! -f ${q (toString key)} ] || [ ! -r ${q (toString key)} ]; then
          printf '%s\n' ${q "safix: safix.identity.keyFile is not a readable file: ${toString key}"} >&2
          safixIdentityFailed=1
        fi
      ''}
      ${lib.optionalString (gpg != null) ''
        if [ ! -d ${q (toString gpg)} ] || [ ! -r ${q (toString gpg)} ]; then
          printf '%s\n' ${q "safix: safix.identity.gnupgHome is not a readable directory: ${toString gpg}"} >&2
          safixIdentityFailed=1
        fi
      ''}
      ${lib.optionalString (key == null && gpg == null) ''
        safixIdentityUsable=0
        ${lib.concatMapStrings (p: ''
          if [ -f ${q p} ] && [ -r ${q p} ]; then safixIdentityUsable=1; fi
        '') sshKeyPaths}
        if [ "$safixIdentityUsable" = 0 ]; then
          echo 'safix: no readable SSH identity; check provisioning and installer ordering' >&2
          safixIdentityFailed=1
        fi
        unset safixIdentityUsable
      ''}
      if [ "$safixIdentityFailed" != 0 ]; then
        echo 'safix: secret installation refused. Only presence and readability were checked, not decryption.' >&2
        exit 1
      fi
      unset safixIdentityFailed
    '';

  installerOptions = { cfg, pkgs }: {
    package = mkOption {
      type = types.package;
      default = pkgs.safix or (throw installerPackageMessage);
      defaultText = lib.literalExpression "pkgs.safix";
      description = "Safix package used at activation.";
    };
    validationPackage = mkOption {
      type = types.package;
      default =
        if pkgs.stdenv.buildPlatform == pkgs.stdenv.hostPlatform then
          cfg.installer.package
        else
          pkgs.pkgsBuildBuild.safix or (throw installerPackageMessage);
      defaultText = lib.literalMD "the installer package, or the build-platform safix package for cross builds";
      description = "Build-platform package used to validate manifests.";
    };
    validate = mkOption {
      type = types.bool;
      default = true;
      description = "Validate ciphertext documents and include their input hash. If false, validate only manifest metadata.";
    };
    keepGenerations = mkOption {
      type = types.int;
      default = 1;
      description = "Number of generations retained; zero retains all.";
    };
    log = mkOption {
      type = types.listOf (
        types.enum [
          "keyImport"
          "secretChanges"
        ]
      );
      default = [ ];
      description = "Optional installer logging classes; neither logs secret values.";
    };
    secretsMountPoint = mkOption {
      type = types.str;
      default = if scope == "system" then "/run/safix.d" else "%r/safix.d";
      description = "Generation root. At home scope the installer expands %r to the session runtime directory.";
    };
    symlinkPath = mkOption {
      type = types.str;
      default = if scope == "system" then "/run/safix" else "%r/safix";
      description = "Current-generation symlink and default parent of installed paths.";
    };
    environment = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Installer environment. Safix supplies HOME, tool paths and a PATH including age plugins.";
    };
    agePlugins = mkOption {
      type = types.listOf types.package;
      default = [ ];
      description = "Age plugin packages available to the installer and crypto subprocesses.";
    };
  };

  toolEnvironment =
    { cfg, pkgs }:
    let
      gnupg = pkgs.gnupg.override {
        guiSupport = true;
        pinentry = pkgs.pinentry-curses;
      };
    in
    {
      HOME = if scope == "system" then "/var/empty" else (cfg.installer.environment.HOME or "/var/empty");
      PATH = lib.makeBinPath (
        cfg.installer.agePlugins
        ++ [
          pkgs.age
          gnupg
          pkgs.sops
          pkgs.ssh-to-age
          pkgs.coreutils
        ]
        ++ lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.systemd
      );
      SAFIX_SOPS = "${pkgs.sops}/bin/sops";
      SAFIX_SSH_TO_AGE = "${pkgs.ssh-to-age}/bin/ssh-to-age";
      SAFIX_AGE = "${pkgs.age}/bin/age";
      SAFIX_AGE_KEYGEN = "${pkgs.age}/bin/age-keygen";
      SAFIX_GPG = "${gnupg}/bin/gpg";
      SAFIX_GPGCONF = "${gnupg}/bin/gpgconf";
    }
    // cfg.installer.environment;
  exportEnvironment =
    environment:
    lib.concatStringsSep "\n" (
      lib.mapAttrsToList (
        n: v:
        if builtins.match "[A-Za-z_][A-Za-z0-9_]*" n == null then
          throw "safix: invalid installer environment variable name '${n}'"
        else if lib.hasPrefix "SAFIX_" n || n == "SOPS_GPG_EXEC" then
          "if [ -z \"\${${n}:-}\" ]; then export ${n}=${lib.escapeShellArg v}; fi"
        else
          "export ${n}=${lib.escapeShellArg v}"
      ) environment
    );

  makeManifest =
    {
      cfg,
      pkgs,
      name,
      entries,
      templates,
      sshKeyPaths,
      early ? false,
    }:
    let
      errors = deploymentErrors cfg;
      files = lib.unique (map (e: e.sopsFile) entries);
      fileErrors = lib.concatMap (
        e:
        lib.optional (!builtins.pathExists e.sopsFile) (sopsFileMissingMessage {
          inherit (e) name;
          file = e.sopsFile;
        })
        ++
          lib.optional
            (!(builtins.isPath e.sopsFile || lib.hasPrefix "${builtins.storeDir}/" (toString e.sopsFile)))
            (sopsFileOutsideStoreMessage {
              inherit (e) name;
              file = e.sopsFile;
            })
      ) entries;
      failures = errors ++ lib.optionals cfg.installer.validate fileErrors;
      suffix = lib.optionalString early "-for-users";
    in
    if failures != [ ] then
      throw "safix deployment refused:\n${lib.concatStringsSep "\n" failures}"
    else
      pkgs.writeTextFile {
        inherit name;
        text = builtins.toJSON (
          {
            version = 1;
            secrets = map entryManifest entries;
            inherit templates;
            secretsMountPoint = "${cfg.installer.secretsMountPoint}${suffix}";
            symlinkPath = "${cfg.installer.symlinkPath}${suffix}";
            keepGenerations = cfg.installer.keepGenerations;
            ageKeyFile = if cfg.identity.keyFile == null then null else toString cfg.identity.keyFile;
            ageSshKeyPaths = sshKeyPaths;
            useTmpfs = if scope == "system" then cfg.installer.useTmpfs else false;
            userMode = scope != "system";
            logging = {
              keyImport = builtins.elem "keyImport" cfg.installer.log;
              secretChanges = builtins.elem "secretChanges" cfg.installer.log;
            };
            manifestInputHash =
              if cfg.installer.validate then
                builtins.hashString "sha256" (lib.concatMapStrings (file: builtins.hashFile "sha256" file) files)
              else
                null;
          }
          // lib.optionalAttrs (cfg.identity.gnupgHome != null) {
            gnupgHome = toString cfg.identity.gnupgHome;
          }
        );
        checkPhase = ''
          ${cfg.installer.validationPackage}/bin/safix install --check-mode=${
            if cfg.installer.validate then "document" else "manifest"
          } --ignore-passwd "$out"
        '';
      };
in
{
  inherit
    missingLibMessage
    flakelessMessage
    violationMessage
    noIdentityMessage
    noSystemIdentityMessage
    installerPackageMessage
    sopsFileMissingMessage
    sopsFileOutsideStoreMessage
    wasSet
    secretEntryType
    sharedOptions
    resolvedFor
    assertionsFor
    entryManifest
    templateManifests
    deploymentErrors
    identityPreflight
    installerOptions
    toolEnvironment
    exportEnvironment
    makeManifest
    privatePath
    ;
}
