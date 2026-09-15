# The installer safix owns at system scope: the manifest safix builds, and the
# program safix ships to read it.
#
# Every value read here is `safix.*` or the host's own. The installer's
# settings are declarations of safix's below rather than another framework's
# namespace read out of the same evaluation, which is what makes
# `nixosModules.safix` importable with no flake in the tree and what keeps a
# consumer's own use of another secrets framework independent of safix: they
# keep every option they set, and it means only what they meant by it.
#
# The two store roots are manifest fields as well as options — `safix install`
# reads `secretsMountPoint` and `symlinkPath` out of the manifest JSON and
# consults no option — and the manifest's whole schema is written down once, as
# `Manifest` in `crates/safix-core/src/install.rs`, which refuses an unknown
# field rather than ignoring it. A field added on this side without that
# struct moving is therefore a failing check rather than a value the installer
# silently drops, which is what the schema snapshot and round-trip checks in
# `modules/flake/checks/installer.nix` hold.
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

  # safix's own activation-path environment: the consumer's
  # `safix.installer.environment`, with HOME set because sops otherwise
  # searches an unset $HOME for ssh keys and warns, and the age plugins put on
  # PATH because an activation script has none. The plugins are the one thing
  # that has to be found by name — sops execs them itself — so PATH carries
  # them and nothing else.
  activationEnvironment = cfg.installer.environment // {
    HOME = "/var/empty";
    PATH = lib.makeBinPath cfg.installer.agePlugins;
  };

  installerCall = ''
    (
    ${lib.concatStringsSep "\n" (
      lib.mapAttrsToList (n: v: "  export ${n}='${v}'") activationEnvironment
    )}
      ${installerScript}
    )
  '';

  # ── the pre-decryption identity check ──
  # The same shape as the user scope's preflight in `./home.nix`, run by the
  # installer script itself so both mechanisms carry it. A keyFile the consumer
  # set is fatal to the installer when unreadable, so it is required on its
  # own; ssh key paths are individually skipped with a line to stderr, so they
  # are load-bearing only collectively and only while they are the sole source.
  #
  # A gnupg configuration counts for nothing. It is not an identity safix can
  # decrypt with — `safix.identity` carries a key file and ssh keys and nothing
  # else — so tolerating one would suppress safix's own refusal on the strength
  # of a configuration safix neither wrote nor reads.
  requiredIdentities = lib.optional (cfg.identity.keyFile != null) {
    path = toString cfg.identity.keyFile;
    origin = "safix.identity.keyFile";
  };

  # The ssh keys this scope decrypts with: the ones a consumer named, or the
  # ones derived from the host's own where they named none. One binding, read
  # by the preflight, by the manifest and by the unit's mount requirements, so
  # those three cannot disagree about what the identity is.
  identitySshKeyPaths =
    if cfg.identity.sshKeyPaths != [ ] then cfg.identity.sshKeyPaths else cfg.identity.derivedHostKeys;

  sshKeyOrigin =
    i:
    if cfg.identity.sshKeyPaths != [ ] then
      "safix.identity.sshKeyPaths[${toString i}]"
    else
      "derived from services.openssh.hostKeys";

  sufficientIdentities = lib.optionals (cfg.identity.keyFile == null) (
    lib.imap0 (i: p: {
      path = toString p;
      origin = sshKeyOrigin i;
    }) identitySshKeyPaths
  );

  note = identity: state: lib.escapeShellArg "${identity.path}  (${state}) — ${identity.origin}";

  record = accumulator: identity: ''
    if [ ! -e ${lib.escapeShellArg identity.path} ]; then
      ${accumulator}+=(${note identity "missing"})
    elif [ ! -r ${lib.escapeShellArg identity.path} ]; then
      ${accumulator}+=(${note identity "present but not readable"})
    fi
  '';

  checkRequired = record "safixIdentityFailures";

  checkSufficient = identity: ''
    ${record "safixIdentityCandidates" identity}
    if [ -r ${lib.escapeShellArg identity.path} ]; then
      safixIdentityUsable=1
    fi
  '';

  preflightRemediation = ''

    ${toString (builtins.length resolvedEntries)} secret(s) are declared to arrive in ${cfg.installer.symlinkPath}, and none
    of the paths above is a readable decryption identity.

    On a host where another secret store places the identity, the usual cause
    is ordering: the foreign store's activation step or unit has not run yet.
    Name what safix must wait for:

      safix.installer.afterActivation = [ "<that store's activation step>" ];
      safix.installer.afterUnits = [ "<that store's unit>" ];

    Presence and readability were checked; decryption was not. A key that
    exists and is readable but is not a recipient of these files still fails
    afterwards, inside safix's own installer, when it decrypts.
  '';

  installerScript = pkgs.writeShellScript "safix-install-secrets" ''
    safixIdentityFailures=()
    safixIdentityCandidates=()
    safixIdentityUsable=0

    ${lib.concatMapStrings checkRequired requiredIdentities}
    ${lib.concatMapStrings checkSufficient sufficientIdentities}
    ${lib.optionalString (sufficientIdentities != [ ]) ''
      if (( safixIdentityUsable == 0 )); then
        safixIdentityFailures+=("''${safixIdentityCandidates[@]}")
      fi
    ''}
    if (( ''${#safixIdentityFailures[@]} > 0 )); then
      echo "safix: no readable decryption identity — secret installation refused" >&2
      printf '%s\n' "" >&2
      printf '  %s\n' "''${safixIdentityFailures[@]}" >&2
      printf '%s\n' ${lib.escapeShellArg preflightRemediation} >&2
      exit 1
    fi

    unset safixIdentityFailures safixIdentityCandidates safixIdentityUsable

    # Both tools by store path rather than through PATH. An activation script
    # inherits no PATH worth relying on, and the backend must be the build
    # safix pinned rather than whatever the caller happens to have: a
    # divergence in creation-rule interpretation or MAC handling is a silently
    # re-encrypted secret.
    export SAFIX_SOPS=${pkgs.sops}/bin/sops
    export SAFIX_SSH_TO_AGE=${pkgs.ssh-to-age}/bin/ssh-to-age

    exec ${cfg.installer.package}/bin/safix install ${manifest}
  '';

  # The option a consumer reads and the set this manifest is built from are one
  # value, not two computations of one contract: `config.safix.secrets` is the
  # whole of what safix reports arrived at this scope, typed by
  # `common.secretEntryType` — safix's own submodule, which is where each
  # entry's twelve manifest fields and their defaults live, including the path
  # the entry arrives at.
  resolvedEntries = builtins.attrValues config.safix.secrets;

  # The documents the manifest names, each once. The installer decrypts per
  # document rather than per entry, and the hash below is per document for the
  # same reason: one audience gets one file, so the set is small by
  # construction and shared by construction.
  distinctSopsFiles = lib.unique (map (secret: secret.sopsFile) resolvedEntries);

  # What replaces the per-entry `sopsFileHash` safix's own type deliberately
  # does not carry: one hash over every distinct document, which is what makes
  # this derivation a function of the ciphertext and so makes editing a
  # document cause a rebuild. The installer never reads the field — `Manifest`
  # deserializes it into an option it ignores — so its only purpose is the
  # derivation hash, and it is emitted only under the same switch that gates
  # the two refusals below, because a consumer who turned validation off has
  # asked not to read the documents at build time at all.
  manifestInputHash =
    if cfg.installer.validate then
      builtins.hashString "sha256" (
        lib.concatMapStrings (file: builtins.hashFile "sha256" file) distinctSopsFiles
      )
    else
      null;

  # safix's own two refusals, under `safix.installer.validate`. They live in
  # the builder rather than in the entry type because the type asks nothing
  # about the filesystem: an entry whose document is absent, or lies outside
  # the nix store, passes the type unchanged. Without this block nothing
  # refuses at all and the installer meets the missing document at activation
  # instead, on a host, where a build is where it should have been caught.
  failedAssertions = builtins.foldl' (
    acc: secret:
    acc
    ++ lib.optional (!builtins.pathExists secret.sopsFile) (
      common.sopsFileMissingMessage {
        inherit (secret) name;
        file = secret.sopsFile;
      }
    )
    ++
      lib.optional
        (
          !builtins.isPath secret.sopsFile
          && !(builtins.isString secret.sopsFile && lib.hasPrefix builtins.storeDir secret.sopsFile)
        )
        (
          common.sopsFileOutsideStoreMessage {
            inherit (secret) name;
            file = secret.sopsFile;
          }
        )
  ) [ ] resolvedEntries;

  manifest =
    if cfg.installer.validate && failedAssertions != [ ] then
      throw "\nFailed assertions:\n${lib.concatStringsSep "\n" (map (x: "- ${x}") failedAssertions)}"
    else
      pkgs.writeTextFile {
        name = "safix-manifest.json";
        text = builtins.toJSON {
          # The schema version `Manifest::validate_version` reads. A manifest
          # naming a version the binary does not know is refused naming both
          # numbers, rather than decoded field by field on a best-effort
          # basis, which is the whole reason the field exists.
          version = 1;

          secrets = resolvedEntries;

          secretsMountPoint = cfg.installer.secretsMountPoint;
          symlinkPath = cfg.installer.symlinkPath;
          keepGenerations = cfg.installer.keepGenerations;
          ageKeyFile = cfg.identity.keyFile;
          ageSshKeyPaths = identitySshKeyPaths;
          useTmpfs = cfg.installer.useTmpfs;

          # System scope mounts its store, chowns what it writes, and
          # propagates restarts. The user scope's manifest says `true` here
          # and gets none of the three, because each needs privileges that
          # scope does not have.
          userMode = false;

          logging = {
            keyImport = builtins.elem "keyImport" cfg.installer.log;
            secretChanges = builtins.elem "secretChanges" cfg.installer.log;
          };

          inherit manifestInputHash;
        };

        # Checked at build time by the same program that will read it, in the
        # mode `safix.installer.validate` selects, mirroring that conditional
        # rather than picking a branch of it. The branch is load-bearing and
        # `manifest` is the tempting, weaker half: it validates the schema, the
        # version, every mode's octal parse and every owner and group
        # resolution, and never opens a document, where `document`
        # additionally opens each named document and verifies every declared
        # key is in it. Neither mode decrypts: a sops document carries its
        # mapping keys in the clear, so key presence is readable without an
        # identity, which is what lets the stronger mode be the default inside
        # a build sandbox that holds none.
        #
        # `--ignore-passwd` because a build sandbox has no user database: a
        # mode that guessed a uid would be validating a resolution it cannot
        # perform, and the switch says so rather than guessing.
        #
        # `validationPackage` rather than `package`: this runs on the build
        # platform where the activation invocation runs on the host's, and
        # collapsing the two would make every cross build fail here.
        checkPhase = ''
          ${cfg.installer.validationPackage}/bin/safix install --check-mode=${
            if cfg.installer.validate then "document" else "manifest"
          } --ignore-passwd "$out"
        '';
      };
in
{
  options.safix.installer = {
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.safix or (throw common.installerPackageMessage);
      defaultText = lib.literalExpression "pkgs.safix";
      description = ''
        The safix build whose `safix install` an activation on this host runs.

        Defaults to `pkgs.safix`, which is what a consumer who has safix in
        their package set already has. A consumer who reaches safix only
        through a flake input sets this to
        `inputs.safix.packages.''${pkgs.stdenv.hostPlatform.system}.safix`:
        this module imports nothing, by contract, so it has no input of its own
        to read a package out of, and a module that guessed one would make
        every consumer's evaluation seam part of safix's interface.
      '';
    };

    validationPackage = lib.mkOption {
      type = lib.types.package;
      default =
        if pkgs.stdenv.buildPlatform == pkgs.stdenv.hostPlatform then
          cfg.installer.package
        else
          pkgs.pkgsBuildBuild.safix or (throw common.installerPackageMessage);
      defaultText = lib.literalExpression "safix.installer.package, or the build-platform build under a cross build";
      description = ''
        The safix build that checks the manifest while the manifest derivation
        is being built.

        Two options rather than one because the two run on different platforms:
        this one runs inside the build, on the build platform, where
        `safix.installer.package` runs at activation, on the host's. They are
        the same value whenever those platforms are, and collapsing them would
        make every cross build fail on a binary it cannot execute.
      '';
    };

    validate = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Whether the manifest is checked against its documents at build time.

        On, this selects `safix install --check-mode=document`, which opens
        each named document and verifies every declared key is in it, and it
        refuses at evaluation a document that does not exist or lies outside
        the nix store. Off, the check is `--check-mode=manifest`, which
        validates the schema, the version, every mode's octal parse and every
        owner and group resolution without opening a document at all.

        Neither mode decrypts. A sops document carries its mapping keys in the
        clear, so key presence is readable without an identity, which is why
        the stronger mode is the default even though a build sandbox holds no
        identity.

        It also governs `manifestInputHash`: on, the manifest carries one hash
        over every distinct document it names, so editing a document changes
        this derivation and causes a rebuild. Off, the field is null and a
        document edit changes nothing here.

        Turn it off for a tree whose documents are not present at evaluation
        time — a fixture naming paths no committed file backs, most often — and
        accept that both refusals then arrive on a host instead.
      '';
    };

    keepGenerations = lib.mkOption {
      type = lib.types.int;
      default = 1;
      description = ''
        How many generation directories under
        `safix.installer.secretsMountPoint` the installer keeps after a
        successful install, newest first. One is enough to answer what is
        installed now; a larger number keeps the previous values readable,
        which is a decision about the store's contents rather than about disk.
        Zero keeps every generation, since there is then nothing to prune to.
      '';
    };

    useTmpfs = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether the secret store is a tmpfs rather than a ramfs.

        ramfs is the default because it cannot be swapped at all. tmpfs is
        mounted with `noswap` where the kernel supports it, and safix falls
        back to a tmpfs without that flag where it does not — which is a
        filesystem a secret can reach swap through, and the reason this is not
        the default.
      '';
    };

    log = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "keyImport"
          "secretChanges"
        ]
      );
      default = [ ];
      example = [ "secretChanges" ];
      description = ''
        Which of the installer's two optional log classes reach the journal:
        `keyImport` reports each identity the installer assembled, and
        `secretChanges` reports each entry that was new or whose value changed
        against the previous generation. Neither names a value.

        Empty by default, because each says something about which secrets exist
        and when they moved to anyone who reads the journal.
      '';
    };

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = {
        SOPS_GPG_EXEC = "/run/current-system/sw/bin/gpg2";
      };
      description = ''
        Extra environment variables the installer runs with, on both the
        activation-script and the unit path. `HOME` and `PATH` are set by safix
        over whatever this names: an unset HOME makes sops search for ssh keys
        and warn, and PATH is the age plugins alone.
      '';
    };

    agePlugins = lib.mkOption {
      type = lib.types.listOf lib.types.package;
      default = [ ];
      example = lib.literalExpression "[ pkgs.age-plugin-yubikey ]";
      description = ''
        age plugin packages the installer puts on PATH.

        They are on PATH rather than referenced by store path because sops
        execs them itself, by the name the recipient string carries, so a store
        path safix computed would never be consulted. Everything else the
        installer runs — the backend and the ssh key converter — is a store
        path and is never taken from PATH.
      '';
    };

    secretsMountPoint = lib.mkOption {
      type = lib.types.str;
      default = "/run/safix.d";
      description = ''
        Where safix's installer keeps its generation directories. A manifest
        field as well as an option — `safix install` reads it out of the
        manifest JSON and consults no option — so this store is safix's own and
        disjoint from any store another component of the host owns.
      '';
    };

    symlinkPath = lib.mkOption {
      type = lib.types.str;
      default = "/run/safix";
      description = ''
        Where safix's installed secrets appear: a symlink to the latest
        generation under `safix.installer.secretsMountPoint`, and the directory
        every resolved entry that declares no path of its own parks under, as
        `''${symlinkPath}/<name>`.

        The root and that per-entry default move together: the installer
        creates a symlink at any entry path that is not
        `<symlinkPath>/<name>`, so a moved root with an unmoved default would
        write safix's symlinks into the other store's directory instead of
        colliding with it.
      '';
    };

    useSystemdActivation = lib.mkOption {
      type = lib.types.bool;
      default =
        (options.systemd ? sysusers && config.systemd.sysusers.enable)
        || (options.services ? userborn && config.services.userborn.enable);
      defaultText = lib.literalExpression (
        "(options.systemd ? sysusers && config.systemd.sysusers.enable) "
        + "|| (options.services ? userborn && config.services.userborn.enable)"
      );
      description = ''
        Whether safix installs its secrets from a systemd unit rather than
        from an activation script. Defaults from the host's own
        user-management options, because which mechanism can install secrets
        early enough is a fact about the host rather than a preference: where
        systemd-sysusers or userborn create users, the users exist only once a
        unit has run, so an activation script is too early.

        No switch belonging to another installer is consulted to decide it.
        Such a switch is global to every consumer of that installer in the
        tree, so reading it would couple safix's mechanism to a setting made
        about a different program.
      '';
    };

    afterActivation = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "setupSecrets" ];
      description = ''
        Activation steps safix's installer runs after, on a host using
        activation-script installation. Named by the consumer: safix reads no
        option of another secret store to discover its installer, so ordering
        against one is stated here. Naming nothing is supported and leaves
        the installer unordered.
      '';
    };

    afterUnits = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "age-decrypt-secrets.service" ];
      description = ''
        Units safix's installer runs after, on a host using systemd-unit
        installation. The unit-mechanism counterpart of
        `safix.installer.afterActivation`, with the same contract.
      '';
    };
  };

  options.safix.identity.deriveHostKeys = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = ''
      Whether the system scope, given no named identity, derives one from the
      ed25519 entries of `services.openssh.hostKeys` whose paths lie outside
      safix's own secret store.

      The exclusion prefix is `safix.installer.symlinkPath` rather than any
      other store's root, because the exclusion exists to avoid decrypting
      with a key this installer itself deploys, and that is a statement about
      this package's store. A host key another store placed under
      `/run/secrets` before safix runs is exactly the identity safix should
      decrypt with. An identity named through `safix.identity.sshKeyPaths` or
      `safix.identity.keyFile` always wins, and turning this off makes the
      derivation contribute nothing.
    '';
  };

  # The derivation itself, as a read-only option rather than a `let` binding,
  # because two files need the same list: this one builds the manifest and the
  # unit's mount requirements from it, and `./nixos.nix` asks whether anything
  # can decrypt at all while `safix.secrets` is being forced. One declaration
  # is what keeps the two from disagreeing about what the identity is.
  #
  # A default rather than a definition, so it is computable while the option
  # tree is being forced and carries no `mkIf` that could make it depend on
  # `safix.enable` — whose own default reads `safix.secrets`, which is where
  # this is read from.
  options.safix.identity.derivedHostKeys = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    readOnly = true;
    internal = true;
    default =
      if cfg.identity.deriveHostKeys && config.services.openssh.enable then
        map (e: e.path) (
          lib.filter (
            e: e.type == "ed25519" && !(lib.hasPrefix cfg.installer.symlinkPath e.path)
          ) config.services.openssh.hostKeys
        )
      else
        [ ];
    defaultText = lib.literalExpression "the ed25519 entries of config.services.openssh.hostKeys outside safix's own store";
    description = ''
      The ssh host keys `safix.identity.deriveHostKeys` yields on this host:
      read-only, and what `safix.identity.sshKeyPaths` falls back to where a
      consumer named none.
    '';
  };

  config = lib.mkIf cfg.enable {
    # One concrete attribute per artifact, so a check reads what a host would
    # run rather than rebuilding the derivation each time. The installer
    # script is exposed beside the manifest so a check can hold the
    # preflight's text without running it.
    system.build.safix-manifest = manifest;
    system.build.safix-installer = installerScript;

    # The name is load-bearing. Other secret installers register themselves as
    # `setupSecrets`, and two definitions of one step name merge into a single
    # node whose halves run in definition order — one node in the activation
    # DAG has no edge to state. An entry named `safixInstallSecrets` is its own
    # node, which is what makes the consumer-named ordering below expressible
    # at all.
    system.activationScripts.safixInstallSecrets = lib.mkIf (!cfg.installer.useSystemdActivation) (
      lib.stringAfter
        (
          [
            "specialfs"
            "users"
            "groups"
          ]
          ++ cfg.installer.afterActivation
        )
        ''
          [ -e /run/current-system ] || echo installing safix secrets...
          ${installerCall}
        ''
      // lib.optionalAttrs (config.system ? dryActivationScript) {
        supportsDryActivation = true;
      }
    );

    # The unit form, including its `sysinit-reactivation.target` relationship,
    # so the unit re-runs on a `nixos-rebuild switch` rather than only at
    # boot.
    systemd.services.safix-install-secrets = lib.mkIf cfg.installer.useSystemdActivation {
      wantedBy = [ "sysinit.target" ];
      after = [
        "local-fs.target"
        "systemd-sysusers.service"
        "userborn.service"
      ]
      ++ cfg.installer.afterUnits;
      requiredBy = [ "sysinit-reactivation.target" ];
      before = [ "sysinit-reactivation.target" ];
      environment = cfg.installer.environment // {
        SOPS_RESTART_UNITS_VIA_SYSTEMCTL = "1";
      };
      path = cfg.installer.agePlugins;

      serviceConfig = {
        Type = "oneshot";
        ExecStart = [ "${installerScript}" ];
        RemainAfterExit = true;
      };
      unitConfig = {
        DefaultDependencies = "no";
        RequiresMountsFor = lib.concatLists [
          (lib.lists.optional (cfg.identity.keyFile != null) cfg.identity.keyFile)
          identitySshKeyPaths
        ];
      };
    };
  };
}
