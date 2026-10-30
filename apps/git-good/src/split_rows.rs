//! Side-by-side pairing of unified diff rows.
//!
//! A run of deleted rows followed by a run of added rows shares display rows:
//! the k-th deleted row sits beside the k-th added row and the shorter side is
//! padded with empty cells. Every other row (hunk header, context, no-newline
//! note) takes one display row of its own. The layout keeps only the change
//! runs, so it stays small for large diffs and maps display rows to unified
//! rows (and back) by binary search while the rows themselves stay paged.
use gg_engine::diff::RowKind;

/// One contiguous change: `deleted` rows from `start`, then `added` rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    start: usize,
    deleted: usize,
    added: usize,
    /// Display row of `start`.
    display: usize,
}

impl Run {
    fn rows(self) -> usize {
        self.deleted.max(self.added)
    }

    fn saved(self) -> usize {
        self.deleted.min(self.added)
    }
}

/// The unified rows shown in one display row. Both sides name the same row
/// for hunk headers, context, and notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    pub left: Option<usize>,
    pub right: Option<usize>,
}

impl Pair {
    /// Whether the row spans both columns rather than pairing two changes.
    pub fn shared(self) -> bool {
        self.left.is_some() && self.left == self.right
    }

    /// The first unified row this display row shows.
    pub fn first(self) -> Option<usize> {
        self.left.or(self.right)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    runs: Vec<Run>,
}

impl Layout {
    /// Pair the rows of one document, given each row's kind in order.
    pub fn from_kinds(kinds: impl IntoIterator<Item = RowKind>) -> Self {
        let mut runs: Vec<Run> = vec![];
        let mut saved = 0;
        let mut open: Option<Run> = None;
        fn close(run: Option<Run>, runs: &mut Vec<Run>, saved: &mut usize) {
            if let Some(run) = run {
                *saved += run.saved();
                runs.push(run);
            }
        }
        for (index, kind) in kinds.into_iter().enumerate() {
            match kind {
                RowKind::Deleted => {
                    // A deletion after additions starts a new run.
                    if open.is_some_and(|run| run.added > 0) {
                        close(open.take(), &mut runs, &mut saved);
                    }
                    let run = open.get_or_insert(Run {
                        start: index,
                        deleted: 0,
                        added: 0,
                        display: index - saved,
                    });
                    run.deleted += 1;
                }
                RowKind::Added => {
                    let run = open.get_or_insert(Run {
                        start: index,
                        deleted: 0,
                        added: 0,
                        display: index - saved,
                    });
                    run.added += 1;
                }
                _ => close(open.take(), &mut runs, &mut saved),
            }
        }
        close(open, &mut runs, &mut saved);
        Self { runs }
    }

    fn saved_through(&self, run: usize) -> usize {
        // Display offset of the next unified row after this run.
        let r = self.runs[run];
        r.start + r.deleted + r.added - (r.display + r.rows())
    }

    /// Display rows for a document of `count` unified rows.
    pub fn count(&self, count: usize) -> usize {
        match self.runs.len() {
            0 => count,
            n => count.saturating_sub(self.saved_through(n - 1)),
        }
    }

    /// The display row that shows unified row `row`.
    pub fn display(&self, row: usize) -> usize {
        let index = self.runs.partition_point(|run| run.start <= row);
        if index == 0 {
            return row;
        }
        let run = self.runs[index - 1];
        let offset = row - run.start;
        if offset < run.deleted {
            run.display + offset
        } else if offset < run.deleted + run.added {
            run.display + offset - run.deleted
        } else {
            row - self.saved_through(index - 1)
        }
    }

    /// The unified rows shown in display row `display`.
    pub fn pair(&self, display: usize) -> Pair {
        let index = self.runs.partition_point(|run| run.display <= display);
        if index == 0 {
            return Pair {
                left: Some(display),
                right: Some(display),
            };
        }
        let run = self.runs[index - 1];
        let offset = display - run.display;
        if offset < run.rows() {
            return Pair {
                left: (offset < run.deleted).then_some(run.start + offset),
                right: (offset < run.added).then_some(run.start + run.deleted + offset),
            };
        }
        let row = display + self.saved_through(index - 1);
        Pair {
            left: Some(row),
            right: Some(row),
        }
    }

    /// Unified rows covering display rows `range`, for paging. The range is
    /// the visible window, so walking it is cheap.
    pub fn rows(&self, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let mut bounds: Option<(usize, usize)> = None;
        for display in range.clone() {
            let pair = self.pair(display);
            for row in [pair.left, pair.right].into_iter().flatten() {
                bounds = Some(match bounds {
                    Some((lo, hi)) => (lo.min(row), hi.max(row + 1)),
                    None => (row, row + 1),
                });
            }
        }
        match bounds {
            Some((lo, hi)) => lo..hi,
            None => {
                let row = self.pair(range.start).first().unwrap_or(range.start);
                row..row
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RowKind::{Added as A, Context as C, Deleted as D, Hunk as H, NoNewline as N};

    fn pairs(layout: &Layout, count: usize) -> Vec<(Option<usize>, Option<usize>)> {
        (0..layout.count(count))
            .map(|display| {
                let pair = layout.pair(display);
                (pair.left, pair.right)
            })
            .collect()
    }

    #[test]
    fn deletions_pair_with_additions_and_the_shorter_side_is_padded() {
        // 0 H, 1 C, 2 D, 3 D, 4 D, 5 A, 6 C, 7 A, 8 A, 9 C
        let kinds = [H, C, D, D, D, A, C, A, A, C];
        let layout = Layout::from_kinds(kinds);
        assert_eq!(layout.count(kinds.len()), 9);
        assert_eq!(
            pairs(&layout, kinds.len()),
            [
                (Some(0), Some(0)),
                (Some(1), Some(1)),
                (Some(2), Some(5)),
                (Some(3), None),
                (Some(4), None),
                (Some(6), Some(6)),
                (None, Some(7)),
                (None, Some(8)),
                (Some(9), Some(9)),
            ]
        );
    }

    #[test]
    fn more_additions_than_deletions_pad_the_left_side() {
        let kinds = [H, D, A, A, A, C, H, D, D, A, A];
        let layout = Layout::from_kinds(kinds);
        assert_eq!(
            pairs(&layout, kinds.len()),
            [
                (Some(0), Some(0)),
                (Some(1), Some(2)),
                (None, Some(3)),
                (None, Some(4)),
                (Some(5), Some(5)),
                (Some(6), Some(6)),
                (Some(7), Some(9)),
                (Some(8), Some(10)),
            ]
        );
        assert!(layout.pair(0).shared());
        assert!(!layout.pair(1).shared());
    }

    #[test]
    fn display_and_pair_are_inverse_for_every_row() {
        let kinds = [H, D, D, A, C, A, D, A, A, N, C, D, H, A, D, D, A, C];
        let layout = Layout::from_kinds(kinds);
        for row in 0..kinds.len() {
            let pair = layout.pair(layout.display(row));
            assert!(
                pair.left == Some(row) || pair.right == Some(row),
                "row {row} is shown in display row {}",
                layout.display(row)
            );
        }
        let shown: usize = (0..layout.count(kinds.len()))
            .map(|display| {
                let pair = layout.pair(display);
                if pair.shared() {
                    1
                } else {
                    usize::from(pair.left.is_some()) + usize::from(pair.right.is_some())
                }
            })
            .sum();
        assert_eq!(shown, kinds.len(), "every unified row is shown once");
    }

    #[test]
    fn an_addition_followed_by_a_deletion_starts_a_new_run() {
        let kinds = [A, D, A];
        let layout = Layout::from_kinds(kinds);
        assert_eq!(
            pairs(&layout, kinds.len()),
            [(None, Some(0)), (Some(1), Some(2))]
        );
    }

    #[test]
    fn a_document_without_paired_changes_maps_one_to_one() {
        let kinds = [H, A, A, C, D];
        let layout = Layout::from_kinds(kinds);
        assert_eq!(
            (layout.pair(1), layout.pair(4)),
            (
                Pair {
                    left: None,
                    right: Some(1)
                },
                Pair {
                    left: Some(4),
                    right: None
                }
            ),
            "an unpaired change keeps its own side"
        );
        assert_eq!(layout.count(5), 5);
        assert_eq!(layout.display(3), 3);
        assert_eq!(layout.rows(1..3), 1..3);
    }

    #[test]
    fn paging_covers_both_sides_of_the_visible_rows() {
        let kinds = [H, D, D, A, A, C];
        let layout = Layout::from_kinds(kinds);
        // Display rows 1..3 show deleted 1, 2 and added 3, 4.
        assert_eq!(layout.rows(1..3), 1..5);
        assert_eq!(layout.rows(0..4), 0..6);
        // A padded row still pages the addition paired above it.
        let layout = Layout::from_kinds([H, D, D, D, A, C]);
        assert_eq!(layout.rows(1..3), 1..5);
    }
}
