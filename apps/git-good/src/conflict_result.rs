//! The conflict workspace's editable Result: the working file as it will be
//! staged, with each span remembering where it came from (common text, Ours,
//! Theirs, an unresolved block with its markers, or a hand edit). Choosing a
//! block option rewrites only that block's span; hand edits elsewhere stay.
//! Undo and redo restore text and origins together.

use gg_engine::conflict::{BlockChoice, Segment, block_count, parse_diff3};
use std::ops::Range;

/// Where a span of the Result came from; it picks the line tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Common,
    Ours,
    Theirs,
    /// A block without a choice: its markers and both sides.
    Unresolved,
    /// Typed by hand.
    Edited,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The conflict block this span belongs to.
    pub block: Option<usize>,
    pub origin: Origin,
    pub len: usize,
}

/// The labels written on the markers of an unresolved block, such as
/// `<<<<<<< main` and `>>>>>>> origin/main (0f51a473)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Markers {
    pub ours: String,
    pub theirs: String,
}

impl Default for Markers {
    fn default() -> Self {
        Self {
            ours: "ours".into(),
            theirs: "theirs".into(),
        }
    }
}

type Snapshot = (String, Vec<Span>);

/// Typed edits closer together than this undo as one step.
const COALESCE_SECONDS: f64 = 1.0;

#[derive(Clone, Debug, Default)]
pub struct ResultDoc {
    pub text: String,
    pub spans: Vec<Span>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// When the last typed edit was recorded, to coalesce typing.
    last_typed: Option<f64>,
}

fn with_newline(text: &str) -> String {
    if text.is_empty() || text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

/// The text and spans one block contributes for `choice`.
fn block_parts(
    segment: &Segment,
    choice: Option<BlockChoice>,
    markers: &Markers,
) -> Vec<(Origin, String)> {
    let Segment::Block { ours, theirs, .. } = segment else {
        return Vec::new();
    };
    match choice {
        Some(BlockChoice::Ours) => vec![(Origin::Ours, ours.clone())],
        Some(BlockChoice::Theirs) => vec![(Origin::Theirs, theirs.clone())],
        Some(BlockChoice::OursThenTheirs) => vec![
            (Origin::Ours, with_newline(ours)),
            (Origin::Theirs, theirs.clone()),
        ],
        Some(BlockChoice::TheirsThenOurs) => vec![
            (Origin::Theirs, with_newline(theirs)),
            (Origin::Ours, ours.clone()),
        ],
        None => vec![(
            Origin::Unresolved,
            format!(
                "<<<<<<< {}\n{}=======\n{}>>>>>>> {}\n",
                markers.ours,
                with_newline(ours),
                with_newline(theirs),
                markers.theirs
            ),
        )],
    }
}

/// Spans for `new` after the edit that turned `old` into it: the changed
/// range becomes one hand-edited span inside whatever block held it.
pub fn rebase_spans(old: &str, spans: &[Span], new: &str) -> Vec<Span> {
    if old == new {
        return spans.to_vec();
    }
    let prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let max_suffix = old.len().min(new.len()) - prefix;
    let suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    let (start, end) = (prefix, old.len() - suffix);
    let inserted = new.len() - suffix - prefix;
    // The block that holds the edit: the span containing its start, else
    // the span it ends.
    let mut offset = 0;
    let mut holder = None;
    for span in spans {
        let range = offset..offset + span.len;
        if range.contains(&start) || (holder.is_none() && range.end == start && start == end) {
            holder = Some(span.block);
            if range.contains(&start) {
                break;
            }
        }
        offset += span.len;
    }
    let mut out: Vec<Span> = Vec::new();
    // A block emptied by hand keeps an empty edited span as its place.
    let mut push = |span: Span| {
        if span.len == 0 && !(span.origin == Origin::Edited && span.block.is_some()) {
            return;
        }
        if let Some(last) = out.last_mut()
            && last.block == span.block
            && last.origin == span.origin
        {
            last.len += span.len;
            return;
        }
        out.push(span);
    };
    let mut offset = 0;
    let mut placed = false;
    for span in spans {
        let (s, e) = (offset, offset + span.len);
        offset = e;
        // Before the edit.
        if s < start {
            push(Span {
                len: e.min(start) - s,
                ..span.clone()
            });
        }
        if !placed && e >= start {
            push(Span {
                block: holder.flatten(),
                origin: Origin::Edited,
                len: inserted,
            });
            placed = true;
        }
        // After the edit.
        if e > end {
            push(Span {
                len: e - s.max(end),
                ..span.clone()
            });
        }
    }
    if !placed {
        push(Span {
            block: holder.flatten(),
            origin: Origin::Edited,
            len: inserted,
        });
    }
    out
}

impl ResultDoc {
    /// The file with every block unresolved (or chosen, per `choices`).
    pub fn new(segments: &[Segment], choices: &[Option<BlockChoice>], markers: &Markers) -> Self {
        let mut doc = Self::default();
        let mut block = 0;
        for segment in segments {
            match segment {
                Segment::Common(text) => {
                    doc.text.push_str(text);
                    doc.spans.push(Span {
                        block: None,
                        origin: Origin::Common,
                        len: text.len(),
                    });
                }
                Segment::Block { .. } => {
                    let choice = choices.get(block).copied().flatten();
                    for (origin, text) in block_parts(segment, choice, markers) {
                        doc.text.push_str(&text);
                        doc.spans.push(Span {
                            block: Some(block),
                            origin,
                            len: text.len(),
                        });
                    }
                    block += 1;
                }
            }
        }
        doc.spans.retain(|span| span.len > 0);
        doc
    }

    /// The byte range block `index` covers; empty where it was emptied.
    pub fn block_range(&self, index: usize) -> Option<Range<usize>> {
        let mut offset = 0;
        let mut found: Option<Range<usize>> = None;
        for span in &self.spans {
            if span.block == Some(index) {
                found = Some(match found {
                    Some(range) => range.start..offset + span.len,
                    None => offset..offset + span.len,
                });
            }
            offset += span.len;
        }
        found
    }

    /// Block `index` was typed into since its option was chosen.
    pub fn block_edited(&self, index: usize) -> bool {
        self.spans
            .iter()
            .any(|span| span.block == Some(index) && span.origin == Origin::Edited)
    }

    fn snapshot(&mut self) {
        self.undo.push((self.text.clone(), self.spans.clone()));
        self.redo.clear();
    }

    /// Rewrite only block `index` for `choice`, where its span is (a block
    /// emptied by hand keeps an empty span in place).
    pub fn set_block(
        &mut self,
        index: usize,
        segment: &Segment,
        choice: Option<BlockChoice>,
        markers: &Markers,
    ) {
        self.snapshot();
        self.last_typed = None;
        let parts = block_parts(segment, choice, markers);
        let range = self.block_range(index);
        let mut spans = Vec::new();
        let mut text = String::new();
        let mut offset = 0;
        let mut placed = false;
        let place = |text: &mut String, spans: &mut Vec<Span>| {
            for (origin, part) in &parts {
                text.push_str(part);
                spans.push(Span {
                    block: Some(index),
                    origin: *origin,
                    len: part.len(),
                });
            }
        };
        for span in &self.spans {
            let piece = &self.text[offset..offset + span.len];
            offset += span.len;
            if span.block == Some(index) {
                if !placed {
                    place(&mut text, &mut spans);
                    placed = true;
                }
                continue;
            }
            // An emptied block goes back after the blocks before it.
            if !placed && range.is_none() && span.block.is_some_and(|block| block > index) {
                place(&mut text, &mut spans);
                placed = true;
            }
            text.push_str(piece);
            spans.push(span.clone());
        }
        if !placed {
            place(&mut text, &mut spans);
        }
        spans.retain(|span| span.len > 0 || span.origin == Origin::Edited);
        self.text = text;
        self.spans = spans;
    }

    /// Take the editor's text after the user typed; `now` coalesces typing
    /// into one undo step.
    pub fn typed(&mut self, new: String, now: f64) {
        if new == self.text {
            return;
        }
        if self
            .last_typed
            .is_none_or(|last| now - last > COALESCE_SECONDS)
        {
            self.snapshot();
        }
        self.last_typed = Some(now);
        self.spans = rebase_spans(&self.text, &self.spans, &new);
        self.text = new;
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        let current = (
            std::mem::take(&mut self.text),
            std::mem::take(&mut self.spans),
        );
        self.redo.push(current);
        (self.text, self.spans) = previous;
        self.last_typed = None;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let current = (
            std::mem::take(&mut self.text),
            std::mem::take(&mut self.spans),
        );
        self.undo.push(current);
        (self.text, self.spans) = next;
        self.last_typed = None;
        true
    }

    /// Conflict blocks still marked in the text, whatever made them.
    pub fn left(&self) -> usize {
        parse_diff3(&self.text)
            .map(|segments| block_count(&segments))
            .unwrap_or(1)
    }
}

/// Each line's origin for the tint, from `spans` over `text`: a line touched
/// by a hand edit is neutral, otherwise it takes the origin it starts in.
pub fn line_origins(text: &str, spans: &[Span]) -> Vec<Origin> {
    let mut bounds = Vec::new();
    let mut offset = 0;
    for span in spans {
        bounds.push((offset..offset + span.len, span.origin));
        offset += span.len;
    }
    let origin_at = |at: usize| {
        bounds
            .iter()
            .find(|(range, _)| range.contains(&at))
            .map(|(_, origin)| *origin)
            .unwrap_or(Origin::Edited)
    };
    let mut lines = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let end = start + line.len();
        let edited = bounds.iter().any(|(range, origin)| {
            *origin == Origin::Edited && range.start < end.max(start + 1) && range.end > start
        });
        lines.push(if edited {
            Origin::Edited
        } else {
            origin_at(start)
        });
        start = end;
    }
    if text.is_empty() || text.ends_with('\n') {
        // The empty last line the editor shows.
        lines.push(Origin::Common);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segments() -> Vec<Segment> {
        vec![
            Segment::Common("top\n".into()),
            Segment::Block {
                ours: "ours 1\n".into(),
                base: "base 1\n".into(),
                theirs: "theirs 1\n".into(),
            },
            Segment::Common("middle\n".into()),
            Segment::Block {
                ours: "ours 2\n".into(),
                base: String::new(),
                theirs: "theirs 2\n".into(),
            },
            Segment::Common("end\n".into()),
        ]
    }

    fn markers() -> Markers {
        Markers {
            ours: "main".into(),
            theirs: "origin/main (0f51a473)".into(),
        }
    }

    fn doc() -> ResultDoc {
        ResultDoc::new(&segments(), &[None, None], &markers())
    }

    #[test]
    fn unresolved_blocks_carry_named_markers() {
        let doc = doc();
        assert_eq!(
            doc.text,
            "top\n<<<<<<< main\nours 1\n=======\ntheirs 1\n>>>>>>> origin/main (0f51a473)\nmiddle\n<<<<<<< main\nours 2\n=======\ntheirs 2\n>>>>>>> origin/main (0f51a473)\nend\n"
        );
        assert_eq!(doc.left(), 2);
    }

    #[test]
    fn an_option_replaces_only_its_block() {
        let mut doc = doc();
        let segments = segments();
        doc.set_block(1, &segments[3], Some(BlockChoice::Theirs), &markers());
        assert_eq!(doc.left(), 1);
        // A hand edit in the common text and in block 2 …
        let edited = doc.text.replace("top\n", "top edited\n");
        doc.typed(edited, 0.0);
        let edited = doc.text.replace("theirs 2\n", "theirs 2 by hand\n");
        doc.typed(edited, 5.0);
        assert!(doc.block_edited(1));
        assert!(!doc.block_edited(0));
        // … survive choosing for block 1.
        doc.set_block(
            0,
            &segments[1],
            Some(BlockChoice::OursThenTheirs),
            &markers(),
        );
        assert_eq!(
            doc.text,
            "top edited\nours 1\ntheirs 1\nmiddle\ntheirs 2 by hand\nend\n"
        );
        assert_eq!(doc.left(), 0);
        assert_eq!(doc.block_range(0), Some(11..27));
    }

    #[test]
    fn origins_tint_each_line_and_stay_while_typing() {
        let mut doc = doc();
        let segments = segments();
        doc.set_block(
            0,
            &segments[1],
            Some(BlockChoice::OursThenTheirs),
            &markers(),
        );
        use Origin::*;
        assert_eq!(
            line_origins(&doc.text, &doc.spans),
            [
                Common, Ours, Theirs, Common, Unresolved, Unresolved, Unresolved, Unresolved,
                Unresolved, Common, Common
            ]
        );
        // Typing in the middle line makes only that line neutral.
        let edited = doc.text.replace("middle\n", "midXdle\n");
        doc.typed(edited, 0.0);
        let origins = line_origins(&doc.text, &doc.spans);
        assert_eq!(origins[1..4], [Ours, Theirs, Edited]);
        assert_eq!(origins[4], Unresolved);
        // The spans still cover the text exactly.
        assert_eq!(
            doc.spans.iter().map(|span| span.len).sum::<usize>(),
            doc.text.len()
        );
    }

    #[test]
    fn undo_restores_text_and_origins_and_typing_coalesces() {
        let mut doc = doc();
        let original = doc.clone();
        let segments = segments();
        doc.set_block(0, &segments[1], Some(BlockChoice::Ours), &markers());
        doc.typed(doc.text.replace("end", "enX"), 10.0);
        doc.typed(doc.text.replace("enX", "enXY"), 10.4);
        assert!(doc.undo(), "both keystrokes undo together");
        assert!(doc.text.contains("ours 1\nmiddle"));
        assert!(doc.text.ends_with("end\n"));
        assert!(doc.undo());
        assert_eq!(doc.text, original.text);
        assert_eq!(doc.spans, original.spans);
        assert!(!doc.undo());
        assert!(doc.redo());
        assert!(doc.text.contains("ours 1\nmiddle"));
    }

    #[test]
    fn an_emptied_block_comes_back_in_place() {
        let mut doc = doc();
        let segments = segments();
        doc.set_block(0, &segments[1], Some(BlockChoice::Ours), &markers());
        doc.typed(doc.text.replacen("ours 1\n", "", 1), 0.0);
        assert_eq!(doc.block_range(0), Some(4..4));
        assert!(doc.block_edited(0));
        doc.set_block(0, &segments[1], Some(BlockChoice::Theirs), &markers());
        assert!(doc.text.starts_with("top\ntheirs 1\nmiddle\n"));
    }
}
