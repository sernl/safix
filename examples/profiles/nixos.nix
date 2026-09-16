# A system-scope profile, serving the machine `deck`.
#
# It declares no fleet of its own. `safix.lib` binds the one fleet in this
# repository — `../plain-nix/fleet.nix`, the same file the dendritic example
# re-declares one statement per file — so a feature added there is a feature
# this profile resolves, with nothing to keep in agreement by hand.
#
# Every `safix.installer.*` option below is set to something other than its
# default, so what the profile asked for is readable in the manifest it builds
# rather than indistinguishable from what safix would have done unaided. That
# is also why `validate` is off: this fleet's documents are declarations, and no
# committed ciphertext backs them, so the build-time document check has nothing
# to open.
#
# Nothing outside the `safix.*` namespace is defined here, including the
# hostname and sshd: safix reads no option outside its own namespace, and a
# profile that defined a host's identity mechanism for it would make that
# reading mutual.
{ lib, pkgs, ... }:
{
  safix = {
    enable = true;

    # The flakeless binding: `mkVault` is a plain function of `{ lib }`, so
    # reaching it needs no flake reference. A consumer who has one writes
    # `safix.flake = inputs.self;` instead and sets nothing here — the
    # projection is then read off that flake's own `safix.lib` output.
    lib = (import ../../lib { inherit lib; }).mkVault {
      modules = [ ../plain-nix/fleet.nix ];
      root = ../plain-nix;
    };

    # A machine holds what people granted it and declares nothing of its own,
    # so this names one subject rather than a person and a host.
    machine = "deck";
    hostname = "deck";

    # The host's own ed25519 keys are the identity: a machine's declared
    # recipient is the age form of one, so naming a machine mints no second
    # identity and adds no enrollment step.
    identity.deriveHostKeys = true;

    installer = {
      validate = false;
      keepGenerations = 3;
      useTmpfs = true;
      log = [
        "keyImport"
        "secretChanges"
      ];
      secretsMountPoint = "/run/safix-example.d";
      symlinkPath = "/run/safix-example";
      useSystemdActivation = true;
      afterActivation = [ "setupSecrets" ];
      afterUnits = [ "age-decrypt-secrets.service" ];
      environment.SOPS_GPG_EXEC = "/run/current-system/sw/bin/gpg2";
      agePlugins = [ pkgs.age-plugin-yubikey ];
    };
  };
}
