//! Shared merge observations and receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    UpToDate,
    FastForward,
    MergeCommit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Reference requested by the user.
    pub reference: String,
    /// Resolved commit when the merge started.
    pub commit: Option<String>,
    /// How HEAD advanced when the merge completed.
    pub kind: Option<Kind>,
    pub applied: bool,
    pub in_progress: bool,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}
