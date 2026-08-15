# Builds the rust workspace and carries every promise the rust half makes as a
# check of this flake.
#
# `packages.safix` is deliberately not touched here. The shell runtime is what
# ships for the whole of the rewrite, and the rust binary is exposed beside it as
# `packages.safix-rs`; the two swap only when a differential harness has compared
# every subcommand's stdout, standard error, exit code and effect on the
# repository. Until then a partially-ported runtime cannot become what a consumer
# runs by accident.
#
# The advisory scan is a check of its own rather than a fourth `cargo-deny`
# section. The advisory database is a network resource and the build sandbox has
# none, so `cargo-deny` runs bans, licences and sources offline over the vendored
# graph, and the scan reads a database pinned in `flake.lock`. Splitting them
# also means a newly published advisory reddens exactly one check, on a lock
# update, rather than at an unrelated moment.
#
# Formatting is owned by `safix-rs-fmt` rather than by treefmt, so that the
# rust sources have one formatting authority and it is the one the workspace's
# own `rustfmt.toml` configures.
{ inputs, ... }:
{
  perSystem =
    { config, pkgs, ... }:
    let
      craneLib = inputs.crane.mkLib pkgs;

      common = {
        # `filterCargoSources` keeps rust sources and every `.toml`, so
        # `deny.toml`, `clippy.toml` and `rustfmt.toml` reach the sandbox while
        # the nix modules, the README and the openspec tree do not. The `.snap`
        # files are added to it because the refusal snapshots are the test's
        # expected values: without them `safix-rs-test` runs against no
        # expectation and passes by writing one, which is a green check over an
        # assertion nobody made.
        src = pkgs.lib.cleanSourceWith {
          src = ../..;
          name = "source";
          filter =
            path: type: (builtins.match ".*\\.snap$" path != null) || (craneLib.filterCargoSources path type);
        };

        # Named here rather than read from the root manifest, which is a virtual
        # workspace and carries no package to read a name from.
        pname = "safix-rs";
        version = "0.1.0";

        strictDeps = true;

        # Nothing to link against. The runtime drives sops, git and nix as
        # subprocesses and links none of them, which is what keeps the
        # cryptographic surface an upstream one.
        buildInputs = [ ];
      };

      cargoArtifacts = craneLib.buildDepsOnly common;

      withArtifacts = common // {
        inherit cargoArtifacts;
      };
    in
    {
      packages.safix-rs = craneLib.buildPackage (
        withArtifacts
        // {
          # The tests are their own check, so the package build does not run
          # them twice.
          doCheck = false;

          meta.description = "The safix runtime in rust — not yet what ships; see openspec/changes/rewrite-runtime-in-rust";
        }
      );

      checks = {
        safix-rs-build = config.packages.safix-rs;

        safix-rs-test = craneLib.cargoTest withArtifacts;

        safix-rs-clippy = craneLib.cargoClippy (
          withArtifacts
          // {
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          }
        );

        safix-rs-fmt = craneLib.cargoFmt (common // { cargoExtraArgs = "--all"; });

        safix-rs-deny = craneLib.cargoDeny common;

        safix-rs-audit = craneLib.cargoAudit (common // { inherit (inputs) advisory-db; });
      };
    };
}
