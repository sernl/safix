# A validation fragment judges the candidate before anything is written, so a
# bad value never reaches a committed file.
{
  flake.safix.users.alice.private.validated-token.generator = {
    script = ''openssl rand -hex 16 > "$out/validated-token"'';
    runtimeInputs = [ "openssl" ];
    validation = "grep -Eq '^[0-9a-f]{32}$' ";
  };
}
