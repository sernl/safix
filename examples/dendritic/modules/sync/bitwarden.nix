# The Bitwarden vault, and one mapping into it.
# `server` is null, which names whichever server the operator's own client is
# already configured against.
# No `tags`: this vault has no tag concept, so a declared tag is refused at
# evaluation rather than approximated onto a folder.
{
  flake.safix.bitwarden = {
    server = null;
    mappings.vpn = {
      mode = "safix-to-bitwarden";
      safix = {
        user = "alice";
        name = "vpn-password";
      };
      bitwarden = {
        folder = "safix";
        item = "vpn";
        fields = {
          username = "alice";
          url = "https://vpn.example";
          notes = "example mapping";
        };
      };
    };
  };
}
