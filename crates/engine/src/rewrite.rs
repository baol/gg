//! Shared interactive-rebase plans and receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Keep the commit as it is.
    Pick,
    /// Keep the commit with the step's new message.
    Reword,
    /// Keep the commit and stop after it so it can be amended or split.
    Edit,
    /// Combine into the kept commit before it, joining both messages.
    Squash,
    /// Combine into the kept commit before it, keeping only that message.
    Fixup,
    /// Leave the commit out.
    Drop,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pick => "pick",
            Self::Reword => "reword",
            Self::Edit => "edit",
            Self::Squash => "squash",
            Self::Fixup => "fixup",
            Self::Drop => "drop",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub action: Action,
    pub commit: String,
    /// The new full message of a [`Action::Reword`] step; `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Step {
    /// A step without a new message.
    pub fn new(action: Action, commit: impl Into<String>) -> Self {
        Self {
            action,
            commit: commit.into(),
            message: None,
        }
    }

    /// A reword step carrying the commit's new message.
    pub fn reword(commit: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            action: Action::Reword,
            commit: commit.into(),
            message: Some(message.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeCommit {
    pub id: String,
    pub subject: String,
    #[serde(default)]
    pub author: String,
    /// Relative author date, such as `2 days ago`.
    #[serde(default)]
    pub date: String,
    /// Full message, without trailing whitespace.
    #[serde(default)]
    pub message: String,
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
