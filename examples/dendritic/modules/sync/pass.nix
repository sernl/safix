# The `pass` store, and one mapping into it.
# This is the one mapping that carries all four fields, and the only one whose
# `username` is an `{ entry = …; }` source rather than a literal: every field of
# this target crosses on standard input, so a resolved secret never reaches an
# argument vector.
# The path carries no `.safix-sync-state` suffix, which is the name safix
# reserves for the companion a `two-way` mapping records its last agreement in.
{
  flake.safix.pass = {
    store = "~/.password-store";
    mappings.deploy = {
      mode = "two-way";
      safix = {
        user = "alice";
        name = "deploy-token";
      };
      pass = {
        path = "alice/deploy";
        fields = {
          username = {
            entry = "deploy-username";
          };
          url = "https://deploy.example";
          notes = "example mapping";
          tags = [ "example" ];
        };
      };
    };
  };
}
