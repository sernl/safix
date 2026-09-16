# A user-scope profile, serving `alice` on the host `deck`.
#
# The same binding as `./nixos.nix` and the same fleet: one declaration serves
# both scopes, and nothing in `../plain-nix/fleet.nix` states which scope it is
# for. What differs here is what a user scope has — a hostname to select
# `perHost` and `perTag` layers by, an identity a person holds — and what it
# does not have, which is an ownership axis.
#
# No entry is given an `owner` or a `group` by this profile, and none may be:
# only system scope has an ownership axis, so the user-scope materialization
# refuses an entry carrying either rather than dropping the field, naming the
# entry and the field. A dropped ownership field reads afterwards as an
# ownership claim that was honoured. `safix-examples-profiles`' `ownershipRefused`
# row assembles that profile and holds the refusal.
{ lib, ... }:
{
  safix = {
    enable = true;

    lib = (import ../../lib { inherit lib; }).mkVault {
      modules = [ ../plain-nix/fleet.nix ];
      root = ../plain-nix;
    };

    user = "alice";

    # A standalone profile has no host configuration to read a hostname from,
    # so it is named. `deck` is what selects alice's `perHost.deck` layer.
    hostname = "deck";

    # The tag vocabulary is the consumer's, handed over here. `portable` is
    # what selects alice's `perTag.portable` layer, where a `force` re-adds an
    # entry the same layer omits.
    tags = [ "portable" ];

    identity = {
      # A string rather than a nix path: a nix path interpolated into a
      # declaration is copied into the world-readable store, and here that copy
      # would be the identity itself.
      keyFile = "/home/alice/.config/safix/keys.txt";
      sshKeyPaths = [ "/home/alice/.ssh/id_ed25519" ];

      # Activation mints the key file above when it is absent. It mints an
      # identity and never a recipient: the public half still has to reach
      # `flake.safix.users.alice.recipient` before anything is encrypted to it.
      generateKey = true;
    };

    # Off here so the example shows the option rather than its default. Left on,
    # a switch is refused when no configured identity is present and readable —
    # which is the atomic refusal point a user scope has and a system scope does
    # not.
    identityPreflight = false;

    installer = {
      validate = false;
      keepGenerations = 3;
      log = [ "secretChanges" ];
      secretsMountPoint = "%r/safix-example.d";
      symlinkPath = "%r/safix-example";
    };
  };
}
