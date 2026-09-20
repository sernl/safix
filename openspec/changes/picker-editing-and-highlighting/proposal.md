# Proposal

## Why

The `safix view` picker takes Left and Right for horizontal column scrolling, so the query cannot be edited except by backspacing to the mistake. The decrypted value sits directly under the list with no separation, the column titles are at the far end of a list that reads bottom-up, and a typed query narrows the rows without showing what matched or where.

## What Changes

- Left, Right, Home, End and Delete edit the query at a caret; typed characters insert at the caret. Ctrl+Left and Ctrl+Right scroll the columns.
- Column titles move to the bottom of the list, directly above the value pane, so the rows read upward from their headings the way they already read upward from the query.
- A full-width rule separates the list from the decrypted value, with one blank line above the rule and one below the value.
- Matched characters are emphasised inside every cell a query term matched, and the titles of the columns a query searches are emphasised too, so a field term visibly narrows to its column and a bare term visibly searches them all.
- The help line names the new keys.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `safix-cli`: the choosing scenario gains the editing keys, the layout and the match emphasis as observable behaviour of the picker.

## Impact

- `crates/safix/src/picker/{mod.rs,layout.rs,search.rs}`: caret state, key decoding for the modified and editing sequences, styled cells with visible-width truncation, match spans from the search.
- `crates/safix/src/table.rs`: width computation reused for styled cells.
- The picker's layout and search unit tests are updated where they pin the old geometry or the boolean-only search.
- `README.md` / `docs/`: the picker's key list.
- No new dependencies.
