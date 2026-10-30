//! Bounded byte-preserving unified text comparison, independent of Git and UI.
use serde::{Deserialize, Serialize};
use similar::{Algorithm, DiffTag};
use std::time::{Duration, Instant};
pub const MAX_BLOB_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_LINES: usize = 100_000;
/// Time spent looking for the smallest diff; after it the diff is coarser but exact.
pub const COMPUTE_BUDGET: Duration = Duration::from_millis(250);
/// Unchanged lines shown around each change unless the reader asks for more.
pub const CONTEXT: usize = 3;
/// Which whitespace differences count as changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Whitespace {
    /// Every byte counts, like `git diff`.
    #[default]
    All,
    /// Whitespace before the line end does not count, like `--ignore-space-at-eol`.
    IgnoreAtEol,
    /// No whitespace counts, like `-w`.
    IgnoreAll,
}
/// How a comparison is grouped and which differences it reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Options {
    /// Unchanged lines around each change, like `git diff -U<n>`.
    pub context: usize,
    pub whitespace: Whitespace,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            context: CONTEXT,
            whitespace: Whitespace::All,
        }
    }
}
/// Git's whitespace bytes.
fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}
/// The bytes a line is compared by under `whitespace`.
fn key(line: &[u8], whitespace: Whitespace) -> std::borrow::Cow<'_, [u8]> {
    match whitespace {
        Whitespace::All => line.into(),
        Whitespace::IgnoreAtEol => {
            let end = line.iter().rposition(|b| !space(*b)).map_or(0, |i| i + 1);
            line[..end].into()
        }
        Whitespace::IgnoreAll => line.iter().copied().filter(|b| !space(*b)).collect(),
    }
}
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
/// The displayed text of a line with tabs drawn as `→` and trailing spaces
/// as `·`, plus the byte ranges of those markers in the text. Each marker
/// takes the place of one displayed space, so columns match [`display`].
pub fn marked(bytes: &[u8]) -> (String, Vec<std::ops::Range<usize>>) {
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let core = body.strip_suffix(b"\r").unwrap_or(body);
    let trailing = core
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t'))
        .map_or(0, |i| i + 1);
    let mut text = String::new();
    let mut marks = vec![];
    let mut mark = |text: &mut String, marker: char| {
        let start = text.len();
        text.push(marker);
        marks.push(start..text.len());
    };
    let mut offset = 0;
    let mut rest = body;
    while !rest.is_empty() {
        let (valid, invalid) = match std::str::from_utf8(rest) {
            Ok(valid) => (valid, 0),
            Err(e) => (
                std::str::from_utf8(&rest[..e.valid_up_to()]).unwrap_or_default(),
                e.error_len().unwrap_or(rest.len() - e.valid_up_to()),
            ),
        };
        for (index, ch) in valid.char_indices() {
            match ch {
                '\t' => {
                    mark(&mut text, '→');
                    text.push_str("   ");
                }
                ' ' if offset + index >= trailing => mark(&mut text, '·'),
                '\r' => text.push_str("\\r"),
                ch => text.push(ch),
            }
        }
        for b in &rest[valid.len()..valid.len() + invalid] {
            text.push_str(&format!("\\x{b:02x}"));
        }
        offset += valid.len() + invalid;
        rest = &rest[valid.len() + invalid..];
    }
    (text, marks)
}
/// One emitted line: kind, zero-based old and new indices, and its bytes.
type SourceLine<'a> = (RowKind, Option<usize>, Option<usize>, &'a [u8]);
/// Compare with three context lines, counting every byte.
pub fn compare(old: &[u8], new: &[u8], cancelled: impl Fn() -> bool) -> Result<FileDiff, String> {
    compare_with(old, new, Options::default(), cancelled)
}
/// Compare with `options`. Rows keep exact bytes; with whitespace ignored,
/// unchanged lines show the new side, like `git diff -w`.
pub fn compare_with(
    old: &[u8],
    new: &[u8],
    options: Options,
    cancelled: impl Fn() -> bool,
) -> Result<FileDiff, String> {
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
    let old_keys: Vec<_> = old_lines
        .iter()
        .map(|line| key(line, options.whitespace))
        .collect();
    let new_keys: Vec<_> = new_lines
        .iter()
        .map(|line| key(line, options.whitespace))
        .collect();
    let deadline = Instant::now() + COMPUTE_BUDGET;
    let ops = similar::capture_diff_slices_deadline(
        Algorithm::Myers,
        &old_keys,
        &new_keys,
        Some(deadline),
    );
    if cancelled() {
        return Err("Comparison cancelled".into());
    }
    // Past the deadline the algorithm stops refining and returns a coarser
    // but still exact diff; the reader always gets text, never a fallback.
    let mut result = summary(
        DiffKind::Text,
        if old == new { "Content unchanged" } else { "" },
    );
    let groups = similar::group_diff_ops(ops, options.context);
    if groups.is_empty() && old != new {
        result.note = "Only whitespace changed; whitespace changes are hidden".into();
    }
    for group in groups {
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
            let (tag, old_range, new_range) = op.as_tag_tuple();
            let lines: Vec<SourceLine> = if tag == DiffTag::Equal {
                old_range
                    .zip(new_range)
                    .map(|(o, n)| (RowKind::Context, Some(o), Some(n), new_lines[n]))
                    .collect()
            } else {
                old_range
                    .map(|o| (RowKind::Deleted, Some(o), None, old_lines[o]))
                    .chain(new_range.map(|n| (RowKind::Added, None, Some(n), new_lines[n])))
                    .collect()
            };
            for (kind, old_index, new_index, bytes) in lines {
                if cancelled() {
                    return Err("Comparison cancelled".into());
                }
                result.rows.push(DiffRow {
                    kind,
                    old_line: old_index.map(|n| n + 1),
                    new_line: new_index.map(|n| n + 1),
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
    fn a_slow_comparison_still_returns_a_complete_text_diff() -> Result<(), String> {
        // Reversing 20,000 lines makes an exact diff far slower than the
        // compute budget; the reader must still get text, never a fallback.
        let old = (0..20_000)
            .map(|i| format!("line {i}\n"))
            .collect::<String>();
        let new = (0..20_000)
            .rev()
            .map(|i| format!("line {i}\n"))
            .collect::<String>();
        let diff = compare(old.as_bytes(), new.as_bytes(), || false)?;
        assert_eq!(diff.kind, DiffKind::Text, "{}", diff.note);
        assert!(diff.complete);
        let side = |keep: RowKind| {
            diff.rows
                .iter()
                .filter(|row| row.kind == RowKind::Context || row.kind == keep)
                .flat_map(|row| row.bytes.clone())
                .collect::<Vec<u8>>()
        };
        assert_eq!(
            side(RowKind::Deleted),
            old.as_bytes(),
            "the old side is intact"
        );
        assert_eq!(
            side(RowKind::Added),
            new.as_bytes(),
            "the new side is intact"
        );
        Ok(())
    }
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
    fn invisible_markers_keep_display_columns() {
        let (text, marks) = marked(b"\tif a  b \t \r\n");
        assert_eq!(text, "→   if a  b·→   ·\\r");
        let markers: Vec<&str> = marks.iter().map(|range| &text[range.clone()]).collect();
        assert_eq!(markers, ["→", "·", "→", "·"], "inner spaces stay plain");
        for line in [
            &b"\tif a  b \t \r\n"[..],
            b"plain\n",
            b"\xff\t \n",
            b"caf\xc3\xa9  ",
            b"",
        ] {
            let (text, marks) = marked(line);
            let mut plain = text.clone();
            for range in marks.iter().rev() {
                plain.replace_range(range.clone(), " ");
            }
            assert_eq!(plain, display(line), "{line:?}");
        }
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
