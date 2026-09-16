# An entry whose landed path is a function of the configuration materializing
# it. `functionTo str` does not serialize, so no compared field carries it and
# `examples/profiles/home.nix` is where the applied string is observed.
{
  flake.safix.users.alice.private.app-credentials.path =
    cfg: "${cfg.home.homeDirectory}/.config/example-app/credentials.toml";
}
