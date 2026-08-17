# The envelope's behavioural claims, made where a backend runs.
#
# `crates/safix-core/src/sandbox.rs` unit-tests the constructions — the argument
# vector, the darwin profile, the probe's three answers — and those run on every
# platform in `safix-rs-test`. This attribute is the other half: that the
# confinement those strings describe is the confinement the kernel applies, which
# needs the kernel.
#
# Two conditions gate it, and both are real:
#
#   - linux, because the backend here is bubblewrap. darwin's is `sandbox-exec`
#     with a profile this repository constructs, and observing it needs a darwin
#     machine rather than a build sandbox. This is the same shape and the same
#     reasoning as `safix-syscall-proof`'s and `safix-bridge-real-clan`'s.
#   - a kernel that grants unprivileged user namespaces, which bubblewrap is made
#     of and which a hardened kernel refuses. `crates/safix/tests/sandbox.rs`
#     asks that question against an argument vector written there rather than
#     against the construction under test, and where the answer is no each test
#     says what it did not do.
#
# Nesting bubblewrap inside the nix build sandbox was the risk this change's
# design recorded, and it turned out not to be one on this fleet:
# `safix-bridge-real-clan` already runs `clan vars generate` — which is
# bubblewrap — inside a `runCommand`, and this check runs the envelope the same
# way. A machine where the nesting genuinely fails reports the absence through the
# guard below rather than by going quietly green.
#
# # The guard, and why it is not `integration.runOne`
#
# Each test in the target reports the absence and returns where the gate closes,
# so libtest would say five passed over a run that established nothing. `runOne`
# reads the result line, which catches a filter naming nothing and would not catch
# this. So the absence sentence is read out of the output too, exactly as
# `safix-bridge-real-clan` reads its own.
{ ... }:
{
  perSystem =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      integration = import ./integration.nix { inherit pkgs; };
    in
    {
      checks = {
        # The one claim every platform can make about the envelope: no flag
        # suspends it. It is the argument reader that refuses, so no fragment runs
        # and no backend is needed.
        safix-generate-no-bypass =
          integration.runOne config.checks.safix-integration "safix-generate-no-bypass" "sandbox"
            "no_flag_suspends_the_envelope";
      }
      // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-generate-envelope =
          pkgs.runCommand "safix-generate-envelope"
            {
              nativeBuildInputs = integration.backends;
              env = {
                SAFIX_TEST_BINARY = "${config.checks.safix-integration}/libexec/safix";
                SAFIX_TEST_NIX_STUB = "${config.checks.safix-integration}/libexec/safix-nix-stub";
                SAFIX_TEST_SHIM = "${config.checks.safix-integration}/libexec/safix-test-shim";
                SAFIX_TEST_CLAN_STUB = "${config.checks.safix-integration}/libexec/safix-clan-stub";
              };
            }
            ''
              export HOME="$PWD"

              set +e
              output="$(${config.checks.safix-integration}/bin/sandbox \
                --nocapture --test-threads 1 2>&1)"
              status=$?
              set -e

              printf '%s\n' "$output"
              if [ "$status" != 0 ]; then
                echo "the suite refused sandbox" >&2
                exit "$status"
              fi

              case "$output" in
                *"test result: ok. 0 passed"*)
                  echo "no test ran: sandbox built nothing to run" >&2
                  exit 1
                  ;;
                *"test result: ok."*) ;;
                *)
                  echo "the suite reported no result for sandbox" >&2
                  exit 1
                  ;;
              esac
              case "$output" in
                *"no sandbox backend runs here"*)
                  echo "no backend ran, so the confinement was not observed" >&2
                  exit 1
                  ;;
              esac

              touch "$out"
            '';
      };
    };
}
