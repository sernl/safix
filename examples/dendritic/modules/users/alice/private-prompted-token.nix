# A generator that asks the operator for an input. The answer is a file inside
# the staging root, never an argument and never an environment variable.
{
  flake.safix.users.alice.private.prompted-token.generator = {
    prompts.passphrase = {
      type = "hidden";
      description = "the passphrase the token is derived from";
    };
    script = ''sha256sum < "$prompts/passphrase" | cut -d' ' -f1 > "$out/prompted-token"'';
    runtimeInputs = [ "coreutils" ];
  };
}
