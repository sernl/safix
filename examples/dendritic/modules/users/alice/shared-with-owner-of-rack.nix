# sharedWith the owner of a machine: the grant follows rack's `owner` record,
# so a change of owner re-wraps it rather than leaving it pointed at whoever
# held the host when it was written.
{
  flake.safix.users.alice.sharedWith."ownerOf.rack".corp-handover = { };
}
