# A group whose members are a machine, a service and another group, where
# oncall holds people only. One audience algebra over subjects is what keeps a
# nested group from needing a second membership expansion.
{
  flake.safix.groups.infra.members = [
    "deck"
    "web"
    "oncall"
  ];
}
