# Proposal

## Why

The README is a 1068-line manual in which the tutorial ends at line 64 and flake-parts is the assumed shape of a consumer. A NixOS user with an ordinary flake cannot see in one pass how to declare a secret, hand it to a host, and read it from a service. The reference material is dense, nothing outside the README exists, and no check proves the README's code evaluates.

## What Changes

- `README.md` becomes a quickstart of about three hundred lines for a plain NixOS flake. One example runs through it: declare a person and a machine, bind the declarations with `mkVault`, set a value, install it on a host, read it from a service, mint one with a generator, put it on a rotation policy, render a template, and browse it in the picker. Each snippet continues the previous one.
- Every README snippet is lifted from `examples/quickstart/`, a real flake a check evaluates. A second check compares each fenced block in the README with the file or region it names, so the README cannot drift from code that builds.
- `docs/` gains the Diátaxis tree: tutorials, guides, concepts and reference. Everything the README no longer carries lands there once: the full declaration and profile option references, the verb table, the sync chapter with its same-shaped subsections, migration plans, generators, hardware keys, the vault, identity backup and recovery.
- Three guides carry the custody guidance the previous work only answered in conversation: separating machine keys from human keys for unattended hosts, choosing SOPS over raw age where an inspectable recipient roster matters, and rotating a value after removing access.
- flake-parts is documented as fully supported, in one README section and one guide, with `examples/dendritic` as its worked example.
- The documentation contract is restated over the whole set: each verb and option documented exactly once across README and docs, sentences bounded, no check named, rules stated once.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `documentation`: the "exactly once" and "one sync chapter" requirements are stated over README plus `docs/` rather than the README alone; a new requirement holds README snippets to the checked example.

## Impact

- `README.md` rewritten; `docs/` created; `examples/quickstart/` added; `examples/README.md` gains the fourth example.
- `modules/flake/checks/`: `examples-quickstart` evaluates the example's host and home profiles; `readme-snippets` diffs fenced blocks against their sources.
- `CONTRIBUTING.md`: how the snippet check reads a block, so a contributor editing the README knows what it holds.
- The three sibling changes (`recoverable-migration`, `picker-editing-and-highlighting`, `scheduled-rotation`) each land their own paragraphs into this tree; this change owns the tree's shape and the pages that exist regardless of them.
