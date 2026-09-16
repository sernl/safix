# The 1Password mirror, and one mapping into it.
# `account` is null, which lets `op` resolve its own.
# The whole item crosses as one JSON object on standard input, so this target
# carries all four fields and an `{ entry = …; }` source is admissible on every
# one of them.
{
  flake.safix.onepassword = {
    account = null;
    mappings.registry = {
      mode = "backup";
      safix = {
        user = "alice";
        name = "registry-token";
      };
      onepassword = {
        vault = "Private";
        item = "registry";
        fields = {
          username = {
            entry = "deploy-username";
          };
          url = "https://registry.example";
          notes = "example mapping";
          tags = [ "example" ];
        };
      };
    };
  };
}
