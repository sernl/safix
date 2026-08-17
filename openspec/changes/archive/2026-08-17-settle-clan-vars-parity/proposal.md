## Why

The operator asked for feature parity with `clan vars`, naming upload, import and export.
A gap analysis against clan-core at the fleet's pinned rev (`56e35624`) found that those three absences are deliberate, argued, and bound by a committed spec requirement — and found two real gaps nothing records as deliberate: safix cannot report that a generated value was minted by a definition that has since changed, and `safix set` cannot be scripted, though safix's own bridge relies on exactly that contract when it writes into clan.
This change closes the two real gaps and records the rest, so the parity question ends with every absence either closed or on the record.

## What Changes

- `safix generate` records a digest of each generator's definition at mint, and `safix check` gains a finding class over it: a value whose recorded definition no longer matches the declaration is reported, with regeneration and reverting the edit named as the remedies. Today such a value sits silently under a definition that no longer exists — clan reports this (`invalid_generators`); safix cannot.
- `safix set` reads the value from standard input when standard input is not a terminal, exactly as `clan vars set` does, and keeps the hidden double prompt when it is. The core write path is already terminal-free behind a `ValueSource`; this is a CLI-layer source, not a core change. Empty input keeps its refusal; bytes are stored exactly as piped.
- The absences stay absent, restated rather than reopened: no upload (activation already delivers what it would), no plaintext dump and restore (the tree outlives the migration that justified it) — both already required by `extract-safix-from-dotfiles`'s safix-cli spec. A USER-RUN task carries the operator's explicit ask to overturn them, so the decision is taken with the recorded reasoning in view rather than by default.
- One absence gains a first-time recording, pending the operator's confirmation: safix has no analog of clan's flake-level (per-export) generator placement, because safix's axis is people and their custody, not machines or service exports.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-generators`: minting records the definition it minted under; regeneration refreshes the record.
- `safix-cli`: `check` gains the definition-drift finding class; `set` gains the non-interactive source; the recorded-absence requirement gains the per-export recording once confirmed.

## Impact

Affected code:

- `crates/safix-core`: the definition digest (computed over the generator record the runtime already receives), written by `generate`'s commit path, read by `check` as a fifth finding class; the stdin `ValueSource` consumed by `set`'s existing terminal-free core (`set.rs` takes `&mut dyn ValueSource`).
- `crates/safix`: the CLI-layer stdin detection for `set`; `check`'s rendering of the new finding; usage text for both.
- `modules/flake/safix`: wherever the design places the committed definition record — it cannot live under `secrets/` (that path must mean encrypted, without qualification) and should not live under `public/` (that path means declared public outputs); design settles the location.
- `README.md` and `CHANGELOG.md`, per the standing rule.

Ordering: `secret-generators` already carries unarchived deltas from `clan-generator-contract` and `adopt-generator-sandbox`; this change archives after both, keeping the capability's history single-writer.
The non-interactive `set` is also a dependency of the planned YubiKey enrollment and KeePassXC sync work, which need a scripted write path.
