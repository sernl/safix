# Placement, not custody: laptop-token is still alice's everywhere, it simply
# does not land on deck.
{
  flake.safix.users.alice.perHost.deck.omit.laptop-token = { };
}
