# sharedWith a service: the entry arrives on each machine web runs on, keyed
# web/web-token. The private declaration is what the grant hands on.
{
  flake.safix.users.alice = {
    private.web-token = { };
    sharedWith.web.web-token = { };
  };
}
