{ ... }:
{
  perSystem =
    { config, pkgs, ... }:
    {
      devShells.default = pkgs.mkShellNoCC {
        packages = [
          pkgs.age
          pkgs.jq
          pkgs.sops

          # The rust toolchain, taken from the same pinned nixpkgs the flake's
          # cargo checks build with, so a local `cargo clippy` and the check
          # named `safix-rs-clippy` are the same compiler.
          pkgs.cargo
          pkgs.cargo-deny
          pkgs.clippy
          pkgs.rustc
          pkgs.rustfmt

          # The command this repository builds, driven against this repository.
          # It reads flake.safix.* out of whatever flake its working directory
          # belongs to, so a shell holding it is a shell that can operate the
          # declarations beside it.
          config.packages.safix
        ];
      };
    };
}
