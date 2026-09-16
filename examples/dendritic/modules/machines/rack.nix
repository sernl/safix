# A machine acme owns. An organization-owned machine is what makes an
# `ownerOf` grant resolve to custody keys rather than to a person's recipient.
{
  flake.safix.machines.rack = {
    recipient = "age1examplerack0000000000000000000000000000000000000000000000";
    recipientNote = "rack — a machine acme owns";
    owner = "acme";
  };
}
