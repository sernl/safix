# sharedWith a group: the file lands named for the group, so membership change
# re-wraps one file instead of migrating to another. The private declaration is
# what the grant hands on.
{
  flake.safix.users.alice = {
    private.pager-token = { };
    sharedWith.oncall.pager-token = { };
  };
}
