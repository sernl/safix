# A silo: no file's audience may span two groups in this set. One group named
# here can never conflict; the declaration demonstrates the option against a
# group the fleet actually uses rather than an ornamental one.
{
  flake.safix.silos.corp.groups = [ "oncall" ];
}
