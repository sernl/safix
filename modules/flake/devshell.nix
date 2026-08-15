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

          # The integration suite drives these against a throwaway repository,
          # and pinning them here is what makes a local `cargo test` and the
          # check that runs the same test the same backends. `strace` is the
          # syscall proof's observer, which is linux-only for the ptrace reason
          # and absent elsewhere rather than present and unusable.
          pkgs.git
        ]
        ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.strace
        ++ [

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
