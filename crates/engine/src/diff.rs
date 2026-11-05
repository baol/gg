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
/// Lines longer than this, in characters, get no word emphasis.
pub const WORD_LINE_LIMIT: usize = 1000;
/// Nor do lines of more tokens than this.
pub const WORD_TOKEN_LIMIT: usize = 400;

/// The tokens word emphasis compares: runs of letters, digits and `_`,
/// runs of white space, and each other character on its own; as character
/// ranges of `text`.
fn word_tokens(text: &str) -> Vec<(std::ops::Range<usize>, &str)> {
    #[derive(PartialEq)]
    enum Class {
        Word,
        Space,
        Other,
    }
    let class = |ch: char| {
        if ch.is_alphanumeric() || ch == '_' {
            Class::Word
        } else if ch.is_whitespace() {
            Class::Space
        } else {
            Class::Other
        }
    };
    let mut tokens = Vec::new();
    let mut start: Option<(usize, usize, Class)> = None;
    for (at, (byte, ch)) in text.char_indices().enumerate() {
        let kind = class(ch);
        match &start {
            Some((_, _, previous)) if *previous == kind && kind != Class::Other => {}
            _ => {
                if let Some((from, byte_from, _)) = start.take() {
                    tokens.push((from..at, &text[byte_from..byte]));
                }
                start = Some((at, byte, kind));
            }
        }
    }
    if let Some((from, byte_from, _)) = start {
        tokens.push((from..text.chars().count(), &text[byte_from..]));
    }
    tokens
}

/// Character ranges within one line.
pub type Spans = Vec<std::ops::Range<usize>>;

/// The words that differ between a deleted line and the added line paired
/// with it: character ranges of each, adjacent ranges joined. `None` when a
/// line is too long or has too many tokens to compare (see
/// [`WORD_LINE_LIMIT`], [`WORD_TOKEN_LIMIT`]), or when the two share no
/// word, so a line that changed throughout is not painted twice.
pub fn word_changes(old: &str, new: &str) -> Option<(Spans, Spans)> {
    if old.chars().count() > WORD_LINE_LIMIT || new.chars().count() > WORD_LINE_LIMIT {
        return None;
    }
    let (a, b) = (word_tokens(old), word_tokens(new));
    if a.len() > WORD_TOKEN_LIMIT || b.len() > WORD_TOKEN_LIMIT {
        return None;
    }
    // Longest common subsequence of tokens.
    let mut table = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i][j] = if a[i].1 == b[j].1 {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let shared_word = {
        let (mut i, mut j, mut any) = (0, 0, false);
        while i < a.len() && j < b.len() {
            if a[i].1 == b[j].1 {
                any |= a[i].1.chars().any(|ch| ch.is_alphanumeric());
                i += 1;
                j += 1;
            } else if table[i + 1][j] >= table[i][j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
        any
    };
    if !shared_word {
        return None;
    }
    let mut old_changed = Vec::new();
    let mut new_changed = Vec::new();
    let push =
        |ranges: &mut Vec<std::ops::Range<usize>>, range: std::ops::Range<usize>| match ranges
            .last_mut()
        {
            Some(last) if last.end == range.start => last.end = range.end,
            _ => ranges.push(range),
        };
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i].1 == b[j].1 {
            i += 1;
            j += 1;
        } else if j >= b.len() || (i < a.len() && table[i + 1][j] >= table[i][j + 1]) {
            push(&mut old_changed, a[i].0.clone());
            i += 1;
        } else {
            push(&mut new_changed, b[j].0.clone());
            j += 1;
        }
    }
    Some((old_changed, new_changed))
}

/// Word emphasis for the diff rows `rows` covers: each deleted line paired,
/// in order, with an added line of the same block (deleted lines then added
/// lines), and the character ranges [`word_changes`] finds in each. Blocks
/// reaching outside `rows` are read whole so the pairs are the same on every
/// page; at most `limit` pairs are compared.
pub fn word_emphasis(
    all: &[DiffRow],
    rows: std::ops::Range<usize>,
    limit: usize,
) -> Vec<(usize, Spans)> {
    let changed = |row: usize| {
        all.get(row)
            .is_some_and(|row| matches!(row.kind, RowKind::Deleted | RowKind::Added))
    };
    let mut start = rows.start.min(all.len());
    while start > 0 && changed(start - 1) {
        start -= 1;
    }
    let mut out = Vec::new();
    let mut pairs = 0;
    let mut at = start;
    while at < all.len() && at < rows.end {
        if !changed(at) {
            at += 1;
            continue;
        }
        let block = at;
        while at < all.len() && all[at].kind == RowKind::Deleted {
            at += 1;
        }
        let deleted = block..at;
        while at < all.len() && all[at].kind == RowKind::Added {
            at += 1;
        }
        let added = deleted.end..at;
        for (old, new) in deleted.clone().zip(added.clone()) {
            if pairs >= limit {
                return out;
            }
            pairs += 1;
            if !rows.contains(&old) && !rows.contains(&new) {
                continue;
            }
            if let Some((old_changed, new_changed)) = word_changes(&all[old].text, &all[new].text) {
                if rows.contains(&old) && !old_changed.is_empty() {
                    out.push((old, old_changed));
                }
                if rows.contains(&new) && !new_changed.is_empty() {
                    out.push((new, new_changed));
                }
            }
        }
        if deleted.is_empty() && added.is_empty() {
            at += 1;
        }
    }
    out.sort_by_key(|(row, _)| *row);
    out
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)] // One changed word is the usual case.
mod tests {
    use super::*;

    #[test]
    fn changed_words_are_found_between_paired_lines() {
        let found = word_changes("let total = price * count;", "let total = price * amount;");
        assert_eq!(found, Some((vec![20..25], vec![20..26])));
        // Character ranges, past wide characters.
        assert_eq!(
            word_changes("naïve café ok", "naïve tea ok"),
            Some((vec![6..10], vec![6..9]))
        );
        assert_eq!(word_changes("same", "same"), Some((vec![], vec![])));
        assert_eq!(
            word_changes("alpha beta", "gamma delta"),
            None,
            "nothing shared"
        );
        let long = "x ".repeat(WORD_LINE_LIMIT);
        assert_eq!(word_changes(&long, "x"), None, "too long to compare");
    }

    #[test]
    fn emphasis_pairs_deleted_and_added_lines_in_order_on_any_page() {
        let row = |kind, text: &str| DiffRow {
            kind,
            old_line: None,
            new_line: None,
            text: text.into(),
            bytes: vec![],
        };
        let rows = vec![
            row(RowKind::Hunk, "@@"),
            row(RowKind::Deleted, "one red"),
            row(RowKind::Deleted, "two red"),
            row(RowKind::Added, "one blue"),
            row(RowKind::Added, "two green"),
            row(RowKind::Added, "three new"),
            row(RowKind::Context, "same"),
        ];
        let all = word_emphasis(&rows, 0..rows.len(), 100);
        assert_eq!(
            all,
            [
                (1, vec![4..7]),
                (2, vec![4..7]),
                (3, vec![4..8]),
                (4, vec![4..9])
            ],
            "pairs in order; the unpaired third added line gets none"
        );
        // A page starting inside the block gives the same pairs.
        assert_eq!(
            word_emphasis(&rows, 3..5, 100),
            [(3, vec![4..8]), (4, vec![4..9])]
        );
        // The limit bounds the work.
        assert_eq!(word_emphasis(&rows, 0..rows.len(), 1).len(), 2);
    }
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
