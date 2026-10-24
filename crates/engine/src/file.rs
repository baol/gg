//! Bounded single-side file text and place anchors for the reading surface.
use crate::diff::{DiffRow, RowKind, display};
use crate::review::FileSide;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A full-file read keeps one side within the same text budget as a diff side.
pub const MAX_FILE_LINES: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingSide {
    Before,
    After,
}

impl ReadingSide {
    pub fn label(self) -> &'static str {
        match self {
            Self::Before => "Before",
            Self::After => "After",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Before => Self::After,
            Self::After => Self::Before,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Text,
    Empty,
    Binary,
    Limited,
    Gitlink,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileLine {
    /// One-based line number on this side.
    pub number: usize,
    pub text: String,
    /// Exact line bytes, including its terminator.
    pub bytes: Vec<u8>,
    /// True when the line was added (After) or deleted (Before) by the comparison.
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileText {
    pub kind: FileKind,
    pub side: ReadingSide,
    pub size: u64,
    pub complete: bool,
    pub note: String,
    pub lines: Vec<FileLine>,
}

impl FileText {
    pub fn missing(side: ReadingSide, note: impl Into<String>) -> Self {
        Self {
            kind: FileKind::Empty,
            side,
            size: 0,
            complete: true,
            note: note.into(),
            lines: vec![],
        }
    }

    pub fn summary(kind: FileKind, side: ReadingSide, size: u64, note: impl Into<String>) -> Self {
        Self {
            kind,
            side,
            size,
            complete: kind != FileKind::Limited,
            note: note.into(),
            lines: vec![],
        }
    }

    /// Split exact blob bytes into display lines and mark changed lines.
    pub fn from_bytes(side: ReadingSide, bytes: &[u8], changed: &BTreeSet<usize>) -> FileText {
        let size = bytes.len() as u64;
        if bytes.is_empty() {
            return FileText::summary(FileKind::Empty, side, size, "Empty file");
        }
        if bytes.contains(&0) {
            return FileText::summary(FileKind::Binary, side, size, "Binary content");
        }
        let count = bytes.iter().filter(|byte| **byte == b'\n').count()
            + usize::from(!bytes.ends_with(b"\n"));
        if count > MAX_FILE_LINES {
            return FileText::summary(
                FileKind::Limited,
                side,
                size,
                "Text budget: 100,000 source lines",
            );
        }
        let lines = bytes
            .split_inclusive(|byte| *byte == b'\n')
            .enumerate()
            .map(|(index, line)| {
                let number = index + 1;
                FileLine {
                    number,
                    text: display(line),
                    bytes: line.to_vec(),
                    changed: changed.contains(&number),
                }
            })
            .collect();
        FileText {
            kind: FileKind::Text,
            side,
            size,
            complete: true,
            note: String::new(),
            lines,
        }
    }

    /// Contiguous one-based inclusive ranges of changed lines.
    pub fn changed_groups(&self) -> Vec<(usize, usize)> {
        let mut groups: Vec<(usize, usize)> = vec![];
        for line in &self.lines {
            if !line.changed {
                continue;
            }
            match groups.last_mut() {
                Some((_, end)) if *end + 1 == line.number => *end = line.number,
                _ => groups.push((line.number, line.number)),
            }
        }
        groups
    }
}

/// Lines of one side that the unified diff marks as added or deleted.
pub fn changed_lines(rows: &[DiffRow], side: ReadingSide) -> BTreeSet<usize> {
    rows.iter()
        .filter_map(|row| match (side, row.kind) {
            (ReadingSide::Before, RowKind::Deleted) => row.old_line,
            (ReadingSide::After, RowKind::Added) => row.new_line,
            _ => None,
        })
        .collect()
}

/// Exact identity for history requests: path bytes, pinned revision, and a line range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub path_bytes: Vec<u8>,
    pub revision: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl Place {
    pub fn new(
        path_bytes: impl Into<Vec<u8>>,
        revision: impl Into<String>,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        let (start_line, end_line) = if start_line <= end_line {
            (start_line, end_line)
        } else {
            (end_line, start_line)
        };
        Self {
            path_bytes: path_bytes.into(),
            revision: revision.into(),
            start_line: start_line.max(1),
            end_line: end_line.max(1),
        }
    }
}

/// Resolve the side that exists for an added or deleted file.
pub fn existing_side(old: Option<&FileSide>, new: Option<&FileSide>) -> ReadingSide {
    if new.is_some() || old.is_none() {
        ReadingSide::After
    } else {
        ReadingSide::Before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffKind, FileDiff};

    fn row(kind: RowKind, old: Option<usize>, new: Option<usize>) -> DiffRow {
        DiffRow {
            kind,
            old_line: old,
            new_line: new,
            text: String::new(),
            bytes: vec![],
        }
    }

    #[test]
    fn splits_exact_lines_and_marks_each_side() {
        let lines = changed_lines(
            &[
                row(RowKind::Context, Some(1), Some(1)),
                row(RowKind::Deleted, Some(2), None),
                row(RowKind::Added, None, Some(2)),
                row(RowKind::Context, Some(3), Some(3)),
            ],
            ReadingSide::Before,
        );
        assert_eq!(lines, BTreeSet::from([2]));
        let file = FileText::from_bytes(ReadingSide::Before, b"one\r\ntwo\nthree", &lines);
        assert_eq!(file.kind, FileKind::Text);
        assert_eq!(file.size, 14);
        assert_eq!(file.lines.len(), 3);
        assert_eq!(file.lines[0].text, "one\\r");
        assert_eq!(file.lines[1].text, "two");
        assert!(!file.lines[0].changed && file.lines[1].changed);
        assert_eq!(file.lines[2].bytes, b"three");
        assert_eq!(file.changed_groups(), vec![(2, 2)]);
    }

    #[test]
    fn empty_binary_and_over_budget_are_explicit() {
        let empty = FileText::from_bytes(ReadingSide::After, b"", &BTreeSet::new());
        assert_eq!(empty.kind, FileKind::Empty);
        assert!(empty.lines.is_empty());
        let binary = FileText::from_bytes(ReadingSide::After, b"a\0b", &BTreeSet::new());
        assert_eq!(binary.kind, FileKind::Binary);
        let wide = vec![b'\n'; MAX_FILE_LINES + 1];
        let limited = FileText::from_bytes(ReadingSide::Before, &wide, &BTreeSet::new());
        assert_eq!(limited.kind, FileKind::Limited);
        assert!(!limited.complete);
    }

    #[test]
    fn consecutive_changes_form_groups() {
        let mut file =
            FileText::from_bytes(ReadingSide::After, b"a\nb\nc\nd\ne\nf\n", &BTreeSet::new());
        for index in [1, 2, 4] {
            file.lines[index].changed = true;
        }
        assert_eq!(file.changed_groups(), vec![(2, 3), (5, 5)]);
    }

    #[test]
    fn place_normalizes_range_and_keeps_exact_path_bytes() {
        let place = Place::new(b"a\xffb".to_vec(), "abc", 9, 4);
        assert_eq!(place.path_bytes, b"a\xffb");
        assert_eq!((place.start_line, place.end_line), (4, 9));
        assert_eq!(Place::new(b"a".to_vec(), "abc", 0, 0).start_line, 1);
    }

    #[test]
    fn change_markers_use_unified_rows() {
        let mut diff = FileDiff::summary(DiffKind::Text, 0, 0, "");
        diff.rows = vec![
            row(RowKind::Hunk, None, None),
            row(RowKind::Added, None, Some(1)),
            row(RowKind::Deleted, Some(1), None),
        ];
        assert_eq!(
            changed_lines(&diff.rows, ReadingSide::After),
            BTreeSet::from([1])
        );
        assert_eq!(
            changed_lines(&diff.rows, ReadingSide::Before),
            BTreeSet::from([1])
        );
    }
}
