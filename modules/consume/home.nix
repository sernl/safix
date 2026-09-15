# safix in a home-manager profile: which resolved set arrives here, how this
# machine opens it, and the installation of it.
#
# This module imports nothing, and `homeModules.safix` and `homeModules.default`
# are one value: both names stay published so every existing `imports` line
# keeps resolving.
#
# ── what this scope installs, and what it does not ──
# A user-mode manifest, a `safix install` invocation of safix's own program, a
# user unit on linux, and a home activation entry on both platforms. The store
# roots are runtime-directory-relative — `%r/safix.d` and `%r/safix` — and `%r`
# is expanded by the installer rather than by nix, against `$XDG_RUNTIME_DIR` on
# linux and `getconf DARWIN_USER_TEMP_DIR` on darwin, because nix cannot know a
# session's runtime directory and a path baked at evaluation would be wrong for
# every session but one.
#
# Three things a user-mode install does not do: it mounts no filesystem, chowns
# nothing, and propagates no restart or reload. Each needs a privilege this scope
# does not have, and each is a `userMode` branch in the installer rather than an
# omission here — which is why the manifest says `userMode = true` instead of
# this file leaving fields out.
#
# ── the activation guard ──
# `home.activation.safixInstall` is registered with
# `lib.hm.dag.entryAfter [ "writeBoundary" ]` rather than as a bare string: a
# bare string becomes `entryAnywhere`, which gives home-manager no ordering to
# reason about, and installation must follow the write boundary because the unit
# it belongs beside is materialized by `linkGeneration`.
#
# `home.activation.safixIdentityPreflight` sorts earlier still, ahead of
# `checkLinkTargets`, where refusing is atomic: nothing has been linked, no
# unit restarted, no secret written. It is read-only, and the ordering is the
# whole of what its failure message claims — `safix-consumption-ordering` holds
# it against a real evaluation of a profile rather than against this comment.
#
# The limit that message states beside the guarantee — that presence and
# readability are all it checked, and an identity which has both and is not a
# recipient still fails afterwards — is `safix-identity-recipiency`, against
# fixture ciphertext. It is the one sentence on this path that an evaluation
# cannot hold.
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

  # safix's own installer aborts on a set-but-unreadable keyFile, naming the
  # path, unless `safix.identity.generateKey` creates it first — which is why
  # that switch is the one thing that makes a named key file not yet required.
  requiredIdentities = lib.optional (cfg.identity.keyFile != null && !cfg.identity.generateKey) {
    path = toString cfg.identity.keyFile;
    origin = "safix.identity.keyFile";
  };

  # Each ssh key path is individually skipped with a line to stderr when absent
  # or unconvertible, so these are load-bearing only collectively, and only
  # while they are the sole identity source.
  sufficientIdentities = lib.optionals (cfg.identity.keyFile == null) (
    lib.imap0 (i: p: {
      path = toString p;
      origin = "safix.identity.sshKeyPaths[${toString i}]";
    }) cfg.identity.sshKeyPaths
  );

  guardedIdentities = requiredIdentities ++ sufficientIdentities;

  # Whether anything at all can open a file here. safix's own two options and
  # nothing else: a gnupg configuration belonging to another framework used to
  # count here as evidence that something could decrypt, which suppressed this
  # refusal on the strength of a configuration safix neither wrote nor can use.
  # There is also nothing to derive at this scope — a person is not a host — so
  # naming neither option is what this refuses.
  hasIdentity = cfg.identity.keyFile != null || cfg.identity.sshKeyPaths != [ ];

  note =
    identity: state: lib.escapeShellArg "${identity.path}  (${state}) — declared by ${identity.origin}";

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

  remediation = ''

    ${toString (builtins.length (lib.attrNames cfg.secrets))} secret(s) resolve for ${config.home.username} here, and safix has no
    other way to decrypt them: this scope's identity is whatever
    safix.identity.keyFile and safix.identity.sshKeyPaths name, and nothing
    else counts.

    Home activation stopped before anything was applied. This check sorts ahead of
    checkLinkTargets, so no home file was linked, no user package installed, no
    user unit restarted and no secret written; the previous home generation is
    the one still in place.

    That is the whole of the guarantee, and it is narrower than it sounds twice
    over. Where home activation runs as a NixOS host's
    home-manager-${config.home.username}.service, systemd starts that unit after
    system activation has already switched the system generation, so a system
    switch is not undone by this refusal — only this user's home generation is
    held back. And presence and readability are all that were checked: a key that
    exists and is readable but is not a recipient of these files still fails
    later, inside safix's own installer, when it decrypts.

    Resolve by either
      - placing a readable identity at the path above (mode 0600) and re-running
        the switch, or
      - removing this person's declarations for this host in flake.safix.users.
  '';

  identityPreflight = ''
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
      errorEcho "safix: no usable decryption identity for ${config.home.username} on $(${pkgs.coreutils}/bin/uname -n) — switch refused" >&2
      printf '%s\n' "" >&2
      printf '  %s\n' "''${safixIdentityFailures[@]}" >&2
      printf '%s\n' ${lib.escapeShellArg remediation} >&2
      exit 1
    fi

    unset safixIdentityFailures safixIdentityCandidates safixIdentityUsable
  '';

  # ── the user-mode manifest ──
  # The same schema the system scope emits, against the one definition of it in
  # `crates/safix-core/src/install.rs`: every field is present and
  # `deny_unknown_fields` refuses a thirteenth, so this scope cannot drift into
  # a dialect of its own.
  #
  # Every entry's twelve fields are filled here rather than by a type, because
  # this scope's `safix.secrets` is the untyped projection: a second typing pass
  # would restate the same defaults `common.secretEntryType` already carries at
  # system scope. Ownership is null and numerically zero on both axes — a
  # user-mode install chowns nothing, and the resolver refuses an ownership
  # field at this scope outright.
  manifestSecrets = lib.mapAttrsToList (name: entry: {
    inherit name;
    key = entry.key or name;
    path = entry.path or "${cfg.installer.symlinkPath}/${name}";
    mode = entry.mode;
    owner = null;
    group = null;
    uid = 0;
    gid = 0;
    inherit (entry) sopsFile;
    format = "yaml";
    restartUnits = [ ];
    reloadUnits = [ ];
  }) cfg.secrets;

  distinctSopsFiles = lib.unique (map (secret: secret.sopsFile) manifestSecrets);

  # One hash over every distinct document, which is what makes this derivation
  # a function of the ciphertext and so makes editing a document cause a
  # rebuild. The installer never reads the field; its only purpose is this
  # derivation's own hash, and it is null where validation is off because a
  # consumer who turned that off has asked not to read the documents at build
  # time at all.
  manifestInputHash =
    if cfg.installer.validate then
      builtins.hashString "sha256" (
        lib.concatMapStrings (file: builtins.hashFile "sha256" file) distinctSopsFiles
      )
    else
      null;

  # The same two refusals the system scope carries, for the same reason and
  # under the same switch: safix's own entry shape asks nothing about the
  # filesystem, so a document that is absent or outside the nix store would
  # otherwise be met by `builtins.hashFile` below, or by the installer on the
  # person's own machine, rather than by a message naming the entry.
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
  ) [ ] manifestSecrets;

  manifest =
    if cfg.installer.validate && failedAssertions != [ ] then
      throw "\nFailed assertions:\n${lib.concatStringsSep "\n" (map (x: "- ${x}") failedAssertions)}"
    else
      pkgs.writeTextFile {
        name = "safix-user-manifest.json";
        text = builtins.toJSON {
          version = 1;
          secrets = manifestSecrets;
          secretsMountPoint = cfg.installer.secretsMountPoint;
          symlinkPath = cfg.installer.symlinkPath;
          keepGenerations = cfg.installer.keepGenerations;
          ageKeyFile = cfg.identity.keyFile;
          ageSshKeyPaths = cfg.identity.sshKeyPaths;

          # No mount happens at this scope at all, so the choice between ramfs and
          # tmpfs has nothing to choose between; the field is carried because the
          # schema is one definition for both scopes.
          useTmpfs = false;

          userMode = true;

          logging = {
            keyImport = builtins.elem "keyImport" cfg.installer.log;
            secretChanges = builtins.elem "secretChanges" cfg.installer.log;
          };

          inherit manifestInputHash;
        };

        # Checked at build time by the same program that will read it, in the mode
        # `safix.installer.validate` selects, mirroring the system scope's own
        # check phase. `validationPackage` because this runs inside the build, on
        # the build platform, where the activation runs on the host's.
        checkPhase = ''
          ${cfg.installer.validationPackage}/bin/safix install --check-mode=${
            if cfg.installer.validate then "document" else "manifest"
          } --ignore-passwd "$out"
        '';
      };

  # Minting the key file at activation, where the profile asked for it and the
  # file is not there. `age-keygen` is reached through `SAFIX_AGE_KEYGEN` — the
  # same override every external program the runtime drives has — defaulted to
  # the build safix pinned, so a hermetic check can point it at a build of its
  # own and observe that the override is read rather than merely declared.
  keyGeneration = lib.optionalString (cfg.identity.generateKey && cfg.identity.keyFile != null) ''
    export SAFIX_AGE_KEYGEN="''${SAFIX_AGE_KEYGEN:-${pkgs.age}/bin/age-keygen}"
    if [ ! -e ${lib.escapeShellArg cfg.identity.keyFile} ]; then
      mkdir -p ${lib.escapeShellArg (builtins.dirOf cfg.identity.keyFile)}
      "$SAFIX_AGE_KEYGEN" -o ${lib.escapeShellArg cfg.identity.keyFile}
    fi
  '';

  # One script, run by both registrations, so the unit and the activation entry
  # cannot install differently.
  #
  # The backend and the ssh key converter are store paths rather than names on
  # PATH: a user unit inherits a session's PATH, and the backend must be the
  # build safix pinned rather than whatever the session has, because a
  # divergence in creation-rule interpretation or MAC handling is a silently
  # re-encrypted secret.
  installScript = pkgs.writeShellScript "safix-install-secrets-user" ''
    set -euo pipefail

    ${keyGeneration}

    export SAFIX_SOPS=${pkgs.sops}/bin/sops
    export SAFIX_SSH_TO_AGE=${pkgs.ssh-to-age}/bin/ssh-to-age

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

        # Known only where home-manager is evaluated as a NixOS module, which is the
        # only seam that hands a home-manager profile the host it is on. Standalone
        # there is no honest default, so the assertion asks for one.
        hostnameDefault =
          if osConfig == null || !(osConfig ? networking) then null else osConfig.networking.hostName;
        hostnameDefaultText = lib.literalExpression "osConfig.networking.hostName, where home-manager is a NixOS module; null standalone";

        # The untyped projection at this scope. The user-mode manifest below is
        # where each entry's twelve fields are filled, so typing here would apply
        # the same defaults twice and nothing downstream would read the second
        # application.
        secretsType = lib.types.attrsOf lib.types.raw;
      })
      {
        identityPreflight = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = ''
            Whether to install the read-only identity check that sorts ahead of
            `checkLinkTargets` and refuses the switch when no configured identity is
            present and readable.

            On by default. It exists because the install entry itself can only be
            registered after the write boundary — the unit it belongs beside is
            materialized by `linkGeneration` — and a refusal there is no longer
            atomic: the generation is already half-applied.

            It checks presence and readability and nothing further. It does not
            decrypt, so a secret declared here but absent from the sops document
            still fails later, inside the installer.
          '';
        };

        identity.generateKey = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = ''
            Whether activation mints `safix.identity.keyFile` when that path is
            absent, before installing.

            Off by default, so adopting this module changes no existing profile's
            behaviour. `safix keygen` is the operator-run way to do the same
            thing, and is the better one where a person can run a command; this
            option exists because a profile that has never been activated has no
            other way to get a key file into place before the first install runs.

            It mints an identity and never a recipient: the age public half still
            has to reach `flake.safix.users.<u>.recipient` before anything is
            encrypted to it.
          '';
        };

        installer = {
          package = lib.mkOption {
            type = lib.types.package;
            default = pkgs.safix or (throw common.installerPackageMessage);
            defaultText = lib.literalExpression "pkgs.safix";
            description = ''
              The safix build whose `safix install` this profile's activation and
              user unit run.

              Defaults to `pkgs.safix`. A profile that reaches safix only through
              a flake input sets this to
              `inputs.safix.packages.''${pkgs.stdenv.hostPlatform.system}.safix`:
              this module imports nothing, by contract, so it holds no input to
              read a package out of.
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
              The safix build that checks this profile's manifest while the
              manifest derivation is being built. Two options rather than one for
              the same reason as at system scope: this one runs on the build
              platform and `safix.installer.package` runs on the host's.
            '';
          };

          validate = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = ''
              Whether this profile's manifest is checked against its documents at
              build time, selecting `safix install --check-mode=document` over
              `--check-mode=manifest`, and whether the manifest carries a hash
              over the documents it names so that editing one causes a rebuild.
            '';
          };

          keepGenerations = lib.mkOption {
            type = lib.types.int;
            default = 1;
            description = ''
              How many generation directories under
              `safix.installer.secretsMountPoint` the installer keeps after a
              successful install, newest first. Zero keeps every generation.
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
              Which of the installer's two optional log classes reach the
              journal: `keyImport` reports each identity assembled, and
              `secretChanges` each entry that was new or changed. Neither names a
              value, and both are off by default because each says something
              about which secrets exist and when they moved.
            '';
          };

          secretsMountPoint = lib.mkOption {
            type = lib.types.str;
            default = "%r/safix.d";
            description = ''
              Where this profile's installer keeps its generation directories.

              `%r` is expanded by the installer, not by nix: `$XDG_RUNTIME_DIR` on
              linux and the output of `getconf DARWIN_USER_TEMP_DIR` on darwin,
              with `%%` yielding a literal `%`. nix cannot know a session's
              runtime directory, and a path baked at evaluation would be wrong for
              every session but one.

              The store therefore does not survive a reboot: nothing arrives until
              this person logs in and their activation runs. Point this at a
              durable path to change that, and accept that the plaintext then
              outlives the session.
            '';
          };

          symlinkPath = lib.mkOption {
            type = lib.types.str;
            default = "%r/safix";
            description = ''
              Where this profile's installed secrets appear: a symlink to the
              latest generation under `safix.installer.secretsMountPoint`, and the
              directory every resolved entry that declares no path of its own
              parks under, as `''${symlinkPath}/<name>`.

              `%r` is expanded as it is for
              `safix.installer.secretsMountPoint`, so this root has the same
              reboot behaviour. The root and the per-entry default move together:
              the installer symlinks any entry path that is not
              `''${symlinkPath}/<name>`, so a moved root with an unmoved default
              would write safix's links outside its own store.
            '';
          };

          manifest = lib.mkOption {
            type = lib.types.package;
            readOnly = true;
            default = manifest;
            defaultText = lib.literalMD "the user-mode manifest this profile builds";
            description = ''
              The user-mode installer manifest this profile installs from,
              read-only, exposed for the same reason the system scope exposes
              `system.build.safix-manifest`: a check reads what a host would
              install rather than rebuilding the derivation to guess at it.
            '';
          };

          # `useTmpfs`, `environment` and `agePlugins` are system-scope only, and
          # each because this scope has nothing for it to govern: a user-mode
          # install mounts no filesystem, so there is no choice between ramfs and
          # tmpfs to make; and it runs no unit environment of its own that a
          # consumer would need to extend — the user unit and the activation
          # entry both run one script whose whole environment is the two store
          # paths it exports.
        };
      };

  config = lib.mkMerge [
    {
      # A `throw` rather than an assertion, and the difference is the whole
      # point: home-manager collects every failed assertion and prints them
      # together, so an assertion here would arrive beside whatever else the
      # profile refuses on rather than instead of it. Throwing while
      # `safix.secrets` is forced pre-empts the collection entirely.
      # `safix-consumption-refusals` holds it off a profile evaluated without
      # home-manager's assertion wrapper, since a profile evaluated with it
      # cannot tell whose refusal fired.
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

      # Outside the enable gate deliberately. Each of these fires exactly when
      # the resolution is empty for want of the option it names, which is when
      # `enable` defaults to false — so an assertion inside the gate would be a
      # refusal that only speaks once the mistake it reports has been repaired.
      #
      # `configured` is read off `options` rather than off `cfg` because
      # `safix.user` defaults to this profile's own username and is therefore
      # never null: on this scope the value cannot tell a consumer's selection
      # from the module's own default, and only a definition can.
      assertions = common.assertionsFor {
        inherit cfg;
        configured =
          common.wasSet options.safix.user
          || common.wasSet options.safix.machine
          || common.wasSet options.safix.hostname;
      };
    }

    (lib.mkIf cfg.enable {
      # Registered with `entryAfter [ "writeBoundary" ]` rather than as a bare
      # string, which home-manager reads as `entryAnywhere` and which gives it
      # no ordering to reason about. After the boundary because the install
      # writes outside the generation — into a runtime directory — and because
      # the user unit it belongs beside is materialized by `linkGeneration`.
      #
      # `run` rather than a bare invocation, so a home-manager dry run reports
      # the install instead of performing it.
      home.activation.safixInstall = lib.mkIf (cfg.secrets != { }) (
        lib.hm.dag.entryAfter [ "writeBoundary" ] ''
          run ${installScript}
        ''
      );

      # linux only: darwin has no systemd, so the activation entry above is the
      # whole of the registration there. The unit exists on linux so that a
      # session which outlives its activation — a runtime directory cleared by
      # a logout, most often — can reinstall without a switch.
      systemd.user.services = lib.mkIf (pkgs.stdenv.hostPlatform.isLinux && cfg.secrets != { }) {
        safix = {
          Unit = {
            Description = "Install the secrets safix resolved for this person";
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

      # The second condition is not redundant with the `mkIf cfg.enable` this
      # sits inside: `enable` can be set by hand over an empty resolution, and a
      # profile with nothing to decrypt must not gain an entry that refuses on
      # behalf of nothing.
      home.activation.safixIdentityPreflight = lib.mkIf (
        cfg.identityPreflight && cfg.secrets != { } && guardedIdentities != [ ]
      ) (lib.hm.dag.entryBefore [ "checkLinkTargets" ] identityPreflight);
    })
  ];
}
