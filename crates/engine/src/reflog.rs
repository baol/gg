//! Shared reflog observations.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Index selector such as `HEAD@{0}`.
    pub selector: String,
    /// Commit recorded by the reflog entry.
    pub id: String,
    /// Unix timestamp recorded by the reflog.
    pub time: i64,
    /// Operation before the first colon, or the whole subject when there is none.
    pub action: String,
    /// Detail after the first colon.
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub reference: String,
    pub entries: Vec<Entry>,
}
