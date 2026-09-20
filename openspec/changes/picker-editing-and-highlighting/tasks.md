# Tasks

## 1. Query editing

- [x] 1.1 Add the caret to `Session`, insert typed characters at it, bind Backspace and Delete on either side of it, and thread it into `layout::cursor`; verify unit tests for insert-in-middle, delete-under-caret and the cursor column.
- [x] 1.2 Extend `sequence` to receive CSI parameters and bind Left/Right (caret), Ctrl+Left/Right (columns), Home/End in the CSI, tilde and SS3 encodings, Delete, and Ctrl+A/E; make unbound complete sequences a no-op; verify with decoder tests over split reads and each encoding.
- [x] 1.3 Update `HELP` to name the editing keys and the Ctrl scroll; verify the help-line test.

## 2. Layout

- [x] 2.1 Extend `Geometry` with `titles` and `rule` lines and rebuild `geometry` from the bottom: help, query, blank, value, rule, blank, titles, rows; verify tests asserting each relation and the short-terminal case that keeps the titles and drops the pane block.
- [x] 2.2 Draw the titles under the rows, the full-width rule carrying the pane title, and the two blank lines; verify the frame tests that the bottom row is the alphabetically first entry and that every drawn line ends with reset.

## 3. Match emphasis

- [x] 3.1 Add `Query::spans` returning per-cell char ranges for admitted rows (contains via lowercased match with a char map, regex via `find_iter`, exact as whole cell, exclusions contribute nothing) and redefine `admits` over it; verify search tests for each term kind, exclusion and unparsable regex.
- [x] 3.2 Add `Query::searched_columns`; verify tests for a field term, a bare term, a mixed query and an empty query.
- [x] 3.3 Introduce styled cells in `layout::Line`, extract `table::widths`, pad by visible width, emit bold+underline around spans, emphasise searched titles, and make `cut` count visible characters and close emphasis; verify tests that emphasis composes with the cursor and tints and that a cut line closes what it opened.
- [x] 3.4 Wire spans and searched columns from `refilter` into `draw`; verify the picker unit tests still hold the single-decrypt invariant.

## 4. Verification

- [x] 4.1 Run the picker, table and search unit tests and the full workspace suite; verify all pass and clippy is clean.
- [x] 4.2 Open `safix view` on the example fleet in a real terminal: edit a query in the middle, scroll with Ctrl+arrows, narrow with `name:` and a bare term, shrink the window below the pane height; verify each behaviour by observation and record it in the change.

## Observed

`safix --entry examples/plain-nix/entry.nix view alice`, driven in a real
pseudoterminal at 140 columns. The resolved fleet needed the consume-only
placement fields (`neededForUsers`, `reloadUnits`, …) stripped by a wrapper
entry file, because `model::Placement` does not read them at this commit —
a skew this change does not touch.

- The frame reads upward: the rows in reverse alphabetical order, the
  alphabetically first (`app-credentials`) at the bottom, `NAME ORIGIN …`
  directly under it, a blank line, the rule
  `── Decrypted Value ───…` filled to column 140, the pane, a blank line, the
  query, the help.
- `deploy-tokex` matched nothing; Left then Delete removed the `x` in place and
  `deploy-token` came back with `deploy-toke` emphasised inside its name cell.
  Home, six Rights and `!` gave `deploy!-toke`; Home then Delete gave
  `eploy!-toke`, so Delete takes the character under the caret at the first
  position rather than nothing.
- Ctrl+Right twice dropped `NAME` and `ORIGIN`, leaving the titles starting at
  `SHARED`; Ctrl+Left twice restored them. The unmodified arrows moved the
  caret throughout and never scrolled.
- The bare term `token` emphasised every title and the letters `token` in every
  cell carrying them; `name:token` emphasised `NAME` alone and the letters in
  the name cell alone.
- Shrunk to 12 rows the rule, the value and both blank lines went together and
  the titles stayed under the rows; back at 40 rows the pane returned with the
  same query.
- Escape left the picker with `safix::selection_cancelled` and the terminal
  restored.
