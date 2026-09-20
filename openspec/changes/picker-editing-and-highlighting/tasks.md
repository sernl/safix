# Tasks

## 1. Query editing

- [ ] 1.1 Add the caret to `Session`, insert typed characters at it, bind Backspace and Delete on either side of it, and thread it into `layout::cursor`; verify unit tests for insert-in-middle, delete-under-caret and the cursor column.
- [ ] 1.2 Extend `sequence` to receive CSI parameters and bind Left/Right (caret), Ctrl+Left/Right (columns), Home/End in the CSI, tilde and SS3 encodings, Delete, and Ctrl+A/E; make unbound complete sequences a no-op; verify with decoder tests over split reads and each encoding.
- [ ] 1.3 Update `HELP` to name the editing keys and the Ctrl scroll; verify the help-line test.

## 2. Layout

- [ ] 2.1 Extend `Geometry` with `titles` and `rule` lines and rebuild `geometry` from the bottom: help, query, blank, value, rule, blank, titles, rows; verify tests asserting each relation and the short-terminal case that keeps the titles and drops the pane block.
- [ ] 2.2 Draw the titles under the rows, the full-width rule carrying the pane title, and the two blank lines; verify the frame tests that the bottom row is the alphabetically first entry and that every drawn line ends with reset.

## 3. Match emphasis

- [ ] 3.1 Add `Query::spans` returning per-cell char ranges for admitted rows (contains via lowercased match with a char map, regex via `find_iter`, exact as whole cell, exclusions contribute nothing) and redefine `admits` over it; verify search tests for each term kind, exclusion and unparsable regex.
- [ ] 3.2 Add `Query::searched_columns`; verify tests for a field term, a bare term, a mixed query and an empty query.
- [ ] 3.3 Introduce styled cells in `layout::Line`, extract `table::widths`, pad by visible width, emit bold+underline around spans, emphasise searched titles, and make `cut` count visible characters and close emphasis; verify tests that emphasis composes with the cursor and tints and that a cut line closes what it opened.
- [ ] 3.4 Wire spans and searched columns from `refilter` into `draw`; verify the picker unit tests still hold the single-decrypt invariant.

## 4. Verification

- [ ] 4.1 Run the picker, table and search unit tests and the full workspace suite; verify all pass and clippy is clean.
- [ ] 4.2 Open `safix view` on the example fleet in a real terminal: edit a query in the middle, scroll with Ctrl+arrows, narrow with `name:` and a bare term, shrink the window below the pane height; verify each behaviour by observation and record it in the change.
