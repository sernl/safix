# safix in a NixOS configuration: the same `safix.*` namespace as the
# home-manager module, materializing at system scope.
#
# This module imports nothing outside safix, and `nixosModules.safix` and
# `nixosModules.default` are one value: both names stay published so every
# existing `imports` line keeps resolving. `./installer.nix` is safix's own and
# travels with this file.
#
# ── no activation guard here, and why ──
# The home-manager module installs a read-only identity preflight that sorts
# ahead of `checkLinkTargets`, where refusing is still atomic: nothing has been
# linked, no unit restarted, no secret written. Nothing equivalent is installed
# here, because no equivalent point has been demonstrated at system activation.
# System-scope secret installation is ordered by units and by
# `system.activationScripts`, and safix has not shown a place in that sequence
# where a refusal leaves the previous generation intact. Claiming one would be
# documenting a guarantee that no code in this repository enforces.
#
# The failure the home-scope guard exists for is also rarer here: safix derives
# a system-scope identity from the host's ed25519 keys, excluding only the ones
# inside its own store, so a host that runs sshd usually decrypts without naming
# one — and where nothing is derivable, the resolution refuses at evaluation in
# safix's own words, before the installer's own pre-decryption check refuses a
# path that is not there yet.
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
in
{
  imports = [ ./installer.nix ];

  options.safix = common.sharedOptions {
    inherit cfg;

    # A system configuration knows its own host, and knows no person: which
    # people's system-scope entries land here is exactly what this module is
    # asked.
    userDefault = null;
    userDefaultText = lib.literalExpression "null";

    hostnameDefault = config.networking.hostName;
    hostnameDefaultText = lib.literalExpression "config.networking.hostName";

    # safix's own entry type, declared in `./common.nix` rather than read off
    # another framework's option declaration in the same evaluation — which is
    # what made that framework's module a required import for a module whose
    # contract is that it imports nothing.
    #
    # The type carries the twelve fields the manifest carries and nothing else:
    # it coerces and defaults, and asks nothing about the filesystem, so an
    # entry whose `sopsFile` does not exist or lies outside the nix store
    # passes it unchanged. What refuses those two is the manifest builder in
    # `./installer.nix`, under `safix.installer.validate`.
    secretsType = lib.types.attrsOf (common.secretEntryType { inherit cfg; });
  };

  # safix defines no option outside its own namespace. The identity it
  # decrypts with, the entries it installs and the manifest it builds are all
  # `safix.*`; a consumer who runs another secret-management framework for
  # their own secrets keeps every option they set there, and safix neither
  # reads nor writes any of them. So this scope's whole configuration is the
  # resolved set, the assertions, and what `./installer.nix` registers.
  config = {
    # A `throw` while this option is forced, mirroring the user scope's
    # refusal, because at this scope nothing else refuses at all: safix is
    # the only installer on this path, so a configuration resolving entries
    # with nothing to decrypt them would evaluate green and install nothing
    # decryptable.
    safix.secrets =
      let
        resolved = common.resolvedFor {
          inherit cfg;
          target = config;
        };

        # safix's own two options plus the derivation, and nothing else
        # counts. A gnupg configuration belonging to another framework used
        # to be tolerated here as evidence that something could decrypt,
        # which suppressed safix's own refusal on the strength of a
        # configuration safix neither wrote nor can use: `safix.identity`
        # carries a key file and ssh keys, and no gnupg identity is
        # expressible, so none is accepted.
        hasIdentity =
          cfg.identity.keyFile != null
          || cfg.identity.sshKeyPaths != [ ]
          || cfg.identity.derivedHostKeys != [ ];
      in
      if resolved != { } && !hasIdentity then
        throw (common.noSystemIdentityMessage { inherit cfg resolved; })
      else
        # An entry that declares no path parks at `<symlinkPath>/<name>`,
        # which is `secretEntryType`'s own `path` default rather than a mint
        # here: the installer symlinks any entry path that is not
        # `<symlinkPath>/<name>`, so safix's store root and that default have
        # to move together, and folding the default into the type is what
        # makes them one thing rather than two that happen to agree.
        #
        # The definition stays ungated and reads the local `resolved` binding
        # rather than `cfg.secrets`, because either alternative is an
        # evaluation cycle: mapping over `cfg.secrets` is this option's own
        # definition reading the option, and wrapping the definition in
        # `lib.mkIf cfg.enable` is the same cycle one hop longer, through
        # `enable`'s own `cfg.secrets != { }` default.
        resolved;

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
  };
}
