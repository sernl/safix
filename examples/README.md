# Two consumers of one fleet

Both examples here declare the identical fleet: alice and bob, a catalogue entry each carries independently and one they share, a machine, a service running on it, a group, an organization holding recovery custody, a silo, one generator, and placement adjustments per host and per tag.
`modules/flake/checks/examples.nix` evaluates both and asserts they resolve field for field, so the two cannot drift apart silently.

The one-line difference: `plain-nix` calls `lib.mkVault` from a plain file with no flake-parts and no flake at all; `dendritic` imports `flakeModules.default` into an ordinary flake-parts flake and scatters its declarations one per file.

## `plain-nix`

`fleet.nix` declares the whole fleet in one file.
`hooks.nix` declares `onboardingHook` and `enrollHook` directly, beside `lib` rather than inside it, per design decision D4 in `openspec/changes/support-plain-nix-consumers/design.md`.
`entry.nix` is the `--entry` target itself: it reaches `lib.mkVault` through `builtins.getFlake`, since a file with no flake of its own has no `inputs.safix` to read it from, and merges the two hooks in beside the projection `mkVault` returns.

Run it directly:

```console
$ safix --entry examples/plain-nix/entry.nix list
```

or export `SAFIX_ENTRY=examples/plain-nix/entry.nix` and drop the flag from every subsequent invocation.
Twelve of safix's thirteen verbs behave identically under `--entry`; `generate` additionally needs `--nixpkgs <flake-ref>` or `SAFIX_NIXPKGS`, because the generator sandbox resolves its tools through a flake regardless of how the declarations themselves were reached.

Copy this one if your tree has no flake at all, or if it has a flake that does not use flake-parts.

## `dendritic`

A complete flake-parts flake: `flake.nix` imports `safix.flakeModules.default` alongside twenty-two single-declaration files under `modules/`, one per secret, subject, or record — a catalogue entry, a person's profile, one `carries` selection, one `sharedWith` grant, one placement adjustment.
None of them read a path, a filename, or each other; the module system merges them into the same fleet `plain-nix/fleet.nix` declares in one file.

Copy this one if your tree already uses flake-parts, or if you want your own declarations to scatter the same way safix's own opinion says they may.

## Reading either as a working example

Both are complete, runnable consumers rather than fragments: every file under `plain-nix/` and every file under `dendritic/` is read by `modules/flake/checks/examples.nix`, which is what keeps this document from describing something nobody evaluates.
