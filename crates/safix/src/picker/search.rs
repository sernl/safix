//! The query language the picker narrows with.
//!
//! `KeePassXC`'s search syntax, because an operator who keeps a password database
//! beside this one already knows it and because it answers the questions a
//! table of named columns raises: "this word, in the name column only", "not
//! this one", "these two names and nothing between them".
//!
//! One query is whitespace-separated terms, each
//! `[modifiers][field:]term`, and every term has to match for a row to be
//! offered — terms are `ANDed`, which is what makes typing more of them narrow
//! the list. A term is matched against one column when it names one and against
//! every column when it does not.
//!
//! | spelling | meaning |
//! | --- | --- |
//! | `token` | case-insensitive substring, any column |
//! | `name:token` | the same, in the `Name` column alone |
//! | `!token` | rows this term does *not* match |
//! | `+Token` | the whole cell is exactly this, case included |
//! | `*^api-.*$` | the term is a regular expression |
//! | `api-*` | `*` is any run, `?` is one character |
//! | `api-token\|mail-password` | either, whole-cell |
//! | `"two words"` | one term with a space in it |
//!
//! # Why a wildcard term is anchored and a plain one is not
//!
//! `api` is a fragment of a name and means "contains"; `api-*` is a shape a
//! whole name has and means "looks like". A wildcard that also matched
//! substrings would make `?` — one character — mean "one character somewhere",
//! which narrows nothing. `|` is part of that shape language rather than a
//! separator between two substring searches, for the same reason: `a|b`
//! anchored is a choice between two names, and unanchored it is the union of
//! two substring searches, which is what typing two terms already cannot ask
//! for.
//!
//! # Why an invalid regular expression matches nothing
//!
//! A query is retyped a character at a time, so every prefix of a finished
//! regular expression is a query the picker has already been asked to run.
//! `*[a-` is a half-typed character class, not an error worth refusing a frame
//! for, and offering every row for it — the other available answer — would
//! flash the whole list up between two keystrokes.

use std::ops::Range;

use regex_lite::Regex;

/// Which column a term is matched against, when it names one.
///
/// The long spelling and its initial, which is `KeePassXC`'s own convention: a
/// person filtering by name types `n:` after the first few times.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    /// The entry's name.
    Name,
    /// How the name reached this user.
    Origin,
    /// Whether one value serves every carrier.
    Shared,
    /// What mints the value, when anything does.
    Generator,
    /// The key the value is read under.
    Key,
    /// When the value was first written.
    Created,
    /// When it last changed.
    Updated,
    /// The document serving it.
    File,
}

impl Field {
    /// The field one prefix names, or nothing when it names none.
    fn parse(text: &str) -> Option<Self> {
        match text {
            "name" | "n" => Some(Self::Name),
            "origin" | "o" => Some(Self::Origin),
            "shared" | "s" => Some(Self::Shared),
            "generator" | "g" => Some(Self::Generator),
            "key" | "k" => Some(Self::Key),
            "created" | "c" => Some(Self::Created),
            "updated" | "u" => Some(Self::Updated),
            "file" | "f" => Some(Self::File),
            _ => None,
        }
    }

    /// Which cell of a row this field is.
    ///
    /// The order is [`crate::render::listing_row`]'s, which is the order the
    /// table draws, so a field names the column an operator is looking at.
    pub(crate) const fn column(self) -> usize {
        match self {
            Self::Name => 0,
            Self::Origin => 1,
            Self::Shared => 2,
            Self::Generator => 3,
            Self::Key => 4,
            Self::Created => 5,
            Self::Updated => 6,
            Self::File => 7,
        }
    }
}

/// How one term's text is compared against one cell.
enum Match {
    /// Case-insensitive substring, which is what an unmodified term is.
    Contains(String),
    /// The whole cell, case included.
    Exact(String),
    /// A regular expression, from `*` or from a wildcard.
    Pattern(Regex),
    /// A regular expression that did not compile, which nothing matches.
    Unusable,
}

/// One term of a query.
struct Term {
    /// Whether a matching row is rejected rather than kept.
    exclude: bool,
    /// The column this term is matched against, or every column.
    field: Option<Field>,
    /// How its text is compared.
    compare: Match,
}

/// A parsed query, ready to be asked of a row.
pub(crate) struct Query {
    /// Every term, all of which have to be satisfied.
    terms: Vec<Term>,
}

/// Which columns' titles a query searches.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Searched {
    /// None of them, which is what an empty query and a query of nothing but
    /// exclusions search.
    Nothing,
    /// Every column, because a positive term names no field and so is asked of
    /// every cell.
    Everything,
    /// These columns alone, which is what field terms name.
    Columns(Vec<usize>),
}

impl Searched {
    /// Whether this column is one the query looked in.
    pub(crate) fn includes(&self, column: usize) -> bool {
        match self {
            Self::Nothing => false,
            Self::Everything => true,
            Self::Columns(columns) => columns.contains(&column),
        }
    }
}

impl Query {
    /// Parse one query as typed.
    pub(crate) fn parse(text: &str) -> Self {
        Self {
            terms: split(text).iter().filter_map(|word| term(word)).collect(),
        }
    }

    /// Where every positive term matched in every cell, or `None` for a row
    /// this query rejects.
    ///
    /// The ranges are characters rather than bytes, because what reads them
    /// draws them. Per cell they are sorted and disjoint, so two terms that
    /// found the same letters emphasise them once.
    ///
    /// An empty query has no terms, every row satisfies all of them and no cell
    /// carries a range, which is how the unfiltered list is the same code path
    /// as a filtered one.
    pub(crate) fn spans(&self, cells: &[String]) -> Option<Vec<Vec<Range<usize>>>> {
        let mut spans: Vec<Vec<Range<usize>>> = vec![Vec::new(); cells.len()];
        for term in &self.terms {
            let mut found = false;
            for (column, cell) in cells.iter().enumerate() {
                if term.field.is_some_and(|field| field.column() != column) {
                    continue;
                }
                if !term.hits(cell) {
                    continue;
                }
                found = true;
                // An exclusion contributes nothing: the characters it found are
                // why a row is not offered, and a row that is offered matched
                // it nowhere.
                if !term.exclude
                    && let Some(cell_spans) = spans.get_mut(column)
                {
                    cell_spans.extend(term.ranges(cell));
                }
            }
            if found == term.exclude {
                return None;
            }
        }
        Some(spans.into_iter().map(merged).collect())
    }

    /// Which columns this query looked in, which are the titles worth
    /// emphasising.
    pub(crate) fn searched_columns(&self) -> Searched {
        let mut columns: Vec<usize> = Vec::new();
        for term in self.terms.iter().filter(|term| !term.exclude) {
            let Some(field) = term.field else {
                return Searched::Everything;
            };
            let column = field.column();
            if !columns.contains(&column) {
                columns.push(column);
            }
        }
        if columns.is_empty() {
            Searched::Nothing
        } else {
            Searched::Columns(columns)
        }
    }
}

impl Term {
    /// Whether this one term matches this one cell.
    fn hits(&self, cell: &str) -> bool {
        match &self.compare {
            Match::Contains(text) => cell.to_lowercase().contains(text),
            Match::Exact(text) => cell == text,
            Match::Pattern(pattern) => pattern.is_match(cell),
            Match::Unusable => false,
        }
    }

    /// Which of a matched cell's characters this term found.
    fn ranges(&self, cell: &str) -> Vec<Range<usize>> {
        let whole = 0..cell.chars().count();
        match &self.compare {
            Match::Contains(text) => {
                let lowered = cell.to_lowercase();
                // Lowercasing can add characters — `\u{130}` becomes two — and
                // a count that changed makes an offset in the lowered copy no
                // offset in the original. Such a cell is emphasised whole
                // rather than at a place the mapping cannot name.
                if lowered.chars().count() != cell.chars().count() {
                    return vec![whole];
                }
                lowered
                    .match_indices(text.as_str())
                    .map(|(at, found)| characters(&lowered, at, found.len()))
                    .collect()
            }
            Match::Exact(_) => vec![whole],
            Match::Pattern(pattern) => pattern
                .find_iter(cell)
                .map(|found| characters(cell, found.start(), found.as_str().len()))
                .collect(),
            Match::Unusable => Vec::new(),
        }
    }
}

/// The character range one byte run of this text covers.
fn characters(text: &str, at: usize, length: usize) -> Range<usize> {
    let start = text.get(..at).unwrap_or_default().chars().count();
    let width = text
        .get(at..at.saturating_add(length))
        .unwrap_or_default()
        .chars()
        .count();
    start..start.saturating_add(width)
}

/// These ranges sorted, with the ones that touch fused into one.
fn merged(spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut sorted = spans;
    sorted.sort_by_key(|span| (span.start, span.end));
    let mut fused: Vec<Range<usize>> = Vec::with_capacity(sorted.len());
    for span in sorted {
        match fused.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => fused.push(span),
        }
    }
    fused
}

/// Which characters make a term a shape rather than a fragment.
fn is_wildcard(character: char) -> bool {
    matches!(character, '*' | '?' | '|')
}

/// One term from one word of a query, or nothing when the word is only
/// modifiers.
fn term(word: &str) -> Option<Term> {
    let mut exclude = false;
    let mut exact = false;
    let mut regex = false;
    let mut rest = word;
    // Any order and any repetition: a person typing `!+name:x` and one typing
    // `+!name:x` have asked the same thing, and refusing one of the two
    // spellings would be a refusal mid-query.
    while let Some((first, tail)) = rest.split_at_checked(1) {
        match first {
            "!" => exclude = true,
            "+" => exact = true,
            "*" => regex = true,
            _ => break,
        }
        rest = tail;
    }

    let (field, text) = match rest.split_once(':') {
        Some((prefix, remainder)) => match Field::parse(prefix) {
            Some(field) => (Some(field), remainder),
            // An unknown prefix is part of the term rather than a refusal: a
            // key is allowed a colon in it, and a query naming one should
            // search for it.
            None => (None, rest),
        },
        None => (None, rest),
    };
    if text.is_empty() {
        return None;
    }

    let compare = if regex {
        compile(&format!("(?i){text}"))
    } else if exact {
        Match::Exact(text.to_owned())
    } else if text.contains(is_wildcard) {
        compile(&anchored(text))
    } else {
        Match::Contains(text.to_lowercase())
    };
    Some(Term {
        exclude,
        field,
        compare,
    })
}

/// A regular expression, or the comparison nothing satisfies.
fn compile(pattern: &str) -> Match {
    Regex::new(pattern).map_or(Match::Unusable, Match::Pattern)
}

/// The anchored, case-insensitive expression one wildcard term describes.
///
/// Everything but `*`, `?` and `|` is escaped, so a name's own `.`, `-` and
/// `+` are the characters they look like rather than regular-expression
/// operators. The alternation is wrapped in a non-capturing group inside the
/// anchors, which is what makes `a*|b` two whole-cell shapes rather than one
/// shape anchored at one end.
fn anchored(text: &str) -> String {
    let mut pattern = String::from("(?i)^(?:");
    for character in text.chars() {
        match character {
            '*' => pattern.push_str(".*"),
            '?' => pattern.push('.'),
            '|' => pattern.push('|'),
            other => {
                if !other.is_alphanumeric() {
                    pattern.push('\\');
                }
                pattern.push(other);
            }
        }
    }
    pattern.push_str(")$");
    pattern
}

/// A query's words, with a double-quoted run counting as one word.
///
/// The quotes are removed, so `"two words"` is the term `two words` and
/// nothing about the term carries the quoting. An unclosed quote runs to the
/// end of the query, which is what a person typing one is in the middle of
/// doing.
fn split(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in text.chars() {
        match character {
            '"' => quoted = !quoted,
            space if space.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::{Query, Range, Searched, split};

    /// One row of the table, in the column order a field names.
    fn row(name: &str, generator: &str, file: &str) -> Vec<String> {
        [
            name,
            "private",
            "-",
            generator,
            "value",
            "01/01/2024 09:00am",
            "02/01/2024 10:00am",
            file,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    /// Whether one query offers one row, which is whether it found anything in
    /// it: the filter and the emphasis are one answer.
    fn admits(query: &str, cells: &[String]) -> bool {
        Query::parse(query).spans(cells).is_some()
    }

    #[test]
    fn an_empty_query_offers_every_row() {
        assert!(admits("", &row("api-token", "-", "secrets/alice.yaml")));
        assert!(admits("   ", &row("api-token", "-", "secrets/alice.yaml")));
    }

    #[test]
    fn a_plain_term_is_a_case_insensitive_substring_of_any_column() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("TOK", &entry), "case was not folded");
        assert!(admits("alice", &entry), "a term reached no other column");
        assert!(!admits("wifi", &entry));
    }

    #[test]
    fn every_term_has_to_match() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("api alice", &entry));
        assert!(!admits("api wifi", &entry), "the terms were ORed");
    }

    #[test]
    fn a_field_term_looks_at_that_column_alone() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("name:api", &entry));
        assert!(admits("n:api", &entry), "the initial is not the long form");
        assert!(
            !admits("name:alice", &entry),
            "a name term matched the file column"
        );
        assert!(admits("file:alice", &entry));
    }

    #[test]
    fn an_exclusion_rejects_the_rows_its_term_matches() {
        let token = row("api-token", "-", "secrets/alice.yaml");
        let mail = row("mail-password", "-", "secrets/alice.yaml");
        assert!(!admits("!api", &token));
        assert!(admits("!api", &mail));
        assert!(
            admits("!name:api alice", &mail),
            "an exclusion and a term together"
        );
    }

    #[test]
    fn an_exact_term_is_the_whole_cell_and_its_case() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("+api-token", &entry));
        assert!(!admits("+api", &entry), "an exact term matched a fragment");
        assert!(!admits("+API-TOKEN", &entry), "case was folded");
    }

    #[test]
    fn a_regex_term_is_a_regular_expression() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("*^api-[a-z]+$", &entry));
        assert!(admits("*n:^API", &entry), "a regex term ignored its field");
        assert!(!admits("*^token", &entry));
    }

    #[test]
    fn a_wildcard_term_is_anchored() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("name:api-*", &entry));
        assert!(admits("name:api-toke?", &entry));
        assert!(
            !admits("name:pi-*", &entry),
            "a wildcard term matched a substring"
        );
        assert!(
            !admits("name:api-toke?n?", &entry),
            "a wildcard term matched a cell shorter than its shape"
        );
    }

    #[test]
    fn a_wildcard_escapes_what_is_not_a_wildcard() {
        // The `.` is a character of the path rather than "any character", so a
        // row whose file differs only there is not offered.
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("file:*alice.yaml", &entry));
        assert!(!admits("file:*alicexyaml", &entry));
    }

    #[test]
    fn an_or_term_offers_either_whole_cell() {
        let token = row("api-token", "-", "secrets/alice.yaml");
        let mail = row("mail-password", "-", "secrets/alice.yaml");
        let other = row("wifi-psk", "-", "secrets/shared.yaml");
        for entry in [&token, &mail] {
            assert!(admits("name:api-token|mail-password", entry));
        }
        assert!(!admits("name:api-token|mail-password", &other));
    }

    #[test]
    fn a_quoted_term_carries_its_space() {
        let entry = row("api-token", "reads a card", "secrets/alice.yaml");
        assert!(admits("\"reads a card\"", &entry));
        assert!(
            !admits("\"reads a bard\"", &entry),
            "the quoted term matched something else"
        );
        // Unquoted, the same words are two terms, and the second matches
        // nothing here.
        assert!(!admits("reads a zard", &entry));
    }

    #[test]
    fn an_unparsable_regex_matches_nothing_rather_than_everything() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(
            !admits("*[a-", &entry),
            "a half-typed character class offered the row"
        );
        assert!(!admits("*(", &entry), "an unclosed group offered the row");
        // And the next keystroke finishing it offers it again, which is what
        // makes this a state of the query rather than a failure of the run.
        assert!(admits("*[a-z]", &entry));
    }

    #[test]
    fn a_word_that_is_only_modifiers_is_not_a_term() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(admits("!", &entry), "a lone modifier filtered something");
        assert!(admits("api !", &entry));
    }

    #[test]
    fn an_unknown_field_prefix_stays_part_of_the_term() {
        let entry = row("api-token", "-", "secrets/weird:name.yaml");
        assert!(admits("weird:name", &entry));
    }

    #[test]
    fn quoting_holds_a_run_together_and_is_removed() {
        assert_eq!(split("a \"b c\" d"), vec!["a", "b c", "d"]);
        assert_eq!(split("  spaced   out "), vec!["spaced", "out"]);
        assert_eq!(split("\"unclosed run"), vec!["unclosed run"]);
    }

    /// Where one query matched in one row, per cell.
    fn spans(query: &str, cells: &[String]) -> Vec<Vec<std::ops::Range<usize>>> {
        Query::parse(query)
            .spans(cells)
            .expect("the query rejected the row")
    }

    /// One span, as the list a cell carries.
    fn only(span: Range<usize>) -> Vec<Range<usize>> {
        std::iter::once(span).collect()
    }

    /// A plain term marks the letters it found, in every cell that has them.
    #[test]
    fn a_contains_term_marks_its_letters_case_insensitively() {
        let entry = row("api-token", "-", "secrets/token.yaml");
        let found = spans("TOK", &entry);
        assert_eq!(found.first(), Some(&only(4..7)), "{found:?}");
        assert_eq!(found.get(7), Some(&only(8..11)), "{found:?}");
        assert_eq!(found.get(1), Some(&Vec::new()), "{found:?}");
    }

    /// A field term marks that column and leaves the others alone, even where
    /// they carry the same letters.
    #[test]
    fn a_field_term_marks_its_own_column_only() {
        let entry = row("api-token", "-", "secrets/token.yaml");
        let found = spans("name:tok", &entry);
        assert_eq!(found.first(), Some(&only(4..7)), "{found:?}");
        assert_eq!(found.get(7), Some(&Vec::new()), "{found:?}");
    }

    /// An exact term is the whole cell, and a regular expression is each run it
    /// finds.
    #[test]
    fn an_exact_term_marks_the_cell_and_a_regex_marks_each_run() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert_eq!(spans("+api-token", &entry).first(), Some(&only(0..9)));
        assert_eq!(spans("*n:to.en", &entry).first(), Some(&only(4..9)));
        assert_eq!(spans("*n:[ao]", &entry).first(), Some(&vec![0..1, 5..6]));
    }

    /// Two terms finding the same letters mark them once.
    #[test]
    fn overlapping_terms_leave_one_range() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert_eq!(
            spans("name:tok name:oke", &entry).first(),
            Some(&only(4..8))
        );
    }

    /// An exclusion marks nothing in the rows it leaves standing.
    #[test]
    fn an_exclusion_marks_nothing() {
        let mail = row("mail-password", "-", "secrets/alice.yaml");
        let found = spans("!tok", &mail);
        assert!(
            found.iter().all(Vec::is_empty),
            "an exclusion emphasised something\n{found:?}"
        );
        assert_eq!(Query::parse("!tok").searched_columns(), Searched::Nothing);
    }

    /// A rejected row has no spans at all, which is what makes the filter and
    /// the emphasis one answer rather than two.
    #[test]
    fn a_rejected_row_has_no_spans() {
        let entry = row("api-token", "-", "secrets/alice.yaml");
        assert!(Query::parse("wifi").spans(&entry).is_none());
        assert!(
            Query::parse("*[a-").spans(&entry).is_none(),
            "an unparsable regex offered the row"
        );
    }

    /// Which titles a query lights: its fields' own, all of them, or none.
    #[test]
    fn the_searched_columns_are_the_fields_or_every_column() {
        assert_eq!(Query::parse("").searched_columns(), Searched::Nothing);
        assert_eq!(Query::parse("tok").searched_columns(), Searched::Everything);
        assert_eq!(
            Query::parse("name:tok file:alice").searched_columns(),
            Searched::Columns(vec![0, 7])
        );
        // A bare term beside a field term still looks everywhere.
        assert_eq!(
            Query::parse("name:tok alice").searched_columns(),
            Searched::Everything
        );
        let name = Query::parse("name:tok").searched_columns();
        assert!(name.includes(0));
        assert!(!name.includes(7));
    }
}
