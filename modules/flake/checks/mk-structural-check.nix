# runCommand JSON-diff helper for asserting that a nix value matches a literal
# expectation.
#
# Inputs and outputs are JSON-serialized at outer eval time and embedded as
# `passAsFile` env vars, so the derivation's input-address closure scopes to "did
# the JSON change?" rather than "did any tracked file in the repository change?".
# Cache invalidation therefore tracks the assertion target rather than the flake
# source.
#
# Failure emits a unified diff, which is far more readable than an
# expression-inequality dump for set-equality failures.
#
# A plain function file rather than an option, so that nothing in the check
# surface has to appear in the namespace a consumer imports.
pkgs:
{
  name,
  actual,
  expected,
}:
pkgs.runCommand "structure-${name}"
  {
    actualJson = builtins.toJSON actual;
    expectedJson = builtins.toJSON expected;
    passAsFile = [
      "actualJson"
      "expectedJson"
    ];
    meta.description = "structural check: ${name}";
  }
  ''
    if ! diff -u "$expectedJsonPath" "$actualJsonPath"; then
      echo ""
      echo "structural check '${name}' failed: actual differs from expected"
      exit 1
    fi
    touch $out
  ''
