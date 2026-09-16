# `force` beats `omit` within one resolution: shelf-item is omitted on every
# portable host by ./per-tag-portable.nix and re-added here.
{
  flake.safix.users.alice.perTag.portable.force.shelf-item = { };
}
