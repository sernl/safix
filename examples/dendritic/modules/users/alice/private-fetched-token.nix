# The one generator granted the network. It widens what a mint may reach and
# nothing else: the filesystem confinement stays in force, so the staging root
# is still the only writable path, and what travels over the connection is
# outside what safix shreds or observes.
{
  flake.safix.users.alice.private.fetched-token.generator = {
    network = true;
    script = ''curl -fsS https://tokens.example/new > "$out/fetched-token"'';
    runtimeInputs = [
      "coreutils"
      "curl"
    ];
  };
}
