# acme's half of the same delegation. It places no key in any audience: a
# manager scaffolds and never reads by virtue of managing.
{
  flake.safix.organizations.acme.managers = [ "alice" ];
}
