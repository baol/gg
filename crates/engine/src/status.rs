//! Read-only working-tree observations shared by CLI and native views.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusIdentity {
    pub path: PathBuf,
    pub head: Option<String>,
    pub index: String,
    /// Tree compared with the index for staged entries. `None` means the HEAD tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_tree: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Staged,
    Unstaged,
    Untracked,
    Conflict,
}
impl Area {
    pub fn label(self) -> &'static str {
        match self {
            Self::Staged => "Staged",
            Self::Unstaged => "Unstaged",
            Self::Untracked => "Untracked",
            Self::Conflict => "Conflict",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Added,
    Modified,
    Deleted,
    TypeChanged,
    IntentToAdd,
    Conflicted,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Modified => "Modified",
            Self::Deleted => "Deleted",
            Self::TypeChanged => "Type changed",
            Self::IntentToAdd => "Intent to add",
            Self::Conflicted => "Unmerged",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEntry {
    pub area: Area,
    pub kind: Kind,
    pub path: String,
    pub path_bytes: Vec<u8>,
}
impl StatusEntry {
    pub fn new(area: Area, kind: Kind, bytes: &[u8]) -> Self {
        Self {
            area,
            kind,
            path: crate::review::display_path(bytes),
            path_bytes: bytes.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingRequest {
    pub repository: StatusIdentity,
    pub entry: StatusEntry,
}
