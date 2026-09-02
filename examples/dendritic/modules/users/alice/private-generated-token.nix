# The value writes itself: `safix generate` runs this script rather than a
# person typing a value in.
{
  flake.safix.users.alice.private.generated-token.generator = {
    script = ''openssl rand -hex 32 > "$out/generated-token"'';
    runtimeInputs = [ "openssl" ];
  };
}
