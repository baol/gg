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
}
