# An entry that names its own mode. No placement field carries a mode, so
# `safix-examples` cannot see it; `examples/profiles/` is where it is observed,
# at the scope that materializes it.
{
  flake.safix.catalogue.deploy-key.mode = "0440";
}
