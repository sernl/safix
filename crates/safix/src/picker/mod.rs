//! Choosing one of the entries a user holds, on a terminal.
//!
//! # Why this is written here rather than depended on
//!
//! A third-party picker cannot render the preview this verb exists for:
//! `nucleo-picker`'s own feature table lists `--preview` as unimplemented and
//! its `Render` trait customises item rows rather than a side pane, so all of
//! the dependency's cost buys none of the feature. Its matcher half is
//! weak-copyleft, which `deny.toml`'s enumerated allow list does not carry, and
//! adding a licence to that list is a spec change rather than a lock update.
//!
//! # How the list is ordered and narrowed
//!
//! Alphabetically, always. The order is the one `safix list` prints — the order
//! the placements' own [`std::collections::BTreeMap`] is in — and typing
//! narrows it without reordering it, so an entry does not move under the cursor
//! between two keystrokes. The query language is [`search`]'s, which is
//! `KeePassXC`'s: terms are `ANDed`, a term may name a column, and a term may be
//! excluded, made exact, wildcarded or spelled as a regular expression.
//!
//! A ranking was tried and retired. Ranking answers "which of these did you
//! most likely mean", which is the question a fuzzy finder over a hundred
//! thousand paths has to answer; the candidates here are the names one person
//! holds, and the question they ask of a table of eight columns is "which of
//! these match", to which the answer is a filter. A ranked list also reorders
//! itself as a query is typed, which is what makes the entry under the cursor
//! change identity on a keystroke.
//!
//! # Where each piece is
//!
//! [`layout`] draws a frame, [`search`] decides what is in it, [`settings`]
//! remembers what the last run left showing, and [`time`] renders the two stamp
//! columns. This module holds the loop, the one decrypted value, and the keys.

mod layout;
mod search;
mod settings;
mod time;

use std::fs::File;
use std::io::{Read as _, Write};
use std::time::{Duration, Instant};

use safix_core::model::Placement;
use safix_core::{Error, Secret, Workspace, stamps};

use crate::reporter::Refusal;
use crate::{render, tty};

/// How long the highlight has to rest before the value under it is decrypted.
///
/// Chosen for feel rather than measured. It is long enough that a held arrow
/// key crossing a dozen entries forks one `sops` subprocess instead of twelve,
/// and short enough that coming to rest on an entry reads as having answered.
const QUIET: Duration = Duration::from_millis(150);

/// The terminal a picker falls back to when the real one will not report its
/// size.
const FALLBACK_SIZE: (usize, usize) = (24, 80);

/// How many columns the table has with the extra ones shown, and how many
/// without.
///
/// The eight are [`render::listing_row`]'s, and the one behind tab is the last
/// of them: the document serving a value answers "where is this" rather than
/// "what is this", which is a question an operator asks of one entry and not of
/// a list.
const COLUMNS: usize = 8;

/// How many of [`COLUMNS`] are shown before tab is pressed.
const DEFAULT_COLUMNS: usize = 7;

/// Which of the entries a user holds are offered.
#[derive(Clone, Copy)]
pub(crate) enum Scope {
    /// Every entry the user holds — `view`.
    Everything,
    /// Every entry except public placements — `edit`.
    ///
    /// A public output is refused for editing, and a refusal reachable by
    /// selection is one the choice should never have offered.
    Editable,
}

impl Scope {
    /// Whether this scope offers that placement.
    fn admits(self, placement: &Placement) -> bool {
        match self {
            Self::Everything => true,
            Self::Editable => placement.public.is_none(),
        }
    }
}

/// What a caller asks of a selection beyond the candidate set.
#[derive(Clone, Copy)]
pub(crate) struct Options {
    /// Whether the highlighted entry's value is decrypted and shown.
    pub preview: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { preview: true }
    }
}

/// One stamp column's cell: a local date, or `-` when nothing recorded one.
///
/// Exposed for [`crate::render::listing_row`], which builds the row both
/// `safix list` and this module show, so the two tables cannot come to render
/// a date differently.
pub(crate) fn stamp(seconds: Option<u64>) -> String {
    time::render(seconds)
}

/// The name the operator chose.
///
/// The only item this module exposes: it returns a name and nothing else, so
/// each verb keeps its own resolve-and-refuse path, and a placement the picker
/// read a moment earlier never becomes a second source of that value.
pub(crate) fn choose(
    workspace: &Workspace,
    user: &str,
    scope: Scope,
    options: Options,
) -> Result<String, Refusal> {
    // Before a candidate is read and long before one is decrypted: a picker on
    // a terminal it cannot open or cannot put in raw mode is unusable rather
    // than degraded.
    let terminal = tty::probe().ok_or(Error::PickerNeedsTerminal)?;

    let placements = workspace.placements()?;
    let held = placements.held_by(user).ok_or_else(|| Error::UnknownUser {
        user: user.to_owned(),
        declared: placements.users().map(str::to_owned).collect(),
    })?;
    let mut candidates: Vec<Candidate> = Vec::new();
    for (name, placement) in held.iter().filter(|(_, held)| scope.admits(held)) {
        let stamps = stamps::read(workspace, placement)?;
        candidates.push(Candidate {
            name: name.clone(),
            cells: render::listing_row(name, placement, stamps),
            file: placement.file.clone(),
            key: placement.key.clone(),
            created: stamps.map(|stamps| stamps.created),
        });
    }
    if candidates.is_empty() {
        return Err(Error::NothingToPick {
            user: user.to_owned(),
        }
        .into());
    }

    let mut remembered = settings::load();
    // The option is the run's own answer and the file is the last run's, so a
    // `--no-preview` run has its preview off whatever the file says. It is the
    // state the picker is in, so it is also what the file records if this run
    // chooses something.
    remembered.preview = remembered.preview && options.preview;

    let raw = tty::Raw::over(&terminal).ok_or(Error::PickerNeedsTerminal)?;
    let chosen = {
        let mut session = Session::new(&terminal, &candidates, workspace, user, remembered);
        session.enter();
        let chosen = session.run();
        session.leave();
        chosen
    };
    // The region is left and the terminal is back before the chosen value is
    // written or the refusal is rendered, so what stays on screen is what the
    // operator asked for and nothing the picker drew to find it.
    drop(raw);

    chosen.ok_or_else(|| Error::SelectionCancelled.into())
}

/// One offered entry: the name it is chosen by, the row it is shown as, and
/// where its value is read from.
struct Candidate {
    /// The entry's name, which is what choosing returns.
    name: String,
    /// The cells `list` would print for it, built by `list`'s own row builder.
    cells: Vec<String>,
    /// The document holding the value, repository-relative.
    file: String,
    /// The key the value is read under inside that document.
    key: String,
    /// When the value was first written, when anything recorded that.
    created: Option<u64>,
}

/// What the escape sequence being read is, when one is being read.
///
/// A sequence can be split across two reads, and the two bytes of `\x1b[`
/// followed by an `A` in the next read is an arrow key rather than an escape
/// and a typed capital A. Carrying the state across reads is what keeps a split
/// sequence from leaking its tail into the query.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    /// Nothing; the next byte is a key.
    None,
    /// An escape, with nothing after it yet.
    Escape,
    /// `\x1b[`, reading parameters until a final byte.
    Csi,
    /// `\x1bO`, one byte from its end.
    Ss3,
}

/// One run of the draw loop.
struct Session<'a> {
    /// The terminal the rows and the preview are drawn on, and the keystrokes
    /// are read from.
    terminal: &'a File,
    /// Every entry on offer, in the order the placements carry them, which is
    /// alphabetical.
    candidates: &'a [Candidate],
    /// The workspace the preview decrypts through.
    workspace: &'a Workspace,
    /// Whose list this is, which is the key the last choice is remembered
    /// under.
    user: &'a str,
    /// What the last run left showing, and what this run will leave.
    settings: settings::Settings,
    /// What the operator has typed.
    query: String,
    /// The candidates the query admits, alphabetically.
    order: Vec<usize>,
    /// Which of [`Session::order`] the cursor is on, counted from the
    /// alphabetically first — which is the bottom row.
    cursor: usize,
    /// How many columns have been scrolled off the left.
    column: usize,
    /// Which entry was created most recently, of those on offer.
    newest: Option<usize>,
    /// The one decrypted value this picker holds.
    ///
    /// One value, not a map: a cache keyed by name is a set of live plaintexts
    /// whose size is how long the operator browsed. It is taken and dropped —
    /// and therefore zeroized — on every cursor change and on exit.
    held: Option<Secret>,
    /// Why there is no value under the cursor to show, when there is not.
    ///
    /// A failed preview is never a failed selection, so the reason is drawn in
    /// the pane and the loop carries on.
    note: Option<String>,
    /// When the cursor came to rest, while nothing has been decrypted for it
    /// yet.
    rested: Option<Instant>,
    /// The escape sequence being read, when one is.
    pending: Pending,
}

impl<'a> Session<'a> {
    /// A session over these candidates, with the alphabetically first of them
    /// under the cursor.
    fn new(
        terminal: &'a File,
        candidates: &'a [Candidate],
        workspace: &'a Workspace,
        user: &'a str,
        settings: settings::Settings,
    ) -> Self {
        let newest = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| candidate.created.map(|created| (created, index)))
            .max_by_key(|(created, _)| *created)
            .map(|(_, index)| index);
        let mut session = Self {
            terminal,
            candidates,
            workspace,
            user,
            settings,
            query: String::new(),
            order: Vec::new(),
            cursor: 0,
            column: 0,
            newest,
            held: None,
            note: None,
            rested: None,
            pending: Pending::None,
        };
        session.refilter();
        session
    }

    /// Enter the terminal's alternate screen buffer.
    ///
    /// Everything this picker draws goes inside it, so a preview leaves nothing
    /// in the emulator's scroll buffer, where it would outlive the process, the
    /// zeroize and the operator's attention.
    fn enter(&mut self) {
        let mut out: &File = self.terminal;
        let _ = out.write_all(b"\x1b[?1049h");
        let _ = out.flush();
    }

    /// Leave it, on every path out of the loop.
    fn leave(&mut self) {
        self.held = None;
        let mut out: &File = self.terminal;
        // The attributes go off before the buffer is left, so nothing this drew
        // can colour what the shell prints next.
        let _ = out.write_all(layout::RESET.as_bytes());
        let _ = out.write_all(b"\x1b[?1049l");
        let _ = out.flush();
    }

    /// The loop: draw, read, act, until a name is chosen or the operator leaves.
    ///
    /// A terminal that stops answering — a read that fails rather than times
    /// out — ends the loop the way leaving does. There is no value to write and
    /// nothing to choose from, which is the same outcome by a different route.
    fn run(&mut self) -> Option<String> {
        let mut buffer = [0_u8; 32];
        loop {
            self.settle();
            self.draw();
            let mut source: &File = self.terminal;
            let read = match source.read(&mut buffer) {
                Ok(read) => read,
                Err(cause) if cause.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            };
            if read == 0 {
                // The read timed out with nothing behind it. An escape waiting
                // for the rest of its sequence is therefore the escape key,
                // which is the only way that key is told from the two bytes a
                // terminal puts in front of an arrow.
                if self.pending == Pending::Escape {
                    return None;
                }
                continue;
            }
            let keys = buffer.get(..read)?;
            match self.act(keys) {
                Act::Continue => {}
                Act::Cancel => return None,
                Act::Choose(name) => {
                    self.settings.choose(self.user, &name);
                    settings::store(&self.settings);
                    return Some(name);
                }
            }
        }
    }

    /// Decrypt the entry the cursor has rested on, once the quiet period has
    /// passed.
    fn settle(&mut self) {
        if !preview_due(self.settings.preview, self.rested) {
            return;
        }
        self.rested = None;
        self.decrypt_highlighted();
    }

    /// Read the entry under the cursor, or the reason there is none to read.
    ///
    /// The previous value is dropped before this one is asked for, so exactly
    /// one decrypted value exists at any moment.
    fn decrypt_highlighted(&mut self) {
        self.held = None;
        self.note = None;
        let Some(candidate) = self.current() else {
            return;
        };
        let path = self.workspace.vault_absolute(&candidate.file);
        if !path.exists() {
            self.note = Some(format!(
                "no value yet \u{2014} {} does not exist",
                candidate.file
            ));
            return;
        }
        match self.workspace.sops().decrypt_key(&path, &candidate.key) {
            Ok(decrypted) if decrypted.status == 0 => self.held = Some(decrypted.value),
            Ok(decrypted) => {
                self.note = Some(format!(
                    "sops exited {} reading this entry",
                    decrypted.status
                ));
            }
            Err(cause) => self.note = Some(format!("this entry did not decrypt: {cause}")),
        }
    }

    /// The candidate under the cursor, if the query admits anything.
    fn current(&self) -> Option<&'a Candidate> {
        let index = self.order.get(self.cursor)?;
        self.candidates.get(*index)
    }

    /// Narrow the candidates to the query, and put the cursor in range.
    fn refilter(&mut self) {
        let query = search::Query::parse(&self.query);
        self.order = self
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| query.admits(&candidate.cells))
            .map(|(index, _)| index)
            .collect();
        if self.cursor >= self.order.len() {
            self.cursor = self.order.len().saturating_sub(1);
        }
        self.moved();
    }

    /// The cursor now rests somewhere else, so whatever was decrypted for where
    /// it was is dropped and the quiet period starts again.
    fn moved(&mut self) {
        self.held = None;
        self.note = None;
        self.rested = Some(Instant::now());
    }

    /// What one read's worth of keystrokes does.
    fn act(&mut self, keys: &[u8]) -> Act {
        let mut typed: Vec<u8> = Vec::new();
        let mut rest = keys;
        while let Some((key, tail)) = rest.split_first() {
            rest = tail;
            match self.pending {
                // Which of the two introducers followed the escape, or neither:
                // an escape with an ordinary byte behind it is the escape key
                // and a modified keystroke this does not bind, and leaving is
                // the reading that cannot lose work.
                Pending::Escape => {
                    self.pending = match *key {
                        b'[' => Pending::Csi,
                        b'O' => Pending::Ss3,
                        _ => return Act::Cancel,
                    };
                    continue;
                }
                // Parameters and intermediates until a final byte, so a
                // sequence this does not bind is consumed whole rather than
                // spilling its tail into the query.
                Pending::Csi => {
                    if (0x40..=0x7e).contains(key) {
                        self.pending = Pending::None;
                        self.sequence(*key);
                    }
                    continue;
                }
                Pending::Ss3 => {
                    self.pending = Pending::None;
                    self.sequence(*key);
                    continue;
                }
                Pending::None => {}
            }

            // Printable bytes accumulate, so a character arriving as several
            // bytes in one read is decoded as the character it is.
            if *key >= 0x20 && *key != 0x7f {
                typed.push(*key);
                continue;
            }
            self.flush(&mut typed);
            match *key {
                0x1b => self.pending = Pending::Escape,
                // ^C is read as a keystroke because raw mode cleared ISIG, and
                // it means here what it means in any picker.
                0x03 => return Act::Cancel,
                b'\r' | b'\n' => {
                    if let Some(candidate) = self.current() {
                        return Act::Choose(candidate.name.clone());
                    }
                }
                // Backspace, however this terminal spells it.
                0x08 | 0x7f => {
                    let _ = self.query.pop();
                    self.refilter();
                }
                // Tab.
                0x09 => self.toggle_columns(),
                // ^P.
                0x10 => self.toggle_preview(),
                // Every other control byte is consumed and does nothing, which
                // is the whole of this picker's answer to a key it does not
                // bind.
                _ => {}
            }
        }
        self.flush(&mut typed);
        Act::Continue
    }

    /// What the final byte of an escape sequence does.
    ///
    /// The four arrows and nothing else. Both spellings of the sequence reach
    /// here — a terminal in application cursor mode sends `O` where one in
    /// normal mode sends `[` — and every other final byte, which is every
    /// function key, `Home`, `End`, `Delete` and every page key, is consumed
    /// and ignored.
    fn sequence(&mut self, final_byte: u8) {
        match final_byte {
            b'A' => self.up(),
            b'B' => self.down(),
            b'C' => self.right(),
            b'D' => self.left(),
            _ => {}
        }
    }

    /// Append what was typed to the query, and narrow against it.
    fn flush(&mut self, typed: &mut Vec<u8>) {
        if typed.is_empty() {
            return;
        }
        self.query.push_str(&String::from_utf8_lossy(typed));
        typed.clear();
        self.refilter();
    }

    /// The cursor one row towards the top of the screen, which is one entry
    /// later in the alphabet, stopping at the last.
    fn up(&mut self) {
        let last = self.order.len().saturating_sub(1);
        self.cursor = self.cursor.saturating_add(1).min(last);
        self.moved();
    }

    /// The cursor one row towards the query, which is one entry earlier in the
    /// alphabet, stopping at the first.
    fn down(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
        self.moved();
    }

    /// One column further along the table.
    ///
    /// Scrolling is not moving: the entry under the cursor does not change, so
    /// nothing is decrypted and nothing already decrypted is dropped. The last
    /// column stays on screen — scrolling past it would leave an empty table
    /// and no way to tell that from a query matching nothing.
    fn right(&mut self) {
        let last = self.shown().saturating_sub(1);
        self.column = self.column.saturating_add(1).min(last);
    }

    /// One column back, stopping at the first.
    fn left(&mut self) {
        self.column = self.column.saturating_sub(1);
    }

    /// How many columns the table has, with or without the ones behind tab.
    fn shown(&self) -> usize {
        if self.settings.extra_columns {
            COLUMNS
        } else {
            DEFAULT_COLUMNS
        }
    }

    /// Show or hide the columns behind tab, and remember which.
    fn toggle_columns(&mut self) {
        self.settings.extra_columns = !self.settings.extra_columns;
        // Hiding the columns can leave the view scrolled past the last one.
        self.column = self.column.min(self.shown().saturating_sub(1));
        settings::store(&self.settings);
    }

    /// Show or hide the value pane, and remember which.
    ///
    /// Turning it off drops whatever was decrypted for the cursor immediately
    /// rather than at the next movement, and turning it on starts the quiet
    /// period: the pane appears with the value under the cursor in it, without
    /// a keystroke asking for one.
    fn toggle_preview(&mut self) {
        self.settings.preview = !self.settings.preview;
        if self.settings.preview {
            self.moved();
        } else {
            self.held = None;
            self.note = None;
            self.rested = None;
        }
        settings::store(&self.settings);
    }

    /// The cells of one row, narrowed to the columns on screen.
    fn narrowed(&self, cells: &[String]) -> Vec<String> {
        let shown = self.shown();
        cells
            .iter()
            .take(shown)
            .skip(self.column.min(shown.saturating_sub(1)))
            .cloned()
            .collect()
    }

    /// One frame: the table, the pane, the query and the keys.
    fn draw(&mut self) {
        let (rows, columns) = size(self.terminal);
        let geometry = layout::geometry(rows, self.settings.preview);
        // The window holds the cursor: it is the bottom `rows` of the list
        // until the cursor climbs past them, and then it follows it.
        let first = self
            .cursor
            .saturating_add(1)
            .saturating_sub(geometry.rows)
            .min(self.cursor);
        let lines: Vec<layout::Line> = self
            .order
            .iter()
            .skip(first)
            .take(geometry.rows)
            .enumerate()
            .filter_map(|(offset, index)| {
                let candidate = self.candidates.get(*index)?;
                Some(layout::Line {
                    cells: self.narrowed(&candidate.cells),
                    tint: self.tint(*index),
                    cursor: first.saturating_add(offset) == self.cursor,
                })
            })
            .collect();
        let view = layout::View {
            geometry,
            columns,
            header: self.narrowed(&render::listing_header()),
            lines,
            query: &self.query,
        };

        let mut out: &File = self.terminal;
        let _ = out.write_all(layout::frame(&view).as_bytes());
        if let Some(line) = geometry.value() {
            let _ = write!(out, "\x1b[{line};1H");
            if let Some(note) = &self.note {
                let _ = write!(out, "{note}{}", layout::RESET);
            } else if let Some(held) = &self.held {
                // The plaintext's lifetime is this write's lifetime: nothing
                // here holds a string of it, and nothing returns one. The
                // colouring is a wrapper around the sink rather than a pass
                // over the rendered text, for that reason.
                let mut green = Green::over(&mut out);
                let _ = held.preview_into(&mut green, layout::VALUE_LINES, columns);
                let _ = green.flush();
            }
            let _ = out.write_all(layout::cursor(geometry, &self.query).as_bytes());
        }
        let _ = out.flush();
    }

    /// What colour one candidate's row is.
    ///
    /// The last choice wins over the newest entry, because it is the row the
    /// operator is most likely reaching for and one row cannot be two colours.
    fn tint(&self, index: usize) -> layout::Tint {
        let chosen = self
            .candidates
            .get(index)
            .zip(self.settings.chosen_by(self.user))
            .is_some_and(|(candidate, chosen)| candidate.name == chosen);
        if chosen {
            return layout::Tint::LastChosen;
        }
        if self.newest == Some(index) {
            return layout::Tint::Newest;
        }
        layout::Tint::Plain
    }
}

/// A sink that paints what passes through it green, one line at a time.
///
/// A wrapper rather than a function over the rendered value: the rendering is
/// written by [`Secret::preview_into`] straight into a terminal, and a version
/// of it that returned the text to be coloured would be a copy of a plaintext
/// with no zeroizing owner. Each line is closed with the attributes off, so a
/// value whose last line is cut short cannot colour the frame under it.
struct Green<'a, W: Write> {
    /// Where the painted bytes go.
    sink: &'a mut W,
    /// Whether the next byte starts a line and so needs the colour in front of
    /// it.
    fresh: bool,
}

impl<'a, W: Write> Green<'a, W> {
    /// Paint everything written to this sink.
    fn over(sink: &'a mut W) -> Self {
        Self { sink, fresh: true }
    }
}

impl<W: Write> Write for Green<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        for piece in buffer.split_inclusive(|byte| *byte == b'\n') {
            if self.fresh {
                self.sink.write_all(layout::GREEN.as_bytes())?;
                self.fresh = false;
            }
            match piece.split_last() {
                Some((b'\n', body)) => {
                    self.sink.write_all(body)?;
                    self.sink.write_all(layout::RESET.as_bytes())?;
                    self.sink.write_all(b"\n")?;
                    self.fresh = true;
                }
                _ => self.sink.write_all(piece)?,
            }
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.fresh {
            self.sink.write_all(layout::RESET.as_bytes())?;
            self.fresh = true;
        }
        self.sink.flush()
    }
}

/// What a keystroke did.
enum Act {
    /// Nothing that ends the loop.
    Continue,
    /// The operator left without choosing.
    Cancel,
    /// This name.
    Choose(String),
}

/// Whether the value under the cursor is due to be read.
///
/// The preview being suppressed is the first thing this answers, so that path
/// constructs no `sops` call at all rather than constructing one and throwing
/// its output away.
fn preview_due(preview: bool, rested: Option<Instant>) -> bool {
    preview && rested.is_some_and(|at| at.elapsed() >= QUIET)
}

/// The terminal's size, or a conservative one when it will not say.
fn size(terminal: &File) -> (usize, usize) {
    rustix::termios::tcgetwinsize(terminal).map_or(FALLBACK_SIZE, |window| {
        let rows = usize::from(window.ws_row);
        let columns = usize::from(window.ws_col);
        (
            if rows == 0 { FALLBACK_SIZE.0 } else { rows },
            if columns == 0 {
                FALLBACK_SIZE.1
            } else {
                columns
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{COLUMNS, DEFAULT_COLUMNS, Options, QUIET, Scope, preview_due};
    use safix_core::model::{Origin, Placement};

    /// One placement, public or not, with nothing else declared about it.
    fn placement(file: &str, public: Option<&str>) -> Placement {
        Placement {
            file: file.to_owned(),
            key: "value".to_owned(),
            origin: Origin::Private,
            owner: "alice".to_owned(),
            shared: false,
            generator: None,
            public: public.map(str::to_owned),
            definition_record: "records/alice/one".to_owned(),
            stamp_record: "records/alice/one.stamps".to_owned(),
            logical_file: None,
            logical_key: None,
            logical_public: None,
            logical_record: None,
            logical_stamp: None,
        }
    }

    #[test]
    fn editable_drops_a_public_placement_and_everything_keeps_it() {
        let secret = placement("secrets/alice.yaml", None);
        let public = placement("secrets/alice.yaml", Some("public/alice.token"));
        assert!(Scope::Everything.admits(&secret));
        assert!(Scope::Everything.admits(&public));
        assert!(Scope::Editable.admits(&secret));
        assert!(
            !Scope::Editable.admits(&public),
            "editing was offered a public output"
        );
    }

    /// The picker's rows are `list`'s rows, built by `list`'s own builder.
    ///
    /// Held against [`crate::render::listing`] itself rather than against a
    /// literal, so re-implementing the row construction inside this module
    /// reddens this as soon as either side's `GENERATOR` fallback moves.
    #[test]
    fn the_offered_rows_are_the_rows_list_prints() {
        let mut held = std::collections::BTreeMap::new();
        held.insert(
            "grafana-token".to_owned(),
            placement("secrets/alice.yaml", None),
        );
        let listing = crate::render::listing(&held, |_| None);
        let rows: Vec<Vec<String>> = held
            .iter()
            .map(|(name, placement)| crate::render::listing_row(name, placement, None))
            .collect();
        assert_eq!(listing.get(1), rows.first());
    }

    /// Tab adds columns rather than replacing them, and the one it adds is the
    /// last.
    #[test]
    fn the_extra_column_is_one_more_than_the_default_set() {
        assert_eq!(COLUMNS, DEFAULT_COLUMNS.saturating_add(1));
        assert_eq!(crate::render::listing_header().len(), COLUMNS);
    }

    /// The preview is on unless a caller says otherwise.
    #[test]
    fn the_preview_is_on_by_default() {
        assert!(Options::default().preview);
    }

    /// Exactly one decrypted value, and it is a field rather than a cache.
    ///
    /// Asserted over the module's own text because a cache is a shape rather
    /// than an observable: a map keyed by name would behave identically for one
    /// browse and hold a live plaintext per entry visited. The literals are
    /// assembled rather than written, so this file's own text does not contain
    /// what it searches for.
    #[test]
    fn one_value_is_held_and_it_is_not_a_cache() {
        let module = include_str!("mod.rs");
        assert!(
            module.contains(concat!("held: Option<Sec", "ret>")),
            "the one held value is no longer a single field of that type"
        );
        assert!(
            !module.contains(concat!("Map<String, Sec", "ret>")),
            "a cache of decrypted values appeared in this module"
        );
        assert!(
            !module.contains(concat!("Vec<Sec", "ret>")),
            "a collection of decrypted values appeared in this module"
        );
    }

    /// Nothing here stages anything.
    ///
    /// A preview is drawn by the runtime itself, so there is no path to hand
    /// anyone and no reason for a staging root to exist on this path.
    #[test]
    fn nothing_here_stages_a_plaintext() {
        let module = include_str!("mod.rs");
        for reached in [concat!("Stag", "ing"), concat!("estab", "lish")] {
            assert!(
                !module.contains(reached),
                "the picker reached for the staging machinery"
            );
        }
    }

    /// No value is decrypted when the preview is off.
    ///
    /// The quiet period is the only thing that reaches a `sops` call, and this
    /// is the question it asks first: a suppressed preview is never due,
    /// however long the cursor has rested.
    #[test]
    fn a_suppressed_preview_is_never_due() {
        let rested = std::time::Instant::now().checked_sub(QUIET.saturating_mul(2));
        assert!(rested.is_some(), "this clock cannot be read backwards");
        assert!(preview_due(true, rested), "a rested cursor is due");
        assert!(
            !preview_due(false, rested),
            "a suppressed preview came due anyway"
        );
        assert!(
            !preview_due(true, None),
            "a cursor that has not rested came due"
        );
    }

    /// The one decrypt is the preview's, and the quiet period is its only way
    /// in.
    #[test]
    fn the_previews_decrypt_is_the_modules_only_one() {
        let module = include_str!("mod.rs");
        assert_eq!(
            module.matches(concat!("decrypt", "_key")).count(),
            1,
            "the preview's decrypt is no longer the module's only one"
        );
        assert_eq!(
            module
                .matches(concat!("fn dec", "rypt_highlighted"))
                .count(),
            1,
            "a second way into the decrypt appeared"
        );
        assert_eq!(
            module
                .matches(concat!("self.dec", "rypt_highlighted()"))
                .count(),
            1,
            "the quiet period is no longer the only caller of the decrypt"
        );
    }
}
