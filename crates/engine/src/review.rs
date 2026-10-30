//! Immutable comparison identities and incremental changed-file summaries.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ReviewRequest {
    pub path: PathBuf,
    pub revision: String,
    pub parent: Option<usize>,
    /// Compare `revision` against this commit instead of one of its parents.
    pub against: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub path: PathBuf,
    pub commit: String,
    pub parent: Option<String>,
    pub parents: Vec<String>,
    pub subject: String,
    /// Author name of the compared commit.
    #[serde(default)]
    pub author: String,
    /// Author date of the compared commit, `YYYY-MM-DD`.
    #[serde(default)]
    pub date: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSide {
    pub id: String,
    pub mode: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Deleted,
    Modified,
    TypeChanged,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub path_bytes: Vec<u8>,
    pub path: String,
    pub kind: ChangeKind,
    pub old: Option<FileSide>,
    pub new: Option<FileSide>,
}
/// Added and deleted line counts for one changed file, as `git diff --numstat`
/// reports them, or why the file is not counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStat {
    Lines {
        added: u32,
        deleted: u32,
    },
    /// Either side holds binary content.
    Binary,
    /// A gitlink, or a side beyond the text-diff budget.
    Skipped,
}
impl LineStat {
    /// Count lines like the text diff, within the same budgets.
    pub fn compare(old: &[u8], new: &[u8]) -> Self {
        use crate::diff::{COMPUTE_BUDGET, MAX_BLOB_BYTES, MAX_LINES};
        if old.len() > MAX_BLOB_BYTES || new.len() > MAX_BLOB_BYTES {
            return Self::Skipped;
        }
        if old.contains(&0) || new.contains(&0) {
            return Self::Binary;
        }
        let old_lines: Vec<_> = old.split_inclusive(|b| *b == b'\n').collect();
        let new_lines: Vec<_> = new.split_inclusive(|b| *b == b'\n').collect();
        if old_lines.len() + new_lines.len() > MAX_LINES {
            return Self::Skipped;
        }
        let deadline = std::time::Instant::now() + COMPUTE_BUDGET;
        let (mut added, mut deleted) = (0u32, 0u32);
        for op in similar::capture_diff_slices_deadline(
            similar::Algorithm::Myers,
            &old_lines,
            &new_lines,
            Some(deadline),
        ) {
            if op.tag() != similar::DiffTag::Equal {
                added = added.saturating_add(op.new_range().len() as u32);
                deleted = deleted.saturating_add(op.old_range().len() as u32);
            }
        }
        Self::Lines { added, deleted }
    }
}
pub enum ReviewStep {
    Change(FileChange),
    Pending,
    Complete,
}
pub trait ReviewReader {
    fn comparison(&self) -> &Comparison;
    /// One traversal step; callers can cancel between directory visits.
    fn step(&mut self) -> Result<ReviewStep, String>;
}

/// One-line, unambiguous display; path_bytes remains the native lookup identity.
pub fn display_path(bytes: &[u8]) -> String {
    let mut result = String::new();
    for chunk in bytes.utf8_chunks() {
        for ch in chunk.valid().chars() {
            match ch {
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                ch if ch.is_control() => result.extend(ch.escape_unicode()),
                ch => result.push(ch),
            }
        }
        for byte in chunk.invalid() {
            result.push_str(&format!("\\x{byte:02x}"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_display_preserves_unicode_and_escapes_ambiguous_bytes() {
        assert_eq!(display_path("café/界.rs".as_bytes()), "café/界.rs");
        assert_eq!(
            display_path(b"a\n\r\t\x1b\\\xff"),
            "a\\n\\r\\t\\u{1b}\\\\\\xff"
        );
        assert_ne!(display_path(b"\xff"), display_path(b"\\xff"));
    }
    #[test]
    fn line_stats_count_changes_and_skip_binary_or_oversized_sides() {
        let lines = |added, deleted| LineStat::Lines { added, deleted };
        assert_eq!(
            LineStat::compare(b"a\nb\nc\n", b"a\nB\nc\nd\n"),
            lines(2, 1)
        );
        // A missing final newline changes the last line, as in Git.
        assert_eq!(LineStat::compare(b"a", b"a\n"), lines(1, 1));
        assert_eq!(LineStat::compare(b"", b"x\ny"), lines(2, 0));
        assert_eq!(LineStat::compare(b"a\n", b"\0"), LineStat::Binary);
        let large = vec![b'a'; crate::diff::MAX_BLOB_BYTES + 1];
        assert_eq!(LineStat::compare(b"", &large), LineStat::Skipped);
        let many = "x\n".repeat(crate::diff::MAX_LINES + 1);
        assert_eq!(LineStat::compare(b"", many.as_bytes()), LineStat::Skipped);
    }
}
