# The password database, and one mapping into it. The far side carries
# `username`, `url` and `notes` as literals; `tags` and an `{ entry = …; }`
# source are both refused for this target, because its only channel for a field
# is an argument vector.
{
  flake.safix.keepassxc = {
    database = "/home/alice/.keys/example.kdbx";
    group = "safix";
    mappings.grafana = {
      mode = "safix-to-keepassxc";
      safix = {
        user = "alice";
        name = "grafana-password";
      };
      kdbx = {
        path = "alice/grafana";
        fields = {
          username = "alice@example.com";
          url = "https://grafana.example";
          notes = "example mapping — no real database";
        };
      };
    };
  };
}
