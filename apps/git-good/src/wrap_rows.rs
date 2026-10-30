//! Wrapped reading rows: a long source row continues on extra fixed-height
//! display rows, so virtualization keeps one row height and the whole
//! document's geometry is known before any of its text is loaded.
use std::ops::Range;

/// Display rows of base rows (unified diff rows, side-by-side rows, or
/// file lines) whose text wraps at a fixed number of character columns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wrap {
    /// First display row of each base row, plus the total at the end.
    starts: Vec<usize>,
}

impl Wrap {
    /// `segments` gives each base row's display rows; zero counts as one.
    pub fn new(segments: impl IntoIterator<Item = usize>) -> Self {
        let mut starts = vec![0];
        let mut total = 0;
        for count in segments {
            total += count.max(1);
            starts.push(total);
        }
        Self { starts }
    }

    /// Display rows a line of `chars` characters takes at `columns`.
    pub fn segments(chars: usize, columns: usize) -> usize {
        chars.div_ceil(columns.max(1)).max(1)
    }

    /// Base rows covered.
    pub fn len(&self) -> usize {
        self.starts.len() - 1
    }

    /// Display rows in total.
    pub fn count(&self) -> usize {
        self.starts.last().copied().unwrap_or(0)
    }

    /// The first display row of base row `row`; rows past the end follow it.
    pub fn start(&self, row: usize) -> usize {
        match self.starts.get(row) {
            Some(start) => *start,
            None => self.count() + (row - self.len()),
        }
    }

    /// The base row shown in display row `display` and its segment.
    pub fn row(&self, display: usize) -> (usize, usize) {
        if display >= self.count() {
            return (self.len() + (display - self.count()), 0);
        }
        let row = self.starts.partition_point(|start| *start <= display) - 1;
        (row, display - self.starts[row])
    }

    /// Base rows touched by `display` rows.
    pub fn rows(&self, display: Range<usize>) -> Range<usize> {
        if display.is_empty() {
            let row = self.row(display.start).0;
            return row..row;
        }
        self.row(display.start).0..self.row(display.end - 1).0 + 1
    }
}

/// The byte range of characters `chars` in `text`, clamped to its end.
pub fn char_range(text: &str, chars: Range<usize>) -> Range<usize> {
    let mut indices = text.char_indices().map(|(index, _)| index);
    let start = indices.nth(chars.start).unwrap_or(text.len());
    let end = if chars.end > chars.start {
        text[start..]
            .char_indices()
            .nth(chars.end - chars.start)
            .map_or(text.len(), |(index, _)| start + index)
    } else {
        start
    };
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_map_both_ways() {
        let wrap = Wrap::new([1, 3, 0, 2]);
        assert_eq!(wrap.len(), 4);
        assert_eq!(wrap.count(), 7);
        assert_eq!(
            (0..7).map(|display| wrap.row(display)).collect::<Vec<_>>(),
            [(0, 0), (1, 0), (1, 1), (1, 2), (2, 0), (3, 0), (3, 1)]
        );
        assert_eq!(
            (0..5).map(|row| wrap.start(row)).collect::<Vec<_>>(),
            [0, 1, 4, 5, 7]
        );
        assert_eq!(wrap.row(9), (6, 0), "rows past the end stay one high");
        assert_eq!(wrap.rows(2..6), 1..4);
        assert_eq!(Wrap::segments(0, 80), 1);
        assert_eq!(Wrap::segments(80, 80), 1);
        assert_eq!(Wrap::segments(81, 80), 2);
    }

    #[test]
    fn character_ranges_respect_unicode() {
        let text = "aé→bc";
        assert_eq!(&text[char_range(text, 1..3)], "é→");
        assert_eq!(&text[char_range(text, 3..9)], "bc");
        assert_eq!(&text[char_range(text, 7..9)], "");
    }
}
