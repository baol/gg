//! Shared stash observations and mutation receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashEntry {
    /// Position in the stash reflog, where zero is the newest entry.
    pub index: u32,
    /// Full reference such as `stash@{0}`.
    pub reference: String,
    /// Commit recorded by the stash reflog.
    pub commit: String,
    /// Reflog subject, such as `WIP on main: abc123 subject`.
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashList {
    pub entries: Vec<StashEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashReceipt {
    /// Reference created, applied, popped, or dropped.
    pub reference: Option<String>,
    /// Message recorded by a push, or the listed entry message.
    pub message: Option<String>,
    /// A new stash entry was recorded.
    pub created: bool,
    /// Working changes were restored.
    pub applied: bool,
    /// The stash entry was removed after a successful pop or drop.
    pub dropped: bool,
    /// Paths left with unresolved conflicts by an apply or pop.
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}
