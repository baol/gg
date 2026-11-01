//! Shared unmerged-path content for conflict resolution.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Base,
    Ours,
    Theirs,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Content {
    pub path: String,
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub binary: bool,
    pub truncated: bool,
}

impl Content {
    /// The side that deleted the file in a delete/modify conflict: exactly
    /// one of Ours and Theirs is absent.
    pub fn deleted_side(&self) -> Option<Side> {
        match (&self.ours, &self.theirs) {
            (None, Some(_)) => Some(Side::Ours),
            (Some(_), None) => Some(Side::Theirs),
            _ => None,
        }
    }
}

/// Who Ours and Theirs are for the operation in progress, such as `main` and
/// `feature/login` for a merge, or `onto main` and `1a2b3c4d Add login` for a
/// rebase. Either is empty when the repository records no identity for it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideNames {
    pub ours: String,
    pub theirs: String,
}

/// Exact conflict inputs captured when a resolution preview is opened.
///
/// Stage object IDs and the working bytes are validated again at publication,
/// so a preview can never publish content from a newer conflict state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    /// Exact working bytes when the file is a bounded regular file.
    pub working: Option<Vec<u8>>,
    /// FNV-1a hash of a larger regular working file.
    pub working_hash: Option<u64>,
    /// Why the working preview is unavailable, when it is.
    pub working_note: Option<String>,
}

/// The result chosen for preview and, after explicit confirmation, publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Base,
    Ours,
    Theirs,
    Working,
    /// Keep the deletion of a delete/modify conflict: remove the path.
    Deletion,
}

/// One piece of a three-way merge: text both sides agree on, or a conflict
/// block with each side's lines (each line keeps its newline).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Common(String),
    Block {
        ours: String,
        base: String,
        theirs: String,
    },
}

/// How one conflict block is resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockChoice {
    Ours,
    Theirs,
    OursThenTheirs,
    TheirsThenOurs,
}

impl BlockChoice {
    pub const ALL: [Self; 4] = [
        Self::Ours,
        Self::Theirs,
        Self::OursThenTheirs,
        Self::TheirsThenOurs,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ours => "Ours",
            Self::Theirs => "Theirs",
            Self::OursThenTheirs => "Ours + theirs",
            Self::TheirsThenOurs => "Theirs + ours",
        }
    }
}

const MARKER: usize = 7;

fn marker(line: &str, symbol: char) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    line.len() >= MARKER
        && line.chars().take(MARKER).all(|c| c == symbol)
        && line[MARKER..].chars().next().is_none_or(|c| c == ' ')
}

/// Split `diff3`-style merge output into common text and conflict blocks.
pub fn parse_diff3(text: &str) -> Result<Vec<Segment>, String> {
    enum State {
        Common,
        Ours,
        Base,
        Theirs,
    }
    let mut segments = Vec::new();
    let mut common = String::new();
    let (mut ours, mut base, mut theirs) = (String::new(), String::new(), String::new());
    let mut state = State::Common;
    for line in text.split_inclusive('\n') {
        match state {
            State::Common if marker(line, '<') => {
                if !common.is_empty() {
                    segments.push(Segment::Common(std::mem::take(&mut common)));
                }
                state = State::Ours;
            }
            State::Common => common.push_str(line),
            State::Ours if marker(line, '|') => state = State::Base,
            State::Ours if marker(line, '=') => state = State::Theirs,
            State::Ours => ours.push_str(line),
            State::Base if marker(line, '=') => state = State::Theirs,
            State::Base => base.push_str(line),
            State::Theirs if marker(line, '>') => {
                segments.push(Segment::Block {
                    ours: std::mem::take(&mut ours),
                    base: std::mem::take(&mut base),
                    theirs: std::mem::take(&mut theirs),
                });
                state = State::Common;
            }
            State::Theirs => theirs.push_str(line),
        }
    }
    if !matches!(state, State::Common) {
        return Err("The merge output has an unterminated conflict block.".into());
    }
    if !common.is_empty() {
        segments.push(Segment::Common(common));
    }
    Ok(segments)
}

pub fn block_count(segments: &[Segment]) -> usize {
    segments
        .iter()
        .filter(|segment| matches!(segment, Segment::Block { .. }))
        .count()
}

/// The resolved text; blocks without a choice keep their conflict markers.
pub fn compose(segments: &[Segment], choices: &[Option<BlockChoice>]) -> String {
    fn line(text: &mut String, part: &str) {
        text.push_str(part);
        if !part.is_empty() && !part.ends_with('\n') {
            text.push('\n');
        }
    }
    let mut text = String::new();
    let mut block = 0;
    for segment in segments {
        match segment {
            Segment::Common(common) => text.push_str(common),
            Segment::Block { ours, base, theirs } => {
                match choices.get(block).copied().flatten() {
                    Some(BlockChoice::Ours) => text.push_str(ours),
                    Some(BlockChoice::Theirs) => text.push_str(theirs),
                    Some(BlockChoice::OursThenTheirs) => {
                        line(&mut text, ours);
                        text.push_str(theirs);
                    }
                    Some(BlockChoice::TheirsThenOurs) => {
                        line(&mut text, theirs);
                        text.push_str(ours);
                    }
                    None => {
                        text.push_str("<<<<<<< ours\n");
                        line(&mut text, ours);
                        text.push_str("||||||| base\n");
                        line(&mut text, base);
                        text.push_str("=======\n");
                        line(&mut text, theirs);
                        text.push_str(">>>>>>> theirs\n");
                    }
                }
                block += 1;
            }
        }
    }
    text
}

/// One side of every segment: common text plus Ours (or Theirs) of each block.
fn side_text(segments: &[Segment], choice: BlockChoice) -> String {
    compose(segments, &vec![Some(choice); block_count(segments)])
}

/// The working file no longer holds what the merge wrote at conflict time:
/// it has no conflict markers left, or its Ours or Theirs text differs from
/// the fresh merge of the stages in `segments`. Marker labels, CRLF line
/// endings, and the conflict style (merge, diff3, zdiff3) are not edits.
pub fn edited_outside(segments: &[Segment], working: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(working) else {
        return true;
    };
    let text = text.replace("\r\n", "\n");
    let Ok(found) = parse_diff3(&text) else {
        return true;
    };
    block_count(&found) == 0
        || [BlockChoice::Ours, BlockChoice::Theirs]
            .into_iter()
            .any(|choice| side_text(&found, choice) != side_text(segments, choice))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MERGED: &str = "top\n<<<<<<< ours\nours 1\n||||||| base\nbase 1\n=======\ntheirs 1\n>>>>>>> theirs\nmiddle\n<<<<<<< ours\nours 2\n||||||| base\n=======\n>>>>>>> theirs\nend\n";

    #[test]
    fn parses_common_text_and_blocks() -> Result<(), String> {
        let segments = parse_diff3(MERGED)?;
        assert_eq!(block_count(&segments), 2);
        assert_eq!(
            segments,
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
                    theirs: String::new(),
                },
                Segment::Common("end\n".into()),
            ]
        );
        assert!(parse_diff3("<<<<<<< ours\nx\n").is_err());
        Ok(())
    }

    #[test]
    fn composes_choices_and_keeps_markers_for_open_blocks() -> Result<(), String> {
        let segments = parse_diff3(MERGED)?;
        assert_eq!(
            compose(
                &segments,
                &[Some(BlockChoice::TheirsThenOurs), Some(BlockChoice::Ours)]
            ),
            "top\ntheirs 1\nours 1\nmiddle\nours 2\nend\n"
        );
        assert_eq!(
            compose(
                &segments,
                &[Some(BlockChoice::OursThenTheirs), Some(BlockChoice::Theirs)]
            ),
            "top\nours 1\ntheirs 1\nmiddle\nend\n"
        );
        let open = compose(&segments, &[Some(BlockChoice::Theirs), None]);
        assert_eq!(
            open,
            "top\ntheirs 1\nmiddle\n<<<<<<< ours\nours 2\n||||||| base\n=======\n>>>>>>> theirs\nend\n"
        );
        // Text that only looks like a marker is common text.
        assert_eq!(
            parse_diff3("<<<<<<<<< not a marker\n")?,
            vec![Segment::Common("<<<<<<<<< not a marker\n".into())]
        );
        Ok(())
    }

    #[test]
    fn working_files_count_as_edited_only_when_a_side_changed() -> Result<(), String> {
        let segments = parse_diff3(MERGED)?;
        // The merge style: other labels, no base section, CRLF endings.
        let merge = "top\r\n<<<<<<< HEAD\r\nours 1\r\n=======\r\ntheirs 1\r\n>>>>>>> feature\r\nmiddle\r\n<<<<<<< HEAD\r\nours 2\r\n=======\r\n>>>>>>> feature\r\nend\r\n";
        assert!(!edited_outside(&segments, merge.as_bytes()));
        assert!(!edited_outside(&segments, MERGED.as_bytes()));
        let changed = MERGED.replace("theirs 1", "theirs 1 by hand");
        assert!(edited_outside(&segments, changed.as_bytes()));
        assert!(edited_outside(&segments, b"top\nmiddle\nend\n"));
        assert!(edited_outside(&segments, b"<<<<<<< ours\nx\n"));
        assert!(edited_outside(&segments, &[0xff, 0xfe]));
        Ok(())
    }

    #[test]
    fn a_deleted_side_is_one_absent_side() {
        let content = |ours: Option<&str>, theirs: Option<&str>| Content {
            path: "a".into(),
            base: Some("base".into()),
            ours: ours.map(str::to_string),
            theirs: theirs.map(str::to_string),
            binary: false,
            truncated: false,
        };
        assert_eq!(content(None, Some("t")).deleted_side(), Some(Side::Ours));
        assert_eq!(content(Some("o"), None).deleted_side(), Some(Side::Theirs));
        assert_eq!(content(Some("o"), Some("t")).deleted_side(), None);
    }
}
