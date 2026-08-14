# safix

safix is a custody-first secrets manager for nix: secrets are declared as flake-parts module options, the encrypted file each secret lives in is derived from the audience that can read it rather than authored by hand, and the `.sops.yaml` recipient policy is generated from those same declarations.
It is a standalone replacement for `clan vars` at user scope, tied to no framework, and it serves NixOS and home-manager alike through sops-nix.
Its headline opinion: declarations may be scattered anywhere across a consumer's tree, one per file, because they are mergeable attrsets — but ciphertext placement is never scattered, because the audience picks the file.

## Status

This repository holds the scaffold and the extraction plan.
The implementation is being ported out of the `home-secret` machinery it originated in; the plan lives in `openspec/changes/extract-safix-from-dotfiles/`, and nothing in this README describes behaviour that already runs here.
The narrative documentation grows from that port rather than preceding it.
