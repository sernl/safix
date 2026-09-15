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
//! # How candidates are ranked
//!
//! Matching is case-insensitive subsequence matching over the entry's name.
//! Matches are ordered by the number of matched-character runs, fewest first;
//! then by whether the first matched character sits at a word boundary — a
//! `-`, `_` or `.`, or the start of the name — with a boundary first; then by
//! the index of that first match, earliest first; then by the name itself, so
//! the order is total and reproducible. An empty query matches everything, and
//! every candidate ties at every rank, which leaves the name tiebreak — the
//! order the placements' own [`std::collections::BTreeMap`] is in, which is the
//! order `list` prints.
//!
//! Candidate counts here are the names one user holds: tens, not millions. The
//! whole set is rescored on every keystroke, with no index and no incremental
//! state.

use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read as _, Write as _};
use std::time::{Duration, Instant};

use safix_core::model::Placement;
use safix_core::{Error, Secret, Workspace};

use crate::reporter::Refusal;
use crate::{render, table, tty};

/// How long the highlight has to rest before the value under it is decrypted.
///
/// Chosen for feel rather than measured. It is long enough that a held arrow
/// key crossing a dozen entries forks one `sops` subprocess instead of twelve,
/// and short enough that coming to rest on an entry reads as having answered.
const QUIET: Duration = Duration::from_millis(150);

/// How many lines of the terminal the preview region occupies.
const PREVIEW_LINES: usize = 8;

/// The terminal a picker falls back to when the real one will not report its
/// size.
const FALLBACK_SIZE: (usize, usize) = (24, 80);

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
    let candidates: Vec<Candidate> = held
        .iter()
        .filter(|(_, placement)| scope.admits(placement))
        .map(|(name, placement)| Candidate {
            name: name.clone(),
            row: render::listing_row(name, placement),
            file: placement.file.clone(),
            key: placement.key.clone(),
        })
        .collect();
    if candidates.is_empty() {
        return Err(Error::NothingToPick {
            user: user.to_owned(),
        }
        .into());
    }

    let raw = tty::Raw::over(&terminal).ok_or(Error::PickerNeedsTerminal)?;
    let chosen = {
        let mut session = Session::new(&terminal, &candidates, workspace, options.preview);
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
    /// The row `list` would print for it, built by `list`'s own row builder.
    row: Vec<String>,
    /// The document holding the value, repository-relative.
    file: String,
    /// The key the value is read under inside that document.
    key: String,
}

/// One run of the draw loop.
struct Session<'a> {
    /// The terminal the rows and the preview are drawn on, and the keystrokes
    /// are read from.
    terminal: &'a File,
    /// Every entry on offer, in the order the placements carry them.
    candidates: &'a [Candidate],
    /// The workspace the preview decrypts through.
    workspace: &'a Workspace,
    /// Whether the highlighted entry is decrypted at all.
    preview: bool,
    /// What the operator has typed.
    query: String,
    /// The candidates the query matches, ranked.
    order: Vec<usize>,
    /// Which of [`Session::order`] is highlighted.
    highlight: usize,
    /// The one decrypted value this picker holds.
    ///
    /// One value, not a map: a cache keyed by name is a set of live plaintexts
    /// whose size is how long the operator browsed. It is taken and dropped —
    /// and therefore zeroized — on every highlight change and on exit.
    held: Option<Secret>,
    /// Why there is no value under the highlight to show, when there is not.
    ///
    /// A failed preview is never a failed selection, so the reason is drawn in
    /// the region and the loop carries on.
    note: Option<String>,
    /// When the highlight came to rest, while nothing has been decrypted for
    /// it yet.
    rested: Option<Instant>,
}

impl<'a> Session<'a> {
    /// A session over these candidates, with the first of them highlighted.
    fn new(
        terminal: &'a File,
        candidates: &'a [Candidate],
        workspace: &'a Workspace,
        preview: bool,
    ) -> Self {
        let mut session = Self {
            terminal,
            candidates,
            workspace,
            preview,
            query: String::new(),
            order: Vec::new(),
            highlight: 0,
            held: None,
            note: None,
            rested: None,
        };
        session.rescore();
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
            let keys = buffer.get(..read)?;
            match self.act(keys) {
                Act::Continue => {}
                Act::Cancel => return None,
                Act::Choose(name) => return Some(name),
            }
        }
    }

    /// Decrypt the entry the highlight has rested on, once the quiet period has
    /// passed.
    fn settle(&mut self) {
        if !preview_due(self.preview, self.rested) {
            return;
        }
        self.rested = None;
        self.decrypt_highlighted();
    }

    /// Read the highlighted entry's value, or the reason there is none to read.
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

    /// The candidate under the highlight, if the query matches anything.
    fn current(&self) -> Option<&'a Candidate> {
        let index = self.order.get(self.highlight)?;
        self.candidates.get(*index)
    }

    /// Rank the candidates the query matches, and put the highlight in range.
    fn rescore(&mut self) {
        let names: Vec<&str> = self
            .candidates
            .iter()
            .map(|candidate| candidate.name.as_str())
            .collect();
        self.order = ranking(&names, &self.query);
        if self.highlight >= self.order.len() {
            self.highlight = self.order.len().saturating_sub(1);
        }
        self.moved();
    }

    /// The highlight now rests somewhere else, so whatever was decrypted for
    /// where it was is dropped and the quiet period starts again.
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
            // Printable bytes accumulate, so a character arriving as several
            // bytes in one read is decoded as the character it is.
            if *key >= 0x20 && *key != 0x7f {
                typed.push(*key);
                rest = tail;
                continue;
            }
            self.flush(&mut typed);
            match *key {
                // Escape with a cursor sequence behind it is an arrow key;
                // escape alone is leaving. Both spellings of the sequence are
                // read, because a terminal in application cursor mode sends
                // `O` where one in normal mode sends `[`.
                0x1b => match tail {
                    [b'[' | b'O', b'A', remainder @ ..] => {
                        self.up();
                        rest = remainder;
                        continue;
                    }
                    [b'[' | b'O', b'B', remainder @ ..] => {
                        self.down();
                        rest = remainder;
                        continue;
                    }
                    _ => return Act::Cancel,
                },
                // ^C and ^D are read as keystrokes because raw mode cleared
                // ISIG, and they mean here what they mean in any picker.
                0x03 | 0x04 => return Act::Cancel,
                b'\r' | b'\n' => {
                    if let Some(candidate) = self.current() {
                        return Act::Choose(candidate.name.clone());
                    }
                }
                // Backspace, however this terminal spells it.
                0x08 | 0x7f => {
                    self.query.pop();
                    self.rescore();
                }
                // ^P and ^N.
                0x10 => self.up(),
                0x0e => self.down(),
                _ => {}
            }
            rest = tail;
        }
        self.flush(&mut typed);
        Act::Continue
    }

    /// Append what was typed to the query, and rank against it.
    fn flush(&mut self, typed: &mut Vec<u8>) {
        if typed.is_empty() {
            return;
        }
        self.query.push_str(&String::from_utf8_lossy(typed));
        typed.clear();
        self.rescore();
    }

    /// The highlight one row towards the top, stopping there.
    fn up(&mut self) {
        self.highlight = self.highlight.saturating_sub(1);
        self.moved();
    }

    /// The highlight one row towards the bottom, stopping there.
    fn down(&mut self) {
        let last = self.order.len().saturating_sub(1);
        self.highlight = self.highlight.saturating_add(1).min(last);
        self.moved();
    }

    /// One frame: the query, the matching rows, and the preview region.
    fn draw(&mut self) {
        let (rows, columns) = size(self.terminal);
        let reserved = if self.preview {
            PREVIEW_LINES.saturating_add(2)
        } else {
            0
        };
        let window = rows.saturating_sub(reserved).saturating_sub(3).max(1);
        let first = self
            .highlight
            .saturating_add(1)
            .saturating_sub(window)
            .min(self.highlight);

        let mut frame = String::from("\x1b[H\x1b[2J");
        let _ = writeln!(
            frame,
            "safix: {}/{} \u{2014} type to narrow, arrows to move, enter to read, escape to leave",
            self.order.len(),
            self.candidates.len()
        );
        let _ = writeln!(frame, "> {}", self.query);

        let mut shown = vec![render::listing_header()];
        let visible: Vec<usize> = self
            .order
            .iter()
            .skip(first)
            .take(window)
            .copied()
            .collect();
        for index in &visible {
            if let Some(candidate) = self.candidates.get(*index) {
                shown.push(candidate.row.clone());
            }
        }
        let table = table::aligned(&shown);
        for (offset, line) in table.lines().enumerate() {
            let is_highlight = offset
                .checked_sub(1)
                .is_some_and(|row| first.saturating_add(row) == self.highlight);
            if is_highlight {
                frame.push_str("\x1b[7m");
                frame.push_str(line);
                frame.push_str("\x1b[0m\n");
            } else {
                frame.push_str(line);
                frame.push('\n');
            }
        }

        let mut out: &File = self.terminal;
        let _ = out.write_all(frame.as_bytes());
        if self.preview {
            let _ = out.write_all(
                b"\n\x1b[2m\xe2\x94\x80\xe2\x94\x80 value \xe2\x94\x80\xe2\x94\x80\x1b[0m\n",
            );
            if let Some(note) = &self.note {
                let _ = writeln!(out, "{note}");
            } else if let Some(held) = &self.held {
                // The plaintext's lifetime is this write's lifetime: nothing
                // here holds a string of it, and nothing returns one.
                let _ = held.preview_into(&mut out, PREVIEW_LINES, columns);
            }
        }
        let _ = out.flush();
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

/// Whether the value under the highlight is due to be read.
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

/// How well one name answers one query, or nothing when it does not.
struct Score {
    /// How many runs of consecutive characters the match is in; fewer is
    /// better.
    runs: usize,
    /// Whether the first matched character sits at a word boundary.
    boundary: bool,
    /// Where the first matched character is; earlier is better.
    first: usize,
}

/// Which characters start a word, for [`Score::boundary`].
fn is_boundary(character: char) -> bool {
    matches!(character, '-' | '_' | '.')
}

/// Score one name against one query.
fn score(name: &str, query: &str) -> Option<Score> {
    let mut wanted = query.chars().flat_map(char::to_lowercase).peekable();
    let mut first: Option<usize> = None;
    let mut previous: Option<usize> = None;
    let mut runs = 0_usize;
    let mut boundary = false;
    let mut characters = name.chars().enumerate().peekable();

    while wanted.peek().is_some() {
        let (index, character) = characters.next()?;
        let matches = character
            .to_lowercase()
            .next()
            .zip(wanted.peek().copied())
            .is_some_and(|(have, want)| have == want);
        if !matches {
            continue;
        }
        let _ = wanted.next();
        if first.is_none() {
            first = Some(index);
            boundary = index == 0
                || name
                    .chars()
                    .nth(index.saturating_sub(1))
                    .is_some_and(is_boundary);
        }
        if previous.is_none_or(|last| last.saturating_add(1) != index) {
            runs = runs.saturating_add(1);
        }
        previous = Some(index);
    }

    Some(Score {
        runs,
        boundary,
        first: first.unwrap_or(0),
    })
}

/// The indices of every name the query matches, best first.
///
/// The name itself is the final tiebreak, so the order is total: two candidates
/// that score identically are ordered the way the placements order them, which
/// is the order `list` prints.
fn ranking(names: &[&str], query: &str) -> Vec<usize> {
    let mut scored: Vec<(usize, Score, &str)> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| score(name, query).map(|score| (index, score, *name)))
        .collect();
    scored.sort_by(|left, right| {
        let (_, one, left_name) = left;
        let (_, other, right_name) = right;
        one.runs
            .cmp(&other.runs)
            .then_with(|| other.boundary.cmp(&one.boundary))
            .then_with(|| one.first.cmp(&other.first))
            .then_with(|| left_name.cmp(right_name))
    });
    scored.into_iter().map(|(index, _, _)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::{Options, QUIET, Scope, preview_due, ranking};
    use safix_core::model::{Origin, Placement};

    /// The names one query ranks, in rank order.
    fn ranked(names: &[&str], query: &str) -> Vec<String> {
        ranking(names, query)
            .into_iter()
            .filter_map(|index| names.get(index).map(|name| (*name).to_owned()))
            .collect()
    }

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
            logical_file: None,
            logical_key: None,
            logical_public: None,
            logical_record: None,
        }
    }

    #[test]
    fn fewer_runs_beats_more() {
        assert_eq!(
            ranked(&["a-b-token", "abtoken"], "ab"),
            vec!["abtoken".to_owned(), "a-b-token".to_owned()]
        );
    }

    #[test]
    fn a_word_boundary_start_beats_a_mid_word_one() {
        assert_eq!(
            ranked(&["xtoken", "x-token"], "token"),
            vec!["x-token".to_owned(), "xtoken".to_owned()]
        );
    }

    #[test]
    fn an_earlier_first_match_beats_a_later_one() {
        assert_eq!(
            ranked(&["zzzab", "ab"], "ab"),
            vec!["ab".to_owned(), "zzzab".to_owned()]
        );
    }

    #[test]
    fn the_name_breaks_a_full_tie() {
        assert_eq!(
            ranked(&["ab-two", "ab-one"], "ab"),
            vec!["ab-one".to_owned(), "ab-two".to_owned()]
        );
    }

    #[test]
    fn an_empty_query_preserves_the_placements_order() {
        assert_eq!(
            ranked(&["alpha", "beta", "gamma"], ""),
            vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()]
        );
    }

    /// A query no name answers offers nothing rather than something near it.
    ///
    /// The ranking has no distance metric, so a non-matching entry cannot
    /// appear in a list whose enter key decrypts something.
    #[test]
    fn a_query_nothing_matches_ranks_nothing() {
        assert!(ranked(&["alpha", "beta"], "zzz").is_empty());
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
        let listing = crate::render::listing(&held);
        let rows: Vec<Vec<String>> = held
            .iter()
            .map(|(name, placement)| crate::render::listing_row(name, placement))
            .collect();
        assert_eq!(listing.get(1), rows.first());
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
        let module = include_str!("picker.rs");
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
        let module = include_str!("picker.rs");
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
    /// however long the highlight has rested.
    #[test]
    fn a_suppressed_preview_is_never_due() {
        let rested = std::time::Instant::now().checked_sub(QUIET.saturating_mul(2));
        assert!(rested.is_some(), "this clock cannot be read backwards");
        assert!(preview_due(true, rested), "a rested highlight is due");
        assert!(
            !preview_due(false, rested),
            "a suppressed preview came due anyway"
        );
        assert!(
            !preview_due(true, None),
            "a highlight that has not rested came due"
        );
    }

    /// The one decrypt is the preview's, and the quiet period is its only way
    /// in.
    #[test]
    fn the_previews_decrypt_is_the_modules_only_one() {
        let module = include_str!("picker.rs");
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
