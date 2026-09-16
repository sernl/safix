# A chained generator: it reads another of alice's values at
# `$in/<generator>/<name>`, which is also what enrols it in that value's
# rotation.
{
  flake.safix.users.alice.private.derived-token.generator = {
    dependencies = [ "generated-token" ];
    script = ''
      sha256sum < "$in/generated-token/generated-token" | cut -d' ' -f1 > "$out/derived-token"
    '';
    runtimeInputs = [ "coreutils" ];
  };
}
