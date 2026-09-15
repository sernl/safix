//! Where each piece of one frame goes, and what colour it is.
//!
//! The frame is built from the bottom of the terminal upwards, because the
//! bottom is where the operator's attention and the shell's own prompt already
//! are: the key help is the last line, the query is above it, the value pane
//! above that, and the table above that with its first row just under the
//! header. The rows are drawn in reverse alphabetical order, so the
//! alphabetically first entry is the bottom row of the table — the row closest
//! to the query being typed, and the row the cursor starts on.
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
//! ordinary case on a narrow terminal rather than an edge one.

use std::fmt::Write as _;

use crate::table;

/// The keys, in the order they are worth knowing.
pub(crate) const HELP: &str = "Enter choose \u{b7} Esc/^C cancel \u{b7} \u{2191}\u{2193} move \
                               \u{b7} \u{2190}\u{2192} scroll \u{b7} Tab columns \u{b7} ^P preview";

/// The value pane's own title.
pub(crate) const PANE_TITLE: &str = "\u{2500}\u{2500} Decrypted Value \u{2500}\u{2500}";

/// How many lines of the value the pane shows.
pub(crate) const VALUE_LINES: usize = 8;

/// Reverse video, which is the cursor.
const CURSOR: &str = "\u{1b}[7m";

/// Bold, which is the header.
const HEADER: &str = "\u{1b}[1m";

/// Dim, which is a cell with nothing in it and the pane's title.
const DIM: &str = "\u{1b}[2m";

/// Back to normal intensity, which ends a [`DIM`] run without ending the
/// colour or the reverse video around it.
const UNDIM: &str = "\u{1b}[22m";

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
    /// How many candidate rows there is room for under the header.
    pub rows: usize,
    /// The line the value pane's title is on, when there is room for a pane.
    pub pane: Option<usize>,
    /// The line the query is typed on.
    pub query: usize,
    /// The line the key help is on, which is the terminal's last.
    pub help: usize,
}

impl Geometry {
    /// The first line of the value, which is under the pane's title.
    pub(crate) fn value(self) -> Option<usize> {
        self.pane.map(|pane| pane.saturating_add(1))
    }
}

/// How this frame's lines fall on a terminal of this height.
///
/// A terminal too short for the pane gets no pane and a table instead: the
/// preview is what a short terminal loses, because the list is what the verb is
/// for. That the value is still decrypted in that state is deliberate — the
/// pane is one line of a resize away, and a picker whose decryption depended on
/// the window height would decrypt on a resize.
pub(crate) fn geometry(lines: usize, preview: bool) -> Geometry {
    let help = lines.max(1);
    let query = help.saturating_sub(1).max(1);
    // Above the query: the pane's title, the value, the header, and at least
    // one row.
    let room = query.saturating_sub(1);
    let pane = if preview && room >= VALUE_LINES.saturating_add(3) {
        Some(query.saturating_sub(VALUE_LINES).saturating_sub(1))
    } else {
        None
    };
    let table = pane.unwrap_or(query).saturating_sub(1);
    Geometry {
        rows: table.saturating_sub(1),
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
}

impl Tint {
    /// The sequence this tint starts a line with.
    const fn sequence(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::LastChosen => YELLOW,
            Self::Newest => CYAN,
        }
    }
}

/// One candidate row on its way to the screen.
pub(crate) struct Line {
    /// Its cells, already narrowed to the columns being shown.
    pub cells: Vec<String>,
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
    /// The header, narrowed to the columns being shown.
    pub header: Vec<String>,
    /// The rows being shown, in alphabetical order: the first of them is the
    /// bottom row of the table.
    pub lines: Vec<Line>,
    /// What has been typed.
    pub query: &'a str,
}

/// One frame, less the decrypted value the caller writes into the pane.
///
/// Ends with the cursor at the end of the query, so a terminal drawing its own
/// block cursor puts it where the typing is going.
pub(crate) fn frame(view: &View) -> String {
    let mut out = String::from("\u{1b}[H\u{1b}[2J");

    let mut rows = vec![view.header.clone()];
    // Reversed here rather than by the caller: which end of the table the
    // alphabetically first row is at is this module's decision, and a caller
    // that reversed its own rows would be making it twice.
    rows.extend(view.lines.iter().rev().map(|line| line.cells.clone()));
    let aligned = table::aligned(&rows);

    let height = view.lines.len().saturating_add(1);
    let bottom = view.geometry.pane.unwrap_or(view.geometry.query);
    let top = bottom.saturating_sub(height);
    for _ in 0..top.saturating_sub(1) {
        out.push('\n');
    }

    for (offset, line) in aligned.lines().enumerate() {
        let cut = cut(line, view.columns);
        match offset.checked_sub(1) {
            // The header, above the rows.
            None => {
                let _ = writeln!(out, "{HEADER}{cut}{RESET}");
            }
            // The rows, drawn from the alphabetically last down to the first,
            // which is why the line at this offset is counted from the end.
            Some(index) => {
                let line = view
                    .lines
                    .len()
                    .checked_sub(index.saturating_add(1))
                    .and_then(|from_start| view.lines.get(from_start));
                let _ = writeln!(out, "{}", painted(line, &cut));
            }
        }
    }

    if view.geometry.pane.is_some() {
        let _ = writeln!(out, "{DIM}{}{RESET}", cut(PANE_TITLE, view.columns));
    }

    let query = cut(&format!("> {}", view.query), view.columns);
    let _ = write!(out, "\u{1b}[{};1H{query}{RESET}", view.geometry.query);
    let _ = write!(
        out,
        "\u{1b}[{};1H{DIM}{}{RESET}",
        view.geometry.help,
        cut(HELP, view.columns)
    );
    out.push_str(&cursor(view.geometry, view.query));
    out
}

/// The sequence that leaves the terminal's cursor after the query.
///
/// Written again after the value pane is filled in, because filling it in moves
/// the cursor: a block cursor resting in the middle of a decrypted value would
/// read as part of it.
pub(crate) fn cursor(geometry: Geometry, query: &str) -> String {
    format!(
        "\u{1b}[{};{}H",
        geometry.query,
        query.chars().count().saturating_add(3)
    )
}

/// One row's line, with its tint, its cursor and its empty cells.
fn painted(line: Option<&Line>, text: &str) -> String {
    let Some(line) = line else {
        return format!("{text}{RESET}");
    };
    let cursor = if line.cursor { CURSOR } else { "" };
    format!("{cursor}{}{}{RESET}", line.tint.sequence(), dimmed(text))
}

/// The same line with every empty cell dimmed.
///
/// Dimmed and then undimmed rather than reset, so a row that is yellow or under
/// the cursor stays yellow and under the cursor across the cell.
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

/// The first `columns` characters of a line.
fn cut(text: &str, columns: usize) -> String {
    text.chars().take(columns).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        CURSOR, CYAN, DIM, Geometry, HEADER, HELP, Line, PANE_TITLE, RESET, Tint, View, frame,
        geometry,
    };

    /// A row of cells.
    fn cells(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
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
            header: cells(&["NAME", "ORIGIN"]),
            lines,
            query,
        }
    }

    /// The frame's lines, undecorated.
    ///
    /// A sequence that moves the cursor starts a line of its own, because that
    /// is what it does on the terminal: the query and the help are written at
    /// absolute line numbers rather than after a newline, so a reader that
    /// dropped the sequence would read them as one line.
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

    #[test]
    fn the_help_is_the_last_line_and_the_query_is_the_one_above_it() {
        let geometry = geometry(24, true);
        assert_eq!(geometry.help, 24);
        assert_eq!(geometry.query, 23);
        assert_eq!(geometry.pane, Some(14));
        assert_eq!(geometry.value(), Some(15));
        let rendered = frame(&view(vec![line("api", Tint::Plain, true)], "", true));
        assert!(
            rendered.contains(&format!("\u{1b}[24;1H{DIM}{HELP}")),
            "the help was not written at the last line\n{rendered:?}"
        );
        assert!(
            rendered.contains("\u{1b}[23;1H> "),
            "the query was not written above it\n{rendered:?}"
        );
    }

    /// The table is drawn bottom-up: the header above the rows, and the
    /// alphabetically first row at the bottom, against the query.
    #[test]
    fn the_bottom_row_is_the_alphabetically_first_entry() {
        let rendered = frame(&view(
            vec![
                line("aliased-secret", Tint::Plain, true),
                line("api-token", Tint::Plain, false),
                line("mail-password", Tint::Plain, false),
            ],
            "",
            true,
        ));
        let lines: Vec<String> = drawn(&rendered)
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let names: Vec<&str> = lines
            .iter()
            .filter_map(|line| line.split_whitespace().next())
            .take(4)
            .collect();
        assert_eq!(
            names,
            vec!["NAME", "mail-password", "api-token", "aliased-secret"],
            "the table was not drawn bottom-up\n{lines:?}"
        );
        // And the pane is under the bottom row rather than over the header.
        assert!(
            lines
                .get(4)
                .is_some_and(|line| line.starts_with(PANE_TITLE)),
            "the value pane is not under the table\n{lines:?}"
        );
    }

    /// The pane's title sits between the rows and the query, and only when the
    /// preview is on.
    #[test]
    fn the_pane_is_drawn_only_when_the_preview_is_on() {
        let with = frame(&view(vec![line("api", Tint::Plain, true)], "", true));
        assert!(with.contains(PANE_TITLE));
        let without = frame(&view(vec![line("api", Tint::Plain, true)], "", false));
        assert!(
            !without.contains(PANE_TITLE),
            "a suppressed preview drew its pane anyway"
        );
        assert_eq!(geometry(24, false).pane, None);
    }

    #[test]
    fn the_header_is_bold_and_every_drawn_line_ends_reset() {
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
            header: cells(&["NAME", "SHARED"]),
            lines: vec![Line {
                cells: cells(&["api-token", "-"]),
                tint: Tint::LastChosen,
                cursor: false,
            }],
            query: "",
        });
        assert!(
            rendered.contains(&format!("{DIM}-\u{1b}[22m")),
            "an empty cell was not dimmed\n{rendered:?}"
        );
    }

    /// A line longer than the terminal is cut rather than wrapped, because a
    /// wrapped line scrolls the bottom line off the screen.
    #[test]
    fn a_line_wider_than_the_terminal_is_cut() {
        let rendered = frame(&View {
            geometry: geometry(24, true),
            columns: 20,
            header: cells(&["NAME"]),
            lines: vec![Line {
                cells: cells(&["a-name-far-longer-than-twenty-columns"]),
                tint: Tint::Plain,
                cursor: true,
            }],
            query: "",
        });
        for line in drawn(&rendered) {
            assert!(
                line.chars().count() <= 20,
                "a line was left wider than the terminal: {line:?}"
            );
        }
    }

    /// The query line carries what was typed, and the cursor lands after it.
    #[test]
    fn the_query_is_drawn_with_the_cursor_after_it() {
        let rendered = frame(&view(vec![line("api", Tint::Plain, true)], "mail", true));
        assert!(rendered.contains("\u{1b}[23;1H> mail"));
        assert!(
            rendered.ends_with("\u{1b}[23;7H"),
            "the cursor was not left after the query\n{rendered:?}"
        );
    }

    /// A terminal too short for the pane draws the list instead of refusing.
    #[test]
    fn a_short_terminal_loses_the_pane_and_keeps_a_row() {
        let short = geometry(6, true);
        assert_eq!(short.pane, None);
        assert_eq!(short.query, 5);
        assert_eq!(short.help, 6);
        assert!(short.rows >= 1, "no room was left for a candidate");
        assert_eq!(
            geometry(1, true),
            Geometry {
                rows: 0,
                pane: None,
                query: 1,
                help: 1
            }
        );
    }
}
