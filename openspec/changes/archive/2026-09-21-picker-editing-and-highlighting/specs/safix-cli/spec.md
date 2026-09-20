# Spec Delta

## ADDED Requirements

### Requirement: The picker edits its query at a caret

The picker SHALL keep a caret inside the query. Typed characters SHALL insert at the caret; Backspace SHALL delete the character before it and Delete the character under it; Left and Right SHALL move it one character; Home and End SHALL move it to either end. Ctrl+Left and Ctrl+Right SHALL scroll the columns. Every edit SHALL re-narrow the rows exactly as appending did. The help line SHALL name these keys. Escape and Ctrl+C SHALL still cancel, and an escape sequence the picker does not bind SHALL be ignored rather than cancelling.

#### Scenario: A mistake in the middle is fixed in place
- **WHEN** the operator moves the caret left past a mistyped character, deletes it and types the correction
- **THEN** the query reads with the correction in that position
- **AND** the rows are narrowed to the corrected query

#### Scenario: Arrows no longer scroll
- **WHEN** Left or Right is pressed with columns scrolled off-screen
- **THEN** the columns do not scroll and the caret moves
- **AND** Ctrl+Left or Ctrl+Right scrolls them

#### Scenario: An unbound sequence is not a cancel
- **WHEN** a key the picker does not bind arrives as an escape sequence, such as Page Down
- **THEN** the picker keeps running with nothing changed

### Requirement: The picker separates the list from the value and titles the list from below

The column titles SHALL be the last line of the list, directly below the alphabetically first row. Below the titles the picker SHALL draw one blank line, one rule spanning the terminal width carrying the pane's title, the decrypted value, one blank line, the query and the help. A terminal too short for the pane SHALL keep the titles beneath the rows and drop the pane, the rule and its padding together.

#### Scenario: The frame reads upward from its headings
- **WHEN** the picker draws on a terminal tall enough for the pane
- **THEN** the titles sit under the rows, the rule sits one blank line under the titles, the value follows the rule, and one blank line separates the value from the query

#### Scenario: A short terminal keeps the titles
- **WHEN** the terminal has no room for the pane
- **THEN** the rows and their titles are drawn and nothing of the pane, the rule or its padding is

### Requirement: The picker shows what matched and where it looked

For every row the query admits, the characters each term matched SHALL be emphasised inside the cell they matched. The title of every column the query searches SHALL be emphasised: a term with a field emphasises that field's title, a term without one emphasises every title, an empty query emphasises none. Exclusion terms SHALL emphasise nothing. Emphasis SHALL compose with the cursor's reverse video and the row tints, and a line wider than the terminal SHALL be cut by visible width so no emphasis sequence is split.

#### Scenario: A field term lights one column
- **WHEN** the query is `name:tok`
- **THEN** only the name title is emphasised
- **AND** in each admitted row the letters `tok` inside the name cell are emphasised and nothing else is

#### Scenario: A bare term lights every column it could match
- **WHEN** the query is `tok`
- **THEN** every column title is emphasised
- **AND** every cell containing `tok` has those letters emphasised, case-insensitively

#### Scenario: An exclusion emphasises nothing
- **WHEN** the query is `!tok`
- **THEN** admitted rows carry no emphasis and no title is emphasised for that term

#### Scenario: Emphasis survives the cut
- **WHEN** an admitted row is wider than the terminal
- **THEN** the drawn line is cut to the terminal width counted in visible characters
- **AND** every emphasis it opened is closed before the line ends
