# Tasks

## 1. The finding

- [x] 1.1 Add `Finding::ValuelessPublic { user, name, file, generator }` to `crates/safix-core/src/check.rs`, documenting why it is apart from `ValuelessName`.
- [x] 1.2 Add `public_finding`, reading `crate::public::holds_a_value` off `workspace.vault_absolute(placement.public)` and naming the producer from `Placements::producer_of`; verify unit tests for minted, empty and unminted over a two-output fixture whose `logicalPublic` is `null`.
- [x] 1.3 Branch the valueless-name walk on `placement.public` before it asks the document, so a public output is never asked the ciphertext question.

## 2. The paragraph

- [x] 2.1 Render `ValuelessPublic` in `crates/safix/src/render.rs` beside `ValuelessName`, in the neighbouring headline's wording, with `safix generate <user> <producer>` as the sole remedy.

## 3. The check

- [x] 3.1 Add `a_public_output_is_judged_by_its_own_file_and_names_its_generator` to `crates/safix/tests/generators.rs`: absent, one finding naming the producing entry and no `set`; present, silence about it while the rest of the report still speaks.
- [x] 3.2 Register it as `checks.safix-check-public-output` in `modules/flake/checks/cli.nix`.
- [x] 3.3 Confirm the perturbation: removing the branch from 1.3 turns the check red on the headline.
