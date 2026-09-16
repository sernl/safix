# sharedWith an organization: the audience gains acme's custody keys for this
# one entry. `escrowedTo` is the other declaration — it covers every file alice
# holds — and the two are deliberately not the same statement.
{
  flake.safix.users.alice.sharedWith.acme.escrow-note = { };
}
