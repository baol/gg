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
