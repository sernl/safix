# Publishes `flake.lib.mkVault`, the entrypoint a consumer without
# flake-parts calls to reach the same resolver projection a flake-parts
# import's `flake.safix.lib` holds. See design decision D1 in
# `openspec/changes/support-plain-nix-consumers/design.md` for why this lives
# at a new top-level `flake.lib.mkVault` rather than inside `flake.safix.lib`
# itself, and D4 for the boundary between what it returns and what an entry
# file built for the CLI declares beside it.
{ lib, ... }:
{
  options.flake.lib.mkVault = lib.mkOption {
    # `functionTo` wraps its merged value in a type-checking functor,
    # which is no longer itself a lambda; `raw` is what
    # `modules/consume/common.nix:260-284` uses for the same reason on
    # `safix.flake` and `safix.lib`, and it is what keeps
    # `nix eval .#lib.mkVault --apply builtins.isFunction` true.
    type = lib.types.raw;
    readOnly = true;
    description = ''
      A function of `{ modules, root }` that evaluates safix's resolver
      module together with `modules` through `lib.evalModules`, with `root`
      supplied as `_module.args.self`, and returns exactly the value a
      flake-parts consumer's `flake.safix.lib` holds for the same
      declarations.

      `modules` merges the same way a flake-parts consumer's own `imports`
      does, because both paths end at one `lib.evalModules` call over one
      module list: declarations scattered across several files named in
      `modules` merge identically to the same declarations in one file, and
      a module in `modules` that declares an option outside `flake.safix` is
      refused by the module system, since neither `./safix` nor this
      function's own composition declares anything freeform for it to land
      in.

      `root` is handed to `_module.args.self` unchanged; the resolver reads
      it only as a path to concatenate, so any path value is sufficient and
      it need not be a flake input.

      The returned value carries no `onboardingHook` or `enrollHook` key.
      Both are siblings of `flake.safix.lib`, not fields inside it, and a
      flake-parts consumer's own `flake.safix.lib` never carried them
      either; a consumer who wants either hook available to a flakeless CLI
      entry file declares it directly in that file, beside the `lib` field
      this function returns.
    '';
    example = lib.literalExpression ''
      inputs.safix.lib.mkVault {
        modules = [ ./secrets.nix ];
        root = ./.;
      }
    '';
  };

  config.flake.lib.mkVault =
    { modules, root }:
    (lib.evalModules {
      modules = [
        ./safix
        { _module.args.self = root; }
      ]
      ++ modules;
    }).config.flake.safix.lib;
}
