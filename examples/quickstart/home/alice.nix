# The user-scope profile for alice, as `../flake.nix` builds it.
#
# The same declarations serve both scopes: a person's custody does not depend
# on the host they are logged into, so nothing here re-declares an entry.
{ inputs, ... }:
{
  # --8<-- [start:bind]
  imports = [ inputs.safix.homeModules.safix ];

  safix.flake = inputs.self;
  safix.user = "alice";
  safix.hostname = "web";
  safix.identity.keyFile = "/home/alice/.config/sops/age/keys.txt";
  # --8<-- [end:bind]

  # --8<-- [start:rotation]
  safix.rotation = {
    enable = false;
    repository = "/home/alice/secrets";
    onCalendar = "daily";
  };
  # --8<-- [end:rotation]

  home.username = "alice";
  home.homeDirectory = "/home/alice";
  home.stateVersion = "24.05";
}
