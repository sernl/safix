# A silo: no file's audience may span two groups in this set. Two groups is the
# only shape in which the refusal means anything — oncall holds alice and bob,
# contractors holds carol, and a grant reaching both would be refused naming
# this declaration.
{
  flake.safix.silos.corp.groups = [
    "oncall"
    "contractors"
  ];
}
