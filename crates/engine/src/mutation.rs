//! Shared history-mutation states and receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    CherryPick,
    Revert,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CherryPick => "cherry-pick",
            Self::Revert => "revert",
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
