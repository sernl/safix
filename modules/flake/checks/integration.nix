# The integration suite, and how one test of it becomes one check.
#
# `crates/safix/tests/` drives the built binary against throwaway repositories
# with real backends. The suite is compiled once — by `checks.safix-integration`
# in `modules/flake/rust.nix`, which also runs it whole — and every other check
# that names a mode runs one test out of that build. Compiling it once per named
# check would be the same evidence at twenty-two times the cost.
#
# The three programs a test drives are named in the environment rather than found
# on a path, because `CARGO_BIN_EXE_*` is fixed when the test compiles and points
# inside the build directory of whatever compiled it. `crates/safix/tests/harness`
# reads these and falls back to the compiled-in path, so `cargo test` in a
# devshell needs none of them.
{ pkgs }:
let
  # Every backend the suite drives for real. Only `nix` is stubbed, and by a
  # binary the suite builds itself: standing a stub in for sops is what would let
  # a check stay green over a command calling something the tree no longer
  # contains.
  #
  # coreutils carries `timeout` and `kill`, which open and close the windows the
  # abort residue check drills, and `id`, which names the staging directory. The
  # syscall proof's observer is `strace`, which needs ptrace and is therefore
  # linux only — the check that uses it says so itself on other platforms.
  backends = [
    pkgs.age
    pkgs.coreutils
    pkgs.git
    # For `nix-instantiate --parse` alone, which is what holds the declaration
    # `adduser` generates to being nix. Parsing needs no store and no daemon, so
    # it is available where an evaluation is not; the `nix` the runtime evaluates
    # with stays the stub, which it reaches through `SAFIX_NIX` rather than by
    # name on a path.
    pkgs.nix
    pkgs.sops
  ]
  ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.strace;
in
{
  inherit backends;

  # One check that runs one test of the compiled suite.
  #
  # An empty `filter` runs every test in the target, which is what the
  # single-runtime checks want: each of those is one claim made of several
  # windows or several channels, and splitting it across attributes would leave a
  # check asserting a fragment of a claim.
  #
  # A filter naming nothing is the failure this guards against: libtest exits
  # zero having run no test, so a renamed test would turn a check green by
  # silently ceasing to assert anything. The result line is read for that.
  runOne =
    suite: name: target: filter:
    pkgs.runCommand name
      {
        nativeBuildInputs = backends;
        env = {
          SAFIX_TEST_BINARY = "${suite}/libexec/safix";
          SAFIX_TEST_NIX_STUB = "${suite}/libexec/safix-nix-stub";
          SAFIX_TEST_SHIM = "${suite}/libexec/safix-test-shim";
          SAFIX_TEST_CLAN_STUB = "${suite}/libexec/safix-clan-stub";
        };
      }
      ''
        export HOME="$PWD"

        set +e
        output="$(${suite}/bin/${target} ${
          if filter == "" then "" else "${filter} --exact"
        } --nocapture --test-threads 1 2>&1)"
        status=$?
        set -e

        printf '%s\n' "$output"
        if [ "$status" != 0 ]; then
          echo "the suite refused ${target} ${filter}" >&2
          exit "$status"
        fi

        case "$output" in
          *"test result: ok. 0 passed"*)
            echo "no test ran: ${target} has no test named '${filter}'" >&2
            exit 1
            ;;
          *"test result: ok."*) ;;
          *)
            echo "the suite reported no result for ${target} ${filter}" >&2
            exit 1
            ;;
        esac

        touch "$out"
      '';
}
