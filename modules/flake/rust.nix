# Builds the rust workspace and carries every promise the rust half makes as a
# check of this flake.
#
# `packages.safix` is this binary. It became this binary when the differential
# harness had compared every subcommand the shell runtime has against it on
# standard output, standard error, exit code and effect on the repository, with
# the divergences it did find recorded and pinned rather than reconciled — see
# `modules/flake/checks/differential.nix` and the CHANGELOG. The shell runtime
# stays in the tree as `packages.safix-sh`, because it is what those modes
# compare against: retiring the oracle would retire the evidence.
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

      integration = import ./checks/integration.nix { inherit pkgs; };
    in
    {
      packages.safix = craneLib.buildPackage (
        withArtifacts
        // {
          # The tests are their own check, so the package build does not run
          # them twice.
          doCheck = false;

          meta.description = "The whole lifecycle of one secret, by name and never by file (set | get | list | generate | check | fix | keygen | adduser)";
          meta.mainProgram = "safix";
        }
      );

      checks = {
        safix-rs-build = config.packages.safix;

        # The in-crate tests alone. `--lib --bins` rather than everything,
        # because the integration suite needs backends this derivation has no
        # reason to carry, and a fast check over the pure logic is worth keeping
        # separable from one that mints keys and runs sops.
        safix-rs-test = craneLib.cargoTest (
          withArtifacts // { cargoTestExtraArgs = "--lib --bins"; }
        );

        # The integration suite: compiled once, run whole, and left in the output
        # so that every check naming one behavioural mode runs one test of this
        # build rather than compiling its own.
        #
        # `HOME` is the build directory because git refuses to record a commit
        # without somewhere to look for a configuration, and the fixtures commit.
        # The suite stages plaintext on `/dev/shm`, which the sandbox provides as
        # tmpfs; it refuses rather than staging on disk if that is ever untrue.
        safix-integration = craneLib.mkCargoDerivation (
          withArtifacts
          // {
            pnameSuffix = "-integration";
            nativeBuildInputs = integration.backends ++ [ pkgs.jq ];
            doInstallCargoArtifacts = false;

            buildPhaseCargoCommand = ''
              cargoWithProfile test --locked --no-run --message-format json >artifacts.json
            '';

            doCheck = true;
            checkPhaseCargoCommand = ''
              export HOME="$PWD"
              cargoWithProfile test --locked
            '';

            # The test binaries carry a hash in their file name and the programs
            # they drive do not, so each is installed under the name cargo gave
            # the target. `.profile.test` is what separates the suite's own
            # binaries from the three programs it invokes.
            installPhaseCommand = ''
              install -d "$out/bin" "$out/libexec"

              jq -r 'select(.reason == "compiler-artifact" and .executable != null
                            and .profile.test == true
                            and (.target.kind | index("test")))
                     | "\(.target.name)\t\(.executable)"' artifacts.json \
                | while IFS=$'\t' read -r target executable; do
                    install -Dm555 "$executable" "$out/bin/$target"
                  done

              jq -r 'select(.reason == "compiler-artifact" and .executable != null
                            and .profile.test == false
                            and (.target.kind | index("bin")))
                     | "\(.target.name)\t\(.executable)"' artifacts.json \
                | while IFS=$'\t' read -r target executable; do
                    install -Dm555 "$executable" "$out/libexec/$target"
                  done

              for required in safix safix-nix-stub safix-test-shim; do
                [ -x "$out/libexec/$required" ] \
                  || { echo "the suite did not build $required" >&2; exit 1; }
              done
              [ -n "$(ls -A "$out/bin")" ] \
                || { echo "the suite built no test binaries" >&2; exit 1; }
            '';
          }
        );

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
