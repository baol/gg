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

/// The monospace columns `ch` takes: two for East Asian wide and
/// fullwidth characters (CJK ideographs, kana, Hangul, fullwidth forms,
/// most emoji), one for everything else.
pub fn char_columns(ch: char) -> usize {
    let code = u32::from(ch);
    let wide = matches!(code,
        0x1100..=0x115F
        | 0x231A..=0x231B
        | 0x2329..=0x232A
        | 0x23E9..=0x23EC
        | 0x23F0
        | 0x23F3
        | 0x25FD..=0x25FE
        | 0x2614..=0x2615
        | 0x2648..=0x2653
        | 0x267F
        | 0x2693
        | 0x26A1
        | 0x26AA..=0x26AB
        | 0x26BD..=0x26BE
        | 0x26C4..=0x26C5
        | 0x26CE
        | 0x26D4
        | 0x26EA
        | 0x26F2..=0x26F3
        | 0x26F5
        | 0x26FA
        | 0x26FD
        | 0x2705
        | 0x270A..=0x270B
        | 0x2728
        | 0x274C
        | 0x274E
        | 0x2753..=0x2755
        | 0x2757
        | 0x2795..=0x2797
        | 0x27B0
        | 0x27BF
        | 0x2B1B..=0x2B1C
        | 0x2B50
        | 0x2B55
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xA960..=0xA97F
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE10..=0xFE19
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x16FE0..=0x18CFF
        | 0x1AFF0..=0x1B2FF
        | 0x1F004
        | 0x1F0CF
        | 0x1F18E
        | 0x1F191..=0x1F19A
        | 0x1F200..=0x1F251
        | 0x1F300..=0x1F64F
        | 0x1F680..=0x1F6FF
        | 0x1F7E0..=0x1F7EB
        | 0x1F90C..=0x1F9FF
        | 0x1FA70..=0x1FAFF
        | 0x20000..=0x2FFFD
        | 0x30000..=0x3FFFD
    );
    if wide { 2 } else { 1 }
}

/// Whether `text` has a character [`char_columns`] counts as two.
pub fn has_wide(text: &str) -> bool {
    !text.is_ascii() && text.chars().any(|ch| char_columns(ch) == 2)
}

/// The monospace columns `text` takes.
pub fn columns(text: &str) -> usize {
    if text.is_ascii() {
        return text.len();
    }
    text.chars().map(char_columns).sum()
}

/// The columns a line reaches for wrapping: where its last character
/// starts, plus one; 0 when empty. A character belongs to the wrapped row
/// its first column falls in, so a line of `reach` takes
/// [`Wrap::segments`]`(reach, columns)` rows. For text without wide
/// characters it is the character count.
pub fn reach(text: &str) -> usize {
    if text.is_ascii() {
        return text.len();
    }
    let mut at = 0;
    let mut last = None;
    for ch in text.chars() {
        last = Some(at);
        at += char_columns(ch);
    }
    last.map_or(0, |start| start + 1)
}

/// The characters of `text` in wrapped row `segment` at `columns`: those
/// whose first column falls in it. A wide character starting in a row's
/// last column stays in that row and overhangs it by one column.
pub fn segment_chars(text: &str, columns: usize, segment: usize) -> Range<usize> {
    let columns = columns.max(1);
    if !has_wide(text) {
        return segment * columns..(segment + 1) * columns;
    }
    let (from, to) = (segment * columns, (segment + 1) * columns);
    let mut at = 0;
    let mut start = None;
    let mut end = 0;
    for (index, ch) in text.chars().enumerate() {
        if at >= to {
            break;
        }
        if at >= from && start.is_none() {
            start = Some(index);
        }
        at += char_columns(ch);
        end = index + 1;
    }
    match start {
        Some(start) => start..end,
        None => end..end,
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
    fn wide_characters_take_two_columns_and_start_their_row() {
        assert_eq!(char_columns('a'), 1);
        assert_eq!(char_columns('é'), 1);
        assert_eq!(char_columns('漢'), 2);
        assert_eq!(char_columns('ア'), 2);
        assert_eq!(char_columns('한'), 2);
        assert_eq!(char_columns('Ａ'), 2);
        assert_eq!(columns("ab漢字"), 6);
        assert_eq!(reach(""), 0);
        assert_eq!(reach("abc"), 3);
        assert_eq!(reach("a漢"), 2, "the last character starts at column 1");
        // At 4 columns: "ab漢" fills the first row, "字c" the second.
        let text = "ab漢字c";
        assert_eq!(Wrap::segments(reach(text), 4), 2);
        assert_eq!(segment_chars(text, 4, 0), 0..3);
        assert_eq!(segment_chars(text, 4, 1), 3..5);
        assert_eq!(segment_chars(text, 4, 2), 5..5);
        // A wide character starting in the last column stays in its row.
        let text = "abc漢d";
        assert_eq!(segment_chars(text, 4, 0), 0..4);
        assert_eq!(segment_chars(text, 4, 1), 4..5);
        assert_eq!(Wrap::segments(reach(text), 4), 2);
        assert_eq!(
            segment_chars("abcdef", 4, 1),
            4..8,
            "plain text is arithmetic"
        );
    }

    #[test]
    fn character_ranges_respect_unicode() {
        let text = "aé→bc";
        assert_eq!(&text[char_range(text, 1..3)], "é→");
        assert_eq!(&text[char_range(text, 3..9)], "bc");
        assert_eq!(&text[char_range(text, 7..9)], "");
    }
}
