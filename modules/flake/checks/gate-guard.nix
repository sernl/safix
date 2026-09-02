# Regression guard for the platform gate D9 confirmed narrow and correct:
# `consumption.nix:428` conditions only `safix-consumption-system` on
# `isLinux`, and `portability.nix`'s split conditions only
# `safix-portability-system` the same way. Every other check this flake
# publishes is expected to exist for every system `flake.nix` declares.
#
# Nothing in either file enforces that boundary once it is drawn. A future
# edit that folds a platform-independent check back inside an `isLinux` block
# by habit — the way `safix-portability` used to bundle all three consumption
# shapes before this split — would shrink the published set on non-Linux
# systems without turning anything red on the system where the mistake was
# made. This check reads the published check names for every system
# `config.systems` names and asserts that every Linux system matches
# `x86_64-linux` exactly, and that `aarch64-darwin` is missing exactly the
# checks that genuinely need something a non-Linux evaluation cannot give
# them — a real `nixosSystem`, `nix shell --inputs-from` against a real
# builder, or a store realisation this repository's own checks perform.
#
# `config.flake.checks` is the top-level, flake-parts-transposed form of every
# system's `perSystem.checks` (`lib.mkTransposedPerSystemModule` in
# flake-parts' own `lib.nix`), captured here via closure from the outer
# module's `config` rather than the per-system one `perSystem`'s own function
# binds under the same name. Reading `builtins.attrNames` off it, including
# off this very check's own system, is not self-referential in any way that
# matters: attribute names are known from an attrset's structure without
# forcing any attribute's value, so this check's own derivation is never
# built merely to learn that its name is present.
{ config, lib, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # Every check that is legitimately Linux-only today, per D9's own
      # measurement of the checked-out tree before this change (13 names: the
      # eight `safix-installer-*` checks, `safix-bridge-real-clan`,
      # `safix-generate-envelope`, `safix-memory-backing`,
      # `safix-consumption-system`, and `safix-portability`) with
      # `safix-portability`'s own split folded in — `safix-portability-system`
      # keeps the one shape of that check that still needs a `nixosSystem`,
      # `safix-portability-home` no longer belongs on this list at all. Every
      # other check is expected on every system.
      linuxOnlyChecks = [
        "safix-bridge-real-clan"
        "safix-consumption-system"
        "safix-generate-envelope"
        "safix-installer-coexistence"
        "safix-installer-identity"
        "safix-installer-manifest"
        "safix-installer-mechanism"
        "safix-installer-ordering"
        "safix-installer-refusals"
        "safix-installer-sole"
        "safix-installer-store"
        "safix-memory-backing"
        "safix-portability-system"
      ];

      isLinuxSystem = system: lib.hasSuffix "-linux" system;

      referenceSystem = "x86_64-linux";

      namesOf = system: builtins.attrNames config.flake.checks.${system};

      referenceNames = namesOf referenceSystem;

      # Either direction of drift from the reference system is a regression:
      # a name missing here that the reference has, or a name here the
      # reference lacks.
      diffAgainstReference = system: {
        missingHere = lib.subtractLists (namesOf system) referenceNames;
        extraHere = lib.subtractLists referenceNames (namesOf system);
      };

      # Every Linux system is expected to match the reference exactly, since
      # `isLinux` is the only gate either file applies; a non-Linux system is
      # expected to be missing exactly the checks that gate excludes it from.
      expectedDiffOf =
        system:
        if isLinuxSystem system then
          {
            missingHere = [ ];
            extraHere = [ ];
          }
        else
          {
            missingHere = linuxOnlyChecks;
            extraHere = [ ];
          };
    in
    {
      checks.safix-consumption-gate-guard = mkStructuralCheck {
        name = "safix-consumption-gate-guard";
        actual = lib.genAttrs config.systems diffAgainstReference;
        expected = lib.genAttrs config.systems expectedDiffOf;
      };
    };
}
