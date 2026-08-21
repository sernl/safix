# The installer safix owns at system scope: the manifest the provisioner's
# binary will read, built by safix rather than by the provisioner's module.
#
# safix no longer uses the provisioner's installer, and still couples to the
# `sops.*` settings a consumer tunes it with: `package`, `validationPackage`,
# `validateSopsFiles`, `log`, `keepGenerations`, `useTmpfs`, `placeholder`,
# `environment`, `gnupg.home`, `gnupg.sshKeyPaths` and `age.plugins` are read
# from the provisioner's namespace rather than redeclared, so one option
# surface tunes both packages and safix does not mint a second copy of it.
#
# The two store roots are manifest fields rather than NixOS options —
# `sops-install-secrets` reads `secretsMountPoint` and `symlinkPath` from the
# manifest JSON and consults no option — and the provisioner's builder merges
# its `extraJson` argument over its hardcoded values (`manifest-for.nix:52`),
# which is how its own `secrets-for-users` submodule relocates the same two
# fields. safix does not call that builder: `nixosModules.safix` imports
# nothing by contract, so `${inputs.sops-nix}` is not reachable from here, and
# the copy below is instead held against the provisioner's own builder by
# `safix-installer-manifest`, which the flake — which does have the input —
# builds over one fixture from both builders and compares.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.safix;
  sopsCfg = config.sops;

  installedEntries = builtins.attrValues config.sops.secrets;

  # Copied from the provisioner's builder (`manifest-for.nix:11-28`), under the
  # same `validateSopsFiles` gate it carries there. The copy is load-bearing:
  # neither refusal travels with the provisioner's option type — `pathNotInStore`
  # is applied to `sops.age.keyFile` and never to a secret entry, whose
  # `sopsFile` is plain `lib.types.path` — so leaving the builder behind leaves
  # both refusals behind with it.
  failedAssertions = builtins.foldl' (
    acc: secret:
    acc
    ++ lib.optional (
      !builtins.pathExists secret.sopsFile
    ) "safix: cannot find '${secret.sopsFile}', the sops file of resolved entry '${secret.name}'"
    ++
      lib.optional
        (
          !builtins.isPath secret.sopsFile
          && !(builtins.isString secret.sopsFile && lib.hasPrefix builtins.storeDir secret.sopsFile)
        )
        "safix: '${secret.sopsFile}', the sops file of resolved entry '${secret.name}', is not in the Nix store. Add it to the Nix store or set sops.validateSopsFiles to false"
  ) [ ] installedEntries;

  manifest =
    if sopsCfg.validateSopsFiles && failedAssertions != [ ] then
      throw "\nFailed assertions:\n${lib.concatStringsSep "\n" (map (x: "- ${x}") failedAssertions)}"
    else
      pkgs.writeTextFile {
        name = "safix-manifest.json";
        text = builtins.toJSON {
          secrets = installedEntries;

          # safix has no template concept; the field is carried for schema
          # parity with the provisioner's builder, whose emitted field set
          # `safix-installer-manifest` holds this file to.
          templates = [ ];

          secretsMountPoint = cfg.installer.secretsMountPoint;
          symlinkPath = cfg.installer.symlinkPath;
          keepGenerations = sopsCfg.keepGenerations;
          gnupgHome = sopsCfg.gnupg.home;
          sshKeyPaths = sopsCfg.gnupg.sshKeyPaths;
          ageKeyFile = sopsCfg.age.keyFile;
          ageSshKeyPaths = sopsCfg.age.sshKeyPaths;
          useTmpfs = sopsCfg.useTmpfs;

          # Read for schema parity and empty in practice: the provisioner
          # defines `placeholder` only under `mkIf (config.sops.templates != { })`
          # and maps it over `config.sops.secrets`
          # (`modules/sops/templates/default.nix:116`, `:130-134`), and safix
          # leaves both empty.
          placeholderBySecretName = sopsCfg.placeholder;

          userMode = false;
          logging = {
            keyImport = builtins.elem "keyImport" sopsCfg.log;
            secretChanges = builtins.elem "secretChanges" sopsCfg.log;
          };
        };

        # The provisioner's own checkPhase (`manifest-for.nix:54-58`), run by
        # the binary that will read the manifest, mirroring its conditional
        # rather than picking a branch of it. The branch is load-bearing and
        # `manifest` is the tempting, weaker half: `validateSopsFiles` defaults
        # true, so `sopsfile` is what runs over safix's entries today — it
        # reads each ciphertext and verifies each declared `key` resolves —
        # where `manifest` mode returns a stub without reading the file
        # (`main.go:503-505`) and skips the key-presence check (`:558`),
        # validating the JSON schema and nothing else.
        checkPhase = ''
          ${sopsCfg.validationPackage}/bin/sops-install-secrets -check-mode=${
            if sopsCfg.validateSopsFiles then "sopsfile" else "manifest"
          } "$out"
        '';
      };
in
{
  options.safix.installer = {
    secretsMountPoint = lib.mkOption {
      type = lib.types.str;
      default = "/run/safix.d";
      description = ''
        Where safix's installer keeps its generation directories. A manifest
        field rather than a provisioner option: `sops-install-secrets` reads it
        from the manifest JSON, so this store is safix's own and disjoint from
        any store another component of the host owns.
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
  };

  config = lib.mkIf cfg.enable {
    # Mirrors the provisioner's `system.build.sops-nix-manifest`
    # (`modules/sops/default.nix:534`), so checks read one concrete attribute
    # rather than rebuilding the derivation each time.
    system.build.safix-manifest = manifest;
  };
}
