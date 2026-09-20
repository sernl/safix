//! Where each piece of one frame goes, and what colour it is.
//!
//! The frame is built from the bottom of the terminal upwards, because the
//! bottom is where the operator's attention and the shell's own prompt already
//! are: the key help is the last line, the query is above it, the value pane
//! above that under a full-width rule, and the table above that with its column
//! titles as its last line. The rows are drawn in reverse alphabetical order,
//! so the alphabetically first entry is the bottom row of the table — the row
//! closest to its own titles and to the query being typed, and the row the
//! cursor starts on.
//!
//! # Why the titles are under the rows
//!
//! The list reads upwards, so its headings belong at the end a reader arrives
//! at. Titles above the rows are titles at the far end of a table read from the
//! bottom: the column a cell belongs to is then eight rows away from the cell.
//!
//! # Why the query and the help are positioned absolutely
//!
//! The decrypted value is written into the frame by
//! [`Secret::preview_into`](safix_core::Secret::preview_into), which draws
//! between zero and [`VALUE_LINES`] lines depending on what the value is. If
//! the query and the help followed it in the stream they would sit a different
//! number of lines from the bottom for every value, so they are written at
//! absolute line numbers and the pane is filled in afterwards. Nothing here
//! holds the value or counts its lines, which is the property that constraint
//! exists to protect.
//!
//! # Why every line is cut to the terminal's width
//!
//! A line longer than the terminal wraps, and a wrapped line is one more line
//! than the geometry accounted for: everything below it moves, and the bottom
//! line scrolls off. The table is as wide as its widest cell, so this is the
//! ordinary case on a narrow terminal rather than an edge one. The cut counts
//! visible characters, because a line carries the sequences that emphasise what
//! a query matched and those occupy no column.

use std::fmt::Write as _;
use std::ops::Range;

use crate::table;

/// The keys, in the order they are worth knowing.
pub(crate) const HELP: &str = "Enter choose \u{b7} Esc/^C cancel \u{b7} \u{2191}\u{2193} move \
                               \u{b7} \u{2190}\u{2192}/Home/End/Del edit \u{b7} \
                               ^\u{2190}\u{2192} scroll \u{b7} Tab columns \u{b7} ^P preview";

/// The value pane's own title, which the rule under the list carries.
pub(crate) const PANE_TITLE: &str = "\u{2500}\u{2500} Decrypted Value \u{2500}\u{2500}";

/// How many lines of the value the pane shows.
pub(crate) const VALUE_LINES: usize = 8;

/// What the rule is drawn out of, after the title it carries.
const RULE: char = '\u{2500}';

/// Reverse video, which is the cursor.
const CURSOR: &str = "\u{1b}[7m";

/// Bold, which is the titles line.
const HEADER: &str = "\u{1b}[1m";

/// Dim, which is a cell with nothing in it and the rule.
const DIM: &str = "\u{1b}[2m";

/// Red, which is a value past its rotation deadline.
const RED: &str = "\u{1b}[31m";

/// Back to normal intensity, which ends a [`DIM`] run without ending the
/// colour or the reverse video around it.
const UNDIM: &str = "\u{1b}[22m";

/// Bold and underlined, which is a run of characters a query term matched.
///
/// Attributes rather than a colour, so a match reads the same on a plain row,
/// on a tinted one and under the cursor's reverse video.
const EMPHASIS: &str = "\u{1b}[1;4m";

/// Normal intensity and no underline, which ends an [`EMPHASIS`] run without
/// ending the colour or the reverse video around it.
const UNEMPHASIS: &str = "\u{1b}[22;24m";

/// Yellow, which is the entry this user chose last.
const YELLOW: &str = "\u{1b}[33m";

/// Cyan, which is the most recently created entry.
const CYAN: &str = "\u{1b}[36m";

/// Green, which is a decrypted value.
pub(crate) const GREEN: &str = "\u{1b}[32m";

/// Every attribute off, which every line this draws ends with.
pub(crate) const RESET: &str = "\u{1b}[0m";

/// What a cell holds when it holds nothing.
const ABSENT: &str = "-";

/// Which lines of the terminal each piece of the frame occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Geometry {
    /// How many candidate rows there is room for above the titles.
    pub rows: usize,
    /// The line the column titles are on, which is the list's last.
    pub titles: usize,
    /// The line the rule is on, when there is room for a pane.
    pub rule: Option<usize>,
    /// The first line of the decrypted value, when there is room for a pane.
    pub pane: Option<usize>,
    /// The line the query is typed on.
    pub query: usize,
    /// The line the key help is on, which is the terminal's last.
    pub help: usize,
}

/// How this frame's lines fall on a terminal of this height.
///
/// From the bottom: the help, the query, a blank line, [`VALUE_LINES`] of
/// value, the rule, a blank line, the titles, and the rows above them.
///
/// A terminal too short for that block gets no pane and a longer list instead:
/// the preview is what a short terminal loses, because the list is what the
/// verb is for, and the titles stay under the rows because a list without them
/// does not say which column a cell is in. That the value is still decrypted in
/// that state is deliberate — the pane is one line of a resize away, and a
/// picker whose decryption depended on the window height would decrypt on a
/// resize.
pub(crate) fn geometry(lines: usize, preview: bool) -> Geometry {
    let help = lines.max(1);
    let query = help.saturating_sub(1).max(1);
    // Above the query: the blank line, the value, the rule and the blank line
    // over it, then the titles and at least one row.
    let room = query.saturating_sub(1);
    let pane = (preview && room >= VALUE_LINES.saturating_add(5))
        .then(|| query.saturating_sub(VALUE_LINES).saturating_sub(1));
    let titles = pane.map_or_else(|| room.max(1), |pane| pane.saturating_sub(3));
    Geometry {
        rows: titles.saturating_sub(1),
        titles,
        rule: pane.map(|pane| pane.saturating_sub(1)),
        pane,
        query,
        help,
    }
}

/// What tints one row before the cursor is applied to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tint {
    /// Nothing about this row is worth a colour.
    Plain,
    /// The entry this user chose last.
    LastChosen,
    /// The entry created most recently.
    Newest,
    /// The entry whose rotation deadline has passed.
    Due,
}

impl Tint {
    /// The sequence this tint starts a line with.
    const fn sequence(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::LastChosen => YELLOW,
            Self::Newest => CYAN,
            Self::Due => RED,
        }
    }
}

/// One cell on its way to the screen.
///
/// The text and the matches are carried separately all the way to the write,
/// because the padding has to count the text alone: a cell whose width included
/// its emphasis sequences would shift every column to its right.
pub(crate) struct Cell {
    /// What the cell says.
    pub text: String,
    /// Which of its characters a query term matched, as sorted, disjoint
    /// character ranges.
    pub spans: Vec<Range<usize>>,
}

/// One candidate row on its way to the screen.
pub(crate) struct Line {
    /// Its cells, already narrowed to the columns being shown.
    pub cells: Vec<Cell>,
    /// What colour it is.
    pub tint: Tint,
    /// Whether the cursor is on it.
    pub cursor: bool,
}

/// Everything one frame is drawn from.
pub(crate) struct View<'a> {
    /// Where the pieces go.
    pub geometry: Geometry,
    /// How wide the terminal is.
    pub columns: usize,
    /// The column titles, narrowed to the columns being shown, with the ones
    /// the query searches emphasised.
    pub titles: Vec<Cell>,
    /// The rows being shown, in alphabetical order: the first of them is the
    /// bottom row of the table.
    pub lines: Vec<Line>,
    /// What has been typed.
    pub query: &'a str,
    /// Where the caret is in the query, counted in characters.
    pub caret: usize,
}

/// One frame, less the decrypted value the caller writes into the pane.
///
/// Ends with the cursor at the caret, so a terminal drawing its own block
/// cursor puts it where the next character will go.
pub(crate) fn frame(view: &View) -> String {
    let mut out = String::from("\u{1b}[H\u{1b}[2J");

    // Every row and the titles, as plain text, which is what the columns are
    // measured from.
    let mut measured = vec![texts(&view.titles)];
    measured.extend(view.lines.iter().map(|line| texts(&line.cells)));
    let widths = table::widths(&measured);

    let first = view.geometry.titles.saturating_sub(view.lines.len());
    for _ in 0..first.saturating_sub(1) {
        out.push('\n');
    }

    // Reversed here rather than by the caller: which end of the table the
    // alphabetically first row is at is this module's decision, and a caller
    // that reversed its own rows would be making it twice.
    for line in view.lines.iter().rev() {
        let text = cut(&row(&line.cells, &widths, ""), view.columns);
        let _ = writeln!(out, "{}", painted(line, &text));
    }
    // The titles put the bold back after each emphasised run, because ending an
    // underline ends the bold with it.
    let _ = writeln!(
        out,
        "{HEADER}{}{RESET}",
        cut(&row(&view.titles, &widths, HEADER), view.columns)
    );

    if let Some(rule) = view.geometry.rule {
        let _ = write!(
            out,
            "\u{1b}[{rule};1H{DIM}{}{RESET}",
            cut(&ruled(view.columns), view.columns)
        );
    }

    let query = cut(&format!("> {}", view.query), view.columns);
    let _ = write!(out, "\u{1b}[{};1H{query}{RESET}", view.geometry.query);
    let _ = write!(
        out,
        "\u{1b}[{};1H{DIM}{}{RESET}",
        view.geometry.help,
        cut(HELP, view.columns)
    );
    out.push_str(&cursor(view.geometry, view.caret));
    out
}

/// The sequence that leaves the terminal's cursor on the caret.
///
/// Written again after the value pane is filled in, because filling it in moves
/// the cursor: a block cursor resting in the middle of a decrypted value would
/// read as part of it.
pub(crate) fn cursor(geometry: Geometry, caret: usize) -> String {
    format!("\u{1b}[{};{}H", geometry.query, caret.saturating_add(3))
}

/// The rule that separates the list from the value, carrying the pane's title.
fn ruled(columns: usize) -> String {
    let mut rule = String::from(PANE_TITLE);
    for _ in PANE_TITLE.chars().count()..columns {
        rule.push(RULE);
    }
    rule
}

/// Each cell's text, which is what a column's width is measured from.
fn texts(cells: &[Cell]) -> Vec<String> {
    cells.iter().map(|cell| cell.text.clone()).collect()
}

/// One row's cells, padded to these widths, with every match emphasised.
///
/// The padding is [`table::aligned`]'s: the widest cell of the column plus
/// [`table::GAP`], and the row's last cell is not padded at all. Counted in
/// characters of the text, so a cell carrying emphasis is exactly as wide as
/// the same cell without it.
fn row(cells: &[Cell], widths: &[usize], resume: &str) -> String {
    let mut out = String::new();
    let last = cells.len().saturating_sub(1);
    for (index, cell) in cells.iter().enumerate() {
        out.push_str(&emphasised(cell, resume));
        if index == last {
            continue;
        }
        let width = widths.get(index).copied().unwrap_or(0);
        let padding = width
            .saturating_sub(cell.text.chars().count())
            .saturating_add(table::GAP);
        for _ in 0..padding {
            out.push(' ');
        }
    }
    out
}

/// One cell's text with every matched run bold and underlined.
///
/// `resume` is written after each run, because ending an underline and a bold
/// together ends whatever bold the line itself opened; the titles line puts its
/// own back and a row has none to put back.
fn emphasised(cell: &Cell, resume: &str) -> String {
    if cell.spans.is_empty() {
        return cell.text.clone();
    }
    let mut out = String::new();
    let mut at = 0;
    for span in &cell.spans {
        let start = span.start.max(at);
        if span.end <= start {
            continue;
        }
        out.extend(cell.text.chars().skip(at).take(start.saturating_sub(at)));
        out.push_str(EMPHASIS);
        out.extend(
            cell.text
                .chars()
                .skip(start)
                .take(span.end.saturating_sub(start)),
        );
        out.push_str(UNEMPHASIS);
        out.push_str(resume);
        at = span.end;
    }
    out.extend(cell.text.chars().skip(at));
    out
}

/// One row's line, with its tint, its cursor and its empty cells.
fn painted(line: &Line, text: &str) -> String {
    let cursor = if line.cursor { CURSOR } else { "" };
    format!("{cursor}{}{}{RESET}", line.tint.sequence(), dimmed(text))
}

/// The same line with every empty cell dimmed.
///
/// Dimmed and then undimmed rather than reset, so a row that is yellow or under
/// the cursor stays yellow and under the cursor across the cell. A cell a term
/// matched carries its emphasis sequences and is therefore not the bare `-`
/// this dims, which is right: a match is worth more than the absence it is in.
fn dimmed(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let spaces = rest.find(|character: char| character != ' ');
        let (gap, tail) = rest
            .split_at_checked(spaces.unwrap_or(rest.len()))
            .unwrap_or((rest, ""));
        out.push_str(gap);
        rest = tail;
        let width = rest.find(' ').unwrap_or(rest.len());
        let (cell, tail) = rest.split_at_checked(width).unwrap_or((rest, ""));
        if cell == ABSENT {
            let _ = write!(out, "{DIM}{cell}{UNDIM}");
        } else {
            out.push_str(cell);
        }
        rest = tail;
    }
    out
}

/// The first `columns` visible characters of a line.
///
/// Escape sequences pass through uncounted, because they occupy no column. A
/// line that is cut ends with every attribute off, which is what closes an
/// emphasised run the cut fell inside: every sequence this module opens is
/// closed by a plain [`RESET`] as well as by its own ending.
fn cut(text: &str, columns: usize) -> String {
    let mut out = String::new();
    let mut visible = 0_usize;
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            out.push(character);
            for inside in characters.by_ref() {
                out.push(inside);
                if inside.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        if visible >= columns {
            out.push_str(RESET);
            return out;
        }
        out.push(character);
        visible = visible.saturating_add(1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        CURSOR, CYAN, Cell, DIM, EMPHASIS, Geometry, HEADER, HELP, Line, PANE_TITLE, RESET, Range,
        Tint, UNEMPHASIS, VALUE_LINES, View, frame, geometry,
    };

    /// One span, as the list a cell carries.
    fn only(span: Range<usize>) -> Vec<Range<usize>> {
        std::iter::once(span).collect()
    }

    /// A row of cells nothing matched.
    fn cells(values: &[&str]) -> Vec<Cell> {
        values
            .iter()
            .map(|value| Cell {
                text: (*value).to_owned(),
                spans: Vec::new(),
            })
            .collect()
    }

    /// One candidate row.
    fn line(name: &str, tint: Tint, cursor: bool) -> Line {
        Line {
            cells: cells(&[name, "private"]),
            tint,
            cursor,
        }
    }

    /// A view over these rows, in alphabetical order.
    fn view(lines: Vec<Line>, query: &str, preview: bool) -> View<'_> {
        View {
            geometry: geometry(24, preview),
            columns: 80,
            titles: cells(&["NAME", "ORIGIN"]),
            lines,
            query,
            caret: query.chars().count(),
        }
    }

    /// The frame's lines, undecorated.
    ///
    /// A sequence that moves the cursor starts a line of its own, because that
    /// is what it does on the terminal: the rule, the query and the help are
    /// written at absolute line numbers rather than after a newline, so a
    /// reader that dropped the sequence would read them as one line.
    fn drawn(frame: &str) -> Vec<String> {
        let mut plain = String::new();
        let mut rest = frame;
        while let Some(at) = rest.find('\u{1b}') {
            plain.push_str(rest.get(..at).unwrap_or_default());
            let tail = rest.get(at.saturating_add(1)..).unwrap_or_default();
            let (end, final_byte) = tail
                .char_indices()
                .find(|(_, character)| character.is_ascii_alphabetic())
                .map_or((tail.len(), ' '), |(index, character)| {
                    (index.saturating_add(1), character)
                });
            if final_byte == 'H' {
                plain.push('\n');
            }
            rest = tail.get(end..).unwrap_or_default();
        }
        plain.push_str(rest);
        plain.lines().map(str::to_owned).collect()
    }

    /// Every line of a drawn frame that carries something.
    fn written(frame: &str) -> Vec<String> {
        drawn(frame)
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect()
    }

    /// The frame reads in one direction: rows, their titles, the rule, the
    /// value, the query, the help — each piece where the one below it leaves
    /// room for it.
    #[test]
    fn each_piece_of_the_frame_sits_under_the_one_it_titles() {
        let geometry = geometry(24, true);
        assert_eq!(geometry.help, 24, "the help is not the last line");
        assert_eq!(
            geometry.query,
            geometry.help.saturating_sub(1),
            "the query is not the line above the help"
        );
        let pane = geometry.pane.expect("no pane on a terminal with room");
        let rule = geometry.rule.expect("a pane was drawn with no rule");
        assert_eq!(
            pane.saturating_add(VALUE_LINES),
            geometry.query.saturating_sub(1),
            "the blank line between the value and the query is missing"
        );
        assert_eq!(
            rule.saturating_add(1),
            pane,
            "the value does not follow the rule"
        );
        assert_eq!(
            geometry.titles.saturating_add(2),
            rule,
            "the rule is not one blank line under the titles"
        );
        assert_eq!(
            geometry.rows,
            geometry.titles.saturating_sub(1),
            "the rows do not fill everything above the titles"
        );
    }

    /// A terminal too short for the pane keeps the titles and drops the pane,
    /// the rule and both blank lines together.
    #[test]
    fn a_short_terminal_loses_the_pane_and_keeps_the_titles() {
        let short = geometry(6, true);
        assert_eq!(short.pane, None);
        assert_eq!(short.rule, None);
        assert_eq!(short.help, 6);
        assert_eq!(short.query, 5);
        assert_eq!(
            short.titles,
            short.query.saturating_sub(1),
            "the titles are not directly above the query"
        );
        assert!(short.rows >= 1, "no room was left for a candidate");
        assert_eq!(
            geometry(1, true),
            Geometry {
                rows: 0,
                titles: 1,
                rule: None,
                pane: None,
                query: 1,
                help: 1
            }
        );
        let rendered = frame(&View {
            geometry: short,
            columns: 80,
            titles: cells(&["NAME", "ORIGIN"]),
            lines: vec![line("api", Tint::Plain, true)],
            query: "",
            caret: 0,
        });
        assert!(
            !rendered.contains(PANE_TITLE),
            "a short terminal drew the rule anyway\n{rendered:?}"
        );
        assert!(
            written(&rendered)
                .iter()
                .any(|drawn| drawn.starts_with("NAME")),
            "a short terminal dropped the titles\n{rendered:?}"
        );
    }

    /// The table is drawn bottom-up: the alphabetically first row at the
    /// bottom, and its titles under it.
    #[test]
    fn the_bottom_row_is_the_alphabetically_first_entry_and_the_titles_are_under_it() {
        let rendered = frame(&view(
            vec![
                line("aliased-secret", Tint::Plain, true),
                line("api-token", Tint::Plain, false),
                line("mail-password", Tint::Plain, false),
            ],
            "",
            true,
        ));
        let lines = written(&rendered);
        let names: Vec<&str> = lines
            .iter()
            .filter_map(|line| line.split_whitespace().next())
            .take(4)
            .collect();
        assert_eq!(
            names,
            vec!["mail-password", "api-token", "aliased-secret", "NAME"],
            "the frame is not the rows in reverse order with their titles under them\n{lines:?}"
        );
        assert!(
            lines
                .get(4)
                .is_some_and(|line| line.starts_with(PANE_TITLE)),
            "the rule is not under the titles\n{lines:?}"
        );
    }

    /// The rule is drawn only when the pane is, and it spans the terminal.
    #[test]
    fn the_rule_is_as_wide_as_the_terminal_and_only_drawn_with_the_pane() {
        let with = frame(&view(vec![line("api", Tint::Plain, true)], "", true));
        let rule = written(&with)
            .into_iter()
            .find(|line| line.starts_with(PANE_TITLE))
            .expect("no rule was drawn");
        assert_eq!(
            rule.chars().count(),
            80,
            "the rule does not span the terminal: {rule:?}"
        );
        let without = frame(&view(vec![line("api", Tint::Plain, true)], "", false));
        assert!(
            !without.contains(PANE_TITLE),
            "a suppressed preview drew its rule anyway"
        );
        assert_eq!(geometry(24, false).pane, None);
    }

    #[test]
    fn the_titles_are_bold_and_every_drawn_line_ends_reset() {
        let rendered = frame(&view(vec![line("api", Tint::Plain, false)], "", true));
        assert!(rendered.contains(&format!("{HEADER}NAME")));
        for line in rendered.lines() {
            // The lines with nothing on them are the padding above the table;
            // the line the screen is cleared by and the line the cursor
            // sequence ends carry no text to leave an attribute on.
            if line.trim().is_empty() || line.ends_with('H') || line.ends_with('J') {
                continue;
            }
            assert!(
                line.ends_with(RESET),
                "a line did not end with the attributes off: {line:?}"
            );
        }
    }

    #[test]
    fn the_cursor_row_is_reverse_video_over_its_own_colour() {
        let rendered = frame(&view(
            vec![
                line("aliased-secret", Tint::LastChosen, true),
                line("api-token", Tint::Newest, false),
            ],
            "",
            true,
        ));
        assert!(
            rendered.contains(&format!("{CURSOR}\u{1b}[33maliased-secret")),
            "the cursor row lost its colour or its reverse video\n{rendered:?}"
        );
        assert!(
            rendered.contains(&format!("{CYAN}api-token")),
            "the newest row is not cyan\n{rendered:?}"
        );
    }

    /// An empty cell is dimmed without ending the row's own colour.
    #[test]
    fn an_absent_cell_is_dimmed_inside_a_tinted_row() {
        let rendered = frame(&View {
            geometry: geometry(24, true),
            columns: 80,
            titles: cells(&["NAME", "SHARED"]),
            lines: vec![Line {
                cells: cells(&["api-token", "-"]),
                tint: Tint::LastChosen,
                cursor: false,
            }],
            query: "",
            caret: 0,
        });
        assert!(
            rendered.contains(&format!("{DIM}-\u{1b}[22m")),
            "an empty cell was not dimmed\n{rendered:?}"
        );
    }

    /// Emphasis is inside the cell, does not disturb the column the next cell
    /// starts at, and composes with the cursor and the tint.
    #[test]
    fn a_match_is_emphasised_inside_its_own_cell() {
        let rendered = frame(&View {
            geometry: geometry(24, true),
            columns: 80,
            titles: cells(&["NAME", "ORIGIN"]),
            lines: vec![Line {
                cells: vec![
                    Cell {
                        text: "api-token".to_owned(),
                        spans: only(4..7),
                    },
                    Cell {
                        text: "private".to_owned(),
                        spans: Vec::new(),
                    },
                ],
                tint: Tint::LastChosen,
                cursor: true,
            }],
            query: "tok",
            caret: 3,
        });
        assert!(
            rendered.contains(&format!("api-{EMPHASIS}tok{UNEMPHASIS}en")),
            "the matched characters are not emphasised\n{rendered:?}"
        );
        assert!(
            rendered.contains(&format!("{CURSOR}\u{1b}[33mapi-")),
            "the emphasis displaced the cursor or the tint\n{rendered:?}"
        );
        // The padding counts the text alone, so the second cell starts where it
        // would with nothing emphasised: the widest of `api-token` and `NAME`
        // plus the table's gap.
        let row = written(&rendered)
            .into_iter()
            .find(|line| line.starts_with("api-token"))
            .expect("no row was drawn");
        assert_eq!(
            row.find("private"),
            Some("api-token".len() + super::table::GAP),
            "emphasis moved the next column: {row:?}"
        );
    }

    /// A searched column's title is emphasised, and the rest of the titles stay
    /// bold behind it.
    #[test]
    fn a_searched_title_is_emphasised_without_unbolding_the_rest() {
        let rendered = frame(&View {
            geometry: geometry(24, true),
            columns: 80,
            titles: vec![
                Cell {
                    text: "NAME".to_owned(),
                    spans: only(0..4),
                },
                Cell {
                    text: "ORIGIN".to_owned(),
                    spans: Vec::new(),
                },
            ],
            lines: vec![line("api", Tint::Plain, true)],
            query: "name:api",
            caret: 8,
        });
        assert!(
            rendered.contains(&format!("{HEADER}{EMPHASIS}NAME{UNEMPHASIS}{HEADER}")),
            "the searched title is not emphasised over the bold\n{rendered:?}"
        );
    }

    /// A line longer than the terminal is cut rather than wrapped, because a
    /// wrapped line scrolls the bottom line off the screen. The cut counts
    /// visible characters and closes what the line opened.
    #[test]
    fn a_line_wider_than_the_terminal_is_cut_by_visible_width_and_closed() {
        let rendered = frame(&View {
            geometry: geometry(24, true),
            columns: 20,
            titles: cells(&["NAME"]),
            lines: vec![Line {
                cells: vec![Cell {
                    text: "a-name-far-longer-than-twenty-columns".to_owned(),
                    spans: only(2..30),
                }],
                tint: Tint::Plain,
                cursor: true,
            }],
            query: "",
            caret: 0,
        });
        for line in drawn(&rendered) {
            assert!(
                line.chars().count() <= 20,
                "a line was left wider than the terminal: {line:?}"
            );
        }
        let row = rendered
            .lines()
            .find(|line| line.contains("name-far"))
            .expect("no row was drawn");
        assert!(
            row.contains(EMPHASIS),
            "the cut dropped the emphasis it was meant to open: {row:?}"
        );
        assert!(
            !row.contains(UNEMPHASIS),
            "this line is meant to be cut inside its emphasised run: {row:?}"
        );
        assert!(
            row.ends_with(RESET),
            "a cut line left its emphasis open: {row:?}"
        );
    }

    /// The query line carries what was typed, and the terminal's cursor lands
    /// on the caret rather than at the end.
    #[test]
    fn the_cursor_lands_on_the_caret_inside_the_query() {
        let rendered = frame(&View {
            geometry: geometry(24, true),
            columns: 80,
            titles: cells(&["NAME"]),
            lines: vec![line("api", Tint::Plain, true)],
            query: "mail",
            caret: 2,
        });
        assert!(rendered.contains("\u{1b}[23;1H> mail"));
        assert!(
            rendered.ends_with("\u{1b}[23;5H"),
            "the cursor was not left on the caret\n{rendered:?}"
        );
    }

    /// The help names every key that does something.
    #[test]
    fn the_help_names_the_editing_keys_and_the_column_scroll() {
        for key in [
            "Enter choose",
            "Esc/^C cancel",
            "\u{2191}\u{2193} move",
            "\u{2190}\u{2192}/Home/End/Del edit",
            "^\u{2190}\u{2192} scroll",
            "Tab columns",
            "^P preview",
        ] {
            assert!(HELP.contains(key), "the help does not name {key}: {HELP:?}");
        }
    }
}
