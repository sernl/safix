# A file that rides an existing creation rule and that no declaration implies.
# Nothing but this line puts it in policy, which is what the option is for.
{
  flake.safix.extraGovernedFiles = [ "secrets/safix/users/alice/legacy.yaml" ];
}
