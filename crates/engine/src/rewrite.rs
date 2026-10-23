//! Shared reorder and squash plans and receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Pick,
    Squash,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pick => "pick",
            Self::Squash => "squash",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub action: Action,
    pub commit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeCommit {
    pub id: String,
    pub subject: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Oldest commit requested for the rewrite.
    pub from: String,
    /// HEAD before the rewrite.
    pub previous: Option<String>,
    /// Private recovery reference for the pre-rewrite tip.
    pub backup: Option<String>,
    pub applied: bool,
    pub in_progress: bool,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub steps: usize,
}
