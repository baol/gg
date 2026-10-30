//! Shared history-mutation states and receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    CherryPick,
    Revert,
    Merge,
    Rebase,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CherryPick => "cherry-pick",
            Self::Revert => "revert",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationState {
    pub operation: Operation,
    /// Commit being applied or reverted when Git records one.
    pub commit: Option<String>,
    /// Unmerged paths still needing resolution.
    pub conflicts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationReceipt {
    pub operation: Operation,
    /// Commit passed to the operation.
    pub commit: String,
    /// The commit reached HEAD.
    pub applied: bool,
    /// Git left an operation that can continue, skip, or abort.
    pub in_progress: bool,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

/// One commit-bearing line of an interactive rebase.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseStep {
    /// `pick`, `reword`, `edit`, `squash`, `fixup`, or `drop`; `commit` for a
    /// commit made at a stop.
    pub action: String,
    pub commit: String,
    pub subject: String,
}

/// Why an interactive rebase is waiting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RebaseStop {
    /// Unmerged paths need a resolution.
    Conflict,
    /// gg reopened a commit: its changes were staged on `base`.
    Reopened {
        /// The commit the changes were staged on; `None` for a root commit,
        /// which stays at HEAD to be amended.
        base: Option<String>,
        commit: String,
        subject: String,
        /// Full original message, for the commit box.
        message: String,
        /// Commits made since the stop, oldest first.
        made: Vec<RebaseStep>,
    },
    /// Git stopped for another reason, such as a failed `exec`.
    Other,
}

/// Where an interactive rebase stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseProgress {
    /// Branch being rebased, without `refs/heads/`.
    pub branch: Option<String>,
    /// Steps already applied; the last one is the current step.
    pub done: Vec<RebaseStep>,
    /// Steps still to run.
    pub todo: Vec<RebaseStep>,
    pub stop: RebaseStop,
    /// Staged or unstaged changes to tracked files.
    pub tracked_changes: usize,
}

impl RebaseProgress {
    /// One-based current step and total, such as `(3, 6)`.
    pub fn position(&self) -> (usize, usize) {
        (self.done.len(), self.done.len() + self.todo.len())
    }
}
