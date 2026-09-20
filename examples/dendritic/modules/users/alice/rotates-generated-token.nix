# A generated value under a policy. `safix check` names `safix rotate` as the
# remedy when it is past its deadline, because a generator can re-mint it.
{
  flake.safix.users.alice.private.generated-token.rotation = "quarterly";
}
