# safix in a home-manager profile: `safix.*` beside `sops.*`, declaring which
# resolved set arrives here and how this machine opens it.
#
# This module imports nothing. `homeModules.default` is this file plus sops-nix's
# own home-manager module, for a tree that has not already got one; a tree that
# has imports this file instead, as `homeModules.safix`. The reason the choice
# exists is measured rather than assumed: two distinct store paths declaring one
# option set is a hard evaluation error, not a merge, and `imports` cannot depend
# on an option, so no flag could repair it after the fact.
#
# ── the activation guard ──
# sops-nix registers its own activation entry as a bare string, which
# home-manager treats as `entryAnywhere`. It lands after `linkGeneration` and
# after `reloadSystemd`, and it runs `systemctl restart --user sops-nix`, so an
# absent or unreadable identity aborts the switch with the generation already
# half-applied. Pinning that entry earlier is not available as a fix: the unit it
# restarts is materialized by `linkGeneration` and made visible to the daemon by
# `reloadSystemd`, so an early restart aborts unconditionally on the first switch
# that introduces sops and thereafter restarts the previous generation's unit,
# installing the previous manifest's secrets with no signal.
#
# `home.activation.safixIdentityPreflight` closes that window instead. It is
# read-only, it sorts `entryBefore [ "checkLinkTargets" ]`, and the ordering is
# the whole of what its failure message claims — `safix-consumption-ordering`
# holds it against a real evaluation of a profile rather than against this
# comment.
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
  sopsCfg = config.sops;

  # sops-install-secrets aborts on a set-but-unreadable age.keyFile, unless
  # generateKey creates it first.
  requiredIdentities = lib.optional (sopsCfg.age.keyFile != null && !sopsCfg.age.generateKey) {
    path = toString sopsCfg.age.keyFile;
    origin = "sops.age.keyFile";
  };

  # A gnupg source decrypts on its own, so a missing age ssh key is not decisive
  # while one is configured.
  hasGnupgSource =
    sopsCfg.gnupg.home != null
    || sopsCfg.gnupg.sshKeyPaths != [ ]
    || sopsCfg.gnupg.qubes-split-gpg.enable;

  # Each age ssh key path is individually skipped with a warning to stderr, so
  # these are load-bearing only collectively, and only while they are the sole
  # identity source.
  sufficientIdentities = lib.optionals (sopsCfg.age.keyFile == null && !hasGnupgSource) (
    lib.imap0 (i: p: {
      path = toString p;
      origin = "sops.age.sshKeyPaths[${toString i}]";
    }) sopsCfg.age.sshKeyPaths
  );

  guardedIdentities = requiredIdentities ++ sufficientIdentities;

  # Whether anything at all can open a file here. safix's own two options rather
  # than `sops.age.*`, because safix defines those from these and reading them
  # back while `safix.secrets` is being forced is a cycle: `safix.enable`
  # defaults to whether this resolved, and the definitions are gated on it.
  # `sops.gnupg.*` is safe to read and is read, because safix defines none of it
  # and a gnupg source decrypts on its own.
  hasIdentity = cfg.identity.keyFile != null || cfg.identity.sshKeyPaths != [ ] || hasGnupgSource;

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

    ${toString (builtins.length (lib.attrNames sopsCfg.secrets))} secret(s) resolve for ${config.home.username} here, and sops-nix has no
    other way to decrypt them.

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
    later, in sops-install-secrets.

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
in
{
  options.safix =
    common.sharedOptions {
      inherit cfg;

      userDefault = config.home.username;
      userDefaultText = lib.literalExpression "config.home.username";

      # Known only where home-manager is evaluated as a NixOS module, which is the
      # only seam that hands a home-manager profile the host it is on. Standalone
      # there is no honest default, so the assertion asks for one.
      hostnameDefault =
        if osConfig == null || !(osConfig ? networking) then null else osConfig.networking.hostName;
      hostnameDefaultText = lib.literalExpression "osConfig.networking.hostName, where home-manager is a NixOS module; null standalone";
    }
    // {
      identityPreflight = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether to install the read-only identity check that sorts ahead of
          `checkLinkTargets` and refuses the switch when no configured identity is
          present and readable.

          On by default. It exists because sops-nix's own entry sorts after
          `linkGeneration` and `reloadSystemd`, where a refusal is no longer
          atomic, and because its entry cannot be pinned earlier — it restarts a
          unit those two steps materialize.

          It checks presence and readability and nothing further. It does not
          decrypt, so a secret declared here but absent from the sops file still
          fails later, in sops-install-secrets.
        '';
      };
    };

  config = lib.mkMerge [
    {
      # A `throw` rather than an assertion, and the difference is the whole
      # point. sops-nix's key-source assertion sits inside its
      # `mkIf (cfg.secrets != { })`, so collecting the assertion list forces
      # `sops.secrets`, which forces this — and home-manager collects every
      # failed assertion and prints them together, so an assertion here would
      # arrive beside sops-nix's rather than instead of it. Throwing while
      # `safix.secrets` is forced pre-empts the collection entirely.
      # `safix-consumption-refusals` holds both halves, and holds them off a
      # profile evaluated without home-manager's assertion wrapper, since a
      # profile evaluated with it cannot tell whose refusal fired.
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
        configured = common.wasSet options.safix.user || common.wasSet options.safix.hostname;
      };
    }

    (lib.mkIf cfg.enable {
      sops = {
        secrets = cfg.secrets;

        # Defined even when null, so a mkDefault elsewhere in the consumer's tree
        # cannot silently replace it with a path whose absence aborts activation.
        age.keyFile = cfg.identity.keyFile;

        # Defined only when named, so a provisioner default this scope may grow
        # is not clobbered by an empty list.
        age.sshKeyPaths = lib.mkIf (cfg.identity.sshKeyPaths != [ ]) cfg.identity.sshKeyPaths;
      };

      # The second condition is not redundant with the `mkIf cfg.enable` this
      # sits inside: `enable` can be set by hand over an empty resolution, and a
      # profile with nothing to decrypt must not gain an entry that refuses on
      # behalf of nothing.
      home.activation.safixIdentityPreflight = lib.mkIf (
        cfg.identityPreflight && sopsCfg.secrets != { } && guardedIdentities != [ ]
      ) (lib.hm.dag.entryBefore [ "checkLinkTargets" ] identityPreflight);
    })
  ];
}
