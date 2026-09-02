# An organization holding recovery custody. alice consents to it in
# ./users/alice/escrowed-to-acme.nix.
{
  flake.safix.organizations.acme.custody.acme-escrow = {
    key = "age1exampleacme0000000000000000000000000000000000000000000000";
    note = "acme's escrow — held offline by the operator";
  };
}
