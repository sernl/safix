# Design

## Context

See proposal.md. `picker/mod.rs` decodes escape sequences itself (`Pending::{None,Escape,Csi,Ss3}`) and binds only the four plain arrows; the query is an append-only `String`; `layout::frame` builds the frame bottom-up through `table::aligned` over plain strings, cuts each line by character count, and draws the pane title as the only separator; `search::Query` answers `bool` per row.

## Goals / Non-Goals

**Goals:**
- In-place query editing with the keys every line editor uses.
- A frame whose reading order is one direction: rows upward from their titles, value below a rule, query at the bottom.
- Visible evidence of what matched and which columns were searched, without breaking the cursor, the tints or the width cut.

**Non-Goals:**
- Persisting the caret or the column scroll.
- Multi-line queries, history, or a second search syntax.

## Decisions

### D1. Caret as a char index, edits as ordinary string operations

`Session` gains `caret: usize` counted in characters. Typed bytes are inserted at the caret's byte offset; Backspace and Delete remove one char on either side; Home/End clamp. Every edit calls `refilter()`; caret moves do not. `layout::cursor` takes the caret rather than the query length. Alternative considered: a rope or a `Vec<char>` — rejected, queries are short and a `String` with `char_indices` is enough.

### D2. Decoding the modified and editing sequences

`sequence` receives the CSI parameters as well as the final byte. Bindings: `A/B` list, `D/C` caret, `1;5D`/`1;5C` column scroll (Ctrl), `H`/`F`/`1~`/`4~`/`7~`/`8~` Home/End, `3~` Delete, SS3 `H`/`F` Home/End. Any other complete sequence is consumed and ignored; today it is ignored as well, so only the bare `\x1b` + unknown byte cancel changes: a lone Escape (no follow-up within the read) still cancels, a recognised-but-unbound CSI does not. Ctrl+A/Ctrl+E are bound as Home/End because they cost nothing and terminals disagree about Home/End encodings.

### D3. Styled cells instead of a post-hoc paint

`layout::Line.cells` becomes `Vec<Cell>` where `Cell { text: String, spans: Vec<Range<usize>> }` (char ranges). Widths are computed from `text` alone with `table::widths` (a small extraction from `table::aligned` that returns the column widths); `frame` pads by visible width and emits emphasis (`\x1b[1;4m` … `\x1b[22;24m`) around each span. Reverse video and tints stay per-line sequences, so emphasis composes: bold and underline are attributes the cursor's reverse video and the row colour do not touch. `cut` walks chars, counts only visible ones, and appends `RESET` once at the terminal width; because every emphasis is closed with a non-colour reset, an open span at the cut is closed by that final `RESET`.

### D4. Match spans from the search

`search::Query::spans(&self, cells: &[String]) -> Option<Vec<Vec<Range<usize>>>>`: `None` when the row is not admitted, otherwise the char ranges each positive term matched in each cell. `Contains` uses `match_indices` on a lowercased copy with a char-offset map back to the original (lowercasing can change byte length; it does not change char count for the characters that appear in these cells, and the map handles the rest); `Pattern` uses `find_iter`; `Exact` is the whole cell. Exclusion terms add nothing. `admits` becomes `spans(...).is_some()` so the two cannot disagree. `Query::searched_columns()` returns the set of column indices the titles should emphasise: the union of field terms' columns, or all when any positive term is fieldless, or none when the query is empty.

### D5. Geometry

From the bottom: help, query, blank, `VALUE_LINES` of value, rule, blank, titles, rows. The rule is the pane title padded with `─` to the terminal width. `geometry` therefore reserves `VALUE_LINES + 3` for the pane block plus one line for the titles; a terminal that cannot fit the pane block keeps the titles line and the rows. `Geometry` gains `titles` and `rule` line numbers; `pane` remains the first value line for `Secret::preview_into`.

## Risks / Trade-offs

- [Terminals encode Home/End several ways] → All common encodings plus Ctrl+A/E are bound; the unbound-sequence rule means a new encoding is ignored, not a cancel.
- [Emphasis inside reverse video is faint on some palettes] → Bold+underline was chosen over a colour so it reads under both the cursor and the two tints.
- [Lowercase mapping changes char counts for a few scripts] → Spans are computed on the lowercased copy and mapped back by char index; a cell where the mapping is not one-to-one gets whole-cell emphasis rather than a wrong offset.
- [Existing layout tests pin line numbers] → They are rewritten against the new geometry rather than re-pinned by hand: the assertions name the invariants (titles under rows, rule under blank, value under rule).
