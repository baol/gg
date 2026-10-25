//! Version-cursor file history: content states, edges, and bounded navigation.
use crate::review::{ChangeKind, FileSide, display_path};
use serde::{Deserialize, Serialize};

/// Versions loaded ahead of the cursor so a step is usually immediate.
pub const PRELOAD: usize = 24;
/// Hard bound for one worker session. Reaching it is an explicit boundary.
pub const MAX_VERSIONS: usize = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryKind {
    Added,
    Deleted,
    Modified,
    Renamed,
    TypeChanged,
}

impl HistoryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Deleted => "Deleted",
            Self::Modified => "Modified",
            Self::Renamed => "Renamed",
            Self::TypeChanged => "Type changed",
        }
    }

    /// The comparison kind used by the shared diff reader.
    pub fn change_kind(self) -> ChangeKind {
        match self {
            Self::Added => ChangeKind::Added,
            Self::Deleted => ChangeKind::Deleted,
            Self::TypeChanged => ChangeKind::TypeChanged,
            Self::Modified | Self::Renamed => ChangeKind::Modified,
        }
    }
}

/// One content state in a file's history.
///
/// `commit` names the commit that introduced the state along the followed
/// path. Commits that merely carry the same state are skipped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileVersion {
    pub path_bytes: Vec<u8>,
    pub path: String,
    /// `None` means the file was absent at this version.
    pub side: Option<FileSide>,
    pub commit: String,
    pub subject: String,
    pub author: String,
    /// Author time in Unix seconds.
    pub time: i64,
    /// Edge from the predecessor version to this version. `None` when the
    /// predecessor is not unique.
    pub kind: Option<HistoryKind>,
    pub old_path_bytes: Option<Vec<u8>>,
    pub old_path: Option<String>,
    pub previous: Predecessor,
}

impl FileVersion {
    pub fn short_commit(&self) -> String {
        self.commit.chars().take(8).collect()
    }

    pub fn present(&self) -> bool {
        self.side.is_some()
    }

    pub fn path_changed(&self) -> bool {
        self.old_path_bytes.is_some()
    }
}

/// State of the edge to the next older version.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Predecessor {
    /// A unique predecessor exists but has not been loaded yet.
    More,
    /// The next element in the version list is the predecessor.
    Linked,
    /// There is no earlier version.
    Oldest,
    /// The search stopped before proving whether an earlier version exists.
    Limited { reason: String },
    /// Several predecessor states exist; the reader must choose one.
    Candidates { choices: Vec<VersionCandidate> },
}

impl Predecessor {
    pub fn linked(&self) -> bool {
        matches!(self, Self::Linked)
    }

    pub fn ambiguous(&self) -> bool {
        matches!(self, Self::Candidates { .. })
    }
}

/// A predecessor state offered by the ambiguity chooser.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionCandidate {
    pub commit: String,
    pub path_bytes: Vec<u8>,
    pub path: String,
    pub side: Option<FileSide>,
    pub subject: String,
    pub author: String,
    pub time: i64,
    /// Edge from this candidate to the version that offered the choice.
    pub kind: HistoryKind,
    pub old_path_bytes: Option<Vec<u8>>,
    pub old_path: Option<String>,
}

impl VersionCandidate {
    pub fn short_commit(&self) -> String {
        self.commit.chars().take(8).collect()
    }
}

/// Newest-to-oldest content versions and the state of the oldest edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timeline {
    pub versions: Vec<FileVersion>,
    /// True when no further version can be built without a new request.
    pub complete: bool,
    pub note: Option<String>,
}

impl Timeline {
    pub fn new(versions: Vec<FileVersion>, complete: bool, note: Option<String>) -> Self {
        Self {
            versions,
            complete,
            note,
        }
    }

    pub fn current(&self, index: usize) -> Option<&FileVersion> {
        self.versions.get(index)
    }

    pub fn predecessor(&self, index: usize) -> Option<&FileVersion> {
        match self.versions.get(index)?.previous {
            Predecessor::Linked => self.versions.get(index + 1),
            _ => None,
        }
    }

    /// One-based version position with `+` while older versions remain.
    pub fn position(&self, index: usize) -> String {
        let more = if self.complete { "" } else { "+" };
        format!("Version {} of {}{}", index + 1, self.versions.len(), more)
    }

    /// Why the cursor cannot move to an older version, or `None` if it can.
    ///
    /// `loading` names an in-flight request for more versions, so a `More`
    /// boundary reports why it is temporarily disabled rather than inviting
    /// another request.
    pub fn before_reason(&self, index: usize, loading: bool) -> Option<String> {
        let version = self.versions.get(index)?;
        if index + 1 < self.versions.len() {
            return None;
        }
        Some(match &version.previous {
            Predecessor::More => {
                if !loading {
                    return None;
                }
                "Loading earlier versions…".into()
            }
            Predecessor::Oldest if self.versions.len() == 1 => "Only version".into(),
            Predecessor::Oldest => "Oldest version".into(),
            Predecessor::Limited { reason } => reason.clone(),
            Predecessor::Candidates { choices } => {
                format!("Choose one of {} predecessors", choices.len())
            }
            Predecessor::Linked => "Loading earlier versions…".into(),
        })
    }

    /// Why the cursor cannot move to a newer version, or `None` if it can.
    pub fn after_reason(&self, index: usize) -> Option<&'static str> {
        (index == 0).then_some("Newest version")
    }

    pub fn choices(&self, index: usize) -> Option<&[VersionCandidate]> {
        match &self.versions.get(index)?.previous {
            Predecessor::Candidates { choices } => Some(choices),
            _ => None,
        }
    }
}

/// Classify the edge between two content states.
pub fn edge_kind(
    old: Option<&FileSide>,
    new: Option<&FileSide>,
    old_path: &[u8],
    new_path: &[u8],
) -> HistoryKind {
    match (old, new) {
        (None, Some(_)) => HistoryKind::Added,
        (Some(_), None) => HistoryKind::Deleted,
        (Some(old), Some(new)) if old.mode & 0o170000 != new.mode & 0o170000 => {
            HistoryKind::TypeChanged
        }
        (Some(_), Some(_)) if old_path != new_path => HistoryKind::Renamed,
        _ => HistoryKind::Modified,
    }
}

/// Display text for a version path, preserving exact bytes.
pub fn display_version_path(path_bytes: &[u8]) -> String {
    display_path(path_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(id: &str) -> FileSide {
        FileSide {
            id: id.into(),
            mode: 0o100644,
        }
    }

    fn version(previous: Predecessor) -> FileVersion {
        FileVersion {
            path_bytes: b"src/lib.rs".to_vec(),
            path: "src/lib.rs".into(),
            side: Some(side("a")),
            commit: "a".repeat(40),
            subject: "Change".into(),
            author: "Ada".into(),
            time: 1,
            kind: Some(HistoryKind::Modified),
            old_path_bytes: None,
            old_path: None,
            previous,
        }
    }

    #[test]
    fn edge_kinds_cover_add_delete_rename_type_and_content() {
        let blob = side("a");
        assert_eq!(edge_kind(None, Some(&blob), b"a", b"a"), HistoryKind::Added);
        assert_eq!(
            edge_kind(Some(&blob), None, b"a", b"a"),
            HistoryKind::Deleted
        );
        assert_eq!(
            edge_kind(Some(&blob), Some(&blob), b"old", b"new"),
            HistoryKind::Renamed
        );
        assert_eq!(
            edge_kind(Some(&blob), Some(&blob), b"a", b"a"),
            HistoryKind::Modified
        );
        let link = FileSide {
            id: "b".into(),
            mode: 0o120000,
        };
        assert_eq!(
            edge_kind(Some(&blob), Some(&link), b"a", b"a"),
            HistoryKind::TypeChanged
        );
    }

    #[test]
    fn boundaries_distinguish_oldest_only_loading_and_choices() {
        let only = Timeline::new(vec![version(Predecessor::Oldest)], true, None);
        assert_eq!(
            only.before_reason(0, false).as_deref(),
            Some("Only version")
        );
        assert_eq!(only.after_reason(0), Some("Newest version"));

        let oldest = Timeline::new(
            vec![version(Predecessor::Linked), version(Predecessor::Oldest)],
            true,
            None,
        );
        assert_eq!(oldest.before_reason(0, false), None);
        assert_eq!(
            oldest.before_reason(1, false).as_deref(),
            Some("Oldest version")
        );
        assert_eq!(oldest.after_reason(1), None);
        assert_eq!(oldest.after_reason(0), Some("Newest version"));

        let more = Timeline::new(vec![version(Predecessor::More)], false, None);
        assert_eq!(more.before_reason(0, false), None);
        assert_eq!(
            more.before_reason(0, true).as_deref(),
            Some("Loading earlier versions…")
        );
        assert!(!more.complete);

        let limited = Timeline::new(
            vec![version(Predecessor::Limited {
                reason: "History limit reached".into(),
            })],
            true,
            None,
        );
        assert_eq!(
            limited.before_reason(0, false).as_deref(),
            Some("History limit reached")
        );

        let choices = vec![
            VersionCandidate {
                commit: "b".repeat(40),
                path_bytes: b"src/lib.rs".to_vec(),
                path: "src/lib.rs".into(),
                side: Some(side("b")),
                subject: "Older".into(),
                author: "Ada".into(),
                time: 1,
                kind: HistoryKind::Modified,
                old_path_bytes: None,
                old_path: None,
            },
            VersionCandidate {
                commit: "c".repeat(40),
                path_bytes: b"src/lib.rs".to_vec(),
                path: "src/lib.rs".into(),
                side: Some(side("c")),
                subject: "Other".into(),
                author: "Bob".into(),
                time: 2,
                kind: HistoryKind::Modified,
                old_path_bytes: None,
                old_path: None,
            },
        ];
        let ambiguous = Timeline::new(
            vec![version(Predecessor::Candidates { choices })],
            true,
            None,
        );
        assert_eq!(
            ambiguous.before_reason(0, false).as_deref(),
            Some("Choose one of 2 predecessors")
        );
        assert_eq!(ambiguous.choices(0).map(|c| c.len()), Some(2));
    }

    #[test]
    fn position_marks_unloaded_older_versions() {
        let complete = Timeline::new(vec![version(Predecessor::Oldest)], true, None);
        assert_eq!(complete.position(0), "Version 1 of 1");
        let more = Timeline::new(vec![version(Predecessor::More)], false, None);
        assert_eq!(more.position(0), "Version 1 of 1+");
    }

    #[test]
    fn predecessor_only_links_the_adjacent_older_version() {
        let timeline = Timeline::new(
            vec![version(Predecessor::Linked), version(Predecessor::Oldest)],
            true,
            None,
        );
        assert!(timeline.predecessor(0).is_some());
        assert!(timeline.predecessor(1).is_none());
    }
}
