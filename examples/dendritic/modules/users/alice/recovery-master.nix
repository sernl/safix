# A second recipient alice holds herself. That is what independent custody
# looks like: losing the activation key leaves her files openable by her rather
# than by whoever holds an escrow identity.
{
  flake.safix.users.alice.recoveryRecipients.master = {
    key = "age1examplemaster00000000000000000000000000000000000000000000";
    note = "alice's offline master identity — held by her, not by the operator";
  };
}
