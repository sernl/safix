# sharedWith a machine: deck's own service reads the value, no person has to
# be logged in. The private declaration is what the grant hands on.
{
  flake.safix.users.alice = {
    private.fleet-token = { };
    sharedWith.deck.fleet-token = { };
  };
}
