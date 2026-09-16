# One run, three outputs: the entry the generator is declared on, a private
# half encrypted like any other entry, and a public half written in the clear
# for a nix module to read at evaluation. The reading expression belongs in a
# consumer's own module, as `config.flake.safix.lib.publicValue "alice"
# "wg-public"` — safix ships no module that reads it for you.
{
  flake.safix.users.alice.private = {
    wg-key.generator = {
      script = ''
        wg genkey > "$out/wg-private"
        wg pubkey < "$out/wg-private" > "$out/wg-public"
        printf 'example\n' > "$out/wg-key"
      '';
      runtimeInputs = [
        "coreutils"
        "wireguard-tools"
      ];
      files = {
        wg-private.secret = true;
        wg-public.secret = false;
      };
    };

    # Each further output is an entry in its own right, which is what gives it
    # its own placement, and neither may carry a generator of its own.
    wg-private = { };
    wg-public = { };
  };
}
