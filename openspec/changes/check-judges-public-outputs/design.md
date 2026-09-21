# Design

## D1. The placement of a public output is not a lie, and is not an answer either

`resolve.nix` emits `file` and `key` on every placement, a public output's included. They are not wrong: they are the audience's document and the key this name would sit under, and they are what `list` prints, what the audience map is keyed on, and what `mints_in` reads. What they are not is where the value is.

The alternative considered and rejected was emitting `file = null` for a public output, which would make the wrong reading impossible rather than merely incorrect. It was rejected because the fields are read by five callers for four different purposes, every one of which would then need a null case, and because `Placement::public` already exists for exactly this and is already the field `edit` and `generate` route on. The bug was one reader not doing what the other two do, not a field that should not have existed.

## D2. Read `public`, never `logicalPublic`

`public` is the emitted path in both naming modes: the opaque vault destination when `flake.safix.namingKey` is set, the readable one otherwise. `logicalPublic` is the readable input the opaque name was hashed from, and is deliberately `null` outside vault mode (design V14) because outside vault mode the readable name *is* the emitted one.

So a reader that consulted `logicalPublic` would find nothing for every consumer not using a vault — which is most of them — and would be reading the pre-migration location for the ones that are. `public_finding` takes the path `values` read off `public`, and `crate::relocation` remains the only reader of the logical fields.

## D3. A separate finding rather than a flag on `ValuelessName`

`Finding::ValuelessName` carries `generated: bool`, and the smallest possible change would have been to force that flag true for a public output and let the existing renderer offer `generate <user> <name>`.

Rejected on two counts. The `file` a `ValuelessName` carries is the document, and the operator needs the public path here — the two are different files and the paragraph names one of them. And the remedy names the entry rather than the output: one run of one generator writes every one of its outputs, so `generate alice wg-private` is the run to make, while `generate alice wg-public` would read as a second one. `generate` accepts either spelling, so this is about what the sentence says rather than about what it does.

`Finding` is `#[non_exhaustive]` and every renderer already has a fallthrough arm, so the variant is additive for an embedder.

## D4. The remedy's producer is looked up, not assumed

`Placements::producer_of` answers "which generator writes this name for this user" off the placements rather than off the run plan, which is what lets `check` answer on a tree the plan refuses (a cycle, two producers for one name). `public_finding` uses it and falls back to the output's own name when it answers nothing, so a tree whose resolver said "public" and whose placements name no producer still prints a command `generate` accepts rather than an empty one.

## D5. Empty counts as absent

`public::holds_a_value` is the executor's own question — a file exists and has a non-zero length — and is reused verbatim rather than re-asked as `Path::exists`. An empty file is what a truncated write leaves behind, and a report and a run that disagreed about whether a mint is still owed would send an operator in a circle.
