# The two fleet-wide declarations, as a module merged beside the shared fleet
# rather than into it.
#
# Both are fleet-wide in their consequence: relocating the roots moves every
# path in every resolved entry, and a naming key makes every resolved name a
# keyed hash, so declaring either in `../plain-nix/fleet.nix` would stop that
# fleet showing the readable layout the documentation is written against. Merged
# here, both are exercised where their consequence is visible — what a profile
# reads — and `safix-examples-profiles`' `relocated` row is what holds it.
{
  flake.safix = {
    # One parent for all three trees, expressed by three strings sharing it.
    # The names are the consumer's, and so is the promise each name makes: a
    # tree called `encrypted` means everything under it is ciphertext without
    # qualification.
    storage = {
      encrypted = ".safix/encrypted";
      plaintextOutputs = ".safix/plaintext-outputs";
      generatorRecords = ".safix/generator-records";
    };

    # A vault: the second repository every ciphertext document, public value and
    # generator record resolves under, with every name a keyed hash of the
    # entry's identity rather than of any tree's spelling.
    #
    # The key is visible to anyone who can evaluate this tree, so it hides a
    # name from the vault's own host and from a reader holding only the vault —
    # never from the nix store. This one is an example value; mint your own with
    # `openssl rand -hex 32`.
    vault = {
      root = ./vault;
      namingKey = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
    };
  };
}
