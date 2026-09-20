# System-scope selection. Registry policy and explicit ciphertext imports are
# resolved once; installer.nix owns activation and the two independent stores.
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
    userDefault = null;
    userDefaultText = lib.literalExpression "null";
    hostnameDefault = config.networking.hostName;
    hostnameDefaultText = lib.literalExpression "config.networking.hostName";
    secretsType = lib.types.attrsOf (common.secretEntryType { inherit cfg; });
  };

  config = {
    safix.secrets =
      let
        resolved = common.resolvedFor {
          inherit cfg;
          target = config;
        };
        hasIdentity =
          cfg.identity.keyFile != null
          || cfg.identity.gnupgHome != null
          || cfg.identity.sshKeyPaths != [ ]
          || cfg.identity.derivedHostKeys != [ ];
      in
      if resolved != { } && !hasIdentity then
        throw (common.noSystemIdentityMessage { inherit cfg resolved; })
      else
        resolved;

    # These remain outside the enable gate. Imported entries must not hide an
    # explicitly misconfigured registry binding, even if installation is off.
    assertions = common.assertionsFor {
      inherit cfg;
      configured =
        common.wasSet options.safix.user
        || common.wasSet options.safix.machine
        || common.wasSet options.safix.hostname
        || common.wasSet options.safix.flake
        || common.wasSet options.safix.lib;
    };
  };
}
