# A legal `add`: alice already holds web-token through `private`, so this
# adjusts that entry's placement on deck. An `add` that were the only route to
# an entry is refused, because a host-scoped selection puts nobody in an
# audience.
{
  flake.safix.users.alice.perHost.deck.add.web-token.mode = "0440";
}
