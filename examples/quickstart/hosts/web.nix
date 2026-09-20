# The system-scope profile for the host `web`, as `../flake.nix` builds it.
#
# It declares no custody of its own: `safix.flake` reaches the flake output
# `safix.lib`, which is where `../secrets.nix` was bound. What is here is
# arrival — which declared machine this configuration is, which identity opens
# the files, and what reads the values once they land.
{ config, inputs, ... }:
{
  # --8<-- [start:bind]
  imports = [ inputs.safix.nixosModules.safix ];

  safix.flake = inputs.self;
  safix.machine = "web";
  safix.identity.deriveHostKeys = true;
  # --8<-- [end:bind]

  # Outside safix's namespace and set for this host's own sake. The ed25519
  # host key the identity above derives from exists because sshd is enabled,
  # not because safix asked for it.
  networking.hostName = "web";
  services.openssh.enable = true;
  system.stateVersion = "24.05";

  # --8<-- [start:service]
  services.grafana.enable = true;

  systemd.services.grafana.serviceConfig.LoadCredential = [
    "admin-password:${config.safix.secrets."grafana-admin-password".path}"
  ];
  # --8<-- [end:service]

  # --8<-- [start:template]
  safix.templates."grafana.env" = {
    content = ''
      GF_SECURITY_ADMIN_PASSWORD=${config.safix.placeholder."grafana-admin-password"}
      GF_SECURITY_SECRET_KEY=${config.safix.placeholder."grafana-secret-key"}
    '';
    restartUnits = [ "grafana.service" ];
  };
  # --8<-- [end:template]
}
