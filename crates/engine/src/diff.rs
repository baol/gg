//! Bounded byte-preserving unified text comparison, independent of Git and UI.
use serde::{Deserialize, Serialize};
use similar::{Algorithm, ChangeTag};
use std::time::{Duration, Instant};
pub const MAX_BLOB_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_LINES: usize = 100_000;
pub const COMPUTE_BUDGET: Duration = Duration::from_millis(250);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    Hunk,
    Context,
    Added,
    Deleted,
    NoNewline,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiffRow {
    pub kind: RowKind,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
    /// Original line bytes, including its terminator; empty for annotations.
    pub bytes: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Text,
    Metadata,
    Binary,
    Gitlink,
    Limited,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDiff {
    pub kind: DiffKind,
    pub old_size: u64,
    pub new_size: u64,
    pub complete: bool,
    pub note: String,
    pub rows: Vec<DiffRow>,
}
impl FileDiff {
    pub fn summary(kind: DiffKind, old_size: u64, new_size: u64, note: impl Into<String>) -> Self {
        Self {
            kind,
            old_size,
            new_size,
            complete: kind != DiffKind::Limited,
            note: note.into(),
            rows: vec![],
        }
    }
}
pub(crate) fn display(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut rest = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                result.push_str(text);
                break;
            }
            Err(e) => {
                if let Ok(text) = std::str::from_utf8(&rest[..e.valid_up_to()]) {
                    result.push_str(text);
                }
                let len = e.error_len().unwrap_or(rest.len() - e.valid_up_to());
                for b in &rest[e.valid_up_to()..e.valid_up_to() + len] {
                    result.push_str(&format!("\\x{b:02x}"));
                }
                rest = &rest[e.valid_up_to() + len..];
            }
        }
    }
    result.replace('\r', "\\r").replace('\t', "    ")
}
pub fn compare(old: &[u8], new: &[u8], cancelled: impl Fn() -> bool) -> Result<FileDiff, String> {
    let summary = |kind, note| FileDiff::summary(kind, old.len() as u64, new.len() as u64, note);
    if cancelled() {
        return Err("Comparison cancelled".into());
    }
    if old.len() > MAX_BLOB_BYTES || new.len() > MAX_BLOB_BYTES {
        return Ok(summary(DiffKind::Limited, "Text budget: 2 MiB per side"));
    }
    if old.contains(&0) || new.contains(&0) {
        return Ok(summary(DiffKind::Binary, "Binary content"));
    }
    let line_count = |bytes: &[u8]| {
        bytes.iter().filter(|b| **b == b'\n').count()
            + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
    };
    if line_count(old) + line_count(new) > MAX_LINES {
        return Ok(summary(
            DiffKind::Limited,
            "Text budget: 100,000 source lines",
        ));
    }
    let old_lines: Vec<_> = old.split_inclusive(|b| *b == b'\n').collect();
    let new_lines: Vec<_> = new.split_inclusive(|b| *b == b'\n').collect();
    let deadline = Instant::now() + COMPUTE_BUDGET;
    let ops = similar::capture_diff_slices_deadline(
        Algorithm::Myers,
        &old_lines,
        &new_lines,
        Some(deadline),
    );
    if cancelled() {
        return Err("Comparison cancelled".into());
    }
    if Instant::now() >= deadline {
        return Ok(summary(
            DiffKind::Limited,
            "Text comparison reached its time budget",
        ));
    }
    let mut result = summary(
        DiffKind::Text,
        if old == new { "Content unchanged" } else { "" },
    );
    for group in similar::group_diff_ops(ops, 3) {
        if cancelled() {
            return Err("Comparison cancelled".into());
        }
        let Some(first) = group.first() else {
            continue;
        };
        let Some(last) = group.last() else {
            continue;
        };
        let (a, b) = (
            first.old_range().start..last.old_range().end,
            first.new_range().start..last.new_range().end,
        );
        let coordinate = |r: &std::ops::Range<usize>| {
            format!("{},{}", r.start + usize::from(!r.is_empty()), r.len())
        };
        result.rows.push(DiffRow {
            kind: RowKind::Hunk,
            old_line: None,
            new_line: None,
            text: format!("@@ -{} +{} @@", coordinate(&a), coordinate(&b)),
            bytes: vec![],
        });
        for op in group {
            for change in op.iter_changes(&old_lines, &new_lines) {
                if cancelled() {
                    return Err("Comparison cancelled".into());
                }
                let bytes = change.value();
                let kind = match change.tag() {
                    ChangeTag::Equal => RowKind::Context,
                    ChangeTag::Delete => RowKind::Deleted,
                    ChangeTag::Insert => RowKind::Added,
                };
                result.rows.push(DiffRow {
                    kind,
                    old_line: change.old_index().map(|n| n + 1),
                    new_line: change.new_index().map(|n| n + 1),
                    text: display(bytes),
                    bytes: bytes.to_vec(),
                });
                if !bytes.ends_with(b"\n") {
                    result.rows.push(DiffRow {
                        kind: RowKind::NoNewline,
                        old_line: None,
                        new_line: None,
                        text: "\\ No newline at end of file".into(),
                        bytes: vec![],
                    });
                }
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_exact_lines_context_and_missing_newline() -> Result<(), String> {
        let old = b"one\r\nold\nkeep\nlast";
        let new = b"one\r\nnew\nkeep\nlast\n";
        let diff = compare(old, new, || false)?;
        assert_eq!(diff.rows[0].text, "@@ -1,4 +1,4 @@");
        assert!(diff.rows.iter().any(|r| r.kind == RowKind::NoNewline));
        let reconstruct = |addition| {
            diff.rows
                .iter()
                .filter(|r| {
                    r.kind == RowKind::Context
                        || r.kind
                            == if addition {
                                RowKind::Added
                            } else {
                                RowKind::Deleted
                            }
                })
                .flat_map(|r| r.bytes.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(reconstruct(false), old);
        assert_eq!(reconstruct(true), new);
        assert_eq!(diff.rows[1].text, "one\\r");
        let invalid = compare(b"\xff\n", b"\xfe\n", || false)?;
        assert!(invalid.rows.iter().any(|r| r.text == "\\xff"));
        Ok(())
    }
    #[test]
    fn empty_binary_limits_and_cancellation_are_explicit() -> Result<(), String> {
        assert!(compare(b"", b"", || false)?.rows.is_empty());
        assert_eq!(
            compare(b"", b"new\n", || false)?.rows[0].text,
            "@@ -0,0 +1,1 @@"
        );
        assert_eq!(
            compare(b"old\n", b"", || false)?.rows[0].text,
            "@@ -1,1 +0,0 @@"
        );
        assert_eq!(compare(b"\0", b"", || false)?.kind, DiffKind::Binary);
        assert!(!compare(&vec![b'a'; MAX_BLOB_BYTES + 1], b"", || false)?.complete);
        assert!(compare(b"a", b"b", || true).is_err());
        assert_eq!(
            compare(&vec![b'\n'; MAX_LINES + 1], b"", || false)?.kind,
            DiffKind::Limited
        );
        assert!(compare(b"same\n", b"same\n", || false)?.rows.is_empty());
        Ok(())
    }
}
