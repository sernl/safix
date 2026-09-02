# sharedWith a person: alice hands bob a copy of her own thing. A grant hands
# on an entry the granter already holds, it does not create one, so the private
# declaration below is what makes the grant reach an existing entry.
{
  flake.safix.users.alice = {
    private.handoff-note = { };
    sharedWith.bob.handoff-note = { };
  };
}
