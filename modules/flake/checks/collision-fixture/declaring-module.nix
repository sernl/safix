# A module that declares one option and nothing else, used twice from two
# distinct store paths to measure what the module system does with a duplicate
# declaration.
#
# It is deliberately trivial: the claim under test is a property of
# `lib.evalModules`, so anything more here would only add ways for the drill to
# fail for a reason other than the one it exists to detect.
{ lib, ... }:
{
  options.safixCollisionFixture.thing = lib.mkOption {
    type = lib.types.str;
    default = "declared once";
  };
}
