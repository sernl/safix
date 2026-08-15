//! The aligned table `list` prints.
//!
//! The shell runtime pipes tab-separated rows through `column -t -s'\t'`, so
//! this reproduces what util-linux's `column` does with them: every column but
//! the last is padded to its widest cell plus two spaces, the last is not
//! padded at all, and each row ends at its last cell.
//!
//! Width is counted in characters. `column` counts display columns, which
//! differ for east-asian and combining characters; every field this renders is
//! a name, a path or a key drawn from the resolver's alphabet, and the one
//! field that is free text — a generator's description — is the place that
//! difference could show. It is recorded rather than papered over: making the
//! two agree there means a display-width table this does not yet carry.

/// Render rows the way `column -t -s'\t'` renders them, with a trailing newline
/// per row.
///
/// A row with fewer cells than the widest row ends after its own last cell,
/// which is what `column` does and is why the padding is applied per cell
/// rather than per column.
#[must_use]
pub fn aligned(rows: &[Vec<String>]) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();

    let mut rendered = String::new();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            rendered.push_str(cell);
            let is_last = index == row.len().saturating_sub(1);
            if !is_last {
                let width = widths.get(index).copied().unwrap_or(0);
                let padding = width.saturating_sub(cell.chars().count()).saturating_add(2);
                for _ in 0..padding {
                    rendered.push(' ');
                }
            }
        }
        rendered.push('\n');
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|cell| (*cell).to_owned()).collect()
    }

    #[test]
    fn columns_are_padded_to_the_widest_cell_plus_two() {
        let rendered = aligned(&[
            row(&["NAME", "ORIGIN", "SHARED"]),
            row(&["ana-alone", "private", "-"]),
            row(&["api-token", "private", "-"]),
        ]);
        assert_eq!(
            rendered,
            "NAME       ORIGIN   SHARED\nana-alone  private  -\napi-token  private  -\n"
        );
    }

    #[test]
    fn the_last_cell_of_a_row_is_not_padded() {
        let rendered = aligned(&[row(&["a", "long-value"]), row(&["bbbb", "x"])]);
        assert_eq!(rendered, "a     long-value\nbbbb  x\n");
    }

    #[test]
    fn a_single_cell_row_is_itself() {
        assert_eq!(aligned(&[row(&["one line"])]), "one line\n");
    }
}

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::aligned;

    /// Cells with no space in them, so that a rendered row can be split back
    /// into its cells and the reconstruction is a statement about the alignment
    /// rather than about the cells.
    const CELL: &str = "[a-zA-Z0-9._/,-]{1,12}";

    proptest! {
        /// Every cell survives, in order, and nothing else is added.
        #[test]
        fn a_row_reads_back_as_the_cells_it_was_given(
            rows in proptest::collection::vec(
                proptest::collection::vec(CELL, 1..7), 1..7),
        ) {
            let rendered = aligned(&rows);
            let read_back: Vec<Vec<String>> = rendered
                .lines()
                .map(|line| line.split_whitespace().map(str::to_owned).collect())
                .collect();
            prop_assert_eq!(read_back, rows);
        }

        /// Every row starts each column it has at the same offset, which is what
        /// makes the output a table rather than a list of padded strings.
        #[test]
        fn every_column_starts_at_one_offset_across_the_rows(
            rows in proptest::collection::vec(
                proptest::collection::vec(CELL, 3..4), 1..7),
        ) {
            let rendered = aligned(&rows);
            let offsets: Vec<Vec<usize>> = rendered
                .lines()
                .map(|line| {
                    let mut at = Vec::new();
                    let mut in_cell = false;
                    for (index, character) in line.chars().enumerate() {
                        if character == ' ' {
                            in_cell = false;
                        } else if !in_cell {
                            in_cell = true;
                            at.push(index);
                        }
                    }
                    at
                })
                .collect();
            let first = offsets.first().cloned().unwrap_or_default();
            for row in &offsets {
                prop_assert_eq!(row, &first);
            }
        }
    }
}
