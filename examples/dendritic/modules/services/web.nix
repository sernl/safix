# A service subject: its recipients are its machines', and its landed entries
# belong to its own unix user and group.
{
  flake.safix.services.web = {
    machines = [ "deck" ];
    owner = "alice";
    user = "web";
    group = "web";
  };
}
