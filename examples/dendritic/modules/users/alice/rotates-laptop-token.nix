# A typed value under the same policy. Nothing here can mint it, so the remedy
# `safix check` names for it is `safix set` and `safix rotate --due` lists it
# rather than minting it.
{
  flake.safix.users.alice.private.laptop-token.rotation = "quarterly";
}
