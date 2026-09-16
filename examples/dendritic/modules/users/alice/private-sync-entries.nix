# The safix side of every sync mapping this fleet declares, in one file because
# it is one statement: these exist so that a mapping has a safix side. A mapping
# names an entry; it does not declare one.
{
  flake.safix.users.alice.private = {
    ntfy-token = { };
    grafana-password = { };
    deploy-token = { };
    deploy-username = { };
    vpn-password = { };
    registry-token = { };
  };
}
